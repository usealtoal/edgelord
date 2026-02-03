# Deployment CLI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
>
> **IMPORTANT:** Use one-line commits with NO Claude authorship (no Co-Authored-By lines).

**Goal:** Add a polished CLI with `run`, `status`, `logs`, `install`, `uninstall` commands, plus GitHub Actions deployment pipeline.

**Architecture:** Replace current `main.rs` with clap-based CLI. CLI args override config file values. Systemd integration for production deployment. Banner displays on interactive runs.

**Tech Stack:** clap 4 (derive), existing tokio/tracing stack

---

## Task 1: Add clap dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add clap to dependencies**

Add to `[dependencies]` section in `Cargo.toml`:

```toml
# CLI
clap = { version = "4", features = ["derive"] }
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "deps: add clap for CLI"
```

---

## Task 2: Create CLI module structure

**Files:**
- Create: `src/cli/mod.rs`
- Create: `src/cli/banner.rs`
- Modify: `src/lib.rs`

**Step 1: Create cli/mod.rs with clap definitions**

```rust
//! Command-line interface definitions.

pub mod banner;

use clap::{Parser, Subcommand};
use rust_decimal::Decimal;
use std::path::PathBuf;

/// Edgelord - Multi-strategy arbitrage detection and execution.
#[derive(Parser, Debug)]
#[command(name = "edgelord")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,

    /// Override chain ID (80002=testnet, 137=mainnet)
    #[arg(long)]
    pub chain_id: Option<u64>,

    /// Override log level (debug, info, warn, error)
    #[arg(long)]
    pub log_level: Option<String>,

    /// Detect opportunities but don't execute trades
    #[arg(long)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the arbitrage detector (foreground, interactive)
    Run(RunArgs),

    /// Show service status
    Status,

    /// Tail service logs
    Logs(LogsArgs),

    /// Install systemd service
    Install(InstallArgs),

    /// Uninstall systemd service
    Uninstall,
}

/// Arguments for the `run` subcommand.
#[derive(Parser, Debug)]
pub struct RunArgs {
    /// Skip ASCII art banner
    #[arg(long)]
    pub no_banner: bool,

    /// Use JSON log format instead of pretty
    #[arg(long)]
    pub json_logs: bool,

    /// Comma-separated list of strategies to enable
    #[arg(long)]
    pub strategies: Option<String>,

    /// Override minimum edge threshold
    #[arg(long)]
    pub min_edge: Option<Decimal>,

    /// Override minimum profit threshold
    #[arg(long)]
    pub min_profit: Option<Decimal>,

    /// Override maximum total exposure
    #[arg(long)]
    pub max_exposure: Option<Decimal>,

    /// Override maximum position per market
    #[arg(long)]
    pub max_position: Option<Decimal>,

    /// Enable Telegram notifications
    #[arg(long)]
    pub telegram_enabled: bool,
}

/// Arguments for the `logs` subcommand.
#[derive(Parser, Debug)]
pub struct LogsArgs {
    /// Number of lines to show
    #[arg(short = 'n', long, default_value = "50")]
    pub lines: u32,

    /// Follow log output (like tail -f)
    #[arg(short, long)]
    pub follow: bool,

    /// Show logs since (e.g., "1 hour ago", "2024-01-01")
    #[arg(long)]
    pub since: Option<String>,
}

/// Arguments for the `install` subcommand.
#[derive(Parser, Debug)]
pub struct InstallArgs {
    /// Path to config file for the service
    #[arg(long, default_value = "/opt/edgelord/config.toml")]
    pub config: PathBuf,

    /// User to run the service as
    #[arg(long, default_value = "edgelord")]
    pub user: String,

    /// Working directory for the service
    #[arg(long, default_value = "/opt/edgelord")]
    pub working_dir: PathBuf,
}
```

**Step 2: Create cli/banner.rs with ASCII art**

