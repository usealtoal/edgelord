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
  current/                # active release symlink -> releases/<sha>
  releases/<git-sha>/     # immutable deployed releases
  config/config.toml      # persistent runtime config
  repo/                   # git clone for dugout vault access
  data/edgelord.db        # runtime sqlite db
```

No `.env` file needed - secrets are injected by dugout at runtime.

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
sudo apt install -y build-essential pkg-config libssl-dev curl git
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install dugout
cargo install dugout

# Install GitHub CLI (for private repo access)
curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null
sudo apt update && sudo apt install gh
gh auth login  # Choose HTTPS, follow prompts
```

Create directories:

```bash
sudo mkdir -p /opt/edgelord/{releases,config,data}
sudo chown -R "$USER":"$USER" /opt/edgelord
```

Clone repo for dugout vault access:

```bash
git clone https://github.com/usealtoal/edgelord.git /opt/edgelord/repo
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

Build initial release:

```bash
cd /opt/edgelord/repo
cargo build --release
mkdir -p /opt/edgelord/releases/initial
cp target/release/edgelord /opt/edgelord/releases/initial/
ln -sfn /opt/edgelord/releases/initial /opt/edgelord/current
sudo ln -sf /opt/edgelord/current/edgelord /usr/local/bin/edgelord
```

## Phase 3: Production Config

Create the runtime config:

```bash
cp /opt/edgelord/repo/deploy/config.prod.toml /opt/edgelord/config/config.toml
```

Edit for your environment:

```bash
nano /opt/edgelord/config/config.toml
```

Key settings for live mode:

```toml
dry_run = false

[exchange_config]
environment = "mainnet"
chain_id = 137

database = "/opt/edgelord/data/edgelord.db"

[risk]
max_position_per_market = 100.0
max_total_exposure = 500.0
```

## Phase 4: Validate and Install Service

Pull latest vault and validate:

```bash
cd /opt/edgelord/repo && git pull

# Validate config (no secrets needed)
edgelord check config --config /opt/edgelord/config/config.toml

# Validate connectivity (needs secrets)
dugout run -- edgelord check connection --config /opt/edgelord/config/config.toml
dugout run -- edgelord check live --config /opt/edgelord/config/config.toml
```

Install service with dugout:

```bash
sudo edgelord service install \
  --config /opt/edgelord/config/config.toml \
  --user "$USER" \
  --working-dir /opt/edgelord/repo \
  --dugout
```

**Notes:**
- `--working-dir` points to the repo clone so dugout can find `.dugout.toml`
- `--user "$USER"` runs the service as your current user so dugout can access `~/.dugout/identity`

Start and verify:

```bash
sudo systemctl start edgelord
sudo systemctl status edgelord --no-pager
edgelord logs --follow
```

## Phase 5: Fund the Wallet

Get your wallet address:

```bash
dugout run -- edgelord wallet address --config /opt/edgelord/config/config.toml
```

Fund manually:

- Send Polygon USDC to the wallet address for trading capital.
- Send MATIC to the same wallet for gas.

Verify:

```bash
dugout run -- edgelord wallet status --config /opt/edgelord/config/config.toml
```

## Phase 6: GitHub Actions Deploy

### Required GitHub Secrets

Configure in Settings → Secrets → Actions:

| Secret | Description |
|--------|-------------|
| `DEPLOY_HOST` | Droplet IP or hostname |
| `DEPLOY_USER` | SSH user |
| `DEPLOY_SSH_PORT` | SSH port (usually 22) |
| `DEPLOY_SSH_KEY` | Private SSH key (ed25519) |
| `DEPLOY_KNOWN_HOSTS` | Output of `ssh-keyscan -p PORT HOST` |
| `DEPLOY_PATH` | `/opt/edgelord` |

Do NOT store in GitHub:
- Wallet private key
- API keys
- Any secrets (these are in dugout vault)

### Deploy Procedure

1. Push code to `main` (or chosen branch/tag).
2. Open GitHub → Actions → `Manual Deploy`.
3. Click `Run workflow`.
4. Configure:
   - `mode = deploy`
   - `ref = <target git ref>`
   - `environment = production`
   - `confirm = deploy-production`
   - Strategy toggles as needed
   - `dugout = true` (default)
5. Approve environment gate.
6. Watch workflow to completion.
7. Verify on host:

```bash
systemctl status edgelord
edgelord status --db /opt/edgelord/data/edgelord.db
edgelord logs --follow
```

### Rollback Procedure

Preferred (via GitHub Actions):

1. Run `Manual Deploy` with `mode = rollback_previous`.
2. Set `confirm = deploy-production`.
3. Approve and complete workflow.

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
dugout run -- edgelord wallet sweep \
  --to <destination_address> \
  --asset usdc \
  --network polygon \
  --yes \
  --config /opt/edgelord/config/config.toml
```

## Operational Commands

```bash
# Start a shell with secrets loaded
dugout env

# Then run any command
edgelord wallet status --config /opt/edgelord/config/config.toml
edgelord check live --config /opt/edgelord/config/config.toml

# Commands that don't need secrets
edgelord status --db /opt/edgelord/data/edgelord.db
edgelord statistics today --db /opt/edgelord/data/edgelord.db
edgelord logs --follow
```

## Operational Guardrails

- Start new environments in `dry_run = true`.
- Use low exposure caps initially.
- Review logs and daily stats before raising limits.
- Keep dugout identity file `chmod 600 ~/.dugout/identity`.
- Keep SSH access key-only and restricted.
- Update vault recipients when team changes: `dugout team remove <key>`.
