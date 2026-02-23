//! Hexagonal architecture ports for the application core.
//!
//! Ports define extension points (interfaces) around the application core,
//! following the ports-and-adapters (hexagonal) architecture pattern. This
//! module organizes ports by direction:
//!
//! - [`inbound`]: Driving ports consumed by inbound adapters (CLI, Telegram, etc.)
//! - [`outbound`]: Driven ports implemented by outbound adapters (exchanges, storage, etc.)
//!
//! # Architecture Overview
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
//! # Design Principles
//!
//! - Ports depend only on domain types, never on infrastructure
//! - Inbound ports expose application capabilities to drivers
//! - Outbound ports abstract infrastructure dependencies
//! - All traits are designed for testability with mock implementations
//!
pub mod inbound;
pub mod outbound;
