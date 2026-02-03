use newengine_platform_winit::{egui, UiBuildFn};
use std::any::Any;

#[derive(Default)]
pub struct EditorUiBuild {
    demo_text: String,
    counter: u64,
}

impl UiBuildFn for EditorUiBuild {
    fn build(&mut self, ctx_any: &mut dyn Any) {
        let Some(ctx) = ctx_any.downcast_mut::<egui::Context>() else {
            return;
        };

        egui::TopBottomPanel::top("ne_top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("NewEngine Editor");
                ui.separator();
                ui.label("UI backend: egui");
            });
        });

        egui::Window::new("Stats")
            .resizable(true)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.label(format!("Counter: {}", self.counter));

                ui.horizontal(|ui| {
                    if ui.button("+1").clicked() {
                        self.counter = self.counter.saturating_add(1);
                    }
                    if ui.button("Reset").clicked() {
                        self.counter = 0;
                    }
                });

                ui.separator();
                ui.label("Command");
                ui.text_edit_singleline(&mut self.demo_text);

                if ui.button("Execute").clicked() {
                    self.demo_text.clear();
                }
            });
    }
}
