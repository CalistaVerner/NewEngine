use newengine_core::{EngineResult, Module, ModuleCtx};
use std::sync::Arc;
use winit::window::Window;

/// App-level events passed through engine bus.
#[derive(Debug, Clone)]
pub enum AppEvent {
    // later...
}

/// Shared window handle published into Resources.
pub type WindowHandle = Arc<Window>;

pub struct WindowModule;

impl WindowModule {
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

impl Module<AppEvent> for WindowModule {
    fn id(&self) -> &'static str {
        "window"
    }

    fn start(&mut self, ctx: &mut ModuleCtx<'_, AppEvent>) -> EngineResult<()> {
        let _ = ctx;
        Ok(())
    }
}