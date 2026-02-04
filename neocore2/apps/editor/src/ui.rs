use newengine_platform_winit::{egui, UiBuildFn};
use newengine_ui::markup::{UiMarkupDoc, UiState};
use serde::Deserialize;
use std::any::Any;
use std::sync::{Arc, Mutex};

#[derive(Debug, Deserialize)]
struct CommandExecResponse {
    ok: bool,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SuggestItem {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    display: String,
    #[serde(default)]
    insert: String,
    #[serde(default)]
    help: String,
    #[serde(default)]
    usage: String,
}

#[derive(Debug, Deserialize)]
struct SuggestResponse {
    #[serde(default)]
    signature: String,
    #[serde(default)]
    items: Vec<SuggestItem>,
}

#[derive(Debug)]
struct ConsoleUi {
    open: bool,
    input: String,

    lines: Vec<String>,
    stick_to_bottom: bool,

    filter: String,

    history: Vec<String>,
    hist_cursor: usize,

    suggest: SuggestResponse,
    suggest_open: bool,
    suggest_selected: usize,
    last_suggest_input: String,
}

impl Default for ConsoleUi {
    fn default() -> Self {
        Self {
            open: false,
            input: String::new(),

            lines: Vec::new(),
            stick_to_bottom: true,

            filter: String::new(),

            history: Vec::new(),
            hist_cursor: 0,

            suggest: SuggestResponse {
                signature: String::new(),
                items: Vec::new(),
            },
            suggest_open: false,
            suggest_selected: 0,
            last_suggest_input: String::new(),
        }
    }
}

impl ConsoleUi {
    fn toggle_hotkey(&mut self, ctx: &egui::Context) {
        let pressed = ctx.input(|i| i.key_pressed(egui::Key::Backtick));
        if pressed {
            self.open = !self.open;
            self.suggest_open = false;
        }
    }

    fn ui(&mut self, ctx: &egui::Context) {
        self.toggle_hotkey(ctx);

        if !self.open {
            return;
        }

        let screen_h = ctx.screen_rect().height();
        let console_h = (screen_h * 0.40).clamp(260.0, 620.0);

        let bg = egui::Color32::from_rgba_premultiplied(12, 12, 14, 238);
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(60));

