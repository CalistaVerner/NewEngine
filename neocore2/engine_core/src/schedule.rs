use crate::time::{Clock, FixedStep};

pub struct Scheduler {
    pub clock: Clock,
    pub fixed: FixedStep,
    pub frame_index: u64,
}

impl Scheduler {
    pub fn new(fixed_dt: f32) -> Self {
        Self {
            clock: Clock::new(),
            fixed: FixedStep::new(fixed_dt),
            frame_index: 0,
        }
    }

    pub fn next_dt(&mut self) -> f32 {
        self.clock.tick_seconds()
    }
}