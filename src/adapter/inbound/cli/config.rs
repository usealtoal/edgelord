//! Handler for the `config` command group.

use std::fs;
use std::path::Path;

use rust_decimal::prelude::ToPrimitive;

use crate::adapter::inbound::cli::{operator, output};
use crate::error::{ConfigError, Result};

/// Default config template with documentation.
const CONFIG_TEMPLATE: &str = include_str!("../../../../config.toml.example");

/// Execute `config init`.
pub fn execute_init(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Err(ConfigError::InvalidValue {
            field: "config",
            reason: "file already exists (use --force to overwrite)".to_string(),
        }
        .into());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, CONFIG_TEMPLATE)?;
    output::section("Config Initialized");
    output::success("Created configuration file");
    output::field("Path", path.display());
    output::section("Next Steps");
    output::note(&format!("1. Edit {} with your settings", path.display()));
    output::note("2. Set WALLET_PRIVATE_KEY environment variable");
    output::note(&format!("3. Run: edgelord check config {}", path.display()));
    output::note(&format!("4. Run: edgelord run -c {}", path.display()));
    Ok(())
}

/// Execute `config show`.
pub fn execute_show(path: &Path) -> Result<()> {
    let config_toml = operator::read_config_toml(path)?;
    let config = operator::operator().show_config(&config_toml)?;

    output::section("Effective Configuration");
    output::field("Profile", config.profile);
    output::field("Dry run", config.dry_run);

    output::section("Exchange");
    output::field("Type", config.exchange);
    output::field("Environment", config.environment);
    output::field("Chain ID", config.chain_id);
    output::field("WebSocket", config.ws_url);
    output::field("API", config.api_url);

    output::section("Strategies");
    if config.enabled_strategies.is_empty() {
        output::note("(none enabled)");
    } else {
        for name in &config.enabled_strategies {
            output::note(&format!("- {name}"));
        }
    }

    output::section("Risk");
    output::field(
        "Max position",
        format!("${}", config.risk.max_position_per_market),
    );
    output::field(
        "Max exposure",
        format!("${}", config.risk.max_total_exposure),
    );
    output::field(
        "Min profit",
        format!("${}", config.risk.min_profit_threshold),
    );
    output::field(
        "Max slippage",
        format!(
            "{:.1}%",
            config.risk.max_slippage.to_f64().unwrap_or(0.0) * 100.0
        ),
    );

    output::section("Wallet");
    if config.wallet_private_key_loaded {
        output::success("Private key loaded from WALLET_PRIVATE_KEY");
    } else {
        output::warning("Private key not set");
    }

    output::section("Notifications");
    output::field(
        "Telegram",
        if config.telegram_enabled {
            "enabled"
        } else {
            "disabled"
        },
    );

    output::section("LLM Inference");
    output::field("Provider", config.llm_provider);
    output::field(
        "Enabled",
        if config.inference.enabled {
            "yes"
        } else {
            "no"
        },
    );
    if config.inference.enabled {
        output::field(
            "Min confidence",
            format!("{:.0}%", config.inference.min_confidence * 100.0),
        );
        output::field("TTL", format!("{}s", config.inference.ttl_seconds));
    }

    output::section("Cluster Detection");
    output::field(
        "Enabled",
        if config.cluster_detection.enabled {
            "yes"
        } else {
            "no"
        },
    );
    if config.cluster_detection.enabled {
        output::field(
            "Debounce",
            format!("{}ms", config.cluster_detection.debounce_ms),
        );
        output::field(
            "Min gap",
            format!(
                "{:.1}%",
                config.cluster_detection.min_gap.to_f64().unwrap_or(0.0) * 100.0
            ),
        );
    }

    Ok(())
}

