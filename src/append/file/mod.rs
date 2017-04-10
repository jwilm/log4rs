//! The file appender.

use antidote::Mutex;
use log::LogRecord;
use serde_value::Value;
use std::error::Error;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Write, BufWriter};
use std::path::{Path, PathBuf};

use append::Append;
use append::file::serde::FileAppenderConfig;
use encode::Encode;
use encode::pattern::PatternEncoder;
use encode::writer::SimpleWriter;
use file::{Deserialize, Deserializers};

#[cfg_attr(rustfmt, rustfmt_skip)]
mod serde;

/// An appender which logs to a file.
pub struct FileAppender {
    path: PathBuf,
    file: Mutex<SimpleWriter<BufWriter<File>>>,
    encoder: Box<Encode>,
    append: bool,
}

impl fmt::Debug for FileAppender {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("FileAppender")
           .field("file", &self.path)
           .field("encoder", &self.encoder)
           .finish()
    }
}

impl Append for FileAppender {
    fn append(&self, record: &LogRecord) -> Result<(), Box<Error>> {
        let mut file = self.file.lock();
        try!(self.encoder.encode(&mut *file, record));
        try!(file.flush());
        Ok(())
    }

    fn post_rotate(&self) -> io::Result<()> {
        // Get lock for current file and flush any pending writes
        let mut writer = self.file.lock();
        try!(writer.flush());

        // Try and open the file. If this fails, the old writer is still in place.
        let file = try!(OpenOptions::new()
                            .write(true)
                            .append(self.append)
                            .create(true)
                            .open(self.path.as_path()));
        let new_writer = SimpleWriter(BufWriter::with_capacity(1024, file));

        // Swap the new writer with the old writer. Since the new file is already opened, the
        // appender will be left in a working state.
        ::std::mem::replace(&mut *writer, new_writer);

        Ok(())
    }
}

impl FileAppender {
    /// Creates a new `FileAppender` builder.
    pub fn builder() -> FileAppenderBuilder {
        FileAppenderBuilder {
            encoder: Box::new(PatternEncoder::default()),
            append: true,
        }
    }
}

/// A builder for `FileAppender`s.
pub struct FileAppenderBuilder {
    encoder: Box<Encode>,
    append: bool,
}

impl FileAppenderBuilder {
    /// Sets the output encoder for the `FileAppender`.
    pub fn encoder(mut self, encoder: Box<Encode>) -> FileAppenderBuilder {
        self.encoder = encoder;
        self
    }

    /// Determines if the appender will append to or truncate the output file.
    ///
    /// Defaults to `true`.
    pub fn append(mut self, append: bool) -> FileAppenderBuilder {
        self.append = append;
        self
    }

    /// Consumes the `FileAppenderBuilder`, producing a `FileAppender`.
    pub fn build<P: AsRef<Path>>(self, path: P) -> io::Result<FileAppender> {
        let path = path.as_ref().to_owned();
        let file = try!(OpenOptions::new()
                            .write(true)
                            .append(self.append)
                            .create(true)
                            .open(&path));

        Ok(FileAppender {
            path: path,
            file: Mutex::new(SimpleWriter(BufWriter::with_capacity(1024, file))),
            encoder: self.encoder,
            append: self.append,
        })
    }
}


/// A deserializer for the `FileAppender`.
///
/// The `path` key is required, and specifies the path to the log file. The
/// `encoder` key is optional and specifies an `Encoder` to be used for output.
/// The `append` key is optional and specifies whether the output file should be
/// truncated or appended to.
pub struct FileAppenderDeserializer;

impl Deserialize for FileAppenderDeserializer {
    type Trait = Append;

    fn deserialize(&self,
                   config: Value,
                   deserializers: &Deserializers)
                   -> Result<Box<Append>, Box<Error>> {
        let config = try!(config.deserialize_into::<FileAppenderConfig>());
        let mut appender = FileAppender::builder();
        if let Some(append) = config.append {
            appender = appender.append(append);
        }
        if let Some(encoder) = config.encoder {
            appender = appender.encoder(try!(deserializers.deserialize("encoder",
                                                                       &encoder.kind,
                                                                       encoder.config)));
        }
        Ok(Box::new(try!(appender.build(&config.path))))
    }
}
