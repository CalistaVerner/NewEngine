use std::time::Instant;

use crate::{frame::FrameContext, module::Module, phase::FramePhase};

/// Расписание кадра. Единственная точка истины: кто/когда вызывается.
/// На этом слое дальше легко вводить:
/// - параллелизм (jobs)
/// - зависимости систем
/// - профилирование по системам
pub struct FrameSchedule {
    modules: Vec<Box<dyn Module>>,
}

impl FrameSchedule {
    pub fn new() -> Self {
        Self { modules: Vec::new() }
    }

    pub fn add_module<M: Module + 'static>(&mut self, m: M) {
        self.modules.push(Box::new(m));
    }

    pub fn on_register(&mut self, ctx: &mut FrameContext<'_>) {
        for m in self.modules.iter_mut() {
            m.on_register(ctx);
        }
    }

    pub fn on_start(&mut self, ctx: &mut FrameContext<'_>) {
        for m in self.modules.iter_mut() {
            m.on_start(ctx);
        }
    }

    pub fn on_shutdown(&mut self, ctx: &mut FrameContext<'_>) {
        // Shutdown лучше делать в обратном порядке регистрации.
        for m in self.modules.iter_mut().rev() {
            m.on_shutdown(ctx);
        }
    }

    /// Главный вызов фаз.
    /// Встроенное профилирование — фундамент наблюдаемости.
    pub fn run_phase(&mut self, phase: FramePhase, ctx: &mut FrameContext<'_>) {
        let name = phase.as_str();
        let t0 = Instant::now();

        for m in self.modules.iter_mut() {
            m.on_phase(phase, ctx);
        }

        ctx.telemetry.record_scope(name, t0.elapsed());
    }
}