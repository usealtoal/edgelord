//! Notification adapters.
//!
//! Implements the [`Notifier`](crate::port::outbound::notifier::Notifier) trait
//! for various notification backends. Currently supports Telegram notifications
//! when the `telegram` feature is enabled.

#[cfg(feature = "telegram")]
pub mod telegram;

#[cfg(test)]
mod tests;
