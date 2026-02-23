# DigitalOcean Manual Deployment Runbook

This runbook describes an end-to-end, operator-driven deployment flow for running edgelord on a DigitalOcean Droplet with manual (click-run) GitHub Actions deployment.

## Goals

- Keep deployment safe and repeatable.
- Keep secrets encrypted and injected at runtime via dugout.
- Use manual workflow dispatch (no auto-deploy on push).
- Keep USDC ingress/egress operationally simple.

## Assumptions

- You are operating in a jurisdiction and exchange mode that is permitted for your user/entity.
- You already have:
  - A DigitalOcean Droplet you can SSH into.
  - A MetaMask-compatible wallet and funding source (for example Coinbase).
  - Access to this GitHub repository with Actions enabled.
  - [dugout](https://crates.io/crates/dugout) installed locally.
- Deployment is manual:
  - Triggered via `workflow_dispatch` from the GitHub Actions UI.
  - Protected with a GitHub Environment approval gate.
- Trading capital movement is manual:
  - USDC and MATIC are funded manually to the bot wallet.
  - Sweep is operator-initiated via CLI.

## Recommended Droplet Baseline

- Start: `2 vCPU / 4 GB RAM` (cost-effective baseline).
- Scale target: `4 vCPU / 8 GB RAM` if CPU pressure or latency degradation appears.
- Region: London (LON1) for lowest latency to Polymarket.

## Runtime Layout

```text
/opt/edgelord/
  current/                # symlink -> releases/<sha>
  releases/<git-sha>/     # immutable deployed releases
  config/config.toml      # production config
  data/edgelord.db        # runtime sqlite db
  .dugout.toml            # encrypted secrets vault
```

No repo clone needed - the workflow syncs everything.

## Phase 1: Local Secrets Setup

Before deploying, set up secrets locally with dugout:

```console
$ cargo install dugout
$ dugout setup                     # First-time identity
$ cd /path/to/edgelord
$ dugout init
$ dugout set WALLET_PRIVATE_KEY
$ dugout set TELEGRAM_BOT_TOKEN    # Optional
$ dugout set TELEGRAM_CHAT_ID      # Optional
$ git add .dugout.toml && git commit -m "chore: add secrets vault" && git push
```

## Phase 2: One-Time Host Bootstrap

SSH to your Droplet:

```console
$ sudo apt update && sudo apt upgrade -y
$ sudo apt install -y build-essential pkg-config libssl-dev curl
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ source ~/.cargo/env
$ cargo install dugout
```

Set up dugout identity on VPS. Copy your local identity:

```console
$ scp ~/.dugout/identity user@droplet:~/.dugout/identity
```

Or generate a new identity and add as recipient:

```console
$ dugout setup
$ dugout whoami    # Copy this public key, then locally run:
                   # dugout team add vps <key> && dugout sync && git push
```

## Phase 3: GitHub Actions Setup

### Required Secrets

Configure in Settings → Secrets → Actions:

| Secret | Description |
|--------|-------------|
| `DEPLOY_HOST` | Droplet IP or hostname |
| `DEPLOY_USER` | SSH user (e.g., `root`) |
| `DEPLOY_SSH_PORT` | SSH port (usually 22) |
| `DEPLOY_SSH_KEY` | Private SSH key (ed25519) |
| `DEPLOY_KNOWN_HOSTS` | Output of `ssh-keyscan -p PORT HOST` |
| `DEPLOY_PATH` | `/opt/edgelord` |

Do NOT store in GitHub:
- Wallet private key
- API keys
- Any secrets (these are in dugout vault)

### First Deploy

1. Go to GitHub → Actions → `Manual Deploy`
2. Click `Run workflow`
3. Configure:
   - `mode = deploy`
   - `ref = main`
   - `environment = production`
   - `confirm = deploy-production`
   - Strategy toggles as needed
   - `dugout = true` (default)
4. Approve environment gate
5. Watch workflow to completion

### Available Runtime Overrides

The workflow supports CLI overrides that get baked into the systemd service:

| Input | Description | Example |
|-------|-------------|---------|
| `dry_run` | Detect but don't trade | `true` |
| `telegram_enabled` | Enable Telegram notifications | `true` |
| `min_edge` | Minimum edge threshold | `0.05` |
| `min_profit` | Minimum profit threshold | `0.50` |
| `max_exposure` | Maximum total exposure | `5000` |
| `max_position` | Maximum position per market | `500` |
| `max_slippage` | Maximum slippage tolerance | `0.02` |
| `max_markets` | Maximum markets to track | `100` |
| `max_connections` | Maximum WebSocket connections | `5` |
| `execution_timeout` | Execution timeout in seconds | `30` |

Example conservative first deploy:
- `dry_run = true`
- `max_exposure = 1000`
- `max_position = 100`
- `max_slippage = 0.02`
- `max_markets = 50`

After validation, redeploy with `dry_run = false` and higher limits.

The workflow will:
- Build and test the binary
- Upload binary, dugout vault, and config to VPS
- Create directories if needed
- Install systemd service
- Start and verify health

### Subsequent Deploys

Same process. The workflow:
- Always syncs the binary and dugout vault
- Preserves your existing `config.toml` customizations
- Restarts the service

## Phase 4: Fund the Wallet

Get your wallet address:

```console
$ ssh your-droplet
$ cd /opt/edgelord
$ dugout run -- edgelord wallet address --config config/config.toml
```

Send Polygon USDC and MATIC to the wallet address. Verify:

```console
$ dugout run -- edgelord wallet status --config config/config.toml
```

## Operational Commands

```console
$ ssh your-droplet
$ cd /opt/edgelord
```

Commands that need secrets:

```console
$ dugout run -- edgelord wallet status --config config/config.toml
$ dugout run -- edgelord check health --config config/config.toml
$ dugout run -- edgelord check live --config config/config.toml
```

Commands that don't need secrets:

```console
$ edgelord status --db data/edgelord.db
$ edgelord statistics today --db data/edgelord.db
$ sudo systemctl status edgelord
$ journalctl -u edgelord -f
```

## Rollback Procedure

Via GitHub Actions:
1. Run `Manual Deploy` with `mode = rollback_previous`
2. Set `confirm = deploy-production`

Emergency host-side rollback:

```console
$ ls -lt /opt/edgelord/releases/
$ sudo ln -sfn /opt/edgelord/releases/<previous-sha> /opt/edgelord/current
$ sudo systemctl restart edgelord
```

## USDC Ingress/Egress

**Ingress:** Send USDC + MATIC manually from your custody wallet.

**Egress:**

```console
$ cd /opt/edgelord
$ dugout run -- edgelord wallet sweep --to <address> --yes --config config/config.toml
```

## Updating Secrets

```console
$ dugout set NEW_SECRET
$ git add .dugout.toml && git commit -m "chore: update secrets" && git push
```

Then run any deploy—vault syncs automatically.

## Operational Guardrails

- Start new environments in `dry_run = true`
- Use low exposure caps initially via workflow inputs
- Review logs and daily stats before raising limits
- Keep dugout identity file `chmod 600 ~/.dugout/identity`
- Keep SSH access key-only and restricted
- Update vault recipients when team changes: `dugout team remove <name>`

### Recommended Rollout Pattern

1. **Initial deploy**: `dry_run=true`, `max_markets=50`, observe for 24h
2. **Validation**: Review `journalctl -u edgelord -f` and verify strategies detect opportunities
3. **Low-risk live**: `dry_run=false`, `max_exposure=500`, `max_position=100`
4. **Scale up**: Gradually increase limits as confidence grows
5. **Production**: Full limits, `telegram_enabled=true` for alerts

All changes are made via workflow redeploys - no SSH required for config changes.
