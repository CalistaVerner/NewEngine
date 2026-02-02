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
///
/// Usage pattern:
/// - Feed input events (winit) into `input`.
/// - Begin frame, build UI, end frame.
/// - Translate shapes into UiDrawList.
pub struct EguiUi {
    ctx: egui::Context,
    input: winit_input::EguiWinitInput,
    draw_list: UiDrawList,
}

impl EguiUi {
    #[inline]
    pub fn new() -> Self {
        Self {
            ctx: egui::Context::default(),
            input: winit_input::EguiWinitInput::new(),
            draw_list: UiDrawList::new(),
        }
    }

    #[inline]
    pub fn ctx(&self) -> &egui::Context {
        &self.ctx
    }

    #[inline]
    pub fn input_mut(&mut self) -> &mut winit_input::EguiWinitInput {
        &mut self.input
    }

    /// Run one UI frame.
    ///
    /// `build` should create all UI widgets for this frame.
    pub fn run_frame<F>(&mut self, build: F) -> EguiFrameOutput
    where
        F: FnOnce(&egui::Context),
    {
        let raw_input = self.input.take_egui_input();

        self.ctx.begin_frame(raw_input);
        build(&self.ctx);
        let full_output = self.ctx.end_frame();

        self.draw_list.clear();
        translate::egui_output_to_draw_list(&self.ctx, full_output, &mut self.draw_list);

        EguiFrameOutput {
            draw_list: self.draw_list.clone(),
        }
    }
}