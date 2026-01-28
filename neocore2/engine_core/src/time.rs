#[derive(Debug, Clone)]
pub struct Time {
    /// delta time (сек) текущего кадра (clamped)
    pub dt_sec: f32,

    /// абсолютное время (сек) с запуска
    pub t_sec: f64,

    /// индекс кадра
    pub frame_index: u64,

    /// индекс fixed тика
    pub fixed_tick_index: u64,

    /// alpha интерполяции между fixed шагами
    pub fixed_alpha: f32,

    /// fixed dt (сек)
    pub fixed_dt_sec: f32,
}

impl Time {
    pub fn new(fixed_dt_sec: f32) -> Self {
        Self {
            dt_sec: 0.0,
            t_sec: 0.0,
            frame_index: 0,
            fixed_tick_index: 0,
            fixed_alpha: 0.0,
            fixed_dt_sec,
        }
    }
}