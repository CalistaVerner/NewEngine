/// Frame timing snapshot.
///
/// fixed_alpha is interpolation factor in [0..1) for render smoothing if needed.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub frame_index: u64,
    pub dt: f32,
    pub fixed_dt: f32,
    pub fixed_alpha: f32,
    pub fixed_steps: u32,
}