use serde_json::Value;
use std::collections::HashMap;

/// Maps upgrade authorities for Solana programs
/// Reveals WHO controls each DeFi protocol
pub struct AuthorityMapper {
    client: reqwest::Client,
    rpc_url: String,
}

#[derive(Debug, Clone)]
pub struct AuthorityInfo {
    pub program_id: String,
    pub program_name: String,
    pub programdata_account: Option<String>,
    pub upgrade_authority: Option<String>,
    pub authority_sol_balance: Option<f64>,
    pub authority_tx_count: Option<usize>,
}

impl AuthorityMapper {
    pub fn new(rpc_url: String) -> Self {
        Self { client: reqwest::Client::new(), rpc_url }
    }

    pub async fn map_authority(&self, program_id: &str, name: &str) -> Result<AuthorityInfo, Box<dyn std::error::Error>> {
        // Step 1: Get program account to find programdata address
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getAccountInfo",
            "params": [program_id, { "encoding": "base64" }]
        });
        let resp: Value = self.client.post(&self.rpc_url).json(&body).send().await?.json().await?;
        
        let owner = resp["result"]["value"]["owner"].as_str().unwrap_or("");
        
        if owner != "BPFLoaderUpgradeab1e11111111111111111111111" {
            return Ok(AuthorityInfo {
                program_id: program_id.to_string(),
                program_name: name.to_string(),
                programdata_account: None,
                upgrade_authority: None,
                authority_sol_balance: None,
                authority_tx_count: None,
            });
        }

        // Step 2: Decode programdata address from program account data
        // For BPF Upgradeable, program account data = [4 bytes type][32 bytes programdata pubkey]
        let data_b64 = resp["result"]["value"]["data"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let programdata_addr = if !data_b64.is_empty() {
            if let Ok(bytes) = base64_decode(data_b64) {
                if bytes.len() >= 36 {
                    Some(bs58_encode(&bytes[4..36]))
                } else { None }
            } else { None }
        } else { None };

        // Step 3: Get programdata account to find upgrade authority
        let upgrade_authority = if let Some(ref pda) = programdata_addr {
            let body2 = serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "method": "getAccountInfo",
                "params": [pda, { "encoding": "base64" }]
            });
            let resp2: Value = self.client.post(&self.rpc_url).json(&body2).send().await?.json().await?;
            
            let pda_data = resp2["result"]["value"]["data"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            if !pda_data.is_empty() {
                if let Ok(bytes) = base64_decode(pda_data) {
                    // ProgramData account: [4 bytes type][8 bytes slot][1 byte option][32 bytes authority]
                    if bytes.len() >= 45 && bytes[12] == 1 {
                        Some(bs58_encode(&bytes[13..45]))
                    } else { None }
                } else { None }
            } else { None }
        } else { None };

        // Step 4: Get authority wallet info
        let (authority_sol_balance, authority_tx_count) = if let Some(ref auth) = upgrade_authority {
            let bal = self.get_balance(auth).await.unwrap_or(0.0);
            let txs = self.get_sig_count(auth).await.unwrap_or(0);
            (Some(bal), Some(txs))
        } else {
            (None, None)
        };

        Ok(AuthorityInfo {
            program_id: program_id.to_string(),
            program_name: name.to_string(),
            programdata_account: programdata_addr,
            upgrade_authority,
            authority_sol_balance,
            authority_tx_count,
        })
    }

    async fn get_balance(&self, addr: &str) -> Result<f64, Box<dyn std::error::Error>> {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getBalance",
            "params": [addr, {"commitment": "confirmed"}]
        });
        let resp: Value = self.client.post(&self.rpc_url).json(&body).send().await?.json().await?;
        Ok(resp["result"]["value"].as_u64().unwrap_or(0) as f64 / 1_000_000_000.0)
    }

    async fn get_sig_count(&self, addr: &str) -> Result<usize, Box<dyn std::error::Error>> {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getSignaturesForAddress",
            "params": [addr, {"limit": 100}]
        });
        let resp: Value = self.client.post(&self.rpc_url).json(&body).send().await?.json().await?;
        Ok(resp["result"].as_array().map_or(0, |a| a.len()))
    }
}

fn base64_decode(input: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Read;
    // Simple base64 decoder
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &c in input.as_bytes() {
        if c == b'=' || c == b'\n' || c == b'\r' { continue; }
        let val = table.iter().position(|&t| t == c).ok_or("invalid base64")? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

fn bs58_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    if bytes.is_empty() { return String::new(); }
    
    let mut digits = vec![0u8];
    for &byte in bytes {
        let mut carry = byte as u32;
        for d in digits.iter_mut() {
            carry += (*d as u32) * 256;
            *d = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }
    
    // Leading zeros
    let mut result = String::new();
    for &b in bytes {
        if b == 0 { result.push('1'); } else { break; }
    }
    
    for &d in digits.iter().rev() {
        result.push(ALPHABET[d as usize] as char);
    }
    result
}

pub async fn map_all_authorities(rpc_url: &str) -> Vec<AuthorityInfo> {
    let mapper = AuthorityMapper::new(rpc_url.to_string());
    let mut results = Vec::new();
    
    for (program_id, name) in crate::scanner::PROGRAMS {
        eprint!("  🔑 {}... ", name);
        match mapper.map_authority(program_id, name).await {
            Ok(info) => {
                if let Some(ref auth) = info.upgrade_authority {
                    eprintln!("authority: {}...{}", &auth[..8], &auth[auth.len()-4..]);
                } else {
                    eprintln!("immutable ✅");
                }
                results.push(info);
            }
            Err(e) => eprintln!("error: {}", e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
    }
    results
}

pub fn print_power_map(results: &[AuthorityInfo]) {
    println!("\n🗺️  Solana DeFi Power Map — Who Controls What");
    println!("══════════════════════════════════════════════════════════");
    
    // Group by authority
    let mut authority_protocols: HashMap<String, Vec<String>> = HashMap::new();
    
    for info in results {
        let key = info.upgrade_authority.clone().unwrap_or_else(|| "IMMUTABLE".to_string());
        authority_protocols.entry(key).or_default().push(info.program_name.clone());
    }
    
    println!("\n  📋 Per-Protocol:");
    for info in results {
        match &info.upgrade_authority {
            Some(auth) => {
                println!("    {} → 🔓 {}...{} ({:.2} SOL, {} txs)",
                    info.program_name,
                    &auth[..8], &auth[auth.len()-4..],
                    info.authority_sol_balance.unwrap_or(0.0),
                    info.authority_tx_count.unwrap_or(0));
            }
            None => {
                println!("    {} → 🔒 IMMUTABLE", info.program_name);
            }
        }
    }
    
    // Power concentration
    println!("\n  🏛️  Authority Concentration:");
    let mut sorted: Vec<_> = authority_protocols.iter().collect();
    sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    
    for (auth, protocols) in &sorted {
        if protocols.len() > 1 {
            println!("    ⚠️  {}...{} controls {} protocols: {}",
                &auth[..8.min(auth.len())], &auth[auth.len().saturating_sub(4)..],
                protocols.len(),
                protocols.join(", "));
        }
    }
    
    let immutable = results.iter().filter(|r| r.upgrade_authority.is_none()).count();
    let total = results.len();
    println!("\n  📊 Summary:");
    println!("    Total protocols: {}", total);
    println!("    Immutable: {}/{} ({:.0}%)", immutable, total, immutable as f64 / total as f64 * 100.0);
    println!("    Unique authorities: {}", authority_protocols.len());
}
