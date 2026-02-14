use std::collections::{HashMap, HashSet};

/// Multi-wallet analyzer — finds connections between Solana wallets
pub struct WalletGraph {
    /// wallet -> set of token mints held
    pub holdings: HashMap<String, HashSet<String>>,
    /// wallet -> SOL balance
    pub balances: HashMap<String, f64>,
}

impl WalletGraph {
    pub fn new() -> Self {
        Self {
            holdings: HashMap::new(),
            balances: HashMap::new(),
        }
    }

    pub fn add_wallet(&mut self, address: String, balance: f64, tokens: Vec<String>) {
        self.balances.insert(address.clone(), balance);
        self.holdings.insert(address, tokens.into_iter().collect());
    }

    /// Find tokens held in common between wallets
    pub fn common_tokens(&self) -> Vec<(String, Vec<String>)> {
        let mut mint_holders: HashMap<String, Vec<String>> = HashMap::new();
        for (wallet, mints) in &self.holdings {
            for mint in mints {
                mint_holders.entry(mint.clone()).or_default().push(wallet.clone());
            }
        }
        mint_holders.into_iter()
            .filter(|(_, holders)| holders.len() > 1)
            .collect()
    }

    /// Score wallets by similarity (Jaccard index on token sets)
    pub fn similarity(&self, w1: &str, w2: &str) -> f64 {
        let empty = HashSet::new();
        let s1 = self.holdings.get(w1).unwrap_or(&empty);
        let s2 = self.holdings.get(w2).unwrap_or(&empty);
        let intersection = s1.intersection(s2).count() as f64;
        let union = s1.union(s2).count() as f64;
        if union == 0.0 { 0.0 } else { intersection / union }
    }

    /// Find whale wallets (top N by balance)
    pub fn whales(&self, n: usize) -> Vec<(String, f64)> {
        let mut sorted: Vec<_> = self.balances.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        sorted.truncate(n);
        sorted
    }

    /// Detect clusters — wallets that hold the same obscure tokens
    pub fn clusters(&self, min_shared: usize) -> Vec<Vec<String>> {
        let wallets: Vec<String> = self.holdings.keys().cloned().collect();
        let mut clusters: Vec<Vec<String>> = Vec::new();
        let mut visited = HashSet::new();

        for i in 0..wallets.len() {
            if visited.contains(&wallets[i]) { continue; }
            let mut cluster = vec![wallets[i].clone()];
            for j in (i+1)..wallets.len() {
                if visited.contains(&wallets[j]) { continue; }
                let s1 = self.holdings.get(&wallets[i]).unwrap();
                let s2 = self.holdings.get(&wallets[j]).unwrap();
                let shared = s1.intersection(s2).count();
                if shared >= min_shared {
                    cluster.push(wallets[j].clone());
                    visited.insert(wallets[j].clone());
                }
            }
            if cluster.len() > 1 {
                visited.insert(wallets[i].clone());
                clusters.push(cluster);
            }
        }
        clusters
    }
}

pub fn print_analysis(graph: &WalletGraph) {
    println!("\n🔗 Multi-Wallet Analysis");
    println!("═══════════════════════════════════════════");
    println!("  Wallets tracked: {}", graph.balances.len());

    let total_sol: f64 = graph.balances.values().sum();
    println!("  Total SOL: {:.4}", total_sol);

    let common = graph.common_tokens();
    if !common.is_empty() {
        println!("\n  🎯 Shared Tokens:");
        for (mint, holders) in &common {
            println!("    {}...{} held by {} wallets",
                &mint[..8], &mint[mint.len()-4..], holders.len());
        }
    }

    let clusters = graph.clusters(2);
    if !clusters.is_empty() {
        println!("\n  🕸️ Wallet Clusters (≥2 shared tokens):");
        for (i, cluster) in clusters.iter().enumerate() {
            println!("    Cluster {}: {} wallets", i+1, cluster.len());
            for w in cluster {
                println!("      {}...{}", &w[..8], &w[w.len()-4..]);
            }
        }
    }

    let whales = graph.whales(3);
    if !whales.is_empty() {
        println!("\n  🐋 Top Whales:");
        for (addr, bal) in &whales {
            println!("    {}...{}: {:.4} SOL",
                &addr[..8], &addr[addr.len()-4..], bal);
        }
    }
}
