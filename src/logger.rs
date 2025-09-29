use std::sync::atomic::{AtomicBool, Ordering};

use log::{Level, LevelFilter, Log};

pub fn init(verbose: bool) {
    log::set_logger(&LOGGER).expect("failed to set logger");
    let level = if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    log::set_max_level(level);
    if verbose {
        LOGGER.verbose.store(true, Ordering::Relaxed);
    }
}

static LOGGER: Logger = Logger {
    verbose: AtomicBool::new(false),
};

struct Logger {
    verbose: AtomicBool,
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let lvl = if self.verbose.load(Ordering::Relaxed) {
            Level::Debug
        } else {
            Level::Info
        };
        metadata.level() >= lvl
    }

    fn log(&self, record: &log::Record) {
        eprintln!(
            "{:>5} [{}] {}",
            record.level(),
            record.target(),
            record.args()
        );
    }

    fn flush(&self) {}
}
