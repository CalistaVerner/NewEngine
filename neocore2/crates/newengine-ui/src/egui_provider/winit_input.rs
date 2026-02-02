#![cfg(feature = "winit")]

/// Winit -> egui input adapter.
/// This module does not require any renderer, only window events.
pub struct EguiWinitInput {
    state: egui_winit::State,
    pending: egui::RawInput,
}

impl EguiWinitInput {
    #[inline]
    pub fn new() -> Self {
        Self {
            state: egui_winit::State::new(egui::Context::default()),
            pending: egui::RawInput::default(),
        }
    }

    /// Must be called when you have access to the actual egui context used by the UI.
    /// This ensures correct modifier state and input translations.
    #[inline]
    pub fn set_ctx(&mut self, ctx: &egui::Context) {
        self.state = egui_winit::State::new(ctx.clone());
    }

    /// Handle one winit event. Call this from your platform layer.
    #[inline]
    pub fn on_event<T>(&mut self, window: &winit::window::Window, event: &winit::event::Event<T>) {
        let _ = self.state.on_event(window, event);
    }

    /// Update time and screen info once per frame.
    #[inline]
    pub fn begin_frame(&mut self, window: &winit::window::Window) {
        self.pending = self.state.take_egui_input(window);
    }

    /// Consume prepared egui input for `egui::Context::begin_frame`.
    #[inline]
    pub fn take_egui_input(&mut self) -> egui::RawInput {
        std::mem::take(&mut self.pending)
    }

    /// Apply egui output back to the window (cursor icon, clipboard, etc.)
    #[inline]
    pub fn end_frame(&mut self, window: &winit::window::Window, ctx: &egui::Context, output: &egui::FullOutput) {
        self.state.handle_platform_output(window, ctx, &output.platform_output);
    }
}