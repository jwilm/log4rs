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

#[cfg(test)]
mod tests {
    use std::path::{Path};
    use std::fs::{self, File};
    use std::io::{self, Read, Write};
    use append::Append;
    use super::FileAppender;

    /// Read the contents of a file into a string
    fn contents(path: &Path) -> io::Result<String> {
        let mut f = try!(File::open(path));
        let mut buffer = String::new();
        try!(f.read_to_string(&mut buffer));

        Ok(buffer)
    }

    /// Type that removes the file pointed to by &Path on drop.
    ///
    /// Used to clean up files in case of test failure or success. Since removal is usually done at
    /// the end of a test, a panic would prevent execution from reaching that step.
    struct RemoveGuard<'a>(&'a Path);

    impl<'a> Drop for RemoveGuard<'a> {
        fn drop(&mut self) {
            // Can't panic during a panic, so just ignore the result. The call is likely to fail in
            // the case a file is never created.
            fs::remove_file(self.0).ok();
        }
    }

    /// Check that file rotation results in logs arriving at the reopened file
    ///
    /// This test accesses the Write object directly to perform I/O. It could be improved by using
    /// the log macros, but that would require a bunch of other setup.
    #[test]
    fn file_reopen_post_rotate() {
        let target = &Path::new("./appender.log");
        let rotated = &Path::new("./appender.log.0");

        // Guards to cleanup files
        let _remove_target = RemoveGuard(target);
        let _remove_rotated = RemoveGuard(rotated);

        let appender = FileAppender::builder().build(target).unwrap();

        // Helper to write into the appender
        macro_rules! append {
            ($s:expr) => {{
                let buf = $s.as_bytes();
                let mut file = appender.file.lock();
                file.write_all(buf).unwrap();
                file.flush().unwrap();
            }}
        }

        // Log before rotating file
        append!("before");
        // Rotate file
        fs::rename(target, rotated).unwrap();
        // Log after rotate file before appender is notifiedd
        append!("renamed");

        // Appender reopens file
        appender.post_rotate().unwrap();

        // Log after rotating and reopening file
        append!("after");

        // Read file contents
        let old_contents = contents(rotated).unwrap();
        let new_contents = contents(target).unwrap();

        // Check before rotate and before reopen logs are in old_contents (rotated)
        assert!(old_contents.contains("before"));
        assert!(old_contents.contains("renamed"));

        // Check after rotate and reopen logs are in new_contents (target)
        assert!(new_contents.contains("after"));
    }
}
