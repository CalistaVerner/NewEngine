use std::time::{Duration, Instant};

use crate::log::Logger;

/// Минимальная телеметрия/профайлер.
/// Это "скелет": позже можно расширить до Tracy/Chrome trace и т.п.
pub struct Telemetry {
    log: Logger,

    pub fps: f32,
    pub dt_ms: f32,
    pub fixed_alpha: f32,
    pub fixed_tick: u64,

    // fps accumulation
    fps_last: Instant,
    fps_frames: u32,
    fps_period_sec: f32,
    fps_enabled: bool,

    // last recorded scopes (cheap, for debug/inspection later)
    last_scope_name: &'static str,
    last_scope_ms: f32,
}

impl Telemetry {
    pub fn new() -> Self {
        Self {
            log: Logger::new("Telemetry"),
            fps: 0.0,
            dt_ms: 0.0,
            fixed_alpha: 0.0,
            fixed_tick: 0,
            fps_last: Instant::now(),
            fps_frames: 0,
            fps_period_sec: 1.0,
            fps_enabled: true,
            last_scope_name: "none",
            last_scope_ms: 0.0,
        }
    }

    pub fn configure_fps_logging(&mut self, enabled: bool, period_sec: f32) {
        self.fps_enabled = enabled;
        self.fps_period_sec = period_sec.max(0.25);
    }

    pub fn frame_tick(&mut self, dt: Duration, fixed_alpha: f32, fixed_tick: u64) {
        self.dt_ms = dt.as_secs_f32() * 1000.0;
        self.fixed_alpha = fixed_alpha;
        self.fixed_tick = fixed_tick;

        if !self.fps_enabled {
            return;
        }

        self.fps_frames += 1;
        let elapsed = self.fps_last.elapsed().as_secs_f32();

        if elapsed >= self.fps_period_sec {
            let secs = elapsed.max(0.0001);
            self.fps = (self.fps_frames as f32) / secs;

            self.log.info(format!(
                "fps={:.1} dt_ms={:.2} fixed_alpha={:.2} fixed_tick={}",
                self.fps, self.dt_ms, self.fixed_alpha, self.fixed_tick
            ));

            self.fps_frames = 0;
            self.fps_last = Instant::now();
        }
    }

    /// Записать измерение секции (фазы/системы).
    /// Сейчас хранит только последнее значение (дёшево).
    /// Дальше сюда легко добавить агрегацию/скользящие средние/трейс.
    #[inline]
    pub fn record_scope(&mut self, name: &'static str, dur: Duration) {
        self.last_scope_name = name;
        self.last_scope_ms = dur.as_secs_f32() * 1000.0;

        // Чтобы не шуметь — не логируем каждую фазу.
        // Если захочешь: включим порог (например > 2ms) и будем warn/debug.
        // if self.last_scope_ms > 2.0 { self.log.debug(format!("scope {name} {:.3}ms", self.last_scope_ms)); }
    }

    #[allow(dead_code)]
    pub fn last_scope(&self) -> (&'static str, f32) {
        (self.last_scope_name, self.last_scope_ms)
    }
}