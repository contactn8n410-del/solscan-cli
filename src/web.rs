use std::collections::{HashMap, HashSet, VecDeque};
use serde_json::Value;

/// Recursive Solana wallet/token graph crawler
/// Given a starting wallet, discovers all connected wallets through shared tokens
pub struct SolWeb {
    client: reqwest::Client,
    rpc_url: String,
    /// wallet -> tokens held
    pub wallet_tokens: HashMap<String, Vec<String>>,
    /// token -> holders discovered  
    pub token_holders: HashMap<String, Vec<String>>,
    /// wallets already visited
    visited: HashSet<String>,
    /// max wallets to crawl
    max_depth: usize,
}

impl SolWeb {
    pub fn new(rpc_url: String, max_depth: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            rpc_url,
            wallet_tokens: HashMap::new(),
            token_holders: HashMap::new(),
            visited: HashSet::new(),
            max_depth,
        }
    }

    /// Crawl starting from a wallet, discover connected wallets via shared tokens
    pub async fn crawl(&mut self, start: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(start.to_string());

        while let Some(wallet) = queue.pop_front() {
            if self.visited.contains(&wallet) || self.visited.len() >= self.max_depth {
                break;
            }
            self.visited.insert(wallet.clone());

            // Get tokens for this wallet
            let tokens = self.get_tokens(&wallet).await.unwrap_or_default();
            
            if !tokens.is_empty() {
                eprintln!("  🕸️  {}...{}: {} tokens", &wallet[..8], &wallet[wallet.len()-4..], tokens.len());
            }

            for mint in &tokens {
                self.token_holders.entry(mint.clone()).or_default().push(wallet.clone());
                
                // For each token, find largest holders (top accounts)
                if !self.token_holders.get(mint).map_or(false, |h| h.len() > 3) {
                    if let Ok(holders) = self.get_largest_accounts(mint).await {
                        for holder_wallet in &holders {
                            if !self.visited.contains(holder_wallet) {
                                queue.push_back(holder_wallet.clone());
                            }
                            self.token_holders.entry(mint.clone()).or_default().push(holder_wallet.clone());
                        }
                    }
                }
            }

            self.wallet_tokens.insert(wallet, tokens);

            // Rate limit
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        Ok(())
    }

    async fn get_tokens(&self, wallet: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getTokenAccountsByOwner",
            "params": [
                wallet,
                { "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" },
                { "encoding": "jsonParsed" }
            ]
        });
        let resp: Value = self.client.post(&self.rpc_url).json(&body).send().await?.json().await?;
        
        let mut mints = Vec::new();
        if let Some(accounts) = resp["result"]["value"].as_array() {
            for acc in accounts {
                if let Some(mint) = acc["account"]["data"]["parsed"]["info"]["mint"].as_str() {
                    mints.push(mint.to_string());
                }
            }
        }

        // Also Token-2022
        let body2 = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getTokenAccountsByOwner",
            "params": [
                wallet,
                { "programId": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" },
                { "encoding": "jsonParsed" }
            ]
        });
        let resp2: Value = self.client.post(&self.rpc_url).json(&body2).send().await?.json().await?;
        if let Some(accounts) = resp2["result"]["value"].as_array() {
            for acc in accounts {
                if let Some(mint) = acc["account"]["data"]["parsed"]["info"]["mint"].as_str() {
                    mints.push(mint.to_string());
                }
            }
        }

        Ok(mints)
    }

    async fn get_largest_accounts(&self, mint: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getTokenLargestAccounts",
            "params": [mint]
        });
        let resp: Value = self.client.post(&self.rpc_url).json(&body).send().await?.json().await?;
        
        let mut owners = Vec::new();
        if let Some(accounts) = resp["result"]["value"].as_array() {
            for acc in accounts.iter().take(5) {
                if let Some(addr) = acc["address"].as_str() {
                    // Get the owner of this token account
                    if let Ok(owner) = self.get_account_owner(addr).await {
                        owners.push(owner);
                    }
                }
            }
        }
        Ok(owners)
    }

    async fn get_account_owner(&self, token_account: &str) -> Result<String, Box<dyn std::error::Error>> {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getAccountInfo",
            "params": [token_account, { "encoding": "jsonParsed" }]
        });
        let resp: Value = self.client.post(&self.rpc_url).json(&body).send().await?.json().await?;
        resp["result"]["value"]["data"]["parsed"]["info"]["owner"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or("no owner".into())
    }

    /// Print the web as a graph summary
    pub fn print_web(&self) {
        println!("\n🕸️  SolWeb — Wallet Connection Graph");
        println!("═══════════════════════════════════════════");
        println!("  Wallets discovered: {}", self.wallet_tokens.len());
        println!("  Unique tokens: {}", self.token_holders.len());
        
        // Find tokens held by multiple wallets (connections)
        let mut connections: Vec<(&String, usize)> = self.token_holders.iter()
            .map(|(mint, holders)| {
                let unique: HashSet<&String> = holders.iter().collect();
                (mint, unique.len())
            })
            .filter(|(_, count)| *count > 1)
            .collect();
        connections.sort_by(|a, b| b.1.cmp(&a.1));
        
        if !connections.is_empty() {
            println!("\n  🔗 Connecting Tokens (held by multiple wallets):");
            for (mint, count) in connections.iter().take(10) {
                println!("    {}...{} → {} wallets",
                    &mint[..8], &mint[mint.len()-4..], count);
            }
        }

        // Find most connected wallets
        let mut wallet_connections: Vec<(&String, usize)> = self.wallet_tokens.iter()
            .map(|(wallet, tokens)| {
                let shared = tokens.iter()
                    .filter(|t| self.token_holders.get(*t).map_or(false, |h| {
                        let unique: HashSet<&String> = h.iter().collect();
                        unique.len() > 1
                    }))
                    .count();
                (wallet, shared)
            })
            .collect();
        wallet_connections.sort_by(|a, b| b.1.cmp(&a.1));
        
        if !wallet_connections.is_empty() {
            println!("\n  🏠 Most Connected Wallets:");
            for (wallet, shared) in wallet_connections.iter().take(5) {
                println!("    {}...{} → {} shared tokens",
                    &wallet[..8], &wallet[wallet.len()-4..], shared);
            }
        }

        // JSON output
        println!("\n  📊 Export: solscan <wallet> --web --json | jq");
    }
}
