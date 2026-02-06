//! Handlers for `install` and `uninstall` commands.

mod common;
mod install;
mod uninstall;

pub use install::execute_install;
pub use uninstall::execute_uninstall;
