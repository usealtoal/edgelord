mod config;
mod error;

use config::Config;

fn main() {
    // Load environment variables from .env if present
    let _ = dotenvy::dotenv();

    let config = Config::load("config.toml").expect("Failed to load config");
    println!("Config loaded: {:?}", config);
}