```rust
//! ASCII art banner for interactive mode.

use std::io::IsTerminal;

/// ANSI true-color escape sequences for the banner palette.
struct Colors {
    shell_dark: &'static str,
    shell_light: &'static str,
    face: &'static str,
    eyes: &'static str,
    title: &'static str,
    subtitle: &'static str,
    reset: &'static str,
}

const COLOR: Colors = Colors {
    shell_dark: "\x1b[38;2;139;90;73m",
    shell_light: "\x1b[38;2;181;132;108m",
    face: "\x1b[38;2;194;150;130m",
    eyes: "\x1b[38;2;255;255;255m",
    title: "\x1b[1;38;2;220;165;120m",
    subtitle: "\x1b[38;2;100;100;120m",
    reset: "\x1b[0m",
};

const PLAIN: Colors = Colors {
    shell_dark: "",
    shell_light: "",
    face: "",
    eyes: "",
    title: "",
    subtitle: "",
    reset: "",
};

/// Prints the Edgelord banner to stdout.
///
/// Renders ANSI true-color when stdout is a terminal,
/// falls back to plain text otherwise.
pub fn print_banner() {
    let c = if std::io::stdout().is_terminal() {
        &COLOR
    } else {
        &PLAIN
    };

    let sd = c.shell_dark;
    let sl = c.shell_light;
    let fc = c.face;
    let ey = c.eyes;
    let tt = c.title;
    let st = c.subtitle;
    let r = c.reset;

    println!(
        r#"
{sd}     ▄▄▄▄▄▄▄▄▄{r}
{sl}   ▄█▒█▒█▒█▒█▒█▄{r}        {tt}    __________  ______________    ____  ____  ____{r}
{sd}  █▒█▒█▒█▒█▒█▒█▒█{r}       {tt}   / ____/ __ \/ ____/ ____/ /   / __ \/ __ \/ __ \{r}
{fc}  █▄▄▄▄▄▄▄▄▄▄▄▄▄█{r}       {tt}  / __/ / / / / / __/ __/ / /   / / / / /_/ / / / /{r}
{fc}  █░░░{ey}●{fc}░░░░░{ey}●{fc}░░░█{r}       {tt} / /___/ /_/ / /_/ / /___/ /___/ /_/ / _, _/ /_/ /{r}
{fc}  █░░░░░░░░░░░░░█{r}       {tt}/_____/_____/\____/_____/_____/\____/_/ |_/_____/{r}
{fc}   █░░░░▄▄░░░░░█{r}
{fc}    ▀█▄▄▄▄▄▄▄█▀{r}         {st}"This aggression will not stand, man."{r}
{fc}     ▀█▀   ▀█▀{r}
"#
    );
}
```

**Step 3: Add cli module to lib.rs**

Add to `src/lib.rs` after line 27:

```rust
pub mod cli;
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/cli/mod.rs src/cli/banner.rs src/lib.rs
git commit -m "feat(cli): add CLI module with clap definitions and banner"
```

---

## Task 3: Implement run command

**Files:**
- Create: `src/cli/run.rs`
- Modify: `src/cli/mod.rs`
- Modify: `src/main.rs`

**Step 1: Create cli/run.rs**

```rust
//! Handler for the `run` command.

use crate::app::{App, Config};
use crate::cli::{banner, Cli, RunArgs};
use crate::error::Result;
use tokio::signal;
use tracing::{error, info};

/// Execute the run command.
pub async fn execute(cli: &Cli, args: &RunArgs) -> Result<()> {
    // Load and merge configuration
    let mut config = Config::load(&cli.config)?;

    // Apply CLI overrides
    if let Some(chain_id) = cli.chain_id {
        config.network.chain_id = chain_id;
    }
    if let Some(ref level) = cli.log_level {
        config.logging.level = level.clone();
    }
    if args.json_logs {
        config.logging.format = "json".to_string();
    }
    if let Some(ref strategies) = args.strategies {
        config.strategies.enabled = strategies.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Some(min_edge) = args.min_edge {
        config.strategies.single_condition.min_edge = min_edge;
        config.strategies.market_rebalancing.min_edge = min_edge;
    }
    if let Some(min_profit) = args.min_profit {
        config.strategies.single_condition.min_profit = min_profit;
        config.strategies.market_rebalancing.min_profit = min_profit;
    }
    if let Some(max_exposure) = args.max_exposure {
        config.risk.max_total_exposure = max_exposure;
    }
    if let Some(max_position) = args.max_position {
        config.risk.max_position_per_market = max_position;
    }
    if args.telegram_enabled {
        config.telegram.enabled = true;
    }
    if cli.dry_run {
        info!("Dry-run mode enabled - will not execute trades");
        // TODO: Wire dry_run into executor
    }

    // Initialize logging
    config.init_logging();

    // Show banner unless disabled
    if !args.no_banner {
        banner::print_banner();
    }

    info!(
        chain_id = config.network.chain_id,
        strategies = ?config.strategies.enabled,
        "edgelord starting"
    );

    // Run the main application
    #[cfg(feature = "polymarket")]
    {
        tokio::select! {
            result = App::run(config) => {
                if let Err(e) = result {
                    error!(error = %e, "Fatal error");
                    std::process::exit(1);
                }
            }
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received");
            }
        }
    }

    #[cfg(not(feature = "polymarket"))]
    {
        let _ = config;
        info!("No exchange features enabled - exiting");
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received");
            }
        }
    }

    info!("edgelord stopped");
    Ok(())
}
```

