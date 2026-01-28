use engine_core::{
    engine::{Engine, EngineConfig},
    frame::FrameContext,
    module::Module,
    phase::FramePhase,
};

struct DebugModule;

impl Module for DebugModule {
    fn on_start(&mut self, ctx: &mut FrameContext<'_>) {
        // В новой архитектуре лог живёт в telemetry и/или в engine (ядре),
        // но контекст у нас пока даёт только window/time/telemetry/exit.
        // Поэтому для "модульных" логов лучше использовать telemetry,
        // либо расширить FrameContext, добавив ctx.log.
        //
        // Вариант 1 (прямо сейчас): писать в telemetry (тихо) — не подходит.
        // Вариант 2 (правильно): добавить Logger в FrameContext.
        //
        // Пока делаем минимум без изменения core: просто ставим флаг выхода/метрики.
        let _ = ctx.window; // чтобы показать доступность
        // Если хочешь лог именно отсюда — скажи, я добавлю `log: &Logger` в FrameContext.
    }

    fn on_phase(&mut self, phase: FramePhase, ctx: &mut FrameContext<'_>) {
        if phase == FramePhase::BeginFrame && ctx.time.frame_index == 1 {
            // Лучший вариант: лог через ctx.telemetry.record_scope или добавить ctx.log в контекст.
            // Пока оставим "сигнал" через telemetry (не шумит).
            ctx.telemetry.record_scope("Debug:first_frame", std::time::Duration::from_millis(0));
        }

        if phase == FramePhase::FixedUpdate && (ctx.time.fixed_tick_index % 60 == 0) {
            // Аналогично: можно хранить/показывать это в overlay позже.
            // Или добавим ctx.log и вернём твой прежний вывод.
            let _tick = ctx.time.fixed_tick_index;
        }
    }

    fn on_shutdown(&mut self, ctx: &mut FrameContext<'_>) {
        let _ = ctx;
    }
}

fn main() -> anyhow::Result<()> {
    let mut engine = Engine::new(EngineConfig::default());
    engine.add_module(DebugModule);
    engine.run()
}