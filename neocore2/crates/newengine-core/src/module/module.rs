use crate::error::EngineResult;
use crate::frame::Frame;

use super::ModuleCtx;

/// Module lifecycle contract.
///
/// Modules must be self-contained and communicate only via:
/// - Resources (typed APIs/handles)
/// - Bus events
/// - external events injected by platform adapter
pub trait Module<E: Send + 'static>: Send {
    fn id(&self) -> &'static str;

    fn init(&mut self, _ctx: &mut ModuleCtx<'_, E>) -> EngineResult<()> {
        Ok(())
    }

    fn start(&mut self, _ctx: &mut ModuleCtx<'_, E>) -> EngineResult<()> {
        Ok(())
    }

    fn update(&mut self, _ctx: &mut ModuleCtx<'_, E>, _frame: &Frame) -> EngineResult<()> {
        Ok(())
    }

    fn fixed_update(&mut self, _ctx: &mut ModuleCtx<'_, E>, _frame: &Frame) -> EngineResult<()> {
        Ok(())
    }

    fn render(&mut self, _ctx: &mut ModuleCtx<'_, E>, _frame: &Frame) -> EngineResult<()> {
        Ok(())
    }

    fn on_external_event(
        &mut self,
        _ctx: &mut ModuleCtx<'_, E>,
        _event: &dyn std::any::Any,
    ) -> EngineResult<()> {
        Ok(())
    }

    fn shutdown(&mut self, _ctx: &mut ModuleCtx<'_, E>) -> EngineResult<()> {
        Ok(())
    }
}