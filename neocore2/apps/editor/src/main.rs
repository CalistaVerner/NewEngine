use crossbeam_channel::unbounded;

use newengine_core::{Bus, Engine, EngineResult, Services, ShutdownToken};
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
    let bus: Bus<EditorEvent> = Bus::new(tx, rx);

    let services: Box<dyn Services> = Box::new(AppServices::new());

    // ВАЖНО: четвертый параметр
    let shutdown = ShutdownToken::new(); // или ShutdownToken::default()

    let mut engine: Engine<EditorEvent> = Engine::new(16, services, bus, shutdown)?;

    engine.register_module(Box::new(newengine_modules_cef::CefModule::new()))?;

    engine.start()?;
    run_winit_app(engine)
}