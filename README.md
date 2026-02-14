# 🔍 solscan-cli

Scan any Solana wallet from your terminal. Balances, tokens, transactions, DeFi positions — one command.

## Install

```bash
cargo install solscan-cli
```

## Usage

```bash
# Basic scan — SOL balance
solscan <wallet_address>

# Show all token holdings
solscan <wallet_address> --tokens

# Show recent transaction history
solscan <wallet_address> --history

# Show DeFi positions (Marinade, Jito, Raydium)
solscan <wallet_address> --defi

# Everything at once
solscan <wallet_address> --tokens --history --defi
```

## Example

```
$ solscan EXEDJvuAaYt9yN5mwZRPdCP19tYuF6LWztnu6qpbepTq --tokens

╔══════════════════════════════════════════════════════════════╗
║  🔍 Solana Wallet Scanner                                   ║
╠══════════════════════════════════════════════════════════════╣
║  Address: EXEDJvuA...u6qpbepTq
║  SOL Balance: 0.003254 SOL
╚══════════════════════════════════════════════════════════════╝

📦 Token Accounts:
─────────────────────────────────────────────────
       Balance    Decimals  Mint
       ───────    ────────  ────
  900000000.0           6  C9vx1mu1...xCrzVY
       6076.0           6  EoP9nKZM...Vpump
        510.0           6  9S8edqWx...Hpump

  Total token accounts: 3
```

## Features

- ✅ SOL balance
- ✅ SPL Token accounts (Token + Token-2022)
- ✅ Recent transaction history with status
- ✅ DeFi position detection (mSOL, jitoSOL)
- 🔜 Full DeFi scanning (Raydium LP, Orca positions)
- 🔜 Token price lookup via Jupiter
- 🔜 Portfolio value in USD
- 🔜 Export to CSV/JSON

## Why?

Every Solana dev lives in the terminal. But to check a wallet, you open a browser, navigate to Solscan or Explorer, paste the address... 

`solscan` brings wallet inspection to where you already are.

## Support

**Solana:** `EXEDJvuAaYt9yN5mwZRPdCP19tYuF6LWztnu6qpbepTq`

## License

MIT
