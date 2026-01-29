use crossbeam_channel::unbounded;

use newengine_core::{Bus, Engine, EngineResult, Services};
use newengine_platform_winit::run_winit_app;

struct AppServices;

impl AppServices {
    #[inline]
    fn new() -> Self {
        Self
    }
}

impl Services for AppServices {
    fn logger(&self) -> &dyn log::Log {
        log::logger()
    }
}

#[derive(Debug, Clone)]
enum EditorEvent {
    Exit,
}

fn main() -> EngineResult<()> {
    let (tx, rx) = unbounded::<EditorEvent>();
    let bus = Bus::new(tx, rx);

    let services: Box<dyn Services> = Box::new(AppServices::new());
    let mut engine = Engine::new(16, services, bus)?;

    engine.register_module(Box::new(newengine_modules_cef::CefModule::new()))?;

    engine.start()?;
    run_winit_app(engine)
}