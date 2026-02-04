# Wallet Setup

Polymarket runs on **Polygon** (an Ethereum L2). You need an Ethereum-compatible wallet funded with USDC and MATIC.

## Choose a Wallet

| Wallet | Type | Notes |
|--------|------|-------|
| [MetaMask](https://metamask.io/) | Browser extension | Most common, easy setup |
| [Rabby](https://rabby.io/) | Browser extension | Better UX than MetaMask |
| [Rainbow](https://rainbow.me/) | Mobile | Good mobile option |
| [Frame](https://frame.sh/) | Desktop app | Privacy-focused |
| Raw keypair | CLI | Generate with `cast wallet new` |

**Important**: Use a dedicated wallet for trading, not your main holdings.

## MetaMask Setup

### 1. Install MetaMask

1. Go to [metamask.io](https://metamask.io/)
2. Install browser extension
3. Create new wallet or import existing
4. **Save your seed phrase securely**

### 2. Add Polygon Network

1. Click network dropdown (top, says "Ethereum Mainnet")
2. Click "Add network"
3. Search for "Polygon" or add manually:

```
Network Name: Polygon
RPC URL: https://polygon-rpc.com
Chain ID: 137
Symbol: MATIC
Explorer: https://polygonscan.com
```

### 3. Add USDC Token

USDC may not appear automatically. To add it:

1. Switch to Polygon network
2. Click "Import tokens"
3. Paste USDC contract address:
   ```
   0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359
   ```
4. Click "Add Custom Token"

**Note**: This is native USDC on Polygon. There's also USDC.e (bridged) — both work on Polymarket.

## Alternative: Generate Raw Wallet

If you prefer command line:

```bash
# Install foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Generate wallet
cast wallet new
```

Output:
```
Successfully created new keypair.
Address:     0x7a3B...
Private key: 0xabc123...
```

Save the private key (without `0x` prefix) for edgelord config.

## Funding Your Wallet

You need:
- **USDC** — Trading capital
- **MATIC** — Gas fees (~$5-10 worth)

### Option A: Exchange Withdrawal (Recommended)

1. Buy USDC on Coinbase, Kraken, or Binance
2. Withdraw to your wallet address
3. **Select Polygon network** (not Ethereum!)
4. Buy small amount of MATIC, withdraw same way

Exchanges with Polygon support:
- Coinbase
- Kraken
- Binance
- Crypto.com

### Option B: Card Purchase

Buy directly with card via:
- [MoonPay](https://www.moonpay.com/)
- [Transak](https://transak.com/)
- MetaMask's built-in "Buy" button

### Option C: Bridge from Ethereum

If you have funds on Ethereum mainnet:
- [Polygon Bridge](https://wallet.polygon.technology/bridge) — Official
- [Jumper](https://jumper.exchange/) — Aggregator
- [Bungee](https://bungee.exchange/) — Aggregator

Note: Bridging from Ethereum costs ETH gas fees.

## Verify Funds

1. Switch MetaMask to Polygon network
2. Check MATIC balance shows
3. Check USDC balance shows (import token if needed)
4. Verify on [Polygonscan](https://polygonscan.com/) — paste your address

## Export Private Key

Edgelord needs your private key to sign transactions.

### From MetaMask

1. Click account menu (three dots)
2. Account details
3. "Show private key"
4. Enter password
5. Copy key (without 0x prefix)

### Security

- Never share your private key
- Use a dedicated trading wallet
- Start with small amounts
- Store key in `.env` file with `chmod 600`

## How Much to Start?

| Purpose | Amount |
|---------|--------|
| Gas (MATIC) | $5-10 |
| Trading (USDC) | Start small: $100-500 |

The bot's `max_total_exposure` config limits open positions. Start conservative until you verify everything works.

## Polymarket Approval

Before trading, Polymarket contracts need approval to spend your USDC:

```bash
# The bot handles this automatically, or manually:
./target/release/edgelord approve --amount 1000
```

This is a one-time transaction per wallet.
