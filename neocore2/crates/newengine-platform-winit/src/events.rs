use winit::event::WindowEvent;

/// External events injected into engine from winit.
///
/// Modules can downcast `dyn Any` into this type.
#[derive(Debug)]
pub enum WinitExternalEvent {
    WindowEvent(WindowEvent),
}