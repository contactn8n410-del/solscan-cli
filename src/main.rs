use std::env;
mod analyze;
mod web;
mod audit;
mod scanner;
mod authority;
mod daemon;

fn rpc_url() -> String {
    env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string())
}
const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: solscan <wallet_address> [OPTIONS]");
        eprintln!("\nScan any Solana wallet from the command line.");
        eprintln!("\nOptions:");
        eprintln!("  --tokens          Show all token accounts and balances");
        eprintln!("  --history         Show recent transaction history");
        eprintln!("  --json            Output as JSON");
        eprintln!("  --defi            Show DeFi positions (mSOL, jitoSOL)");
        eprintln!("  --watch           Live monitoring mode (poll for changes)");
        eprintln!("  --interval <N>    Poll interval in seconds (default: 5)");
        eprintln!("\nExamples:");
        eprintln!("  solscan EXEDJvuA...epTq --tokens");
        eprintln!("  solscan EXEDJvuA...epTq --watch --interval 10");
        eprintln!("  solscan EXEDJvuA...epTq --watch --json | jq '.change'");
        eprintln!("  solscan x --power-map              # Who controls Solana DeFi");
        eprintln!("  solscan x --scan-defi              # Audit top 15 protocols");
        eprintln!("  solscan x --guardian                # Autonomous monitoring daemon");
        eprintln!("\n💰 Tip: EXEDJvuAaYt9yN5mwZRPdCP19tYuF6LWztnu6qpbepTq (SOL)");
        std::process::exit(1);
    }

    let wallet = &args[1];
    let show_tokens = args.contains(&"--tokens".to_string());
    let show_history = args.contains(&"--history".to_string());
    let output_json = args.contains(&"--json".to_string());
    let show_defi = args.contains(&"--defi".to_string());
    let watch_mode = args.contains(&"--watch".to_string());
    let analyze_mode = args.contains(&"--analyze".to_string());
    let web_mode = args.contains(&"--web".to_string());
    let audit_mode = args.contains(&"--audit".to_string());
    let scan_all = args.contains(&"--scan-defi".to_string());
    let power_map = args.contains(&"--power-map".to_string());
    let guardian_mode = args.contains(&"--guardian".to_string());
    let guardian_interval: u64 = args.iter()
        .position(|a| a == "--every")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    let web_depth: usize = args.iter()
        .position(|a| a == "--depth")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let watch_interval: u64 = args.iter()
        .position(|a| a == "--interval")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    // Collect extra wallets for analyze mode
    let extra_wallets: Vec<String> = if analyze_mode {
        args.iter().skip(2)
            .filter(|a| !a.starts_with("--") && a.len() > 30)
            .cloned().collect()
    } else { vec![] };

    rt.block_on(async {
        if guardian_mode {
            daemon::run_daemon(&rpc_url(), guardian_interval, output_json).await;
        } else if power_map {
            let results = authority::map_all_authorities(&rpc_url()).await;
            if output_json {
                let json: Vec<_> = results.iter().map(|r| serde_json::json!({
                    "name": r.program_name,
                    "program_id": r.program_id,
                    "upgrade_authority": r.upgrade_authority,
                    "programdata": r.programdata_account,
                    "authority_balance_sol": r.authority_sol_balance,
                    "authority_tx_count": r.authority_tx_count,
                })).collect();
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
            } else {
                authority::print_power_map(&results);
            }
        } else if scan_all {
            let results = scanner::scan_all(&rpc_url()).await;
            if output_json {
                let json: Vec<_> = results.iter().map(|(name, r)| serde_json::json!({
                    "name": name, "program_id": r.program_id,
                    "upgradeable": r.is_upgradeable, "risk_score": r.risk_score,
                    "warnings": r.warnings,
                })).collect();
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
            } else {
                scanner::print_report(&results);
            }
        } else if audit_mode {
            let auditor = audit::ContractAudit::new(rpc_url());
            match auditor.audit(wallet).await {
                Ok(result) => {
                    if output_json {
                        println!("{}", serde_json::json!({
                            "program_id": result.program_id,
                            "executable": result.is_executable,
                            "upgradeable": result.is_upgradeable,
                            "owner": result.owner,
                            "data_size": result.data_size,
                            "risk_score": result.risk_score,
                            "warnings": result.warnings,
                        }));
                    } else {
                        audit::print_audit(&result);
                    }
                }
                Err(e) => { eprintln!("Audit error: {}", e); std::process::exit(1); }
            }
        } else if web_mode {
            if let Err(e) = run_web(wallet, web_depth, output_json).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        } else if analyze_mode {
            if let Err(e) = run_analyze(wallet, &extra_wallets).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        } else if watch_mode {
            if let Err(e) = watch_wallet(wallet, watch_interval, output_json).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        } else if let Err(e) = scan_wallet(wallet, show_tokens, show_history, show_defi, output_json).await {
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

    // Collect data
    let tokens = if show_tokens || output_json { get_token_accounts(&client, wallet).await.unwrap_or_default() } else { vec![] };
    let signatures = if show_history || output_json { get_recent_signatures(&client, wallet, 10).await.unwrap_or_default() } else { vec![] };

    if output_json {
        let mut json = serde_json::json!({
            "address": wallet,
            "sol_balance": balance,
        });
        if show_tokens || true {
            json["tokens"] = serde_json::json!(tokens.iter().map(|t| serde_json::json!({
                "mint": t.mint,
                "balance": t.ui_amount,
                "decimals": t.decimals
            })).collect::<Vec<_>>());
        }
        if show_history {
            json["transactions"] = serde_json::json!(signatures.iter().map(|s| serde_json::json!({
                "signature": s.signature,
                "slot": s.slot,
                "error": s.err,
                "time": s.block_time_str()
            })).collect::<Vec<_>>());
        }
        println!("{}", serde_json::to_string_pretty(&json)?);
        return Ok(());
    }

    // Pretty output
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  🔍 Solana Wallet Scanner                                   ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Address: {}...{}", &wallet[..8], &wallet[wallet.len()-8..]);
    println!("║  SOL Balance: {:.6} SOL", balance);
    println!("╚══════════════════════════════════════════════════════════════╝");

    if show_tokens {
        println!("\n📦 Token Accounts:");
        println!("─────────────────────────────────────────────────");
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

    if show_history {
        println!("\n📜 Recent Transactions (last 10):");
        println!("─────────────────────────────────────────────────");
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

    if show_defi {
        println!("\n🏦 DeFi Positions:");
        println!("─────────────────────────────────────────────────");
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

// === Web Crawl Mode ===

async fn run_web(wallet: &str, max_depth: usize, json_output: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🕸️  SolWeb — Crawling from {}...{} (depth: {})", &wallet[..8], &wallet[wallet.len()-4..], max_depth);
    let mut spider = web::SolWeb::new(rpc_url(), max_depth);
    spider.crawl(wallet).await?;
    
    if json_output {
        let out = serde_json::json!({
            "wallets": spider.wallet_tokens.len(),
            "tokens": spider.token_holders.len(),
            "wallet_tokens": spider.wallet_tokens,
            "token_holders": spider.token_holders,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        spider.print_web();
    }
    Ok(())
}

// === Analyze Mode ===

async fn run_analyze(primary: &str, others: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut graph = analyze::WalletGraph::new();

    let mut all_wallets = vec![primary.to_string()];
    all_wallets.extend(others.iter().cloned());

    for wallet in &all_wallets {
        eprint!("  Scanning {}...{} ", &wallet[..8], &wallet[wallet.len()-4..]);
        let balance = get_sol_balance(&client, wallet).await.unwrap_or(0.0);
        let tokens = get_token_accounts(&client, wallet).await.unwrap_or_default();
        let mints: Vec<String> = tokens.iter().map(|t| t.mint.clone()).collect();
        eprintln!("({:.4} SOL, {} tokens)", balance, mints.len());
        graph.add_wallet(wallet.clone(), balance, mints);
        // Rate limit courtesy
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    analyze::print_analysis(&graph);
    Ok(())
}

// === Watch Mode ===

async fn watch_wallet(
    wallet: &str,
    interval_secs: u64,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut last_balance: f64 = -1.0;
    let mut last_sig = String::new();
    let mut iteration = 0u64;

    if !json_output {
        println!("👁️  Watching wallet: {}...{}", &wallet[..8], &wallet[wallet.len()-8..]);
        println!("    Polling every {}s — Ctrl+C to stop\n", interval_secs);
    }

    loop {
        let balance = get_sol_balance(&client, wallet).await.unwrap_or(-1.0);
        let sigs = get_recent_signatures(&client, wallet, 1).await.unwrap_or_default();
        let newest_sig = sigs.first().map(|s| s.signature.clone()).unwrap_or_default();

        let balance_changed = last_balance >= 0.0 && (balance - last_balance).abs() > 0.000000001;
        let new_tx = !newest_sig.is_empty() && newest_sig != last_sig && !last_sig.is_empty();

        if iteration == 0 || balance_changed || new_tx {
            let now = chrono::Local::now().format("%H:%M:%S").to_string();
            if json_output {
                let event = serde_json::json!({
                    "time": now,
                    "balance": balance,
                    "change": if balance_changed { Some(balance - last_balance) } else { None },
                    "new_tx": if new_tx { Some(&newest_sig) } else { None },
                });
                println!("{}", event);
            } else {
                if balance_changed {
                    let diff = balance - last_balance;
                    let arrow = if diff > 0.0 { "📈" } else { "📉" };
                    println!("[{}] {} SOL: {:.9} ({}{:.9})",
                        now, arrow, balance,
                        if diff > 0.0 { "+" } else { "" }, diff);
                } else if new_tx {
                    println!("[{}] 🔔 New TX: {}...{}", now,
                        &newest_sig[..16], &newest_sig[newest_sig.len()-8..]);
                } else if iteration == 0 {
                    println!("[{}] ✅ SOL: {:.9}", now, balance);
                }
            }
        }

        last_balance = balance;
        if !newest_sig.is_empty() { last_sig = newest_sig; }
        iteration += 1;

        tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
    }
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
