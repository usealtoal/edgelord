//! Exchange provisioning commands.

use clap::{Parser, Subcommand};

use crate::error::Result;

/// Subcommands for `edgelord provision`.
#[derive(Subcommand, Debug)]
pub enum ProvisionCommand {
    /// Provision Polymarket configuration
    Polymarket(ProvisionPolymarketArgs),
}

/// Arguments for `edgelord provision polymarket`.
#[derive(Parser, Debug, Default)]
pub struct ProvisionPolymarketArgs {}

/// Execute provisioning command.
pub async fn execute(command: ProvisionCommand) -> Result<()> {
    match command {
        ProvisionCommand::Polymarket(_args) => {
            println!("Provisioning not implemented yet.");
            Ok(())
        }
    }
}
