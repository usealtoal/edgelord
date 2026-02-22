//! Trait definitions (hexagonal ports). Depend only on domain.
//!
//! Ports define extension points around the application core. This module is
//! grouped by direction:
//! - `inbound`: drivers that call into application capabilities.
//! - `outbound`: dependencies the application calls out to.
//!
//! # Architecture
//!
//! ```text
//!                    ┌─────────────────────────┐
//!                    │      Application        │
//!                    │                         │
//!     ┌──────────────┤  Domain + Port          ├──────────────┐
//!     │              │                         │              │
//!     │              └─────────────────────────┘              │
//!     │                         │                             │
//!     ▼                         ▼                             ▼
//! ┌─────────┐            ┌─────────────┐              ┌───────────┐
//! │Exchange │            │  Notifier   │              │  Solver   │
//! │ Adapter │            │   Adapter   │              │  Adapter  │
//! └─────────┘            └─────────────┘              └───────────┘
//! ```
//!
pub mod inbound;
pub mod outbound;
