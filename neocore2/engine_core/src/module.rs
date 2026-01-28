use crate::{frame::FrameContext, phase::FramePhase};

/// Базовый контракт "модуля" (подсистемы/фичи/пакета).
/// Дальше вы можете развить это в "SystemGraph" + jobs,
/// но этот интерфейс уже совместим с масштабированием, если соблюдать границы.
pub trait Module {
    fn on_register(&mut self, _ctx: &mut FrameContext<'_>) {}
    fn on_start(&mut self, _ctx: &mut FrameContext<'_>) {}
    fn on_phase(&mut self, _phase: FramePhase, _ctx: &mut FrameContext<'_>) {}
    fn on_shutdown(&mut self, _ctx: &mut FrameContext<'_>) {}
}