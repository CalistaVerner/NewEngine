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

use crate::{
    frame::{FrameConstitution, FrameContext},
    log::Logger,
    phase::FramePhase,
    schedule::FrameSchedule,
    signals::ExitSignal,
    telemetry::Telemetry,
    time::Time,
};

pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,

    pub frame: FrameConstitution,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "NEOCORE2".to_string(),
            width: 1280,
            height: 720,
            frame: FrameConstitution::default(),
        }
    }
}

pub struct Engine {
    cfg: EngineConfig,
    log: Logger,
    schedule: FrameSchedule,
}

impl Engine {
    pub fn new(cfg: EngineConfig) -> Self {
        Self {
            cfg,
            log: Logger::new("Engine"),
            schedule: FrameSchedule::new(),
        }
    }

    pub fn add_module<M: crate::module::Module + 'static>(&mut self, m: M) {
        self.schedule.add_module(m);
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

    constitution: FrameConstitution,
    time: Time,
    telemetry: Telemetry,

    last: Instant,
    accumulator: f32,

    exit_signal: ExitSignal,

    // debug counters for logs like "fixed tick 60"
    last_fixed_tick_logged: u64,
}

impl EngineApp {
    fn new(engine: Engine) -> Self {
        let constitution = engine.cfg.frame.clone();
        let fixed_dt = constitution.fixed_dt_sec;

        let exit_signal = ExitSignal::new();
        let _ = exit_signal.install_ctrlc_handler();

        let mut telemetry = Telemetry::new();
        telemetry.configure_fps_logging(constitution.log_fps, constitution.fps_log_period_sec);

        Self {
            engine,

            window: None,
            window_id: None,

            exit_requested: false,
            shutdown_done: false,
            started: false,

            constitution,
            time: Time::new(fixed_dt),
            telemetry,

            last: Instant::now(),
            accumulator: 0.0,

            exit_signal,

            last_fixed_tick_logged: 0,
        }
    }

    fn start_if_needed(&mut self) {
        if self.started {
            return;
        }
        let Some(window) = self.window.as_ref() else { return; };

        self.engine.log.info("boot");

        // ctx живёт только на период вызова register/start
        let mut ctx = FrameContext {
            window,
            time: &mut self.time,
            telemetry: &mut self.telemetry,
            exit_requested: &mut self.exit_requested,
        };

        self.engine.schedule.on_register(&mut ctx);
        self.engine.schedule.on_start(&mut ctx);

        self.started = true;
        self.last = Instant::now();
        self.accumulator = 0.0;

        self.engine.log.info("first frame");
    }

    fn shutdown_once(&mut self, el: &ActiveEventLoop) {
        if self.shutdown_done {
            return;
        }
        self.shutdown_done = true;

        if let Some(window) = self.window.as_ref() {
            let mut ctx = FrameContext {
                window,
                time: &mut self.time,
                telemetry: &mut self.telemetry,
                exit_requested: &mut self.exit_requested,
            };
            self.engine.schedule.on_shutdown(&mut ctx);
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
                eprintln!("failed to create window: {e}");
                el.exit();
                return;
            }
        };

        self.window_id = Some(window.id());
        self.window = Some(window);

        self.start_if_needed();
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

        // Ctrl+C → мягкий выход через нашу конституцию shutdown.
        if self.exit_signal.is_exit_requested() {
            self.exit_requested = true;
        }

        if self.exit_requested {
            self.shutdown_once(el);
            return;
        }

        let now = Instant::now();
        let raw_dt = now.duration_since(self.last);
        self.last = now;

        // clamp dt
        let dt_sec = raw_dt.as_secs_f32().min(self.constitution.max_dt_sec);

        // Эти поля можно обновлять до ctx (пока time не заняли borrow'ом)
        self.time.dt_sec = dt_sec;
        self.time.t_sec += raw_dt.as_secs_f64();
        self.time.frame_index += 1;

        self.accumulator += dt_sec;

        let Some(window) = self.window.as_ref() else { return; };

        // Теперь создаём ctx и дальше трогаем time/telemetry/exit только через ctx
        let mut ctx = FrameContext {
            window,
            time: &mut self.time,
            telemetry: &mut self.telemetry,
            exit_requested: &mut self.exit_requested,
        };

        self.engine.schedule.run_phase(FramePhase::BeginFrame, &mut ctx);
        self.engine.schedule.run_phase(FramePhase::Input, &mut ctx);

        // FixedUpdate with cap (anti spiral-of-death)
        let mut steps: u32 = 0;
        while self.accumulator >= self.constitution.fixed_dt_sec {
            if steps >= self.constitution.max_fixed_steps_per_frame {
                // не пытаемся “догонять” бесконечно
                self.accumulator = 0.0;
                break;
            }

            ctx.time.fixed_tick_index += 1;
            self.engine.schedule.run_phase(FramePhase::FixedUpdate, &mut ctx);

            self.accumulator -= self.constitution.fixed_dt_sec;
            steps += 1;

            // debug tick log каждые 60
            let tick = ctx.time.fixed_tick_index;
            if tick / 60 != self.last_fixed_tick_logged / 60 && (tick % 60 == 0) {
                self.last_fixed_tick_logged = tick;
                self.engine.log.debug(format!("fixed tick {}", tick));
            }
        }

        ctx.time.fixed_alpha =
            (self.accumulator / self.constitution.fixed_dt_sec).clamp(0.0, 1.0);

        self.engine.schedule.run_phase(FramePhase::Update, &mut ctx);
        self.engine.schedule.run_phase(FramePhase::LateUpdate, &mut ctx);
        self.engine.schedule.run_phase(FramePhase::Render, &mut ctx);
        self.engine.schedule.run_phase(FramePhase::Present, &mut ctx);
        self.engine.schedule.run_phase(FramePhase::EndFrame, &mut ctx);

        // Telemetry tick (fps log и т.п.) — тоже через ctx.telemetry
        ctx.telemetry
            .frame_tick(raw_dt, ctx.time.fixed_alpha, ctx.time.fixed_tick_index);

        // Сохраняем флаг выхода, пока ctx ещё жив (он владеет &mut exit_requested)
        let exit_now = *ctx.exit_requested;

        // Освобождаем borrows time/telemetry/exit_requested
        drop(ctx);

        if exit_now {
            self.shutdown_once(el);
            return;
        }

        window.request_redraw();
    }
}