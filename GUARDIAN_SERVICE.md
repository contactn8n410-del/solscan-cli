# 🔮 Solana Guardian — DeFi Security Monitoring Service

## What We Found

We scanned the top 15 Solana DeFi protocols and discovered:

- **87% are upgradeable** — code can be changed by a single authority key
- **Raydium CPMM + AMM v4 share the SAME authority key** — single point of failure for 2 major protocols
- **8 authority wallets are dormant** — zero recent transactions, unknown key status
- Only **Orca Token Swap** and **Mango Markets v3** are fully immutable

Full report: [SECURITY_REPORT.md](SECURITY_REPORT.md)

## The Problem

DeFi users have no way to know when:
- A protocol's code is **silently upgraded**
- An upgrade authority **changes hands**
- An authority wallet shows **suspicious activity**
- A previously immutable program is **somehow modified**

## Our Solution: Guardian

`solscan --guardian` monitors all major Solana DeFi protocols continuously and alerts on:

| Alert | Severity | Example |
|-------|----------|---------|
| Authority changed | 🔴 CRITICAL | Someone took control of the upgrade key |
| Program upgraded | 🟠 HIGH | New code deployed to production |
| Authority balance shift | 🟡 MEDIUM | Large SOL movement on authority wallet |
| Immutable → Upgradeable | 🔴 CRITICAL | Should be impossible — indicates attack |

## Free Tier

```bash
cargo install --git https://github.com/contactn8n410-del/solscan-cli
solscan x --guardian --every 300
```

Run it yourself. Open source. Free forever.

## Pro Tier (Coming Soon)

For protocols and funds that need more:

- **Real-time monitoring** (5-second intervals)
- **Custom program watchlist** (your contracts, not just top 15)
- **Webhook/Telegram/Discord alerts** 
- **Historical authority change database**
- **Upgrade diff analysis** (what changed in the code)
- **Authority wallet profiling** (behavioral patterns)
- **Monthly security report** with recommendations

### Pricing

| Plan | Price | For |
|------|-------|-----|
| Open Source | Free | Individual users, researchers |
| Protocol | $500/mo | DeFi protocols monitoring their own + competitors |
| Fund | $2,000/mo | Funds monitoring their portfolio exposure |
| Enterprise | Custom | Exchanges, custodians, compliance |

## Contact

- GitHub: [@contactn8n410-del](https://github.com/contactn8n410-del)
- Email: contact.n8n410@gmail.com

## Why Trust Us?

- All findings are **reproducible** — run the tool yourself
- Open source core — **verify** every detection
- Built by security researchers, not marketers
- Our reports led to issues on [Raydium](https://github.com/raydium-io/raydium-cp-swap/issues/66) and [Meteora](https://github.com/MeteoraAg/dlmm-sdk/issues/269)
