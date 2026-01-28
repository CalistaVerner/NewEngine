use crate::{engine::EngineContext, phase::FramePhase};

pub trait Module: Send {
    fn name(&self) -> &'static str;

    fn on_register(&mut self, _ctx: &mut EngineContext) {}
    fn on_start(&mut self, _ctx: &mut EngineContext) {}
    fn on_shutdown(&mut self, _ctx: &mut EngineContext) {}

    fn on_phase(&mut self, _phase: FramePhase, _ctx: &mut EngineContext) {}
}
