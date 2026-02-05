use clap::Parser;
use edgelord::cli::{
    CheckCommand, Cli, Commands, ConfigCommand, ServiceCommand, StatsCommand, WalletCommand,
};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run(args) => edgelord::cli::run::execute(&args).await,
        Commands::Status(args) => {
            edgelord::cli::status::execute(&args.db);
            Ok(())
        }
        Commands::Stats(cmd) => match cmd {
            StatsCommand::Today(args) => edgelord::cli::stats::execute_today(&args.db),
            StatsCommand::Week(args) => edgelord::cli::stats::execute_week(&args.db),
            StatsCommand::History(args) => {
                edgelord::cli::stats::execute_history(&args.db, args.days)
            }
            StatsCommand::Export(args) => {
                edgelord::cli::stats::execute_export(&args.db, args.days, args.output.as_deref())
            }
            StatsCommand::Prune(args) => {
                edgelord::cli::stats::execute_prune(&args.db, args.days)
            }
        },
        Commands::Config(cmd) => match cmd {
            ConfigCommand::Init(args) => edgelord::cli::config::execute_init(&args.path, args.force),
            ConfigCommand::Show(args) => edgelord::cli::config::execute_show(&args.config),
            ConfigCommand::Validate(args) => edgelord::cli::config::execute_validate(&args.config),
        },
        Commands::Logs(args) => {
            edgelord::cli::logs::execute(&args);
            Ok(())
        }
        Commands::Service(cmd) => match cmd {
            ServiceCommand::Install(args) => edgelord::cli::service::execute_install(&args),
            ServiceCommand::Uninstall => edgelord::cli::service::execute_uninstall(),
        },
        Commands::Check(cmd) => match cmd {
            CheckCommand::Config(args) => edgelord::cli::check::execute_config(&args.config),
            CheckCommand::Connection(args) => {
                edgelord::cli::check::execute_connection(&args.config).await
            }
            CheckCommand::Telegram(args) => {
                edgelord::cli::check::execute_telegram(&args.config).await
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
