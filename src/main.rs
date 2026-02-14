use std::env;

fn rpc_url() -> String {
    env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string())
}
const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: solscan <wallet_address> [--tokens] [--history] [--json]");
        eprintln!("\nScan any Solana wallet from the command line.");
        eprintln!("\nOptions:");
        eprintln!("  --tokens   Show all token accounts and balances");
        eprintln!("  --history  Show recent transaction history");
        eprintln!("  --json     Output as JSON");
        eprintln!("  --defi     Show DeFi positions (Raydium, Orca, Marinade)");
        eprintln!("\nExamples:");
        eprintln!("  solscan So11111111111111111111111111111111111111112");
        eprintln!("  solscan EXEDJvuAaYt9yN5mwZRPdCP19tYuF6LWztnu6qpbepTq --tokens");
        std::process::exit(1);
    }

    let wallet = &args[1];
    let show_tokens = args.contains(&"--tokens".to_string());
    let show_history = args.contains(&"--history".to_string());
    let output_json = args.contains(&"--json".to_string());
    let show_defi = args.contains(&"--defi".to_string());

    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    rt.block_on(async {
        if let Err(e) = scan_wallet(wallet, show_tokens, show_history, show_defi, output_json).await {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    });
}

async fn scan_wallet(
    wallet: &str,
    show_tokens: bool,
    show_history: bool,
    show_defi: bool,
    output_json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    // 1. Get SOL balance
    let balance = get_sol_balance(&client, wallet).await?;

    if !output_json {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  🔍 Solana Wallet Scanner                                   ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║  Address: {}...{}", &wallet[..8], &wallet[wallet.len()-8..]);
        println!("║  SOL Balance: {:.6} SOL", balance);
        println!("╚══════════════════════════════════════════════════════════════╝");
    }

    // 2. Get token accounts
    if show_tokens || output_json {
        println!("\n📦 Token Accounts:");
        println!("─────────────────────────────────────────────────");

        let tokens = get_token_accounts(&client, wallet).await?;
        if tokens.is_empty() {
            println!("  No token accounts found.");
        } else {
            println!("  {:>12}  {:>10}  {}", "Balance", "Decimals", "Mint");
            println!("  {:>12}  {:>10}  {}", "───────", "────────", "────");
            for token in &tokens {
                println!(
                    "  {:>12}  {:>10}  {}...{}",
                    token.ui_amount,
                    token.decimals,
                    &token.mint[..8],
                    &token.mint[token.mint.len()-4..]
                );
            }
            println!("\n  Total token accounts: {}", tokens.len());
        }
    }

    // 3. Recent transactions
    if show_history {
        println!("\n📜 Recent Transactions (last 10):");
        println!("─────────────────────────────────────────────────");

        let signatures = get_recent_signatures(&client, wallet, 10).await?;
        if signatures.is_empty() {
            println!("  No recent transactions.");
        } else {
            for sig in &signatures {
                let status = if sig.err { "❌" } else { "✅" };
                println!(
                    "  {} {}...{} | slot {} | {}",
                    status,
                    &sig.signature[..16],
                    &sig.signature[sig.signature.len()-8..],
                    sig.slot,
                    sig.block_time_str()
                );
            }
        }
    }

    // 4. DeFi positions
    if show_defi {
        println!("\n🏦 DeFi Positions:");
        println!("─────────────────────────────────────────────────");
        println!("  Scanning Raydium, Orca, Marinade...");

        // Check for staked SOL (Marinade mSOL)
        let tokens = get_token_accounts(&client, wallet).await?;
        let msol_mint = "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So";
        let jitosol_mint = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn";

        for token in &tokens {
            if token.mint == msol_mint {
                println!("  🌊 Marinade mSOL: {} mSOL", token.ui_amount);
            }
            if token.mint == jitosol_mint {
                println!("  ⚡ Jito jitoSOL: {} jitoSOL", token.ui_amount);
            }
        }
        println!("  (Full DeFi position scanning coming in v0.2)");
    }

    Ok(())
}

// === RPC Helpers ===

async fn rpc_call(
    client: &reqwest::Client,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });

    let resp = client
        .post(&rpc_url())
        .json(&body)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    if let Some(err) = resp.get("error") {
        return Err(format!("RPC error: {}", err).into());
    }

    Ok(resp["result"].clone())
}

async fn get_sol_balance(
    client: &reqwest::Client,
    wallet: &str,
) -> Result<f64, Box<dyn std::error::Error>> {
    let result = rpc_call(client, "getBalance", serde_json::json!([wallet, {"commitment": "confirmed"}])).await?;
    let lamports = result["value"].as_u64().unwrap_or(0);
    Ok(lamports as f64 / LAMPORTS_PER_SOL)
}

struct TokenAccount {
    mint: String,
    ui_amount: String,
    decimals: u8,
}

async fn get_token_accounts(
    client: &reqwest::Client,
    wallet: &str,
) -> Result<Vec<TokenAccount>, Box<dyn std::error::Error>> {
    let result = rpc_call(
        client,
        "getTokenAccountsByOwner",
        serde_json::json!([
            wallet,
            { "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" },
            { "encoding": "jsonParsed" }
        ]),
    )
    .await?;

    // Also check Token-2022
    let result_2022 = rpc_call(
        client,
        "getTokenAccountsByOwner",
        serde_json::json!([
            wallet,
            { "programId": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" },
            { "encoding": "jsonParsed" }
        ]),
    )
    .await?;

    let mut accounts = Vec::new();

    for result_set in [&result, &result_2022] {
        if let Some(values) = result_set["value"].as_array() {
            for val in values {
                let info = &val["account"]["data"]["parsed"]["info"];
                let mint = info["mint"].as_str().unwrap_or("unknown").to_string();
                let token_amount = &info["tokenAmount"];
                let ui_str = token_amount["uiAmountString"]
                    .as_str()
                    .unwrap_or("0")
                    .to_string();
                let decimals = token_amount["decimals"].as_u64().unwrap_or(0) as u8;

                // Skip zero balances
                let amount = token_amount["uiAmount"].as_f64().unwrap_or(0.0);
                if amount > 0.0 {
                    accounts.push(TokenAccount {
                        mint,
                        ui_amount: ui_str,
                        decimals,
                    });
                }
            }
        }
    }

    Ok(accounts)
}

struct SignatureInfo {
    signature: String,
    slot: u64,
    block_time: Option<i64>,
    err: bool,
}

impl SignatureInfo {
    fn block_time_str(&self) -> String {
        match self.block_time {
            Some(ts) => {
                // Simple timestamp formatting
                let secs = ts;
                format!("ts:{}", secs)
            }
            None => "unknown".to_string(),
        }
    }
}

async fn get_recent_signatures(
    client: &reqwest::Client,
    wallet: &str,
    limit: usize,
) -> Result<Vec<SignatureInfo>, Box<dyn std::error::Error>> {
    let result = rpc_call(
        client,
        "getSignaturesForAddress",
        serde_json::json!([wallet, { "limit": limit }]),
    )
    .await?;

    let mut sigs = Vec::new();
    if let Some(arr) = result.as_array() {
        for item in arr {
            sigs.push(SignatureInfo {
                signature: item["signature"].as_str().unwrap_or("").to_string(),
                slot: item["slot"].as_u64().unwrap_or(0),
                block_time: item["blockTime"].as_i64(),
                err: item["err"].is_object(),
            });
        }
    }

    Ok(sigs)
}
