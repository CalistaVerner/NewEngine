use newengine_core::config::EngineConfig;
use newengine_core::engine::Engine;
use newengine_core::logsys;
use newengine_core::module::builtin::{CefModule, TelemetryModule};

fn main() -> anyhow::Result<()> {
    logsys::init();

    let cfg = EngineConfig::load_or_default("config.toml")?;

    let mut engine = Engine::new(cfg)?;
    engine.add_module(TelemetryModule::new());
    engine.add_module(CefModule::new());
    engine.run()?;

    Ok(())
}