use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

/// Host window lifecycle + size/focus events.
/// Platform crates must emit these, modules may consume them.
#[derive(Debug, Clone, Copy)]
pub enum WindowHostEvent {
    Ready {
        window: RawWindowHandle,
        display: RawDisplayHandle,
        width: u32,
        height: u32,
    },
    Resized {
        width: u32,
        height: u32,
    },
    Focused(bool),
}