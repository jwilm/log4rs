//! Implementations of the `encode::Write` trait.

pub use self::ansi::AnsiWriter;
pub use self::console::{ConsoleWriter, ConsoleWriterLock};
pub use self::simple::SimpleWriter;

mod ansi;
mod console;
mod simple;
