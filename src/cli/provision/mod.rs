//! Exchange provisioning command handlers.

mod polymarket;

use clap::Subcommand;

use crate::error::Result;

pub use polymarket::{ProvisionPolymarketArgs, WalletMode};

/// Subcommands for `edgelord provision`.
#[derive(Subcommand, Debug)]
pub enum ProvisionCommand {
    /// Provision Polymarket configuration
    Polymarket(ProvisionPolymarketArgs),
}

/// Execute provisioning command.
pub async fn execute(command: ProvisionCommand) -> Result<()> {
    match command {
        ProvisionCommand::Polymarket(args) => polymarket::execute_polymarket(args),
    }
}
