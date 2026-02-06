//! Wallet operations CLI handlers.
//!
//! Command handlers are split by subcommand to keep each file focused.

mod address;
mod approve;
mod status;
mod sweep;

pub use address::execute_address;
pub use approve::execute_approve;
pub use status::execute_status;
pub use sweep::execute_sweep;
