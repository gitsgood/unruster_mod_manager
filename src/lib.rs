use std::{cmp::Ordering, fs::File, str::FromStr, io::Write};
use semver::Version;
use rayon::prelude::*;

pub mod json_structs;
pub mod constants_for_posterity;
pub mod helpers;
pub mod enums;
pub mod unruster_state;
pub mod logger;

pub use json_structs::*;
pub use constants_for_posterity::*;
pub use helpers::*;
pub use enums::*;
pub use unruster_state::*;
pub use logger::*;

pub type UResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

use once_cell::sync::Lazy;

// Global logger instance
pub static LOGGER: Lazy<Logger> = Lazy::new(|| Logger::new());

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        LOGGER.info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        LOGGER.warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        LOGGER.error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        LOGGER.debug(&format!($($arg)*))
    };
}