#[macro_use]
extern crate log;
extern crate log4rs;

use std::path::Path;
use std::fs::{self, File};
use std::io::{self, Read};
use std::error::Error;

use log::LogLevelFilter;
use log4rs::config::{Appender, Logger, Config, Root};
use log4rs::append::file::FileAppender;

/// Get config for a simple logger
fn logger_config(target: &Path) -> Result<Config, Box<Error>> {
    let appender =  Box::new(try!(FileAppender::builder().build(target)));

    let builder = Config::builder()
        .appender(Appender::builder()
            .build("file".into(), appender))
        .logger(Logger::builder()
            .appender("file".into())
            .additive(false)
            .build("reopen".into(), LogLevelFilter::Warn));

    let root = Root::builder().appender("file".into()).build(LogLevelFilter::Error);

    Ok(try!(builder.build(root)))
}

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
#[test]
fn file() {
    let target = &Path::new("./appender.log");
    let rotated = &Path::new("./appender.log.0");

    // Guards to cleanup files
    let _remove_target = RemoveGuard(target);
    let _remove_rotated = RemoveGuard(rotated);

    let config = logger_config(target).unwrap();
    let handle = log4rs::init_config(config).unwrap();

    // Log before rotating file
    warn!("before");
    // Rotate file
    fs::rename(target, rotated).unwrap();
    // Log after rotate file before appender is notifiedd
    warn!("renamed");

    // Reopen appenders
    handle.reopen().unwrap();

    // Log after rotating and reopening file
    warn!("after");

    // Read file contents
    let old_contents = contents(rotated).unwrap();
    let new_contents = contents(target).unwrap();

    // Check before rotate and before reopen logs are in old_contents (rotated)
    assert!(old_contents.contains("before"));
    assert!(old_contents.contains("renamed"));

    // Check after rotate and reopen logs are in new_contents (target)
    assert!(new_contents.contains("after"));
}
