#[macro_use]
extern crate log;
extern crate log4rs;

use std::default::Default;
use std::thread;
use std::time::Duration;

fn main() {
    log4rs::init_file("log.toml", Default::default()).unwrap();

    loop {
        thread::sleep(Duration::from_secs(1));
        warn!("main");
        error!("error main");
        a::foo();
    }
}

mod a {
    pub fn foo() {
        info!("a");
    }
}
