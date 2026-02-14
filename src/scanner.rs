use crate::audit::{ContractAudit, AuditResult};
use std::collections::HashMap;

/// Known Solana DeFi programs to audit
pub const PROGRAMS: &[(&str, &str)] = &[
    ("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4", "Jupiter v6"),
    ("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc", "Orca Whirlpool"),
    ("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK", "Raydium CPMM"),
    ("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", "Raydium AMM v4"),
    ("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA", "Marginfi v2"),
    ("So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo", "Solend"),
    ("SSwpkEEcbUqx4vtoEByFjSkhKdCT862DNVb52nZg1UZ", "Saber Stable Swap"),
    ("DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1", "Orca Token Swap"),
    ("PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY", "Phoenix DEX"),
    ("6m2CDdhRgxpH4WjvdzxAYbGxwdGUz5MziiL5jek2kBma", "Drift Protocol"),
    ("MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac", "Mango Markets v3"),
    ("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX", "Serum DEX v3"),
    ("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo", "Meteora DLMM"),
    ("FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X", "FluxBeam"),
    ("MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD", "Marinade Finance"),
];

pub async fn scan_all(rpc_url: &str) -> Vec<(String, AuditResult)> {
    let auditor = ContractAudit::new(rpc_url.to_string());
    let mut results = Vec::new();
    
    for (program_id, name) in PROGRAMS {
        eprint!("  Auditing {}... ", name);
        match auditor.audit(program_id).await {
            Ok(result) => {
                eprintln!("{} {}/100", 
                    match result.risk_score { 0..=20 => "🟢", 21..=50 => "🟡", 51..=75 => "🟠", _ => "🔴" },
                    result.risk_score);
                results.push((name.to_string(), result));
            }
            Err(e) => {
                eprintln!("❌ {}", e);
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }

    results
}

pub fn print_report(results: &[(String, AuditResult)]) {
    println!("\n🔬 Solana DeFi Security Scanner — Full Report");
    println!("══════════════════════════════════════════════════════════");
    println!("  {:30} {:12} {:10} {:6}", "Protocol", "Upgradeable", "Data Size", "Risk");
    println!("  {:30} {:12} {:10} {:6}", "────────", "───────────", "─────────", "────");
    
    for (name, r) in results {
        let risk_emoji = match r.risk_score { 0..=20 => "🟢", 21..=50 => "🟡", 51..=75 => "🟠", _ => "🔴" };
        println!("  {:30} {:12} {:>10} {} {:>3}", 
            name,
            if r.is_upgradeable { "🔓 YES" } else { "🔒 NO" },
            format!("{}B", r.data_size),
            risk_emoji,
            r.risk_score);
    }
    
    let avg_risk: f64 = results.iter().map(|(_, r)| r.risk_score as f64).sum::<f64>() / results.len().max(1) as f64;
    let upgradeable_count = results.iter().filter(|(_, r)| r.is_upgradeable).count();
    
    println!("\n  📊 Summary:");
    println!("    Programs scanned: {}", results.len());
    println!("    Upgradeable: {}/{}", upgradeable_count, results.len());
    println!("    Average risk score: {:.1}/100", avg_risk);
    
    if upgradeable_count > results.len() / 2 {
        println!("    ⚠️  WARNING: Majority of programs are upgradeable!");
    }
}
