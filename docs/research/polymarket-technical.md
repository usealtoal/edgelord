# Polymarket Technical Infrastructure

## Overview

Polymarket uses a **hybrid-decentralized CLOB** (Central Limit Order Book):
- **Off-chain:** Order matching and ordering
- **On-chain:** Settlement via signed EIP-712 messages (non-custodial)

---

## Contract Addresses

| Network | Address |
|---------|---------|
| **Polygon (mainnet)** | `0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E` |
| **Amoy (testnet)** | `0xdFE02Eb6733538f8Ea35D585af8DE5958AD99E40` |

Event contract for historical data:
- `0x4D97DCd97eC945f40cF65F87097ACe5EA0476045`

Events to monitor:
- `OrderFilled` — trades executed
- `PositionSplit` — new tokens minted
- `PositionsMerge` — tokens burned

---

## API Endpoints

Base documentation: https://docs.polymarket.com

### REST API
| Endpoint | Purpose |
|----------|---------|
| Market data | `/markets/{id}`, `/markets?slug={slug}` |
| Order book | `/book?token_id={id}` |
| Prices | `/prices?token_ids={ids}` |
| Historical | `/timeseries?token_id={id}` |

### WebSocket
**Endpoint:** `wss://ws-subscriptions-clob.polymarket.com/ws/`

**Channels:**
- `market` — Order book updates, price changes (public)
- `user` — Order status updates (authenticated)

**Subscription message:**
```json
{
  "auth": { ... },
  "type": "MARKET",
  "assets_ids": ["token_id_1", "token_id_2"]
}
```

**Dynamic subscription:**
```json
{
  "assets_ids": ["new_token_id"],
  "operation": "subscribe"
}
```

---

## Token Structure

- **Collateral:** ERC20 (USDC)
- **Outcome tokens:** CTF ERC1155 (Conditional Tokens Framework)
- **Settlement:** Atomic swaps between outcome tokens and collateral

---

## Fees

As of current docs: **0% maker and taker fees**

(This may change — verify before deploying)

---

## Authentication Levels

| Level | Access |
|-------|--------|
| **Public** | Market data, prices, order books |
| **L1** | Wallet signer setup (private key) |
| **L2** | User API credentials for trading |
| **Builder** | Special credentials for order attribution |

---

## Client Libraries

### Official
- **Rust:** [rs-clob-client](https://github.com/Polymarket/rs-clob-client)
- **Exchange contracts:** [ctf-exchange](https://github.com/Polymarket/ctf-exchange)

### Third-party
- **Python:** [polymarket-apis](https://pypi.org/project/polymarket-apis/)
- **NautilusTrader:** [Integration docs](https://nautilustrader.io/docs/latest/integrations/polymarket/)

---

## Rate Limits

See [Polymarket Rate Limits](https://docs.polymarket.com/developers/quickstart/introduction/rate-limits) for current limits.

---

## References

- [CLOB Introduction](https://docs.polymarket.com/developers/CLOB/introduction)
- [WSS Overview](https://docs.polymarket.com/developers/CLOB/websocket/wss-overview)
- [Market Channel](https://docs.polymarket.com/developers/CLOB/websocket/market-channel)
- [Trading Docs](https://docs.polymarket.com/developers/market-makers/trading)
