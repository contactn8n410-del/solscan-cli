use crate::audit::ContractAudit;
use crate::scanner::PROGRAMS;
use std::collections::HashMap;

/// Autonomous monitoring daemon
/// Watches DeFi protocols for authority changes, upgrade events, anomalies
pub struct Daemon {
    rpc_url: String,
    /// Last known state of each program
    last_state: HashMap<String, ProgramState>,
    /// Alerts generated
    pub alerts: Vec<Alert>,
}

#[derive(Clone, Debug)]
struct ProgramState {
    is_upgradeable: bool,
    authority: Option<String>,
    authority_balance: f64,
    data_size: usize,
}

#[derive(Clone, Debug)]
pub struct Alert {
    pub timestamp: String,
    pub severity: Severity,
    pub program: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Severity::Critical => write!(f, "🔴 CRITICAL"),
            Severity::High => write!(f, "🟠 HIGH"),
            Severity::Medium => write!(f, "🟡 MEDIUM"),
            Severity::Info => write!(f, "🔵 INFO"),
        }
    }
}

impl Daemon {
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url,
            last_state: HashMap::new(),
            alerts: Vec::new(),
        }
    }

    pub async fn run_cycle(&mut self) -> Vec<Alert> {
        let auditor = ContractAudit::new(self.rpc_url.clone());
        let authority_mapper = crate::authority::AuthorityMapper::new(self.rpc_url.clone());
        let mut new_alerts = Vec::new();
        let now = chrono::Local::now().format("%H:%M:%S").to_string();

        for (program_id, name) in PROGRAMS {
            // Audit current state
            let audit = match auditor.audit(program_id).await {
                Ok(a) => a,
                Err(_) => continue,
            };
            
            let auth_info = authority_mapper.map_authority(program_id, name).await.ok();
            
            let current = ProgramState {
                is_upgradeable: audit.is_upgradeable,
                authority: auth_info.as_ref().and_then(|a| a.upgrade_authority.clone()),
                authority_balance: auth_info.as_ref().and_then(|a| a.authority_sol_balance).unwrap_or(0.0),
                data_size: audit.data_size,
            };

            // Compare with last known state
            if let Some(prev) = self.last_state.get(*program_id) {
                // CRITICAL: Authority changed
                if prev.authority != current.authority {
                    let alert = Alert {
                        timestamp: now.clone(),
                        severity: Severity::Critical,
                        program: name.to_string(),
                        message: format!(
                            "AUTHORITY CHANGED! {} → {}",
                            prev.authority.as_deref().unwrap_or("none"),
                            current.authority.as_deref().unwrap_or("none")
                        ),
                    };
                    new_alerts.push(alert);
                }

                // HIGH: Program was upgraded (data size changed)
                if prev.data_size != current.data_size && prev.data_size > 0 {
                    let alert = Alert {
                        timestamp: now.clone(),
                        severity: Severity::High,
                        program: name.to_string(),
                        message: format!(
                            "PROGRAM UPGRADED! Size {} → {} bytes",
                            prev.data_size, current.data_size
                        ),
                    };
                    new_alerts.push(alert);
                }

                // MEDIUM: Authority balance changed significantly (>10 SOL movement)
                let bal_diff = (current.authority_balance - prev.authority_balance).abs();
                if bal_diff > 10.0 {
                    let alert = Alert {
                        timestamp: now.clone(),
                        severity: Severity::Medium,
                        program: name.to_string(),
                        message: format!(
                            "Authority balance shift: {:.2} → {:.2} SOL (Δ{:.2})",
                            prev.authority_balance, current.authority_balance, bal_diff
                        ),
                    };
                    new_alerts.push(alert);
                }

                // HIGH: Previously immutable program became upgradeable (should be impossible but check)
                if !prev.is_upgradeable && current.is_upgradeable {
                    let alert = Alert {
                        timestamp: now.clone(),
                        severity: Severity::Critical,
                        program: name.to_string(),
                        message: "IMMUTABLE PROGRAM BECAME UPGRADEABLE — POSSIBLE ATTACK".to_string(),
                    };
                    new_alerts.push(alert);
                }
            } else {
                // First scan — just record baseline
                new_alerts.push(Alert {
                    timestamp: now.clone(),
                    severity: Severity::Info,
                    program: name.to_string(),
                    message: format!(
                        "Baseline: {} | auth: {}",
                        if current.is_upgradeable { "upgradeable" } else { "immutable" },
                        current.authority.as_deref().unwrap_or("none")
                    ),
                });
            }

            self.last_state.insert(program_id.to_string(), current);

            // Rate limit
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }

        self.alerts.extend(new_alerts.clone());
        new_alerts
    }
}

pub async fn run_daemon(rpc_url: &str, interval_secs: u64, json_output: bool) {
    let mut daemon = Daemon::new(rpc_url.to_string());
    let mut cycle = 0u64;

    if !json_output {
        println!("🔮 Solana DeFi Guardian — Autonomous Monitor");
        println!("    Tracking {} protocols every {}s", PROGRAMS.len(), interval_secs);
        println!("    Watching for: authority changes, upgrades, balance anomalies");
        println!("    Press Ctrl+C to stop\n");
    }

    loop {
        cycle += 1;
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        if !json_output {
            eprint!("[{}] Cycle {}... ", now, cycle);
        }

        let alerts = daemon.run_cycle().await;
        
        let critical = alerts.iter().filter(|a| matches!(a.severity, Severity::Critical | Severity::High)).count();
        let info = alerts.iter().filter(|a| matches!(a.severity, Severity::Info)).count();

        if json_output {
            for alert in &alerts {
                if !matches!(alert.severity, Severity::Info) || cycle == 1 {
                    println!("{}", serde_json::json!({
                        "cycle": cycle,
                        "time": alert.timestamp,
                        "severity": format!("{:?}", alert.severity),
                        "program": alert.program,
                        "message": alert.message,
                    }));
                }
            }
        } else {
            if critical > 0 {
                eprintln!("⚠️  {} ALERTS!", critical);
                for alert in &alerts {
                    if !matches!(alert.severity, Severity::Info) {
                        println!("  {} [{}] {}", alert.severity, alert.program, alert.message);
                    }
                }
            } else if cycle == 1 {
                eprintln!("{} programs baselined ✅", info);
            } else {
                eprintln!("all clear ✅");
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
    }
}
