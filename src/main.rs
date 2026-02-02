use edgelord::app::Config;
#[cfg(feature = "polymarket")]
use edgelord::app::App;
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let config = match Config::load("config.toml") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {e}");
            std::process::exit(1);
        }
    };

    config.init_logging();
    info!("edgelord starting");

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
        // Without polymarket feature, the binary just loads config and exits
        // This allows testing that core domain/exchange modules compile
        let _ = config;
        info!("No exchange features enabled - exiting");
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received");
            }
        }
    }

    info!("edgelord stopped");
}
