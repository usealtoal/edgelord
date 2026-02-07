# Deployment Guide

This section covers production deployment and operations for edgelord.

## Deployment Tracks

- [Wallet Setup](wallet.md)
  - Prepare a dedicated Polygon wallet and funding model.
- [Infrastructure](infrastructure.md)
  - Provision a VPS, install runtime dependencies, and run as a service.
- [Telegram Alerts](telegram.md)
  - Configure alerting for execution and risk events.
- [Operations](operations.md)
  - Monitoring, incident response, and hardening checklist.

## Live Trading Prerequisites

- Non-testnet configuration (`environment = "mainnet"`, `chain_id = 137`)
- `dry_run = false`
- Wallet available through private key or keystore
- `edgelord check live --config <path>` returns no blockers

## Regulatory and Compliance Note

This documentation is technical guidance, not legal advice. Ensure your operating jurisdiction and user profile are compatible with the exchanges and assets you use.
