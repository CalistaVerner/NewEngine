use std::time::Instant;

use anyhow::Result;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{log::Logger, module::Module, phase::FramePhase, time::Time};

pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,

    pub fixed_dt_sec: f32,

    pub log_fps: bool,
    pub fps_log_period_sec: f32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "NEOCORE2".to_string(),
            width: 1280,
            height: 720,
            fixed_dt_sec: 1.0 / 60.0,
            log_fps: true,
            fps_log_period_sec: 1.0,
        }
    }
}

pub struct EngineContext<'a> {
    pub log: &'a Logger,
    pub window: &'a Window,
    pub time: &'a Time,
    pub exit_requested: &'a mut bool,
}

pub struct Engine {
    cfg: EngineConfig,
    log: Logger,
    modules: Vec<Box<dyn Module>>,
}

impl Engine {
    pub fn new(cfg: EngineConfig) -> Self {
        Self {
            cfg,
            log: Logger::new("Engine"),
            modules: Vec::new(),
        }
    }

    pub fn add_module<M: Module + 'static>(&mut self, m: M) {
        self.modules.push(Box::new(m));
    }

    pub fn run(self) -> Result<()> {
        let event_loop = EventLoop::new()?;

        let mut app = EngineApp::new(self);
        event_loop.run_app(&mut app)?;

        Ok(())
    }
}

struct EngineApp {
    engine: Engine,

    window: Option<Window>,
    window_id: Option<WindowId>,

    exit_requested: bool,
    shutdown_done: bool,
    started: bool,

    time: Time,
    last: Instant,
    accumulator: f32,

    fps_last: Instant,
    fps_frames: u32,
}

impl EngineApp {
    fn new(engine: Engine) -> Self {
        let fixed_dt = engine.cfg.fixed_dt_sec;
        Self {
            engine,
            window: None,
            window_id: None,

            exit_requested: false,
            shutdown_done: false,
            started: false,

            time: Time::new(fixed_dt),
            last: Instant::now(),
            accumulator: 0.0,

            fps_last: Instant::now(),
            fps_frames: 0,
        }
    }

    fn start_modules(&mut self) {
        if self.started {
            return;
        }
        let Some(window) = self.window.as_ref() else { return; };

        self.engine.log.info("boot");

        {
            let mut ctx = EngineContext {
                log: &self.engine.log,
                window,
                time: &self.time,
                exit_requested: &mut self.exit_requested,
            };
            for m in self.engine.modules.iter_mut() {
                m.on_register(&mut ctx);
            }
            for m in self.engine.modules.iter_mut() {
                m.on_start(&mut ctx);
            }
        }

        self.started = true;
        self.last = Instant::now();
        self.fps_last = Instant::now();
        self.fps_frames = 0;
        self.accumulator = 0.0;
    }

    fn phase(&mut self, phase: FramePhase) {
        let Some(window) = self.window.as_ref() else { return; };

        let mut ctx = EngineContext {
            log: &self.engine.log,
            window,
            time: &self.time,
            exit_requested: &mut self.exit_requested,
        };

        for m in self.engine.modules.iter_mut() {
            m.on_phase(phase, &mut ctx);
        }
    }

    fn maybe_shutdown(&mut self, el: &ActiveEventLoop) {
        if self.shutdown_done {
            return;
        }
        if !self.exit_requested {
            return;
        }

        self.shutdown_done = true;

        if let Some(window) = self.window.as_ref() {
            let mut ctx = EngineContext {
                log: &self.engine.log,
                window,
                time: &self.time,
                exit_requested: &mut self.exit_requested,
            };
            for m in self.engine.modules.iter_mut().rev() {
                m.on_shutdown(&mut ctx);
            }
        }

        self.engine.log.info("shutdown");
        el.exit();
    }
}

impl ApplicationHandler for EngineApp {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title(self.engine.cfg.title.clone())
            .with_inner_size(LogicalSize::new(self.engine.cfg.width, self.engine.cfg.height));

        let window = match el.create_window(attrs) {
            Ok(w) => w,
            Err(e) => {
                // Нечего “красиво” делать — лучше честно выйти.
                eprintln!("failed to create window: {e}");
                el.exit();
                return;
            }
        };

        self.window_id = Some(window.id());
        self.window = Some(window);

        self.start_modules();
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if Some(id) != self.window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => self.exit_requested = true,
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    if let PhysicalKey::Code(code) = event.physical_key {
                        if code == KeyCode::Escape {
                            self.exit_requested = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        el.set_control_flow(ControlFlow::Poll);

        if !self.started {
            return;
        }

        // Если уже попросили выйти — закрываемся один раз, без дублей.
        if self.exit_requested {
            self.maybe_shutdown(el);
            return;
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last);
        self.last = now;

        let dt_sec = dt.as_secs_f32().min(0.25);
        self.time.dt_sec = dt_sec;
        self.time.t_sec += dt.as_secs_f64();
        self.time.frame_index += 1;

        self.accumulator += dt_sec;

        self.phase(FramePhase::BeginFrame);
        self.phase(FramePhase::Input);

        while self.accumulator >= self.engine.cfg.fixed_dt_sec {
            self.time.fixed_tick_index += 1;
            self.phase(FramePhase::FixedUpdate);
            self.accumulator -= self.engine.cfg.fixed_dt_sec;
        }

        self.time.fixed_alpha = (self.accumulator / self.engine.cfg.fixed_dt_sec).clamp(0.0, 1.0);

        self.phase(FramePhase::Update);
        self.phase(FramePhase::LateUpdate);
        self.phase(FramePhase::Render);
        self.phase(FramePhase::Present);
        self.phase(FramePhase::EndFrame);

        if self.engine.cfg.log_fps {
            self.fps_frames += 1;
            let period = self.engine.cfg.fps_log_period_sec.max(0.25);
            if self.fps_last.elapsed().as_secs_f32() >= period {
                let secs = self.fps_last.elapsed().as_secs_f32().max(0.0001);
                let fps = (self.fps_frames as f32) / secs;

                self.engine.log.info(format!(
                    "fps={:.1} dt_ms={:.2} fixed_alpha={:.2} fixed_tick={}",
                    fps,
                    self.time.dt_sec * 1000.0,
                    self.time.fixed_alpha,
                    self.time.fixed_tick_index
                ));

                self.fps_frames = 0;
                self.fps_last = Instant::now();
            }
        }

        // На всякий случай — если какой-то модуль поставил exit_requested в фазах.
        if self.exit_requested {
            self.maybe_shutdown(el);
            return;
        }

        if let Some(w) = self.window.as_ref() {
            w.request_redraw();
        }
    }
}