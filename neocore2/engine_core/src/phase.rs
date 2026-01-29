#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Phase {
    Init,
    Start,
    Update,
    FixedUpdate,
    Render,
    Shutdown,
}