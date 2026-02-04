//! Status file for external monitoring.
//!
//! Writes a JSON status file that external tools can poll to monitor
//! the health and activity of the running edgelord instance.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Current status file format version.
const STATUS_VERSION: &str = "1";

/// Top-level status file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusFile {
    /// Schema version for forward compatibility.
    pub version: String,
    /// When the process started.
    pub started_at: DateTime<Utc>,
    /// Process ID.
    pub pid: u32,
    /// Static configuration snapshot.
    pub config: StatusConfig,
    /// Runtime state (positions, exposure).
    pub runtime: StatusRuntime,
    /// Today's activity counters.
    pub today: StatusToday,
    /// When this file was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Static configuration snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusConfig {
    /// Active exchange name.
    pub exchange: String,
    /// Environment (testnet/mainnet).
    pub environment: String,
    /// Blockchain chain ID (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<u64>,
    /// Enabled strategy names.
    pub strategies: Vec<String>,
    /// Whether running in dry-run mode.
    pub dry_run: bool,
}

/// Runtime state information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusRuntime {
    /// Number of currently open positions.
    pub positions_open: usize,
    /// Current total exposure in dollars.
    pub exposure_current: Decimal,
    /// Maximum allowed exposure.
    pub exposure_max: Decimal,
}

/// Today's activity counters (reset daily).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusToday {
    /// Arbitrage opportunities detected.
    pub opportunities_detected: u64,
    /// Trades executed.
    pub trades_executed: u64,
    /// Profit realized in dollars.
    pub profit_realized: Decimal,
}

/// Writer for the status file.
///
/// Thread-safe wrapper that manages atomic updates to the status file.
pub struct StatusWriter {
    /// Path to write the status file.
    path: PathBuf,
    /// Current status state.
    status: Mutex<StatusFile>,
}

impl StatusWriter {
    /// Create a new status writer.
    ///
    /// # Arguments
    /// * `path` - Path where the status file will be written
    /// * `config` - Static configuration to include in the status
    #[must_use]
    pub fn new(path: PathBuf, config: StatusConfig) -> Self {
        let now = Utc::now();
        let status = StatusFile {
            version: STATUS_VERSION.to_string(),
            started_at: now,
            pid: std::process::id(),
            config,
            runtime: StatusRuntime::default(),
            today: StatusToday::default(),
            updated_at: now,
        };

        Self {
            path,
            status: Mutex::new(status),
        }
    }

    /// Write the current status to the file atomically.
    ///
    /// Uses write-to-temp-then-rename pattern for atomicity.
    /// Creates parent directory if it doesn't exist.
    #[allow(clippy::result_large_err)]
    pub fn write(&self) -> Result<()> {
        // Clone status while holding lock, release before I/O
        let json = {
            let mut status = self.status.lock();
            status.updated_at = Utc::now();
            serde_json::to_string_pretty(&*status)?
        };

        // Create parent directory if it doesn't exist
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to temp file first for atomicity
        let temp_path = self.path.with_extension("tmp");
        let mut file = fs::File::create(&temp_path)?;

        // Helper to clean up temp file on failure
        let cleanup_and_err = |e| {
            let _ = fs::remove_file(&temp_path);
            e
        };

        file.write_all(json.as_bytes()).map_err(cleanup_and_err)?;
        file.sync_all().map_err(cleanup_and_err)?;

        // Atomic rename
        fs::rename(&temp_path, &self.path).map_err(cleanup_and_err)?;

        Ok(())
    }

    /// Update runtime state.
    pub fn update_runtime(
        &self,
        positions_open: usize,
        exposure_current: Decimal,
        exposure_max: Decimal,
    ) {
        let mut status = self.status.lock();
        status.runtime.positions_open = positions_open;
        status.runtime.exposure_current = exposure_current;
        status.runtime.exposure_max = exposure_max;
    }

    /// Record an opportunity detection.
    pub fn record_opportunity(&self) {
        let mut status = self.status.lock();
        status.today.opportunities_detected += 1;
    }