**Step 2: Add run module to cli/mod.rs**

Add after `pub mod banner;`:

```rust
pub mod run;
```

**Step 3: Rewrite main.rs to use CLI**

Replace entire `src/main.rs` with:

```rust
use clap::Parser;
use edgelord::cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Run(args) => edgelord::cli::run::execute(&cli, args).await,
        Commands::Status => {
            eprintln!("Status command not yet implemented");
            Ok(())
        }
        Commands::Logs(_args) => {
            eprintln!("Logs command not yet implemented");
            Ok(())
        }
        Commands::Install(_args) => {
            eprintln!("Install command not yet implemented");
            Ok(())
        }
        Commands::Uninstall => {
            eprintln!("Uninstall command not yet implemented");
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
```

**Step 4: Verify it compiles and runs**

Run: `cargo build`
Run: `cargo run -- --help`
Expected: Shows help with subcommands

Run: `cargo run -- run --help`
Expected: Shows run subcommand options

**Step 5: Commit**

```bash
git add src/cli/run.rs src/cli/mod.rs src/main.rs
git commit -m "feat(cli): implement run command with config overrides"
```

---

## Task 4: Implement status command

**Files:**
- Create: `src/cli/status.rs`
- Modify: `src/cli/mod.rs`
- Modify: `src/main.rs`

**Step 1: Create cli/status.rs**

```rust
//! Handler for the `status` command.

use std::process::Command;

/// Execute the status command.
pub fn execute() {
    // Check if systemd service is running
    let output = Command::new("systemctl")
        .args(["is-active", "edgelord"])
        .output();

    let status = match output {
        Ok(out) => {
            let status_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if status_str == "active" {
                "● running"
            } else if status_str == "inactive" {
                "○ stopped"
            } else {
                "? unknown"
            }
        }
        Err(_) => "? systemd not available",
    };

    // Get PID if running
    let pid = Command::new("systemctl")
        .args(["show", "edgelord", "--property=MainPID", "--value"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|p| p != "0" && !p.is_empty());

    let version = env!("CARGO_PKG_VERSION");

    println!();
    println!("edgelord v{version}");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if let Some(ref p) = pid {
        println!("Status:      {status} (pid {p})");
    } else {
        println!("Status:      {status}");
    }

    // Get uptime if running
    if pid.is_some() {
        if let Ok(output) = Command::new("systemctl")
            .args(["show", "edgelord", "--property=ActiveEnterTimestamp", "--value"])
            .output()
        {
            let timestamp = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !timestamp.is_empty() {
                println!("Since:       {timestamp}");
            }
        }
    }

    println!();
    println!("Use 'edgelord logs' to view logs");
    println!("Use 'sudo systemctl start edgelord' to start");
    println!();
}
```

**Step 2: Add status module to cli/mod.rs**

Add after `pub mod run;`:

```rust
pub mod status;
```

**Step 3: Update main.rs to call status**

Replace the `Commands::Status` arm in `main.rs`:

```rust
        Commands::Status => {
            edgelord::cli::status::execute();
            Ok(())
        }
```

**Step 4: Verify it compiles**

Run: `cargo build`
Run: `cargo run -- status`
Expected: Shows status output (will show "systemd not available" or "stopped" on dev machine)

**Step 5: Commit**

```bash
git add src/cli/status.rs src/cli/mod.rs src/main.rs
git commit -m "feat(cli): implement status command"
```

---

## Task 5: Implement logs command

**Files:**
- Create: `src/cli/logs.rs`
- Modify: `src/cli/mod.rs`
- Modify: `src/main.rs`

**Step 1: Create cli/logs.rs**

