use crate::events::WinitExternalEvent;
use newengine_core::{Engine, EngineError, EngineResult};

use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

struct App<E: Send + 'static> {
    engine: Engine<E>,
    window: Option<Window>,
}

impl<E: Send + 'static> App<E> {
    #[inline]
    fn new(engine: Engine<E>) -> Self {
        Self { engine, window: None }
    }

    #[inline]
    fn request_redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    #[inline]
    fn exit(event_loop: &ActiveEventLoop) {
        event_loop.exit();
    }
}

impl<E: Send + 'static> ApplicationHandler for App<E> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(WindowAttributes::default())
            .expect("window create failed");
        self.window = Some(window);

        // Kick first frame.
        self.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // Detect exit intent before moving the event.
        let close_requested = matches!(event, WindowEvent::CloseRequested);

        let esc_pressed = match &event {
            WindowEvent::KeyboardInput {
                event:
                KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    ..
                },
                ..
            } => true,
            _ => false,
        };

        // Forward event to engine/modules.
        let _ = self
            .engine
            .dispatch_external_event(&WinitExternalEvent::WindowEvent(event));

        if close_requested || esc_pressed {
            Self::exit(event_loop);
            return;
        }

        // Ensure we keep rendering after input/resize/etc.
        self.request_redraw();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Drive the engine once per loop iteration.
        match self.engine.step() {
            Ok(_) => {
                // Keep pumping frames (simple continuous loop).
                self.request_redraw();
            }
            Err(EngineError::ExitRequested) => {
                Self::exit(event_loop);
            }
            Err(_) => {
                // Up to you: log error here if desired.
                Self::exit(event_loop);
            }
        }
    }
}

/// Run winit-based application.
///
/// The platform crate owns the loop and injects external events into engine.
pub fn run_winit_app<E: Send + 'static>(engine: Engine<E>) -> EngineResult<()> {
    let event_loop =
        EventLoop::new().map_err(|e| newengine_core::EngineError::Other(e.to_string()))?;
    let mut app = App::new(engine);

    event_loop
        .run_app(&mut app)
        .map_err(|e| newengine_core::EngineError::Other(e.to_string()))
}