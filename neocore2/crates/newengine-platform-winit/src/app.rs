use crate::events::WinitExternalEvent;
use newengine_core::{Engine, EngineError, EngineResult, WindowHostEvent};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
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

    #[inline]
    fn window_size(&self) -> Option<(u32, u32)> {
        self.window.as_ref().map(|w| {
            let PhysicalSize { width, height } = w.inner_size();
            (width, height)
        })
    }

    fn emit_window_ready(&mut self) {
        let Some(w) = &self.window else { return; };
        let Some((width, height)) = self.window_size() else { return; };

        let window = match w.window_handle() {
            Ok(h) => h.as_raw(),
            Err(_) => return,
        };

        let display = match w.display_handle() {
            Ok(h) => h.as_raw(),
            Err(_) => return,
        };

        let _ = self.engine.dispatch_external_event(&WinitExternalEvent::WindowReady {
            window,
            display,
            width,
            height,
        });

        let _ = self.engine.dispatch_external_event(&WindowHostEvent::Ready {
            window,
            display,
            width,
            height,
        });
    }

    #[inline]
    fn emit_resized(&mut self, width: u32, height: u32) {
        let _ = self.engine.dispatch_external_event(&WinitExternalEvent::WindowResized { width, height });
        let _ = self.engine.dispatch_external_event(&WindowHostEvent::Resized { width, height });
    }

    #[inline]
    fn emit_focused(&mut self, focused: bool) {
        let _ = self.engine.dispatch_external_event(&WinitExternalEvent::WindowFocused(focused));
        let _ = self.engine.dispatch_external_event(&WindowHostEvent::Focused(focused));
    }
}

impl<E: Send + 'static> ApplicationHandler for App<E> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(WindowAttributes::default())
            .expect("window create failed");

        self.window = Some(window);

        self.emit_window_ready();
        self.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let close_requested = matches!(event, WindowEvent::CloseRequested);

        let esc_pressed = matches!(
            &event,
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    ..
                },
                ..
            }
        );

        let _ = self.engine.dispatch_external_event(&WinitExternalEvent::WindowEvent(event.clone()));

        match &event {
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                self.emit_resized(*width, *height);
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some((w, h)) = self.window_size() {
                    self.emit_resized(w, h);
                }
            }
            WindowEvent::Focused(f) => {
                self.emit_focused(*f);
            }
            _ => {}
        }

        if close_requested || esc_pressed {
            Self::exit(event_loop);
            return;
        }

        self.request_redraw();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.engine.step() {
            Ok(_) => self.request_redraw(),
            Err(EngineError::ExitRequested) => Self::exit(event_loop),
            Err(_) => Self::exit(event_loop),
        }
    }
}

pub fn run_winit_app<E: Send + 'static>(engine: Engine<E>) -> EngineResult<()> {
    let event_loop =
        EventLoop::new().map_err(|e| newengine_core::EngineError::Other(e.to_string()))?;

    let mut app = App::new(engine);

    event_loop
        .run_app(&mut app)
        .map_err(|e| newengine_core::EngineError::Other(e.to_string()))
}