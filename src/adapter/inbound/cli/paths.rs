//! Path utilities for edgelord.
//!
//! All data lives under `~/.edgelord/`:
//! - `~/.edgelord/config.toml` - main configuration
//! - `~/.edgelord/edgelord.db` - trading database
//! - `~/.edgelord/keystores/` - encrypted wallet keystores

use std::path::PathBuf;

/// Returns the edgelord home directory (`~/.edgelord/`).
pub fn home_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".edgelord")
}

/// Returns the default config file path (`~/.edgelord/config.toml`).
pub fn default_config() -> PathBuf {
    home_dir().join("config.toml")
}

/// Returns the default database path (`~/.edgelord/edgelord.db`).
pub fn default_database() -> PathBuf {
    home_dir().join("edgelord.db")
}

/// Returns the default keystore directory (`~/.edgelord/keystores/`).
pub fn keystore_dir() -> PathBuf {
    home_dir().join("keystores")
}

/// Ensures the edgelord home directory exists.
pub fn ensure_home_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(home_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_under_edgelord_home() {
        let home = home_dir();
        let config = default_config();
        let db = default_database();

        assert!(home.to_string_lossy().contains(".edgelord"));
        assert!(config.to_string_lossy().contains(".edgelord"));
        assert!(db.to_string_lossy().contains(".edgelord"));
    }
}
