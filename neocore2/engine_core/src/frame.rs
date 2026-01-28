use crate::{telemetry::Telemetry, time::Time};
use winit::window::Window;

/// Жёсткая "конституция" кадра.
/// Это ваш фундамент масштабирования: любые подсистемы обязаны вписываться сюда.
#[derive(Debug, Clone)]
pub struct FrameConstitution {
    /// Fixed timestep (сек).
    pub fixed_dt_sec: f32,

    /// Лимит fixed-steps за кадр (anti spiral-of-death).
    pub max_fixed_steps_per_frame: u32,

    /// Ограничение dt (сек). Например 0.25 для защиты от пауз/свёртываний.
    pub max_dt_sec: f32,

    /// Логировать FPS раз в N секунд.
    pub log_fps: bool,
    pub fps_log_period_sec: f32,
}

impl Default for FrameConstitution {
    fn default() -> Self {
        Self {
            fixed_dt_sec: 1.0 / 60.0,
            max_fixed_steps_per_frame: 8,
            max_dt_sec: 0.25,
            log_fps: true,
            fps_log_period_sec: 1.0,
        }
    }
}

/// Контекст кадра — то, через что должны общаться системы/модули.
/// Это основной "контракт" между ядром и подсистемами.
pub struct FrameContext<'a> {
    pub window: &'a Window,
    pub time: &'a mut Time,
    pub telemetry: &'a mut Telemetry,

    /// Мягкая заявка на выход.
    pub exit_requested: &'a mut bool,
}