use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Clock {
    last: Instant,
}

impl Clock {
    pub fn new() -> Self {
        Self { last: Instant::now() }
    }

    pub fn tick_seconds(&mut self) -> f32 {
        let now = Instant::now();
        let dt = now.duration_since(self.last);
        self.last = now;
        dt.as_secs_f32()
    }
}

#[derive(Debug)]
pub struct FixedStep {
    pub fixed_dt: f32,
    acc: f32,
}

impl FixedStep {
    pub fn new(fixed_dt: f32) -> Self {
        Self { fixed_dt, acc: 0.0 }
    }

    pub fn push(&mut self, dt: f32) {
        self.acc += dt;
        // чтобы не улетать в “spiral of death” на лагах:
        self.acc = self.acc.min(self.fixed_dt * 8.0);
    }

    pub fn pop(&mut self) -> bool {
        if self.acc >= self.fixed_dt {
            self.acc -= self.fixed_dt;
            true
        } else {
            false
        }
    }

    pub fn alpha(&self) -> f32 {
        (self.acc / self.fixed_dt).clamp(0.0, 1.0)
    }
}

pub fn ms(ms: f32) -> Duration {
    Duration::from_secs_f32(ms / 1000.0)
}