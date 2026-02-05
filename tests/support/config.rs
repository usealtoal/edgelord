use edgelord::app::ReconnectionConfig;

pub fn test_reconnection_config() -> ReconnectionConfig {
    ReconnectionConfig {
        initial_delay_ms: 0,
        max_delay_ms: 0,
        backoff_multiplier: 1.0,
        max_consecutive_failures: 3,
        circuit_breaker_cooldown_ms: 0,
    }
}
