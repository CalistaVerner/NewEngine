use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Logger {
    tag: &'static str,
}

impl Logger {
    pub fn new(tag: &'static str) -> Self {
        Self { tag }
    }

    #[inline]
    pub fn info(&self, msg: impl AsRef<str>) {
        self.print("INFO", msg.as_ref());
    }

    #[inline]
    pub fn debug(&self, msg: impl AsRef<str>) {
        self.print("DEBUG", msg.as_ref());
    }

    #[inline]
    pub fn warn(&self, msg: impl AsRef<str>) {
        self.print("WARN", msg.as_ref());
    }

    #[inline]
    pub fn error(&self, msg: impl AsRef<str>) {
        self.print("ERROR", msg.as_ref());
    }

    fn print(&self, lvl: &str, msg: &str) {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let ms = ts.as_millis();
        println!("[{}] [{}] [{}] {}", ms, lvl, self.tag, msg);
    }
}