    /// Record a trade execution with its profit.
    pub fn record_execution(&self, profit: Decimal) {
        let mut status = self.status.lock();
        status.today.trades_executed += 1;
        status.today.profit_realized += profit;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn test_config() -> StatusConfig {
        StatusConfig {
            exchange: "polymarket".to_string(),
            environment: "mainnet".to_string(),
            chain_id: Some(137),
            strategies: vec!["single_condition".to_string()],
            dry_run: false,
        }
    }

    #[test]
    fn test_status_file_serialization() {
        let status = StatusFile {
            version: "1".to_string(),
            started_at: Utc::now(),
            pid: 12345,
            config: test_config(),
            runtime: StatusRuntime::default(),
            today: StatusToday::default(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string_pretty(&status).unwrap();
        assert!(json.contains("\"version\": \"1\""));
        assert!(json.contains("\"pid\": 12345"));
        assert!(json.contains("\"exchange\": \"polymarket\""));
        assert!(json.contains("\"environment\": \"mainnet\""));
        assert!(json.contains("\"dry_run\": false"));
    }

    #[test]
    fn test_status_config_serialization() {
        let config = test_config();
        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("\"exchange\":\"polymarket\""));
        assert!(json.contains("\"environment\":\"mainnet\""));
        assert!(json.contains("\"strategies\":[\"single_condition\"]"));
        assert!(json.contains("\"dry_run\":false"));
    }

    #[test]
    fn test_status_runtime_default() {
        let runtime = StatusRuntime::default();
        assert_eq!(runtime.positions_open, 0);
        assert_eq!(runtime.exposure_current, Decimal::ZERO);
        assert_eq!(runtime.exposure_max, Decimal::ZERO);
    }

    #[test]
    fn test_status_today_default() {
        let today = StatusToday::default();
        assert_eq!(today.opportunities_detected, 0);
        assert_eq!(today.trades_executed, 0);
        assert_eq!(today.profit_realized, Decimal::ZERO);
    }

    #[test]
    fn test_status_writer_new() {
        let path = PathBuf::from("/tmp/test_status.json");
        let writer = StatusWriter::new(path.clone(), test_config());

        let status = writer.status.lock();
        assert_eq!(status.version, "1");
        assert_eq!(status.config.chain_id, Some(137));
        assert_eq!(status.pid, std::process::id());
    }

    #[test]
    fn test_status_writer_update_runtime() {
        let path = PathBuf::from("/tmp/test_status.json");
        let writer = StatusWriter::new(path, test_config());

        writer.update_runtime(5, dec!(1000), dec!(10000));

        let status = writer.status.lock();
        assert_eq!(status.runtime.positions_open, 5);
        assert_eq!(status.runtime.exposure_current, dec!(1000));
        assert_eq!(status.runtime.exposure_max, dec!(10000));
    }

    #[test]
    fn test_status_writer_record_opportunity() {
        let path = PathBuf::from("/tmp/test_status.json");
        let writer = StatusWriter::new(path, test_config());

        writer.record_opportunity();
        writer.record_opportunity();
        writer.record_opportunity();

        let status = writer.status.lock();
        assert_eq!(status.today.opportunities_detected, 3);
    }

    #[test]
    fn test_status_writer_record_execution() {
        let path = PathBuf::from("/tmp/test_status.json");
        let writer = StatusWriter::new(path, test_config());

        writer.record_execution(dec!(5.50));
        writer.record_execution(dec!(3.25));

        let status = writer.status.lock();
        assert_eq!(status.today.trades_executed, 2);
        assert_eq!(status.today.profit_realized, dec!(8.75));
    }

    #[test]
    fn test_status_writer_write_file() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("edgelord_test_status.json");

        let writer = StatusWriter::new(path.clone(), test_config());
        writer.update_runtime(2, dec!(500), dec!(5000));
        writer.record_opportunity();
        writer.record_execution(dec!(1.50));

        writer.write().unwrap();

        // Verify file was written
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"version\": \"1\""));
        assert!(content.contains("\"positions_open\": 2"));
        assert!(content.contains("\"opportunities_detected\": 1"));
        assert!(content.contains("\"trades_executed\": 1"));

        // Cleanup
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_status_writer_creates_parent_directory() {
        let temp_dir = std::env::temp_dir();
        let nested_path = temp_dir.join("edgelord_test_nested/subdir/status.json");

        // Ensure directory doesn't exist
        let parent = nested_path.parent().unwrap();
        let _ = fs::remove_dir_all(parent);

        let writer = StatusWriter::new(nested_path.clone(), test_config());
        writer.write().unwrap();

        // Verify file was written
        assert!(nested_path.exists());
        let content = fs::read_to_string(&nested_path).unwrap();
        assert!(content.contains("\"version\": \"1\""));

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir.join("edgelord_test_nested"));
    }

    #[test]
    fn test_status_file_deserialization() {
        let json = r#"{
            "version": "1",
            "started_at": "2024-01-15T10:30:00Z",
            "pid": 12345,
            "config": {
                "exchange": "polymarket",
                "environment": "mainnet",
                "chain_id": 137,
                "strategies": ["single_condition", "combinatorial"],
                "dry_run": false
            },
            "runtime": {
                "positions_open": 3,
                "exposure_current": "1500.50",
                "exposure_max": "10000"
            },
            "today": {
                "opportunities_detected": 42,
                "trades_executed": 5,
                "profit_realized": "25.75"
            },
            "updated_at": "2024-01-15T11:00:00Z"
        }"#;

        let status: StatusFile = serde_json::from_str(json).unwrap();
        assert_eq!(status.version, "1");
        assert_eq!(status.pid, 12345);
        assert_eq!(status.config.exchange, "polymarket");
        assert_eq!(status.config.environment, "mainnet");
        assert_eq!(status.config.chain_id, Some(137));
        assert_eq!(status.config.strategies.len(), 2);
        assert!(!status.config.dry_run);
        assert_eq!(status.runtime.positions_open, 3);
        assert_eq!(status.runtime.exposure_current, dec!(1500.50));
        assert_eq!(status.runtime.exposure_max, dec!(10000));
        assert_eq!(status.today.opportunities_detected, 42);
        assert_eq!(status.today.trades_executed, 5);
        assert_eq!(status.today.profit_realized, dec!(25.75));
    }

    #[test]
    fn test_status_file_roundtrip() {
        let original = StatusFile {
            version: "1".to_string(),
            started_at: Utc::now(),
            pid: 54321,
            config: test_config(),
            runtime: StatusRuntime {
                positions_open: 7,
                exposure_current: dec!(2500),
                exposure_max: dec!(5000),
            },
            today: StatusToday {
                opportunities_detected: 100,
                trades_executed: 10,
                profit_realized: dec!(50.25),
            },
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string_pretty(&original).unwrap();
        let parsed: StatusFile = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.version, original.version);
        assert_eq!(parsed.pid, original.pid);
        assert_eq!(parsed.config.chain_id, original.config.chain_id);
        assert_eq!(
            parsed.runtime.positions_open,
            original.runtime.positions_open
        );
        assert_eq!(parsed.today.trades_executed, original.today.trades_executed);
    }
}