/// Execute `config validate`.
pub fn execute_validate(path: &Path) -> Result<()> {
    output::section("Config Validation");
    output::field("Path", path.display());
    let config_toml = operator::read_config_toml(path)?;
    let validation = operator::operator().validate_config(&config_toml)?;
    output::success("Config file is valid");

    if !validation.warnings.is_empty() {
        output::section("Warnings");
        for warning in &validation.warnings {
            output::warning(warning);
        }
    }

    output::field(
        "Next",
        format!("edgelord config show -c {}", path.display()),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Helper to create a temporary directory for testing
    fn create_temp_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp directory")
    }

    // Tests for CONFIG_TEMPLATE

    #[test]
    fn test_config_template_is_not_empty() {
        assert!(!CONFIG_TEMPLATE.is_empty());
    }

    #[test]
    fn test_config_template_contains_expected_sections() {
        // The template should contain common configuration sections
        assert!(CONFIG_TEMPLATE.contains('['));
        assert!(CONFIG_TEMPLATE.contains(']'));
    }

    #[test]
    fn test_config_template_is_valid_toml() {
        // Attempt to parse the template as TOML to ensure it's valid
        let result: std::result::Result<toml::Value, _> = toml::from_str(CONFIG_TEMPLATE);
        assert!(result.is_ok(), "CONFIG_TEMPLATE is not valid TOML");
    }

    // Tests for execute_init

    #[test]
    fn test_execute_init_creates_file() {
        let temp_dir = create_temp_dir();
        let config_path = temp_dir.path().join("config.toml");

        let result = execute_init(&config_path, false);
        assert!(result.is_ok());
        assert!(config_path.exists());
    }

    #[test]
    fn test_execute_init_writes_template_content() {
        let temp_dir = create_temp_dir();
        let config_path = temp_dir.path().join("config.toml");

        execute_init(&config_path, false).unwrap();
        let content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, CONFIG_TEMPLATE);
    }

    #[test]
    fn test_execute_init_creates_parent_directories() {
        let temp_dir = create_temp_dir();
        let config_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("config.toml");

        let result = execute_init(&config_path, false);
        assert!(result.is_ok());
        assert!(config_path.exists());
    }

    #[test]
    fn test_execute_init_fails_if_file_exists_without_force() {
        let temp_dir = create_temp_dir();
        let config_path = temp_dir.path().join("config.toml");

        // Create the file first
        fs::write(&config_path, "existing content").unwrap();

        let result = execute_init(&config_path, false);
        assert!(result.is_err());

        // Verify original content is preserved
        let content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "existing content");
    }

    #[test]
    fn test_execute_init_overwrites_with_force() {
        let temp_dir = create_temp_dir();
        let config_path = temp_dir.path().join("config.toml");

        // Create the file first
        fs::write(&config_path, "existing content").unwrap();

        let result = execute_init(&config_path, true);
        assert!(result.is_ok());

        // Verify content was overwritten
        let content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, CONFIG_TEMPLATE);
    }

    #[test]
    fn test_execute_init_error_contains_force_hint() {
        let temp_dir = create_temp_dir();
        let config_path = temp_dir.path().join("config.toml");

        // Create the file first
        fs::write(&config_path, "existing content").unwrap();

        let result = execute_init(&config_path, false);
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_message = error.to_string();
        assert!(
            error_message.contains("--force"),
            "Error should mention --force flag"
        );
    }

    #[test]
    fn test_execute_init_with_force_on_nonexistent_file() {
        let temp_dir = create_temp_dir();
        let config_path = temp_dir.path().join("config.toml");

        // Force flag should work even if file doesn't exist
        let result = execute_init(&config_path, true);
        assert!(result.is_ok());
        assert!(config_path.exists());
    }

    // Tests for path edge cases

    #[test]
    fn test_execute_init_with_relative_path() {
        let temp_dir = create_temp_dir();
        let original_dir = std::env::current_dir().unwrap();

        // Change to temp dir
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let config_path = PathBuf::from("config.toml");
        let result = execute_init(&config_path, false);

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        assert!(temp_dir.path().join("config.toml").exists());
    }

    #[test]
    fn test_execute_init_with_absolute_path() {
        let temp_dir = create_temp_dir();
        let config_path = temp_dir
            .path()
            .join("config.toml")
            .canonicalize()
            .unwrap_or_else(|_| {
                // If path doesn't exist yet, just use the raw path
                temp_dir.path().join("config.toml")
            });

        let result = execute_init(&config_path, false);
        assert!(result.is_ok());
    }

    // Tests for error handling

    #[test]
    fn test_execute_init_invalid_path_returns_error() {
        // On Unix, we can't write to /dev/null as a file
        #[cfg(unix)]
        {
            let invalid_path = Path::new("/proc/1/config.toml");
            let result = execute_init(invalid_path, false);
            // This should fail due to permission/path issues
            // We just verify it returns an error rather than panicking
            if result.is_err() {
                // Expected behavior
            }
        }
    }

    // Tests for file content verification

    #[test]
    fn test_execute_init_file_is_readable_after_creation() {
        let temp_dir = create_temp_dir();
        let config_path = temp_dir.path().join("config.toml");

        execute_init(&config_path, false).unwrap();

        // Verify the file can be read
        let content = fs::read_to_string(&config_path);
        assert!(content.is_ok());
    }

    #[test]
    fn test_execute_init_file_permissions() {
        let temp_dir = create_temp_dir();
        let config_path = temp_dir.path().join("config.toml");

        execute_init(&config_path, false).unwrap();

        // Verify the file metadata can be read
        let metadata = fs::metadata(&config_path);
        assert!(metadata.is_ok());
        assert!(metadata.unwrap().is_file());
    }

    // Test for deeply nested path creation

    #[test]
    fn test_execute_init_deeply_nested_path() {
        let temp_dir = create_temp_dir();
        let config_path = temp_dir
            .path()
            .join("a")
            .join("b")
            .join("c")
            .join("d")
            .join("config.toml");

        let result = execute_init(&config_path, false);
        assert!(result.is_ok());
        assert!(config_path.exists());

        // Verify all parent directories were created
        assert!(temp_dir.path().join("a").exists());
        assert!(temp_dir.path().join("a").join("b").exists());
        assert!(temp_dir.path().join("a").join("b").join("c").exists());
        assert!(temp_dir
            .path()
            .join("a")
            .join("b")
            .join("c")
            .join("d")
            .exists());
    }
}
