#![cfg(feature = "egui")]

mod translate;
mod winit_input;

pub use translate::*;
pub use winit_input::*;

use crate::draw::UiDrawList;

/// UI frame output from egui.
pub struct EguiFrameOutput {
    pub draw_list: UiDrawList,
}

/// High-level egui UI driver. Does not know about renderer.
pub struct EguiUi {
    ctx: egui::Context,
    input: EguiWinitInput,
    draw_list: UiDrawList,
}

impl EguiUi {
    #[inline]
    pub fn new(window: &winit::window::Window) -> Self {
        let ctx = egui::Context::default();
        let input = EguiWinitInput::new(window, &ctx);

        Self {
            ctx,
            input,
            draw_list: UiDrawList::new(),
        }
    }

    #[inline]
    pub fn ctx(&self) -> &egui::Context {
        &self.ctx
    }

    #[inline]
    pub fn input_mut(&mut self) -> &mut EguiWinitInput {
        &mut self.input
    }

    /// Run one UI frame.
    pub fn run_frame<F>(&mut self, window: &winit::window::Window, build: F) -> EguiFrameOutput
    where
        F: FnOnce(&egui::Context),
    {
        self.input.begin_frame(window);

        let raw_input = self.input.take_egui_input();
        self.ctx.begin_frame(raw_input);

        build(&self.ctx);

        let full_output = self.ctx.end_frame();
        let full_output = self.input.end_frame(window, full_output);

        self.draw_list.clear();
        translate::egui_output_to_draw_list(&self.ctx, full_output, &mut self.draw_list);

        EguiFrameOutput {
            draw_list: self.draw_list.clone(),
        }
    }
}