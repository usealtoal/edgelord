use edgelord::cli::{Cli, Commands};
use clap::Parser;

#[test]
fn provision_command_is_registered() {
    let cli = Cli::parse_from(["edgelord", "provision", "polymarket"]);
    match cli.command {
        Commands::Provision(_) => {}
        _ => panic!("expected provision subcommand"),
    }
}
