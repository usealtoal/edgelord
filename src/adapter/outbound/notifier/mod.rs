//! Notification adapters.
//!
//! Implements the `port::Notifier` trait for various notification backends.

#[cfg(feature = "telegram")]
pub mod telegram;

#[cfg(test)]
mod tests;