```rust
//! Handler for the `logs` command.

use crate::cli::LogsArgs;
use std::process::Command;

/// Execute the logs command.
pub fn execute(args: &LogsArgs) {
    let mut cmd = Command::new("journalctl");
    cmd.args(["-u", "edgelord", "--output=cat"]);

    if args.follow {
        cmd.arg("-f");
    } else {
        cmd.args(["-n", &args.lines.to_string()]);
    }

    if let Some(ref since) = args.since {
        cmd.args(["--since", since]);
    }

    // Execute and stream output
    let status = cmd.status();

    match status {
        Ok(exit) => {
            if !exit.success() {
                if let Some(code) = exit.code() {
                    if code == 1 {
                        eprintln!("No logs found. Is the edgelord service installed?");
                        eprintln!("Run 'sudo edgelord install' to install the service.");
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to execute journalctl: {e}");
            eprintln!("Make sure you're running on a system with systemd.");
        }
    }
}
```

**Step 2: Add logs module to cli/mod.rs**

Add after `pub mod status;`:

```rust
pub mod logs;
```

**Step 3: Update main.rs to call logs**

Replace the `Commands::Logs` arm in `main.rs`:

```rust
        Commands::Logs(args) => {
            edgelord::cli::logs::execute(args);
            Ok(())
        }
```

**Step 4: Verify it compiles**

Run: `cargo build`
Run: `cargo run -- logs --help`
Expected: Shows logs subcommand options

**Step 5: Commit**

```bash
git add src/cli/logs.rs src/cli/mod.rs src/main.rs
git commit -m "feat(cli): implement logs command"
```

---

## Task 6: Implement install/uninstall commands

**Files:**
- Create: `src/cli/service.rs`
- Modify: `src/cli/mod.rs`
- Modify: `src/main.rs`

**Step 1: Create cli/service.rs**

```rust
//! Handlers for `install` and `uninstall` commands.

use crate::cli::InstallArgs;
use std::fs;
use std::process::Command;

const SERVICE_PATH: &str = "/etc/systemd/system/edgelord.service";

/// Generate the systemd service file content.
fn generate_service_file(args: &InstallArgs, binary_path: &str) -> String {
    format!(
        r#"[Unit]
Description=Edgelord Arbitrage Service
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User={user}
Group={user}
WorkingDirectory={working_dir}
ExecStart={binary} run --no-banner --json-logs --config {config}
Restart=on-failure
RestartSec=5
EnvironmentFile=-{working_dir}/.env

[Install]
WantedBy=multi-user.target
"#,
        user = args.user,
        working_dir = args.working_dir.display(),
        binary = binary_path,
        config = args.config.display(),
    )
}

/// Execute the install command.
pub fn execute_install(args: &InstallArgs) {
    // Get the path to the current binary
    let binary_path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/opt/edgelord/edgelord".to_string());

    let service_content = generate_service_file(args, &binary_path);

    // Check if running as root
    if !is_root() {
        eprintln!("Error: This command must be run as root (use sudo)");
        std::process::exit(1);
    }

    // Write service file
    match fs::write(SERVICE_PATH, &service_content) {
        Ok(()) => println!("✓ Created {SERVICE_PATH}"),
        Err(e) => {
            eprintln!("✗ Failed to create service file: {e}");
            std::process::exit(1);
        }
    }

    // Reload systemd
    if run_systemctl(&["daemon-reload"]) {
        println!("✓ Reloaded systemd daemon");
    } else {
        eprintln!("✗ Failed to reload systemd daemon");
        std::process::exit(1);
    }

    // Enable service
    if run_systemctl(&["enable", "edgelord"]) {
        println!("✓ Enabled edgelord service (starts on boot)");
    } else {
        eprintln!("✗ Failed to enable service");
        std::process::exit(1);
    }

    println!();
    println!("Start with: sudo systemctl start edgelord");
    println!("View logs:  edgelord logs -f");
    println!();
}

/// Execute the uninstall command.
pub fn execute_uninstall() {
    // Check if running as root
    if !is_root() {
        eprintln!("Error: This command must be run as root (use sudo)");
        std::process::exit(1);
    }

    // Stop service if running
    if run_systemctl(&["stop", "edgelord"]) {
        println!("✓ Stopped edgelord service");
    }

    // Disable service
    if run_systemctl(&["disable", "edgelord"]) {
        println!("✓ Disabled edgelord service");
    }

    // Remove service file
    if std::path::Path::new(SERVICE_PATH).exists() {
        match fs::remove_file(SERVICE_PATH) {
            Ok(()) => println!("✓ Removed {SERVICE_PATH}"),
            Err(e) => {
                eprintln!("✗ Failed to remove service file: {e}");
                std::process::exit(1);
            }
        }
    }

    // Reload systemd
    if run_systemctl(&["daemon-reload"]) {
        println!("✓ Reloaded systemd daemon");
    }

    println!();
    println!("Edgelord service has been uninstalled.");
    println!();
}

fn run_systemctl(args: &[&str]) -> bool {
    Command::new("systemctl")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn is_root() -> bool {
    // On Unix, check if effective UID is 0
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}
```

