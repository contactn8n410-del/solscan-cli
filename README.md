# solscan-cli 🔍

**Scan any Solana wallet from the command line.**

A fast, lightweight Rust CLI that queries Solana directly via JSON-RPC — no SDK dependencies, no API keys, no bloat.

## Features

- ✅ **SOL balance** — instant lookup
- ✅ **All tokens** — SPL + Token-2022 (pump.fun tokens included)
- ✅ **Transaction history** — recent signatures with status
- ✅ **DeFi positions** — detects mSOL, jitoSOL, and liquid staking
- ✅ **JSON output** — pipe into jq, scripts, dashboards
- ✅ **Tiny binary** — minimal deps (tokio + reqwest + serde_json)

## Install

```bash
cargo install --git https://github.com/contactn8n410-del/solscan-cli
```

Or build from source:

```bash
git clone https://github.com/contactn8n410-del/solscan-cli
cd solscan-cli
cargo build --release
```

## Usage

```bash
# Basic — show SOL balance
solscan <WALLET_ADDRESS>

# Show all token accounts
solscan <WALLET_ADDRESS> --tokens

# Show recent transactions
solscan <WALLET_ADDRESS> --history

# Show DeFi positions
solscan <WALLET_ADDRESS> --defi

# JSON output (for scripting)
solscan <WALLET_ADDRESS> --tokens --json

# Everything at once
solscan <WALLET_ADDRESS> --tokens --history --defi
```

## Example

```
$ solscan EXEDJvuAaYt9yN5mwZRPdCP19tYuF6LWztnu6qpbepTq --tokens
╔══════════════════════════════════════════════════════════════╗
║  🔍 Solana Wallet Scanner                                   ║
╠══════════════════════════════════════════════════════════════╣
║  Address: EXEDJvuA...6qpbepTq
║  SOL Balance: 0.003254 SOL
╚══════════════════════════════════════════════════════════════╝

📦 Token Accounts:
─────────────────────────────────────────────────
       Balance    Decimals  Mint
       ───────    ────────  ────
    6076.10756           6  EoP9nKZM...pump
     900000000           6  C9vx1mu1...rzVY
    510.286342           6  9S8edqWx...pump

  Total token accounts: 3
```

## Why?

Existing tools either:
- Require API keys (Helius, QuickNode)
- Are web-only (solscan.io, solana.fm)
- Need the full Solana SDK (~100+ deps)

**solscan-cli** talks directly to public RPC endpoints. No keys. No accounts. Just `cargo install` and go.

## Custom RPC

Set `SOLANA_RPC_URL` environment variable to use your own endpoint:

```bash
export SOLANA_RPC_URL=https://your-rpc.example.com
solscan <ADDRESS> --tokens
```

## Support

If solscan-cli is useful to you:

**SOL**: `EXEDJvuAaYt9yN5mwZRPdCP19tYuF6LWztnu6qpbepTq`

**ETH/Base**: `0x0282BdE2f138babC6ABa3bb010121112cC1d7eDa`

Or [sponsor on GitHub](https://github.com/sponsors/contactn8n410-del).

## License

MIT
