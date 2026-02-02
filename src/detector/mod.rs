mod config;
mod single;

pub use config::DetectorConfig;
pub use single::detect_single_condition;

#[allow(unused_imports)]
pub use single::scan_all;
