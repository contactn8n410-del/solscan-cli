use serde_json::Value;

/// Quick smart contract audit — detects dangerous patterns via account analysis
pub struct ContractAudit {
    client: reqwest::Client,
    rpc_url: String,
}

#[derive(Debug)]
pub struct AuditResult {
    pub program_id: String,
    pub is_executable: bool,
    pub is_upgradeable: bool,
    pub owner: String,
    pub data_size: usize,
    pub warnings: Vec<String>,
    pub risk_score: u8, // 0-100
}

impl ContractAudit {
    pub fn new(rpc_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            rpc_url,
        }
    }

    pub async fn audit(&self, program_id: &str) -> Result<AuditResult, Box<dyn std::error::Error>> {
        let mut warnings = Vec::new();
        let mut risk_score: u8 = 0;

        // 1. Get account info
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getAccountInfo",
            "params": [program_id, { "encoding": "jsonParsed" }]
        });
        let resp: Value = self.client.post(&self.rpc_url).json(&body).send().await?.json().await?;
        
        let account = &resp["result"]["value"];
        let is_executable = account["executable"].as_bool().unwrap_or(false);
        let owner = account["owner"].as_str().unwrap_or("unknown").to_string();
        let data_size = account["data"].as_array().map_or(0, |d| {
            d.first().and_then(|v| v.as_str()).map_or(0, |s| s.len())
        });

        if !is_executable {
            warnings.push("⚠️  Not an executable program".to_string());
            risk_score += 20;
        }

        // 2. Check if upgradeable (BPF Upgradeable Loader)
        let is_upgradeable = owner == "BPFLoaderUpgradeab1e11111111111111111111111";
        if is_upgradeable {
            warnings.push("🔓 UPGRADEABLE — owner can change code at any time".to_string());
            risk_score += 30;

            // Check programdata account for upgrade authority
            let body2 = serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "method": "getAccountInfo",
                "params": [program_id, { "encoding": "base64" }]
            });
            let resp2: Value = self.client.post(&self.rpc_url).json(&body2).send().await?.json().await?;
            if let Some(data) = resp2["result"]["value"]["data"].as_array() {
                if let Some(b64) = data.first().and_then(|v| v.as_str()) {
                    if b64.len() < 100 {
                        warnings.push("📦 Small program — likely a proxy/pointer".to_string());
                        risk_score += 10;
                    }
                }
            }
        }

        // 3. Check if it's a known program
        let known_safe = vec![
            "11111111111111111111111111111111",
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
            "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
            "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
        ];
        if known_safe.contains(&program_id) {
            warnings.clear();
            warnings.push("✅ Known system program".to_string());
            risk_score = 0;
        }

        // 4. Check data size (very small = suspicious, very large = complex)
        if data_size > 0 && data_size < 500 && is_executable {
            warnings.push("🔍 Very small program — may be a proxy".to_string());
            risk_score += 15;
        }
        if data_size > 500_000 {
            warnings.push("📏 Very large program (>500KB) — complex, more attack surface".to_string());
            risk_score += 10;
        }

        Ok(AuditResult {
            program_id: program_id.to_string(),
            is_executable,
            is_upgradeable,
            owner,
            data_size,
            warnings,
            risk_score: risk_score.min(100),
        })
    }
}

pub fn print_audit(result: &AuditResult) {
    println!("\n🛡️  Contract Audit: {}...{}", &result.program_id[..8], &result.program_id[result.program_id.len()-4..]);
    println!("═══════════════════════════════════════════");
    println!("  Executable: {}", if result.is_executable { "✅" } else { "❌" });
    println!("  Upgradeable: {}", if result.is_upgradeable { "🔓 YES" } else { "🔒 NO" });
    println!("  Owner: {}...{}", &result.owner[..8], &result.owner[result.owner.len()-4..]);
    println!("  Data size: {} bytes", result.data_size);
    
    let risk_emoji = match result.risk_score {
        0..=20 => "🟢",
        21..=50 => "🟡",
        51..=75 => "🟠",
        _ => "🔴",
    };
    println!("  Risk score: {} {}/100", risk_emoji, result.risk_score);
    
    if !result.warnings.is_empty() {
        println!("\n  Findings:");
        for w in &result.warnings {
            println!("    {}", w);
        }
    }
}
