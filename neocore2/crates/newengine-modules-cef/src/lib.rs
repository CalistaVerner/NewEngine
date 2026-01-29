mod api;

pub use api::{CefApi, CefApiRef};

use newengine_core::{EngineResult, Module, ModuleCtx};
use std::sync::Arc;

struct CefApiStub;

impl CefApi for CefApiStub {
    fn load_local_html(&self, _html: &str) {}
    fn eval_js(&self, _js: &str) {}
}

pub struct CefModule;

impl CefModule {
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

impl<E: Send + 'static> Module<E> for CefModule {
    fn id(&self) -> &'static str {
        "cef"
    }

    fn start(&mut self, ctx: &mut ModuleCtx<'_, E>) -> EngineResult<()> {
        let api: CefApiRef = Arc::new(CefApiStub);
        ctx.resources().insert::<CefApiRef>(Arc::new(api));
        Ok(())
    }
}