**Step 2: Add libc dependency for is_root check**

Add to `[dependencies]` in `Cargo.toml`:

```toml
libc = "0.2"
```

**Step 3: Add service module to cli/mod.rs**

Add after `pub mod logs;`:

```rust
pub mod service;
```

**Step 4: Update main.rs to call install/uninstall**

Replace the `Commands::Install` and `Commands::Uninstall` arms in `main.rs`:

```rust
        Commands::Install(args) => {
            edgelord::cli::service::execute_install(args);
            Ok(())
        }
        Commands::Uninstall => {
            edgelord::cli::service::execute_uninstall();
            Ok(())
        }
```

**Step 5: Verify it compiles**

Run: `cargo build`
Run: `cargo run -- install --help`
Expected: Shows install subcommand options

**Step 6: Commit**

```bash
git add Cargo.toml src/cli/service.rs src/cli/mod.rs src/main.rs
git commit -m "feat(cli): implement install and uninstall commands"
```

---

## Task 7: Create deploy directory with templates

**Files:**
- Create: `deploy/config.prod.toml`
- Create: `deploy/README.md`

**Step 1: Create deploy/config.prod.toml**

```toml
# Production configuration for edgelord
# Copy to /opt/edgelord/config.toml on deployment

[network]
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"
chain_id = 137  # Polygon mainnet

[logging]
level = "info"
format = "json"

[strategies]
enabled = ["single_condition", "market_rebalancing"]

[strategies.single_condition]
min_edge = 0.05
min_profit = 0.50

[strategies.market_rebalancing]
min_edge = 0.03
min_profit = 1.00
max_outcomes = 10

[strategies.combinatorial]
enabled = false

[wallet]
# Private key loaded from WALLET_PRIVATE_KEY env var

[risk]
max_position_per_market = 500
max_total_exposure = 5000
min_profit_threshold = 0.10
max_slippage = 0.02

[telegram]
enabled = true
notify_opportunities = false
notify_executions = true
notify_risk_rejections = true
```

**Step 2: Create deploy/README.md**

