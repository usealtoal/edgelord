# DigitalOcean Manual Deployment Runbook

This runbook describes an end-to-end, operator-driven deployment flow for running edgelord on a DigitalOcean Droplet with manual (click-run) GitHub Actions deployment.

## Goals

- Keep deployment safe and repeatable.
- Keep wallet secrets on the VPS only.
- Use manual workflow dispatch (no auto-deploy on push).
- Keep USDC ingress/egress operationally simple.

## Assumptions

- You are operating in a jurisdiction and exchange mode that is permitted for your user/entity.
- You already have:
  - A DigitalOcean Droplet you can SSH into.
  - A MetaMask-compatible wallet and funding source (for example Coinbase).
  - Access to this GitHub repository with Actions enabled.
- Deployment is manual:
  - Triggered via `workflow_dispatch` from the GitHub Actions UI.
  - Protected with a GitHub Environment approval gate.
- Trading capital movement is manual:
  - USDC and MATIC are funded manually to the bot wallet.
  - Sweep is operator-initiated via CLI.

## Recommended Droplet Baseline

- Start: `2 vCPU / 4 GB RAM` (cost-effective baseline).
- Scale target: `4 vCPU / 8 GB RAM` if CPU pressure or latency degradation appears.
- Region: choose the lowest-latency legally valid region for your operating model.

## Runtime Layout (Recommended)

```text
/opt/edgelord/
  current/                # active release symlink
  releases/<git-sha>/     # immutable deployed releases
  config/config.toml      # persistent runtime config
  secrets/keystore.json   # encrypted wallet keystore (600)
  secrets/keystore.pass   # keystore password file (600)
  data/edgelord.db        # runtime sqlite db
  .env                    # runtime env file (600)
```

## Phase 1: One-Time Host Bootstrap

```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential pkg-config libssl-dev curl git
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

Create directories and permissions:

```bash
sudo mkdir -p /opt/edgelord/{releases,config,secrets,data}
sudo chown -R "$USER":"$USER" /opt/edgelord
chmod 700 /opt/edgelord/secrets
```

Clone and initial build:

```bash
git clone https://github.com/usealtoal/edgelord.git /opt/edgelord/src
cd /opt/edgelord/src
cargo build --release
```

## Phase 2: Production Config

Create the runtime config:

```bash
cp /opt/edgelord/src/config.toml.example /opt/edgelord/config/config.toml
```

For live mode, set:

```toml
dry_run = false

[exchange_config]
environment = "mainnet"
chain_id = 137
```

Point database to persistent storage:

```toml
database = "/opt/edgelord/data/edgelord.db"
```

Keep conservative initial risk settings:

```toml
[risk]
max_position_per_market = 100.0
max_total_exposure = 500.0
```

## Phase 3: Wallet Provisioning (On VPS)

### Option A: Import Existing Wallet Key

```bash
export EDGELORD_PRIVATE_KEY="0x..."
export EDGELORD_KEYSTORE_PASSWORD="strong-password"
/opt/edgelord/src/target/release/edgelord provision polymarket \
  --wallet import \
  --config /opt/edgelord/config/config.toml \
  --keystore-path /opt/edgelord/secrets/keystore.json
unset EDGELORD_PRIVATE_KEY
```

### Option B: Generate New Wallet

```bash
export EDGELORD_KEYSTORE_PASSWORD="strong-password"
/opt/edgelord/src/target/release/edgelord provision polymarket \
  --config /opt/edgelord/config/config.toml \
  --keystore-path /opt/edgelord/secrets/keystore.json
```

Persist keystore password via file:

```bash
printf '%s\n' "strong-password" > /opt/edgelord/secrets/keystore.pass
chmod 600 /opt/edgelord/secrets/keystore.pass
```

Set runtime env:

```bash
cat > /opt/edgelord/.env <<'EOF'
EDGELORD_KEYSTORE_PASSWORD_FILE=/opt/edgelord/secrets/keystore.pass
EOF
chmod 600 /opt/edgelord/.env
```

Verify wallet:

```bash
/opt/edgelord/src/target/release/edgelord wallet address --config /opt/edgelord/config/config.toml
/opt/edgelord/src/target/release/edgelord wallet status --config /opt/edgelord/config/config.toml
```

Fund manually:

- Send Polygon USDC to the wallet address for trading capital.
- Send MATIC to the same wallet for gas.

## Phase 4: Service Install and First Start

Install service:

```bash
sudo /opt/edgelord/src/target/release/edgelord service install \
  --config /opt/edgelord/config/config.toml \
  --user "$USER" \
  --working-dir /opt/edgelord