        egui::TopBottomPanel::bottom("ne_engine_console")
            .exact_height(console_h)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(bg)
                    .stroke(stroke)
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0)),
            )
            .show(ctx, |ui| {
                self.header_row(ui);

                ui.add_space(6.0);

                let available = ui.available_height();
                let log_h = (available * 0.60).max(160.0);

                self.log_area(ui, log_h);

                ui.add_space(6.0);

                self.input_row(ui);

                if self.suggest_open && !self.suggest.items.is_empty() {
                    ui.add_space(4.0);
                    self.suggest_panel(ui);
                }
            });
    }

    fn header_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("NE Console")
                    .strong()
                    .monospace()
                    .color(egui::Color32::from_gray(220)),
            );

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
                self.refresh_suggest();
            }

            ui.separator();

            ui.label(
                egui::RichText::new("Filter:")
                    .monospace()
                    .color(egui::Color32::from_gray(160)),
            );

            ui.add(
                egui::TextEdit::singleline(&mut self.filter)
                    .desired_width(180.0)
                    .hint_text("text")
                    .font(egui::TextStyle::Monospace),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    self.open = false;
                    self.suggest_open = false;
                }
                if ui.button("Clear").clicked() {
                    self.lines.clear();
                }
                ui.checkbox(&mut self.stick_to_bottom, "Stick");
            });
        });
    }

    fn log_area(&mut self, ui: &mut egui::Ui, log_h: f32) {
        let f = self.filter.trim().to_lowercase();

        egui::ScrollArea::vertical()
            .max_height(log_h)
            .stick_to_bottom(self.stick_to_bottom)
            .show(ui, |ui| {
                for l in &self.lines {
                    if !f.is_empty() && !l.to_lowercase().contains(&f) {
                        continue;
                    }

                    let mut rt = egui::RichText::new(l).monospace();
                    if l.starts_with("ERR:") {
                        rt = rt.color(egui::Color32::from_rgb(255, 96, 96));
                    } else if l.starts_with("> ") {
                        rt = rt.color(egui::Color32::from_rgb(128, 220, 140));
                    } else if l.starts_with('[') {
                        rt = rt.color(egui::Color32::from_gray(190));
                    }
                    ui.label(rt);
                }
            });
    }

    fn input_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("> ").monospace().strong());

            let input_id = ui.make_persistent_id("ne_console_input");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.input)
                    .id(input_id)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("Type a command (Tab for suggestions)")
                    .lock_focus(true),
            );

            let has_focus = resp.has_focus();

            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let tab = ui.input(|i| i.key_pressed(egui::Key::Tab));
            let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
            let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
            let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));

            if resp.changed() {
                self.refresh_suggest();
                if !self.suggest.items.is_empty() {
                    self.suggest_open = true;
                    self.suggest_selected = self
                        .suggest_selected
                        .min(self.suggest.items.len().saturating_sub(1));
                }
            }

            if esc {
                self.suggest_open = false;
            }

            if tab {
                self.refresh_suggest();
                if !self.suggest.items.is_empty() {
                    if !self.suggest_open {
                        self.suggest_open = true;
                        self.suggest_selected = 0;
                    } else {
                        self.suggest_selected =
                            (self.suggest_selected + 1) % self.suggest.items.len();
                    }
                }
                resp.request_focus();
            }

            if self.suggest_open {
                if up {
                    self.suggest_selected = self.suggest_selected.saturating_sub(1);
                    resp.request_focus();
                } else if down {
                    self.suggest_selected = (self.suggest_selected + 1)
                        .min(self.suggest.items.len().saturating_sub(1));
                    resp.request_focus();
                }
            } else {
                if up {
                    self.hist_up();
                    self.refresh_suggest();
                    resp.request_focus();
                } else if down {
                    self.hist_down();
                    self.refresh_suggest();
                    resp.request_focus();
                }
            }

            if has_focus && enter {
                if self.suggest_open && !self.suggest.items.is_empty() {
                    let idx =
                        self.suggest_selected.min(self.suggest.items.len().saturating_sub(1));
                    let ins = self.suggest.items[idx].insert.clone();
                    if !ins.is_empty() {
                        self.input = ins;
                        self.refresh_suggest();
                        self.suggest_open = true;
                        self.suggest_selected = 0;
                    }
                    resp.request_focus();
                    return;
                }

                let line = self.input.trim().to_string();
                self.input.clear();
                self.suggest_open = false;

                if !line.is_empty() {
                    self.exec_line(&line);
                }

                resp.request_focus();
            }
        });
    }

    fn suggest_panel(&mut self, ui: &mut egui::Ui) {
        let bg = egui::Color32::from_rgba_premultiplied(16, 16, 18, 245);
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(55));

        egui::Frame::none()
            .fill(bg)
            .stroke(stroke)
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .rounding(egui::Rounding::same(6.0))
            .show(ui, |ui| {
                if !self.suggest.signature.is_empty() {
                    ui.label(
                        egui::RichText::new(self.suggest.signature.clone())
                            .monospace()
                            .color(egui::Color32::from_gray(200)),
                    );
                    ui.add_space(6.0);
                }

                ui.columns(2, |cols| {
                    let (left, right) = cols.split_at_mut(1);
                    let left = &mut left[0];
                    let right = &mut right[0];

                    let list_h: f32 = 140.0;

                    let mut click_select: Option<usize> = None;
                    let mut accept_insert: Option<String> = None;
                    let mut accept_refresh = false;

                    egui::ScrollArea::vertical()
                        .max_height(list_h)
                        .show(left, |ui| {
                            for (i, it) in self.suggest.items.iter().enumerate() {
                                let selected = i == self.suggest_selected;
                                let text = if it.help.is_empty() {
                                    it.display.clone()
                                } else {
                                    format!("{}  -  {}", it.display, it.help)
                                };

                                let mut rt = egui::RichText::new(text).monospace();
                                if selected {
                                    rt = rt.strong().color(egui::Color32::from_gray(240));
                                } else {
                                    rt = rt.color(egui::Color32::from_gray(200));
                                }

                                let resp = ui.selectable_label(selected, rt);

                                if resp.clicked() {
                                    click_select = Some(i);
                                }
                                if resp.double_clicked() {
                                    accept_insert = Some(it.insert.clone());
                                    accept_refresh = true;
                                }
                            }
                        });

                    if let Some(i) = click_select {
                        self.suggest_selected = i;
                    }

                    if let Some(ins) = accept_insert {
                        self.input = ins;
                        if accept_refresh {
                            self.refresh_suggest();
                            self.suggest_open = true;
                            self.suggest_selected = 0;
                        }
                    }


                    let idx = self
                        .suggest_selected
                        .min(self.suggest.items.len().saturating_sub(1));
                    let it = &self.suggest.items[idx];

                    right.label(
                        egui::RichText::new(it.usage.clone())
                            .monospace()
                            .strong()
                            .color(egui::Color32::from_gray(230)),
                    );
                    right.add_space(4.0);

                    if !it.help.is_empty() {
                        right.label(
                            egui::RichText::new(it.help.clone())
                                .monospace()
                                .color(egui::Color32::from_gray(190)),
                        );
                    }

                    if !it.kind.is_empty() {
                        right.add_space(6.0);
                        right.label(
                            egui::RichText::new(format!("type: {}", it.kind))
                                .monospace()
                                .color(egui::Color32::from_gray(160)),
                        );
                    }
                });
            });
    }

    fn refresh_suggest(&mut self) {
        let input = self.input.clone();
        if input == self.last_suggest_input {
            return;
        }

        self.last_suggest_input = input.clone();
        self.suggest = SuggestResponse {
            signature: String::new(),
            items: Vec::new(),
        };

        match newengine_core::call_service_v1("engine.command", "command.suggest", input.as_bytes())
        {
            Ok(bytes) => {
                if let Ok(r) = serde_json::from_slice::<SuggestResponse>(&bytes) {
                    self.suggest = r;
                    self.suggest_selected = self
                        .suggest_selected
                        .min(self.suggest.items.len().saturating_sub(1));
                }
            }
            Err(_) => {}
        }
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
                        let out = r.output.unwrap_or_default();
                        let out = out.trim_end();
                        if !out.is_empty() {
                            for l in out.lines() {
                                self.push_line(l.to_string());
                            }
                        }
                    } else {
                        self.push_line(format!(
                            "ERR: {}",
                            r.error.unwrap_or_else(|| "unknown error".to_string())
                        ));
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

        let maybe_doc = { self.shared_doc.lock().ok().and_then(|g| g.as_ref().cloned()) };
        if let Some(doc) = maybe_doc {
            doc.render(ctx, &mut self.state);
        }

        self.console.ui(ctx);

        if self.state.take_clicked("quit") {
            let _ = newengine_core::call_service_v1("engine.command", "command.exec", b"quit");
        }
    }
}
