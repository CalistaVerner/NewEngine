use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use winit::event::WindowEvent;

/// External events injected into engine from winit.
///
/// Modules can downcast `dyn Any` into this type.
#[derive(Debug, Clone)]
pub enum WinitExternalEvent {
    /// Fired once when the native window is created and handles are available.
    WindowReady {
        window: RawWindowHandle,
        display: RawDisplayHandle,
        width: u32,
        height: u32,
    },

    /// Raw winit event (useful for other modules).
    WindowEvent(WindowEvent),

    /// Convenience events for modules that don't want to parse WindowEvent.
    WindowResized { width: u32, height: u32 },
    WindowFocused(bool),
}