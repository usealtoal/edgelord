# Deployment Guide

Production deployment for running edgelord against live Polymarket.

## Regulatory Note

Polymarket restricts access from the United States. The web interface geo-blocks US IPs, though the underlying smart contracts on Polygon are permissionless. US persons should consult legal counsel regarding regulatory compliance before trading. This guide is technical documentation, not legal advice.

## What You Need

1. **Crypto wallet** — Ethereum-compatible (MetaMask, Rabby, or raw private key)
2. **USDC on Polygon** — Polymarket uses USDC on Polygon network
3. **MATIC on Polygon** — Small amount for gas fees
4. **VPS outside US** — For API access and low latency

## Guides

| Guide | Description |
|-------|-------------|
| [Wallet Setup](wallet.md) | Create wallet, add Polygon, fund with USDC |
| [Infrastructure](infrastructure.md) | VPS selection, server setup, systemd service |
| [Telegram Alerts](telegram.md) | Bot creation, notifications setup |
| [Operations](operations.md) | Monitoring, security, troubleshooting |

## Quick Start

1. **Set up wallet** — [wallet.md](wallet.md)
   - Install MetaMask
   - Add Polygon network
   - Fund with USDC + MATIC

2. **Provision server** — [infrastructure.md](infrastructure.md)
   - Spin up VPS in Europe
   - Install Rust, build edgelord
   - Configure and run as service

3. **Enable alerts** — [telegram.md](telegram.md)
   - Create Telegram bot
   - Configure notifications

4. **Go live** — [operations.md](operations.md)
   - Security checklist
   - Monitoring setup