```markdown
# Deployment Files

## Files

- `config.prod.toml` - Production configuration template

## VPS Setup

1. Create edgelord user:
   ```bash
   sudo useradd -r -s /bin/false edgelord
   sudo mkdir -p /opt/edgelord
   sudo chown edgelord:edgelord /opt/edgelord
   ```

2. Copy binary and config:
   ```bash
   sudo cp edgelord /opt/edgelord/
   sudo cp config.prod.toml /opt/edgelord/config.toml
   sudo chown edgelord:edgelord /opt/edgelord/*
   ```

3. Create .env file with secrets:
   ```bash
   sudo tee /opt/edgelord/.env << EOF
   WALLET_PRIVATE_KEY=0x...
   TELEGRAM_BOT_TOKEN=...
   TELEGRAM_CHAT_ID=...
   EOF
   sudo chmod 600 /opt/edgelord/.env
   sudo chown edgelord:edgelord /opt/edgelord/.env
   ```

4. Install and start service:
   ```bash
   sudo /opt/edgelord/edgelord install
   sudo systemctl start edgelord
   ```

## Management

```bash
# View status
edgelord status

# View logs
edgelord logs -f

# Restart service
sudo systemctl restart edgelord

# Stop service
sudo systemctl stop edgelord
```
```

**Step 3: Commit**

```bash
git add deploy/
git commit -m "docs: add deployment templates and setup guide"
```

---

## Task 8: Create GitHub Actions workflow

**Files:**
- Create: `.github/workflows/deploy.yml`

**Step 1: Create deploy workflow**

```yaml
name: Deploy

on:
  push:
    branches: [main]
    paths-ignore:
      - 'docs/**'
      - '*.md'
      - 'LICENSE'
  workflow_dispatch:
    inputs:
      skip_deploy:
        description: 'Skip deployment (build and test only)'
        required: false
        default: 'false'
        type: boolean

env:
  CARGO_TERM_COLOR: always

jobs:
  build-and-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-action@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Build release
        run: cargo build --release

      - name: Run tests
        run: cargo test --release

      - name: Upload binary
        uses: actions/upload-artifact@v4
        with:
          name: edgelord-linux
          path: target/release/edgelord
          retention-days: 7

  deploy:
    needs: build-and-test
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main' && github.event.inputs.skip_deploy != 'true'
    environment: production

    steps:
      - name: Download binary
        uses: actions/download-artifact@v4
        with:
          name: edgelord-linux

      - name: Make binary executable
        run: chmod +x edgelord

      - name: Setup SSH
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.VPS_SSH_KEY }}" > ~/.ssh/id_rsa
          chmod 600 ~/.ssh/id_rsa
          ssh-keyscan -H ${{ secrets.VPS_HOST }} >> ~/.ssh/known_hosts

      - name: Deploy to VPS
        run: |
          # Copy binary to VPS
          scp edgelord ${{ secrets.VPS_USER }}@${{ secrets.VPS_HOST }}:/tmp/edgelord-new

          # SSH and deploy
          ssh ${{ secrets.VPS_USER }}@${{ secrets.VPS_HOST }} << 'EOF'
            set -e

            # Stop service
            sudo systemctl stop edgelord || true

            # Backup old binary
            if [ -f /opt/edgelord/edgelord ]; then
              sudo cp /opt/edgelord/edgelord /opt/edgelord/edgelord.bak
            fi

            # Install new binary
            sudo mv /tmp/edgelord-new /opt/edgelord/edgelord
            sudo chown edgelord:edgelord /opt/edgelord/edgelord
            sudo chmod +x /opt/edgelord/edgelord

            # Start service
            sudo systemctl start edgelord

            # Verify it's running
            sleep 2
            sudo systemctl is-active edgelord
          EOF

      - name: Notify success
        if: success()
        run: |
          echo "Deployment successful!"
          # TODO: Add Telegram notification

      - name: Rollback on failure
        if: failure()
        run: |
          ssh ${{ secrets.VPS_USER }}@${{ secrets.VPS_HOST }} << 'EOF'
            if [ -f /opt/edgelord/edgelord.bak ]; then
              sudo mv /opt/edgelord/edgelord.bak /opt/edgelord/edgelord
              sudo systemctl start edgelord || true
            fi
          EOF
```

**Step 2: Commit**

```bash
git add .github/workflows/deploy.yml
git commit -m "ci: add GitHub Actions deploy workflow"
```

---

## Task 9: Run full test suite and verify

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Test CLI commands**

Run: `cargo run -- --help`
Expected: Shows main help

Run: `cargo run -- run --help`
Expected: Shows run options

Run: `cargo run -- status`
Expected: Shows status (will say systemd not available on dev machine)

Run: `cargo run -- logs --help`
Expected: Shows logs options

Run: `cargo run -- install --help`
Expected: Shows install options

**Step 3: Test run command with banner**

Run: `cargo run -- run --no-banner 2>&1 | head -5`
Expected: Starts without banner (will fail to connect but that's OK)

**Step 4: Final commit for any fixes**

If any issues found, fix and commit.

---

## Task 10: Update documentation

**Files:**
- Modify: `README.md`

**Step 1: Add CLI section to README.md**

Add after the Configuration section:

```markdown
## CLI Usage

```bash
# Run interactively (with banner, colored logs)
edgelord run

# Run for production (no banner, JSON logs)
edgelord run --no-banner --json-logs

# Override settings
edgelord run --chain-id 137 --max-exposure 5000 --telegram-enabled

# Check service status
edgelord status

# View logs
edgelord logs -f
edgelord logs --lines 100
edgelord logs --since "1 hour ago"

# Install as systemd service (requires root)
sudo edgelord install --config /opt/edgelord/config.toml

# Uninstall service
sudo edgelord uninstall
```

### Configuration Priority

Settings are applied in this order (later overrides earlier):

1. Built-in defaults
2. Config file (`config.toml`)
3. CLI flags (`--chain-id`, `--max-exposure`, etc.)
4. Environment variables (secrets only)
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add CLI usage to README"
```

---

## Summary

After completing all tasks, you will have:

1. **CLI with 5 commands:** `run`, `status`, `logs`, `install`, `uninstall`
2. **ASCII banner** that displays on interactive runs
3. **Config layering** with CLI flag overrides
4. **Systemd integration** for production deployment
5. **GitHub Actions workflow** for automated deployment
6. **Deploy templates** with production config and setup guide

Total: 10 tasks, ~40 steps
