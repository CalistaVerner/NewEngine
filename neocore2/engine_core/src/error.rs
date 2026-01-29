use thiserror::Error;

pub type EngineResult<T> = Result<T, EngineError>;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("winit error: {0}")]
    Winit(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("module error [{module}]: {source}")]
    Module {
        module: &'static str,
        #[source]
        source: anyhow::Error,
    },

    #[error("engine error: {0}")]
    Other(String),
}

impl From<winit::error::EventLoopError> for EngineError {
    fn from(e: winit::error::EventLoopError) -> Self {
        Self::Winit(e.to_string())
    }
}