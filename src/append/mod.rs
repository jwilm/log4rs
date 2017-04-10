//! Appenders

use std::error::Error;
use std::fmt;
use std::io;
use log::LogRecord;

pub mod file;
pub mod console;

/// A trait implemented by log4rs appenders.
///
/// Appenders take a log record and processes them, for example, by writing it
/// to a file or the console.
pub trait Append: fmt::Debug + Send + Sync + 'static {
    /// Processes the provided `LogRecord`.
    fn append(&self, record: &LogRecord) -> Result<(), Box<Error>>;

    /// Called after a log file has been rotated. Doesn't apply to non-file appenders.
    fn post_rotate(&self) -> io::Result<()> {
        Ok(())
    }
}
