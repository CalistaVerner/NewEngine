use std::path::PathBuf;
use std::io;

#[derive(Debug)]
pub enum StartupError {
    Io(PathBuf, io::Error),
    Parse(PathBuf, serde_json::Error),
}