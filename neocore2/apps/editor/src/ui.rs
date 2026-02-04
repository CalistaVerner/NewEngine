use newengine_platform_winit::{egui, UiBuildFn};
use newengine_ui::markup::{UiMarkupDoc, UiState};
use serde::Deserialize;
use std::any::Any;
use std::sync::{Arc, Mutex};

#[derive(Debug, Deserialize)]
struct CommandExecResponse {
    ok: bool,
    output: String,
    error: String,
}

#[derive(Debug, Deserialize)]
struct CommandCompleteResponse {
    items: Vec<String>,
}

#[derive(Debug, Default)]
struct ConsoleUi {
    open: bool,
    input: String,

    lines: Vec<String>,
    stick_to_bottom: bool,

    history: Vec<String>,
    hist_cursor: usize,

    completion: Vec<String>,
    completion_open: bool,
}

impl ConsoleUi {
    fn toggle_hotkey(&mut self, ctx: &egui::Context) {
        let pressed = ctx.input(|i| i.key_pressed(egui::Key::Backtick));
        if pressed {
            self.open = !self.open;
            self.completion_open = false;
        }
    }

    fn ui(&mut self, ctx: &egui::Context) {
        self.toggle_hotkey(ctx);

        if !self.open {
            return;
        }

        let screen_h = ctx.screen_rect().height();
        let console_h = (screen_h * 0.35).clamp(240.0, 520.0);

        egui::TopBottomPanel::bottom("ne_engine_console")
            .exact_height(console_h)
            .resizable(false)
            .frame(        egui::Frame::none()
                               .fill(egui::Color32::from_black_alpha(220))
                               .inner_margin(egui::Margin::symmetric(12.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("CONSOLE").strong().monospace());
                    ui.separator();

                    if ui.button("Help").clicked() {
                        self.exec_line("help");
                    }
                    if ui.button("Services").clicked() {
                        self.exec_line("services");
                    }
                    if ui.button("Refresh").clicked() {
                        let _ = newengine_core::call_service_v1("engine.command", "command.refresh", &[]);
                        self.push_line("[refreshed]".to_string());
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            self.open = false;
                            self.completion_open = false;
                        }
                        if ui.button("Clear").clicked() {
                            self.lines.clear();
                        }
                        ui.checkbox(&mut self.stick_to_bottom, "Stick");
                    });
                });

                ui.add_space(6.0);

                let available = ui.available_height();
                let log_h = available - 42.0;

                egui::ScrollArea::vertical()
                    .max_height(log_h)
                    .stick_to_bottom(self.stick_to_bottom)
                    .show(ui, |ui| {
                        for l in &self.lines {
                            ui.label(egui::RichText::new(l).monospace());
                        }
                    });

                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("> ").monospace().strong());

                    let input_id = ui.make_persistent_id("ne_console_input");
                    let te = egui::TextEdit::singleline(&mut self.input)
                        .id(input_id)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                        .hint_text("help | services | describe <id> | call <id> <method> <payload> | quit");

                    let resp = ui.add(te);

                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let tab = ui.input(|i| i.key_pressed(egui::Key::Tab));
                    let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
                    let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
                    let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));

                    if esc {
                        self.completion_open = false;
                    }

                    if up {
                        self.hist_up();
                    } else if down {
                        self.hist_down();
                    }

                    if tab {
                        self.refresh_completion();
                        if self.completion.len() == 1 {
                            self.input = self.completion[0].clone();
                            self.completion_open = false;
                        } else if !self.completion.is_empty() {
                            self.completion_open = true;
                        }
                        resp.request_focus();
                    }

                    if resp.lost_focus() && enter {
                        let line = self.input.trim().to_string();
                        self.input.clear();
                        self.completion_open = false;

                        if !line.is_empty() {
                            self.exec_line(&line);
                        }
                        resp.request_focus();
                    }
                });

                if self.completion_open && !self.completion.is_empty() {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        for it in &self.completion {
                            if ui
                                .button(egui::RichText::new(it).monospace())
                                .clicked()
                            {
                                self.input = it.clone();
                                self.completion_open = false;
                            }
                        }
                    });
                }
            });
    }

    fn push_line(&mut self, s: String) {
        self.lines.push(s);
        if self.lines.len() > 4000 {
            self.lines.drain(0..512);
        }
    }

    fn hist_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        self.hist_cursor = (self.hist_cursor + 1).min(self.history.len());
        let idx = self.history.len().saturating_sub(self.hist_cursor);
        self.input = self.history.get(idx).cloned().unwrap_or_default();
    }

    fn hist_down(&mut self) {
        if self.history.is_empty() {
            return;
        }
        self.hist_cursor = self.hist_cursor.saturating_sub(1);
        let idx = self.history.len().saturating_sub(self.hist_cursor);
        self.input = self.history.get(idx).cloned().unwrap_or_default();
    }

    fn refresh_completion(&mut self) {
        let prefix = self.input.trim().as_bytes().to_vec();
        self.completion.clear();

        match newengine_core::call_service_v1("engine.command", "command.complete", &prefix) {
            Ok(bytes) => {
                if let Ok(r) = serde_json::from_slice::<CommandCompleteResponse>(&bytes) {
                    self.completion = r.items;
                }
            }
            Err(_) => {}
        }
    }

    fn exec_line(&mut self, line: &str) {
        self.push_line(format!("> {line}"));
        self.history.push(line.to_string());
        if self.history.len() > 256 {
            self.history.drain(0..32);
        }
        self.hist_cursor = 0;

        match newengine_core::call_service_v1("engine.command", "command.exec", line.as_bytes()) {
            Ok(bytes) => match serde_json::from_slice::<CommandExecResponse>(&bytes) {
                Ok(r) => {
                    if r.ok {
                        let out = r.output.trim_end();
                        if !out.is_empty() {
                            for l in out.lines() {
                                self.push_line(l.to_string());
                            }
                        }
                    } else {
                        self.push_line(format!("ERR: {}", r.error));
                    }
                }
                Err(e) => {
                    self.push_line(format!("ERR: bad response json: {e}"));
                    self.push_line(String::from_utf8_lossy(&bytes).to_string());
                }
            },
            Err(e) => self.push_line(format!("ERR: {e}")),
        }
    }
}

pub struct EditorUiBuild {
    shared_doc: Arc<Mutex<Option<UiMarkupDoc>>>,
    state: UiState,
    console: ConsoleUi,
}

impl EditorUiBuild {
    #[inline]
    pub fn new(shared_doc: Arc<Mutex<Option<UiMarkupDoc>>>) -> Self {
        let mut state = UiState::default();
        state.set_var("app.name", "NewEngine Editor");
        Self {
            shared_doc,
            state,
            console: ConsoleUi {
                open: true,
                stick_to_bottom: true,
                ..Default::default()
            },
        }
    }
}

impl UiBuildFn for EditorUiBuild {
    fn build(&mut self, ctx_any: &mut dyn Any) {
        let Some(ctx) = ctx_any.downcast_mut::<egui::Context>() else {
            return;
        };

        // Markup UI (optional)
        let maybe_doc = { self.shared_doc.lock().ok().and_then(|g| g.as_ref().cloned()) };
        if let Some(doc) = maybe_doc {
            doc.render(ctx, &mut self.state);
        }

        // Console overlay (engine-level service)
        self.console.ui(ctx);

        if self.state.take_clicked("quit") {
            let _ = newengine_core::call_service_v1("engine.command", "command.exec", b"quit");
        }
    }
}