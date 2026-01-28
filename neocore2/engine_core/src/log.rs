use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy)]
pub enum Level {
    Info,
    Warn,
    Error,
    Debug,
}

#[derive(Clone)]
pub struct Logger {
    tag: &'static str,
}

impl Logger {
    pub fn new(tag: &'static str) -> Self {
        Self { tag }
    }

    pub fn log(&self, lvl: Level, msg: impl AsRef<str>) {
        let ts = now_ms();
        let lvl_s = match lvl {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
            Level::Debug => "DEBUG",
        };
        eprintln!(
            "[{ts}] [{lvl_s}] [{tag}] {msg}",
            ts = ts,
            tag = self.tag,
            msg = msg.as_ref()
        );
    }

    pub fn info(&self, msg: impl AsRef<str>) {
        self.log(Level::Info, msg);
    }
    pub fn warn(&self, msg: impl AsRef<str>) {
        self.log(Level::Warn, msg);
    }
    pub fn error(&self, msg: impl AsRef<str>) {
        self.log(Level::Error, msg);
    }
    pub fn debug(&self, msg: impl AsRef<str>) {
        self.log(Level::Debug, msg);
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
