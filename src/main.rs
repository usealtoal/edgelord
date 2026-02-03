use clap::Parser;
use edgelord::cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Run(args) => edgelord::cli::run::execute(&cli, args).await,
        Commands::Status => {
            edgelord::cli::status::execute();
            Ok(())
        }
        Commands::Logs(args) => {
            edgelord::cli::logs::execute(args);
            Ok(())
        }
        Commands::Install(args) => {
            edgelord::cli::service::execute_install(args);
            Ok(())
        }
        Commands::Uninstall => {
            edgelord::cli::service::execute_uninstall();
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
