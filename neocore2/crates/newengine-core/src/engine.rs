use crate::error::{EngineError, EngineResult};
use crate::frame::Frame;
use crate::module::{Bus, Module, ModuleCtx, Resources, Services};
use crate::sched::Scheduler;

use std::time::{Duration, Instant};

pub struct Engine<E: Send + 'static> {
    fixed_dt: f32,
    services: Box<dyn Services>,
    modules: Vec<Box<dyn Module<E>>>,

    resources: Resources,
    bus: Bus<E>,
    scheduler: Scheduler,

    exit_requested: bool,

    frame_index: u64,
    last: Instant,
    acc: f32,
}

impl<E: Send + 'static> Engine<E> {
    #[inline]
    pub fn request_exit(&mut self) {
        self.exit_requested = true;
    }
}

impl<E: Send + 'static> Engine<E> {
    pub fn new(fixed_dt_ms: u32, services: Box<dyn Services>, bus: Bus<E>) -> EngineResult<Self> {
        let fixed_dt = (fixed_dt_ms as f32 / 1000.0).max(0.001);
        Ok(Self {
            fixed_dt,
            services,
            modules: Vec::new(),
            resources: Resources::default(),
            bus,
            scheduler: Scheduler::new(),
            exit_requested: false,
            frame_index: 0,
            last: Instant::now(),
            acc: 0.0,
        })
    }

    pub fn register_module(&mut self, mut module: Box<dyn Module<E>>) -> EngineResult<()> {
        let mut ctx = ModuleCtx::new(
            self.services.as_ref(),
            &mut self.resources,
            &self.bus,
            &mut self.scheduler,
            &mut self.exit_requested,
        );
        module.init(&mut ctx)?;
        self.modules.push(module);
        Ok(())
    }

    pub fn start(&mut self) -> EngineResult<()> {
        for m in &mut self.modules {
            let mut ctx = ModuleCtx::new(
                self.services.as_ref(),
                &mut self.resources,
                &self.bus,
                &mut self.scheduler,
                &mut self.exit_requested,
            );
            m.start(&mut ctx)?;
        }
        Ok(())
    }

    /// Advance one frame.
    ///
    /// Platform code calls this from its event loop.
    pub fn step(&mut self) -> EngineResult<Frame> {
        if self.exit_requested {
            return Err(EngineError::ExitRequested);
        }

        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32();
        self.last = now;

        // Prevent spiral-of-death on long stalls.
        self.acc = (self.acc + dt).min(self.fixed_dt * 8.0);

        let mut fixed_steps = 0u32;
        while self.acc >= self.fixed_dt {
            self.acc -= self.fixed_dt;
            fixed_steps += 1;

            let frame = Frame {
                frame_index: self.frame_index,
                dt,
                fixed_dt: self.fixed_dt,
                fixed_alpha: (self.acc / self.fixed_dt).clamp(0.0, 0.999_999),
                fixed_steps,
            };

            for m in &mut self.modules {
                let mut ctx = ModuleCtx::new(
                    self.services.as_ref(),
                    &mut self.resources,
                    &self.bus,
                    &mut self.scheduler,
                    &mut self.exit_requested,
                );
                m.fixed_update(&mut ctx, &frame)?;
            }
        }

        let frame = Frame {
            frame_index: self.frame_index,
            dt,
            fixed_dt: self.fixed_dt,
            fixed_alpha: (self.acc / self.fixed_dt).clamp(0.0, 0.999_999),
            fixed_steps,
        };

        for m in &mut self.modules {
            let mut ctx = ModuleCtx::new(
                self.services.as_ref(),
                &mut self.resources,
                &self.bus,
                &mut self.scheduler,
                &mut self.exit_requested,
            );
            m.update(&mut ctx, &frame)?;
        }

        for m in &mut self.modules {
            let mut ctx = ModuleCtx::new(
                self.services.as_ref(),
                &mut self.resources,
                &self.bus,
                &mut self.scheduler,
                &mut self.exit_requested,
            );
            m.render(&mut ctx, &frame)?;
        }

        self.scheduler.tick(Duration::from_secs_f32(dt));
        self.frame_index = self.frame_index.wrapping_add(1);

        Ok(frame)
    }

    /// Inject an external event into the engine.
    ///
    /// The engine treats it as opaque; modules downcast as needed.
    pub fn dispatch_external_event(&mut self, event: &dyn std::any::Any) -> EngineResult<()> {
        for m in &mut self.modules {
            let mut ctx = ModuleCtx::new(
                self.services.as_ref(),
                &mut self.resources,
                &self.bus,
                &mut self.scheduler,
                &mut self.exit_requested,
            );
            m.on_external_event(&mut ctx, event)?;
        }
        Ok(())
    }

    pub fn shutdown(&mut self) -> EngineResult<()> {
        for m in self.modules.iter_mut().rev() {
            let mut ctx = ModuleCtx::new(
                self.services.as_ref(),
                &mut self.resources,
                &self.bus,
                &mut self.scheduler,
                &mut self.exit_requested,
            );
            let _ = m.shutdown(&mut ctx);
        }
        Ok(())
    }

    #[inline]
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }
}