//! Order building and submission.

#![allow(dead_code)]

use crate::domain::Opportunity;
use crate::error::Result;

/// Executes trades on Polymarket CLOB.
pub struct OrderExecutor;

impl OrderExecutor {
    /// Create new executor (placeholder - will implement authentication in next task).
    pub fn new() -> Self {
        Self
    }

    /// Execute an arbitrage opportunity (placeholder).
    pub async fn execute(&self, _opportunity: &Opportunity) -> Result<()> {
        Ok(())
    }
}

impl Default for OrderExecutor {
    fn default() -> Self {
        Self::new()
    }
}
