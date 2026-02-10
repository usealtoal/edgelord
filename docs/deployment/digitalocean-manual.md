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

```bash
# Install dugout if not already installed
cargo install dugout

# Initialize your identity (first time only)
dugout setup

# Initialize dugout in the project
cd /path/to/edgelord
dugout init

# Add your secrets
dugout set WALLET_PRIVATE_KEY      # Your trading wallet private key
dugout set TELEGRAM_BOT_TOKEN      # Optional: Telegram bot token
dugout set TELEGRAM_CHAT_ID        # Optional: Telegram chat ID
dugout set ANTHROPIC_API_KEY       # Optional: For LLM inference
dugout set OPENAI_API_KEY          # Optional: For LLM inference

# Commit the encrypted vault
git add .dugout.toml
git commit -m "feat: add encrypted secrets vault"
git push
```

## Phase 2: One-Time Host Bootstrap

SSH to your Droplet and run:

```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential pkg-config libssl-dev curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install dugout
cargo install dugout
```

Set up dugout identity on VPS:

```bash
# Option A: Copy your local identity (simpler)
# Run this from your local machine:
scp ~/.dugout/identity user@droplet:~/.dugout/identity

# Option B: Generate new identity and add as recipient
dugout setup
dugout whoami  # Copy this public key

# Then locally:
dugout team add vps <vps-public-key>
dugout sync
git add .dugout.toml && git commit -m "chore: add vps as recipient" && git push
```

That's it for VPS setup. The workflow handles everything else.

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

```bash
ssh your-droplet
cd /opt/edgelord
dugout run -- edgelord wallet address --config /opt/edgelord/config/config.toml
```

Fund manually:
- Send Polygon USDC to the wallet address for trading capital.
- Send MATIC to the same wallet for gas.

Verify:

```bash
dugout run -- edgelord wallet status --config /opt/edgelord/config/config.toml
```

## Operational Commands

SSH to VPS and cd to deploy directory:

```bash
ssh your-droplet
cd /opt/edgelord
```

Commands that need secrets:

```bash
# Start a shell with secrets loaded
dugout env

# Then run any command
edgelord wallet status --config /opt/edgelord/config/config.toml
edgelord check live --config /opt/edgelord/config/config.toml
```

Commands that don't need secrets:

```bash
edgelord status --db /opt/edgelord/data/edgelord.db
edgelord statistics today --db /opt/edgelord/data/edgelord.db
edgelord logs --follow
sudo systemctl status edgelord
```

## Rollback Procedure

Preferred (via GitHub Actions):

1. Run `Manual Deploy` with `mode = rollback_previous`
2. Set `confirm = deploy-production`
3. Approve and complete workflow

Emergency host-side rollback:

```bash
# List available releases
ls -lt /opt/edgelord/releases/

# Point to previous release
sudo ln -sfn /opt/edgelord/releases/<previous-sha> /opt/edgelord/current
sudo systemctl restart edgelord
```

## USDC Ingress/Egress

Keep this manual unless you have additional controls:

**Ingress:**
- Send USDC + MATIC manually from your custody wallet.

**Egress:**

```bash
cd /opt/edgelord
dugout run -- edgelord wallet sweep \
  --to <destination_address> \
  --asset usdc \
  --network polygon \
  --yes \
  --config /opt/edgelord/config/config.toml
```

## Updating Secrets

When you add or change secrets:

```bash
# Locally
dugout set NEW_SECRET
git add .dugout.toml && git commit -m "chore: update secrets" && git push

# Then run any deploy - vault is synced automatically
```

## Operational Guardrails

- Start new environments in `dry_run = true`
- Use low exposure caps initially
- Review logs and daily stats before raising limits
- Keep dugout identity file `chmod 600 ~/.dugout/identity`
- Keep SSH access key-only and restricted
- Update vault recipients when team changes: `dugout team remove <name>`
