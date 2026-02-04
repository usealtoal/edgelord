use clap::Parser;
use edgelord::cli::{CheckCommand, Cli, Commands, ServiceCommand, WalletCommand};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run(args) => edgelord::cli::run::execute(&args).await,
        Commands::Status => {
            edgelord::cli::status::execute();
            Ok(())
        }
        Commands::Logs(args) => {
            edgelord::cli::logs::execute(&args);
            Ok(())
        }
        Commands::Service(cmd) => match cmd {
            ServiceCommand::Install(args) => {
                edgelord::cli::service::execute_install(&args);
                Ok(())
            }
            ServiceCommand::Uninstall => {
                edgelord::cli::service::execute_uninstall();
                Ok(())
            }
        },
        Commands::Check(cmd) => match cmd {
            CheckCommand::Config(args) => {
                edgelord::cli::check::execute_check_config(&args.config);
                Ok(())
            }
            CheckCommand::Connection(args) => {
                edgelord::cli::check::execute_check_connection(&args.config).await
            }
            CheckCommand::Telegram(args) => {
                edgelord::cli::check::execute_test_telegram(&args.config).await
            }
        },
        Commands::Wallet(cmd) => match cmd {
            WalletCommand::Approve(args) => {
                edgelord::cli::wallet::execute_approve(&args.config, args.amount, args.yes).await
            }
            WalletCommand::Status(args) => {
                edgelord::cli::wallet::execute_status(&args.config).await
            }
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
