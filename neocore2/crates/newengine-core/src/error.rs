use std::fmt;

/// Engine-wide error.
///
/// Keep this small and stable. Modules can define their own error types and map them into EngineError.
#[derive(Debug)]
pub enum EngineError {
    ExitRequested,
    Other(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::ExitRequested => write!(f, "exit requested"),
            EngineError::Other(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for EngineError {}

pub type EngineResult<T> = Result<T, EngineError>;