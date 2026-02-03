# Status File Design

## Goal

Enable `edgelord status` to show runtime stats (network, strategies, positions, exposure, today's activity) by having the running app write a JSON status file.

## Architecture

```
App writes → /var/run/edgelord/status.json → status command reads
```

**Why file-based:**
- Simple, no IPC complexity
- Works across process boundaries
- Easy to debug (just `cat` the file)
- Survives status command crashes

## Status File Schema

```json
{
  "version": "0.1.0",
  "started_at": "2026-02-02T12:34:56Z",
  "pid": 12345,
  "config": {
    "chain_id": 137,
    "network": "mainnet",
    "strategies": ["single_condition", "market_rebalancing"],
    "dry_run": false
  },
  "runtime": {
    "positions_open": 2,
    "exposure_current": "847.50",
    "exposure_max": "10000.00"
  },
  "today": {
    "opportunities_detected": 5,
    "trades_executed": 3,
    "profit_realized": "12.40"
  },
  "updated_at": "2026-02-02T15:22:33Z"
}
```

## Implementation

### Task 1: Add StatusFile struct and writer

**Files:**
- Create: `src/app/status_file.rs`
- Modify: `src/app/mod.rs`

```rust
// src/app/status_file.rs
use std::path::PathBuf;
use serde::Serialize;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

#[derive(Serialize)]
pub struct StatusFile {
    pub version: String,
    pub started_at: DateTime<Utc>,
    pub pid: u32,
    pub config: StatusConfig,
    pub runtime: StatusRuntime,
    pub today: StatusToday,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct StatusConfig {
    pub chain_id: u64,
    pub network: String,
    pub strategies: Vec<String>,
    pub dry_run: bool,
}

#[derive(Serialize, Default)]
pub struct StatusRuntime {
    pub positions_open: usize,
    pub exposure_current: Decimal,
    pub exposure_max: Decimal,
}

#[derive(Serialize, Default)]
pub struct StatusToday {
    pub opportunities_detected: u64,
    pub trades_executed: u64,
    pub profit_realized: Decimal,
}

pub struct StatusWriter {
    path: PathBuf,
    status: StatusFile,
}

impl StatusWriter {
    pub fn new(path: PathBuf, config: &Config) -> Self { ... }
    pub fn write(&self) -> Result<()> { ... }
    pub fn update_runtime(&mut self, state: &AppState) { ... }
    pub fn record_opportunity(&mut self) { ... }
    pub fn record_execution(&mut self, profit: Decimal) { ... }
}
```

### Task 2: Wire StatusWriter into orchestrator

**Files:**
- Modify: `src/app/config.rs` - add `status_file` path option
- Modify: `src/app/orchestrator.rs` - create writer, update on events

**Config addition:**
```rust
#[serde(default = "default_status_file")]
pub status_file: Option<PathBuf>,

fn default_status_file() -> Option<PathBuf> {
    Some(PathBuf::from("/var/run/edgelord/status.json"))
}
```

**Orchestrator changes:**
- Create `StatusWriter` in `App::run()`
- Wrap in `Arc<Mutex<StatusWriter>>` for shared access
- Call `record_opportunity()` in `handle_opportunity()`
- Call `record_execution()` in `spawn_execution()` completion
- Update runtime stats periodically or on position changes

### Task 3: Update status command to read file

**Files:**
- Modify: `src/cli/status.rs`

```rust
pub fn execute(config_path: &Path) {
    // Try to load config for status file path
    let status_path = Config::load(config_path)
        .map(|c| c.status_file)
        .flatten()
        .unwrap_or_else(|| PathBuf::from("/var/run/edgelord/status.json"));

    // Check systemd status first
    let service_status = get_systemd_status();

    // Try to read status file
    match read_status_file(&status_path) {
        Ok(status) => print_full_status(&status, &service_status),
        Err(_) => print_basic_status(&service_status),
    }
}

fn print_full_status(status: &StatusFile, service: &str) {
    println!();
    println!("edgelord v{}", status.version);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Status:      {service} (pid {})", status.pid);
    println!("Uptime:      {}", format_uptime(status.started_at));
    println!("Network:     {} (chain {})", status.config.network, status.config.chain_id);
    println!("Strategies:  {}", status.config.strategies.join(", "));
    println!();
    println!("Positions:   {} open", status.runtime.positions_open);
    println!("Exposure:    ${} / ${} max",
        status.runtime.exposure_current, status.runtime.exposure_max);
    println!("Today:       {} opportunities, {} executed, ${} profit",
        status.today.opportunities_detected,
        status.today.trades_executed,
        status.today.profit_realized);
    println!();
}
```

### Task 4: Handle file permissions and directory creation

**Files:**
- Modify: `src/app/status_file.rs`
- Modify: `src/cli/service.rs` (install command)

**StatusWriter::new() creates parent directory if missing:**
```rust
if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
}
```

**Install command creates /var/run/edgelord with correct ownership:**
```rust
// After creating service file
let status_dir = Path::new("/var/run/edgelord");
if !status_dir.exists() {
    fs::create_dir_all(status_dir)?;
    // chown to service user
}
```

## File Locations

| Environment | Status File Path |
|-------------|-----------------|
| Production (systemd) | `/var/run/edgelord/status.json` |
| Development | `./status.json` (configurable) |

## Edge Cases

- **App crashes**: Status file shows stale `updated_at`, status command checks if PID is alive
- **Multiple instances**: Each uses different status file path
- **Permission denied**: Status command falls back to basic systemd status
- **File doesn't exist**: Status command shows systemd status only with note "Run 'edgelord run' to see full stats"

## Testing

1. Unit tests for `StatusFile` serialization
2. Integration test: start app, verify file written, verify status command reads it
3. Test graceful degradation when file missing/unreadable