```

Validate before starting:

```bash
/opt/edgelord/src/target/release/edgelord check config --config /opt/edgelord/config/config.toml
/opt/edgelord/src/target/release/edgelord check connection --config /opt/edgelord/config/config.toml
/opt/edgelord/src/target/release/edgelord check live --config /opt/edgelord/config/config.toml
```

Start and verify:

```bash
sudo systemctl start edgelord
sudo systemctl status edgelord --no-pager
/opt/edgelord/src/target/release/edgelord logs --follow
```

## Phase 5: Manual GitHub Actions Deploy Model

This runbook assumes the repository deploy workflow at `.github/workflows/deploy.yml` with:

- Trigger: `workflow_dispatch` only.
- Inputs:
  - `ref` (branch/tag/sha to deploy)
  - `environment` (for example `production`)
  - `mode` (`deploy`, `validate_only`, `restart_only`, `rollback_previous`)
  - `confirm` (must equal `deploy-production` for prod)
  - `run_connection_check` (optional boolean)
  - `run_live_check` (optional boolean)
  - `change_note` (optional free text)
- Environment protection:
  - Required reviewer approval before production deploy.

### Required GitHub Secrets/Variables

Deploy-only values (safe for GitHub):

- `DEPLOY_HOST`
- `DEPLOY_USER`
- `DEPLOY_SSH_PORT`
- `DEPLOY_SSH_KEY`
- `DEPLOY_KNOWN_HOSTS` (pinned host key entry)
- `DEPLOY_PATH` (for example `/opt/edgelord`)

Do not store in GitHub:

- Wallet private key
- Keystore password
- Keystore file

### Expected Deploy Workflow Behavior

On manual click-run:

1. Validate confirmation phrase and environment gate.
2. Execute action by mode:
   - `deploy`: build/test binary, upload release, switch symlink, run checks, restart service.
   - `validate_only`: run remote checks only (no binary change, no restart).
   - `restart_only`: reinstall/restart service from current release only.
   - `rollback_previous`: switch symlink to previous release, run checks, restart service.
3. Verify health:
   - `systemctl is-active edgelord`

If verification fails:

1. Restore previous symlink.
2. Restart service on previous release.
3. Mark workflow failed.

## Operator Deploy Procedure (Click Path)

1. Push code to `main` (or chosen branch/tag).
2. Open GitHub -> Actions -> `Manual Deploy`.
3. Click `Run workflow`.
4. Choose:
   - `mode = deploy`
   - `ref = <target git ref>`
   - `environment = production`
   - `confirm = deploy-production`
5. (Optional) set `run_connection_check` / `run_live_check`.
6. Approve environment gate.
7. Watch workflow to completion.
8. Verify on host:
   - `systemctl status edgelord`
   - `edgelord status --db /opt/edgelord/data/edgelord.db`
   - `edgelord logs --follow`

## Rollback Procedure (Manual)

Preferred:

1. Re-run `Manual Deploy` with `mode = rollback_previous`.
2. Set `confirm = deploy-production`.
3. Approve and complete workflow.

Emergency host-side rollback:

1. Point `/opt/edgelord/current` to previous release.
2. Restart `edgelord`.
3. Validate status/logs.

## USDC Ingress/Egress Model

Keep this manual unless you have additional controls:

- Ingress:
  - Send USDC + MATIC manually from your custody wallet.
- Egress:
  - Use explicit operator command:

```bash
edgelord wallet sweep \
  --to <destination_address> \
  --asset usdc \
  --network polygon \
  --yes \
  --config /opt/edgelord/config/config.toml
```

## Operational Guardrails

- Start new environments in `dry_run = true`.
- Use low exposure caps initially.
- Review logs and daily stats before raising limits.
- Keep secret files `chmod 600`.
- Keep SSH access key-only and restricted.
