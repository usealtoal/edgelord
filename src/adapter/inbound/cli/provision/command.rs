//! Exchange provisioning command handlers.

use clap::Subcommand;

use super::polymarket::{self, ProvisionPolymarketArgs};
use crate::error::Result;

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
