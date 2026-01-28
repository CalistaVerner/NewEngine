use engine_core::{
    engine::{Engine, EngineConfig, EngineContext},
    module::Module,
    phase::FramePhase,
};

struct DebugModule;

impl Module for DebugModule {
    fn name(&self) -> &'static str {
        "DebugModule"
    }

    fn on_start(&mut self, ctx: &mut EngineContext) {
        ctx.log.info("debug module start");
    }

    fn on_phase(&mut self, phase: FramePhase, ctx: &mut EngineContext) {
        if phase == FramePhase::BeginFrame && ctx.time.frame_index == 1 {
            ctx.log.info("first frame");
        }

        if phase == FramePhase::FixedUpdate && (ctx.time.fixed_tick_index % 60 == 0) {
            ctx.log
                .debug(format!("fixed tick {}", ctx.time.fixed_tick_index));
        }
    }

    fn on_shutdown(&mut self, ctx: &mut EngineContext) {
        ctx.log.info("debug module shutdown");
    }
}

fn main() -> anyhow::Result<()> {
    let mut engine = Engine::new(EngineConfig::default());
    engine.add_module(DebugModule);
    engine.run()
}
