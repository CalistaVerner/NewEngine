use crate::commands::EngineEvent;
use crate::config::EngineConfig;
use crate::error::{EngineError, EngineResult};
use crate::module::{Bus, BusImpl, Module, ModuleCtx, Resources};
use crate::schedule::Scheduler;
use crate::services::Services;
use crate::signals::ShutdownFlag;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes},
};

pub struct Engine {
    cfg: EngineConfig,
    modules: Vec<Box<dyn Module>>,
    bus: BusImpl,
    resources: Resources,
    scheduler: Scheduler,
    shutdown: ShutdownFlag,
    services: EngineServices,
}

/// Engine-wide immutable services.
/// Stored separately to avoid borrowing the whole Engine immutably.
struct EngineServices;

impl EngineServices {
    #[inline(always)]
    fn new() -> Self {
        Self
    }
}

impl Services for EngineServices {
    fn logger(&self) -> &dyn log::Log {
        log::logger()
    }
}

impl Engine {
    pub fn new(cfg: EngineConfig) -> EngineResult<Self> {
        let fixed_dt = (cfg.fixed_dt_ms / 1000.0).max(0.001);

        let shutdown = ShutdownFlag::new();
        shutdown
            .install_ctrlc()
            .map_err(|e| EngineError::Other(e.to_string()))?;

        Ok(Self {
            cfg,
            modules: Vec::new(),
            bus: BusImpl::new(),
            resources: Resources::new(),
            scheduler: Scheduler::new(fixed_dt),
            shutdown,
            services: EngineServices::new(),
        })
    }

    pub fn add_module<M: Module + 'static>(&mut self, m: M) {
        self.modules.push(Box::new(m));
    }

    pub fn run(mut self) -> EngineResult<()> {
        let event_loop = EventLoop::new().map_err(EngineError::from)?;
        let mut app = EngineApp::new(&mut self)?;
        event_loop
            .run_app(&mut app)
            .map_err(|e| EngineError::Winit(e.to_string()))?;
        Ok(())
    }

    /// Build a module context without borrowing the whole Engine immutably.
    #[inline(always)]
    fn make_ctx<'s, 'r>(
        services: &'s dyn Services,
        bus: &'r mut dyn Bus,
        resources: &'r mut Resources,
    ) -> ModuleCtx<'s, 'r> {
        ModuleCtx { services, bus, resources }
    }

    fn with_modules<F>(&mut self, mut f: F) -> EngineResult<()>
    where
        F: FnMut(&mut Engine, &mut [Box<dyn Module>]) -> EngineResult<()>,
    {
        let mut mods = std::mem::take(&mut self.modules);
        let res = f(self, &mut mods);
        self.modules = mods;
        res
    }

    fn modules_register(&mut self) -> EngineResult<()> {
        self.with_modules(|engine, mods| {
            for m in mods.iter_mut() {
                let id = m.id();

                let services = &engine.services as &dyn Services;
                let mut ctx = Engine::make_ctx(services, &mut engine.bus, &mut engine.resources);

                m.register(&mut ctx)
                    .map_err(|e| EngineError::Module { module: id, source: e })?;
            }
            Ok(())
        })
    }

    fn modules_event(&mut self, ev: &EngineEvent) -> EngineResult<()> {
        self.with_modules(|engine, mods| {
            for m in mods.iter_mut() {
                let id = m.id();

                let services = &engine.services as &dyn Services;
                let mut ctx = Engine::make_ctx(services, &mut engine.bus, &mut engine.resources);

                m.on_event(&mut ctx, ev)
                    .map_err(|e| EngineError::Module { module: id, source: e })?;
            }
            Ok(())
        })
    }

    fn modules_update(&mut self, dt: f32) -> EngineResult<()> {
        self.with_modules(|engine, mods| {
            for m in mods.iter_mut() {
                let id = m.id();

                let services = &engine.services as &dyn Services;
                let mut ctx = Engine::make_ctx(services, &mut engine.bus, &mut engine.resources);

                m.update(&mut ctx, dt)
                    .map_err(|e| EngineError::Module { module: id, source: e })?;
            }
            Ok(())
        })
    }

    fn modules_fixed(&mut self, fixed_dt: f32) -> EngineResult<()> {
        self.with_modules(|engine, mods| {
            for m in mods.iter_mut() {
                let id = m.id();

                let services = &engine.services as &dyn Services;
                let mut ctx = Engine::make_ctx(services, &mut engine.bus, &mut engine.resources);

                m.fixed_update(&mut ctx, fixed_dt)
                    .map_err(|e| EngineError::Module { module: id, source: e })?;
            }
            Ok(())
        })
    }

    fn modules_render(&mut self) -> EngineResult<()> {
        self.with_modules(|engine, mods| {
            for m in mods.iter_mut() {
                let id = m.id();

                let services = &engine.services as &dyn Services;
                let mut ctx = Engine::make_ctx(services, &mut engine.bus, &mut engine.resources);

                m.render(&mut ctx)
                    .map_err(|e| EngineError::Module { module: id, source: e })?;
            }
            Ok(())
        })
    }

    fn modules_shutdown(&mut self) -> EngineResult<()> {
        self.with_modules(|engine, mods| {
            for m in mods.iter_mut() {
                let id = m.id();

                let services = &engine.services as &dyn Services;
                let mut ctx = Engine::make_ctx(services, &mut engine.bus, &mut engine.resources);

                m.shutdown(&mut ctx)
                    .map_err(|e| EngineError::Module { module: id, source: e })?;
            }
            Ok(())
        })
    }
}

