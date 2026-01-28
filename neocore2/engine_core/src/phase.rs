#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramePhase {
    BeginFrame,
    Input,
    FixedUpdate,
    Update,
    LateUpdate,
    Render,
    Present,
    EndFrame,
}
