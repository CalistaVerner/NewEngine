#![cfg(feature = "winit")]

pub struct EguiWinitInput {
    state: egui_winit::State,
    pending: egui::RawInput,
}

impl EguiWinitInput {
    #[inline]
    pub fn new(window: &winit::window::Window, ctx: &egui::Context) -> Self {
        let state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        Self {
            state,
            pending: egui::RawInput::default(),
        }
    }

    #[inline]
    pub fn on_window_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) {
        let _ = self.state.on_window_event(window, event);
    }

    #[inline]
    pub fn begin_frame(&mut self, window: &winit::window::Window) {
        self.pending = self.state.take_egui_input(window);
    }

    #[inline]
    pub fn take_egui_input(&mut self) -> egui::RawInput {
        std::mem::take(&mut self.pending)
    }

    #[inline]
    pub fn end_frame(
        &mut self,
        window: &winit::window::Window,
        output: egui::FullOutput,
    ) -> egui::FullOutput {
        self.state
            .handle_platform_output(window, output.platform_output.clone());
        output
    }
}