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
        Commands::CheckConfig => {
            edgelord::cli::check::execute_check_config(&cli.config);
            Ok(())
        }
        Commands::TestTelegram => edgelord::cli::check::execute_test_telegram(&cli.config).await,
        Commands::CheckConnection => {
            edgelord::cli::check::execute_check_connection(&cli.config).await
        }
        Commands::Approve(args) => {
            edgelord::cli::wallet::execute_approve(&cli.config, args.amount, args.yes).await
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
