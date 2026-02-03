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
