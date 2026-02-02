mod config;
mod single;

pub use config::DetectorConfig;
pub use single::{detect_single_condition, scan_all};
