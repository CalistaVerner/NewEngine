#[derive(Debug, Clone)]
pub struct Frame {
    pub frame_index: u64,
    pub dt: f32,
    pub fixed_alpha: f32,
}