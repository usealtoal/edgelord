//! Inbound operator accessor for CLI handlers.

use std::path::Path;
use std::sync::OnceLock;

use crate::error::Result;
use crate::port::inbound::operator::port::OperatorPort;

static OPERATOR: OnceLock<Box<dyn OperatorPort>> = OnceLock::new();

/// Installs the operator implementation used by CLI handlers.
pub fn install(operator: Box<dyn OperatorPort>) -> std::result::Result<(), Box<dyn OperatorPort>> {
    OPERATOR.set(operator)
}

/// Returns the configured operator capability surface for CLI handlers.
#[must_use]
pub fn operator() -> &'static dyn OperatorPort {
    OPERATOR
        .get()
        .expect("CLI operator not installed; call cli::operator::install from main")
        .as_ref()
}

/// Load config TOML from disk for operator-facing use-cases.
pub fn read_config_toml(path: &Path) -> Result<String> {
    Ok(std::fs::read_to_string(path)?)
}

/// Build a sqlite database URL from a filesystem path.
#[must_use]
pub fn sqlite_database_url(path: &Path) -> String {
    format!("sqlite://{}", path.display())
}
