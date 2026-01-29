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
    let _ = env_logger::builder().is_test(false).try_init();

    let (tx, rx) = unbounded::<EditorEvent>();
    let bus: Bus<EditorEvent> = Bus::new(tx, rx);

    let services: Box<dyn Services> = Box::new(AppServices::new());
    let shutdown = ShutdownToken::new();

    let mut engine: Engine<EditorEvent> = Engine::new(16, services, bus, shutdown)?;

    engine.register_module(Box::new(newengine_modules_cef::CefModule::new()))?;

    engine.register_module(Box::new(EditorCefController::new(
        "https://example.com".to_string(),
    )))?;

    engine.start()?;
    run_winit_app(engine)
}

struct EditorCefController {
    initial_url: String,
    loaded: bool,
}

impl EditorCefController {
    #[inline]
    fn new(initial_url: String) -> Self {
        Self { initial_url, loaded: false }
    }
}

impl<E: Send + 'static> newengine_core::Module<E> for EditorCefController {
    fn id(&self) -> &'static str {
        "editor-cef-controller"
    }

    fn update(&mut self, ctx: &mut newengine_core::ModuleCtx<'_, E>) -> EngineResult<()> {
        if self.loaded {
            return Ok(());
        }

        let cef = match ctx.resources().get::<newengine_modules_cef::CefApiRef>() {
            Some(v) => v.clone(),
            None => return Ok(()),
        };

        if !cef.is_ready() {
            return Ok(());
        }

        cef.ensure_primary_view();
        cef.load_url(&self.initial_url);

        self.loaded = true;
        Ok(())
    }
}