struct EngineApp<'a> {
    engine: &'a mut Engine,
    window: Option<Window>,
    started: bool,
}

impl<'a> EngineApp<'a> {
    fn new(engine: &'a mut Engine) -> EngineResult<Self> {
        Ok(Self {
            engine,
            window: None,
            started: false,
        })
    }

    fn tick(&mut self) -> EngineResult<()> {
        if self.engine.shutdown.is_set() {
            self.engine.bus.emit(EngineEvent::ShutdownRequested);
        }

        let dt = self.engine.scheduler.next_dt();
        self.engine.scheduler.fixed.push(dt);

        // Drain events first. Avoid holding a mutable borrow of the bus across module calls.
        let mut drained = Vec::new();
        while let Some(ev) = self.engine.bus.poll_event() {
            drained.push(ev);
        }

        for ev in drained {
            if matches!(ev, EngineEvent::ShutdownRequested) {
                self.engine.modules_shutdown()?;
                self.engine.shutdown.set();
            }
            self.engine.modules_event(&ev)?;
        }

        self.engine.modules_update(dt)?;

        let fixed_dt = self.engine.scheduler.fixed.fixed_dt;
        while self.engine.scheduler.fixed.pop() {
            self.engine.modules_fixed(fixed_dt)?;
        }

        Ok(())
    }
}

impl ApplicationHandler for EngineApp<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = WindowAttributes::default()
                .with_title(self.engine.cfg.title.clone())
                .with_inner_size(PhysicalSize::new(self.engine.cfg.width, self.engine.cfg.height));
            let w = event_loop.create_window(attrs).expect("create window");
            self.window = Some(w);

            if let Err(e) = self.engine.modules_register() {
                log::error!("modules_register failed: {}", e);
                event_loop.exit();
                return;
            }

            self.started = true;
            log::info!("[Engine] boot");
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => self.engine.bus.emit(EngineEvent::ShutdownRequested),
            WindowEvent::Resized(sz) => self
                .engine
                .bus
                .emit(EngineEvent::WindowResized { w: sz.width, h: sz.height }),
            WindowEvent::Focused(f) => self.engine.bus.emit(EngineEvent::FocusChanged { focused: f }),
            WindowEvent::RedrawRequested => {
                if self.started && !self.engine.shutdown.is_set() {
                    if let Err(e) = self.engine.modules_render() {
                        log::error!("render failed: {}", e);
                        self.engine.bus.emit(EngineEvent::ShutdownRequested);
                    }
                }
            }
            _ => {}
        }

        if self.engine.shutdown.is_set() {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.started {
            return;
        }
        if self.engine.shutdown.is_set() {
            event_loop.exit();
            return;
        }

        if let Err(e) = self.tick() {
            log::error!("tick failed: {}", e);
            event_loop.exit();
            return;
        }

        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
