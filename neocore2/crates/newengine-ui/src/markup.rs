#![forbid(unsafe_op_in_unsafe_fn)]

use std::time::{Duration, Instant};
use std::{borrow::Cow, thread};

use ahash::AHashMap;
use smallvec::SmallVec;

use roxmltree::{Document, Node};

use newengine_assets::{AssetKey, AssetState, AssetStore, TextReader};

#[derive(Debug)]
pub enum UiMarkupError {
    Enqueue(String),
    Timeout { path: String },
    Failed(String),
    BlobMissing,
    TextRead(String),
    XmlParse(String),
    Invalid(String),
}

impl std::fmt::Display for UiMarkupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UiMarkupError::Enqueue(e) => write!(f, "ui: load enqueue failed: {e}"),
            UiMarkupError::Timeout { path } => write!(f, "ui: timeout while loading '{path}'"),
            UiMarkupError::Failed(msg) => write!(f, "ui: asset failed: {msg}"),
            UiMarkupError::BlobMissing => write!(f, "ui: asset Ready but blob missing"),
            UiMarkupError::TextRead(e) => write!(f, "ui: TextReader failed: {e}"),
            UiMarkupError::XmlParse(e) => write!(f, "ui: xml parse failed: {e}"),
            UiMarkupError::Invalid(e) => write!(f, "ui: markup invalid: {e}"),
        }
    }
}

impl std::error::Error for UiMarkupError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiEventKind {
    Click,
    Change,
    Submit,
}

#[derive(Debug, Clone)]
pub struct UiEvent {
    pub kind: UiEventKind,
    pub target_id: String,
    pub value: Option<String>,
    pub actions: SmallVec<[String; 2]>,
}

/// Runtime state for UI bindings and events.
///
/// Contract principles:
/// - `clicked` stays for back-compat (existing code).
/// - `events` is the canonical "reactive" stream for XML actions.
/// - `vars` remains for $var substitution and value mirroring.
#[derive(Debug, Default)]
pub struct UiState {
    pub strings: AHashMap<String, String>,
    pub clicked: AHashMap<String, bool>,
    pub vars: AHashMap<String, String>,
    pub unknown_tags: AHashMap<String, u32>,

    events: Vec<UiEvent>,
}

impl UiState {
    #[inline]
    pub fn take_clicked(&mut self, id: &str) -> bool {
        self.clicked.remove(id).unwrap_or(false)
    }

    #[inline]
    pub fn set_var(&mut self, k: impl Into<String>, v: impl Into<String>) {
        self.vars.insert(k.into(), v.into());
    }

    /// Drain UI events produced by declarative XML `on_*` actions.
    #[inline]
    pub fn drain_events(&mut self) -> Vec<UiEvent> {
        std::mem::take(&mut self.events)
    }

    #[inline]
    fn push_event(&mut self, ev: UiEvent) {
        self.events.push(ev);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiVisuals {
    Auto,
    Dark,
    Light,
}

impl Default for UiVisuals {
    #[inline]
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiDensity {
    Default,
    Compact,
    Dense,
    Tight,
}

impl Default for UiDensity {
    #[inline]
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone)]
pub struct UiThemeDesc {
    pub visuals: UiVisuals,
    pub scale: f32,
    pub font_size: f32,
    pub density: UiDensity,
}

impl Default for UiThemeDesc {
    #[inline]
    fn default() -> Self {
        Self {
            visuals: UiVisuals::Auto,
            scale: 1.0,
            font_size: 14.0,
            density: UiDensity::Default,
        }
    }
}

/// Parsed UI document.
#[derive(Debug, Clone)]
pub struct UiMarkupDoc {
    root: UiNode,
    theme: UiThemeDesc,
}

impl UiMarkupDoc {
    /// Load UI markup via AssetStore (engine decides how to pump).
    ///
    /// `pump` should call your AssetManager::pump() (or any equivalent).
    pub fn load_from_store<P>(
        store: &AssetStore,
        mut pump: P,
        logical_path: &str,
        timeout: Duration,
    ) -> Result<Self, UiMarkupError>
    where
        P: FnMut(),
    {
        let key = AssetKey::new(logical_path, 0);

        let id = store
            .load(key)
            .map_err(|e| UiMarkupError::Enqueue(e.to_string()))?;

        let t0 = Instant::now();

        // Backoff: keep calling pump() every iteration, but avoid a pure hot spin.
        let mut spin: u32 = 0;

        loop {
            pump();

            match store.state(id) {
                AssetState::Ready => break,
                AssetState::Failed(msg) => return Err(UiMarkupError::Failed(msg.to_string())),
                AssetState::Loading | AssetState::Unloaded => {}
            }

            if t0.elapsed() >= timeout {
                return Err(UiMarkupError::Timeout {
                    path: logical_path.to_string(),
                });
            }

            spin = spin.saturating_add(1);
            if spin < 32 {
                thread::yield_now();
            } else if spin < 128 {
                thread::sleep(Duration::from_millis(1));
            } else {
                thread::sleep(Duration::from_millis(3));
            }
        }

        let blob = store.get_blob(id).ok_or(UiMarkupError::BlobMissing)?;

        let doc = TextReader::from_blob_parts(&blob.meta_json, &blob.payload)
            .map_err(|e| UiMarkupError::TextRead(e.to_string()))?;

        Self::parse(&doc.text)
    }

    pub fn parse(xml_text: &str) -> Result<Self, UiMarkupError> {
        let parsed =
            Document::parse(xml_text).map_err(|e| UiMarkupError::XmlParse(e.to_string()))?;

        let root = parse_ui_root(&parsed).map_err(UiMarkupError::Invalid)?;
        let theme = parse_theme(&parsed);

        Ok(Self { root, theme })
    }

    /// Render UI into egui (only when `feature="egui"` is enabled).
    #[cfg(feature = "egui")]
    pub fn render(&self, ctx: &egui::Context, state: &mut UiState) {
        apply_theme(ctx, &self.theme);
        self.root.render(ctx, state);
    }

    #[inline]
    pub fn theme(&self) -> &UiThemeDesc {
        &self.theme
    }
}

#[derive(Debug, Clone)]
enum UiNode {
    Ui {
        children: Vec<UiNode>,
    },
    TopBar {
        children: Vec<UiNode>,
    },
    Window {
        title: String,
        open: bool,
        children: Vec<UiNode>,
    },
    Row {
        children: Vec<UiNode>,
    },
    Column {
        children: Vec<UiNode>,
    },

    Label {
        id: Option<String>,
        text: String,
    },
    Button {
        id: String,
        text: String,
        on_click: SmallVec<[String; 2]>,
    },
    TextBox {
        id: String,
        hint: String,
        bind: String,
        multiline: bool,
        on_change: SmallVec<[String; 2]>,
        on_submit: SmallVec<[String; 2]>,
    },

    Spacer,

    Unknown {
        tag: String,
        children: Vec<UiNode>,
    },
}

impl UiNode {
    #[cfg(feature = "egui")]
    fn render(&self, ctx: &egui::Context, state: &mut UiState) {
        match self {
            UiNode::Ui { children } => {
                for c in children {
                    c.render(ctx, state);
                }
            }
            UiNode::TopBar { children } => {
                egui::TopBottomPanel::top("ui_topbar").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        for c in children {
                            render_in_ui(c, ui, state);
                        }
                    });
                });
            }
            UiNode::Window {
                title,
                open,
                children,
            } => {
                let mut is_open = *open;
                egui::Window::new(title).open(&mut is_open).show(ctx, |ui| {
                    for c in children {
                        render_in_ui(c, ui, state);
                    }
                });
            }
            _ => {}
        }
    }
}

#[cfg(feature = "egui")]
fn render_in_ui(node: &UiNode, ui: &mut egui::Ui, state: &mut UiState) {
    match node {
        UiNode::Row { children } => {
            ui.horizontal(|ui| {
                for c in children {
                    render_in_ui(c, ui, state);
                }
            });
        }
        UiNode::Column { children } => {
            ui.vertical(|ui| {
                for c in children {
                    render_in_ui(c, ui, state);
                }
            });
        }
        UiNode::Label { id, text } => {
            let base = if let Some(id) = id.as_deref() {
                state
                    .strings
                    .get(id)
                    .map(String::as_str)
                    .unwrap_or(text.as_str())
            } else {
                text.as_str()
            };
            let s = substitute_vars(base, &state.vars);
            ui.label(s.as_ref());
        }
        UiNode::Button { id, text, on_click } => {
            let s = substitute_vars(text, &state.vars);
            if ui.button(s.as_ref()).clicked() {
                // Back-compat:
                state.clicked.insert(id.clone(), true);

                // Declarative actions:
                if !on_click.is_empty() {
                    state.push_event(UiEvent {
                        kind: UiEventKind::Click,
                        target_id: id.clone(),
                        value: None,
                        actions: on_click.clone(),
                    });
                }
            }
        }
        UiNode::TextBox {
            id,
            hint,
            bind,
            multiline,
            on_change,
            on_submit,
        } => {
            let entry = state.strings.entry(bind.clone()).or_default();
            let hint = substitute_vars(hint, &state.vars);

            let (changed, submit_now, value_snapshot) = {
                let entry = state.strings.entry(bind.clone()).or_default();

                let resp = if *multiline {
                    ui.add(
                        egui::TextEdit::multiline(entry)
                            .hint_text(hint.as_ref())
                            .desired_width(f32::INFINITY),
                    )
                } else {
                    ui.add(
                        egui::TextEdit::singleline(entry)
                            .hint_text(hint.as_ref())
                            .desired_width(f32::INFINITY),
                    )
                };

                let changed = resp.changed();
                let submit_now = resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                (changed, submit_now, entry.clone())
            };

            if changed {
                // Existing behavior:
                state.vars.insert(id.clone(), value_snapshot.clone());

                // Declarative actions:
                if !on_change.is_empty() {
                    state.push_event(UiEvent {
                        kind: UiEventKind::Change,
                        target_id: id.clone(),
                        value: Some(value_snapshot.clone()),
                        actions: on_change.clone(),
                    });
                }
            }

            if submit_now && !on_submit.is_empty() {
                state.push_event(UiEvent {
                    kind: UiEventKind::Submit,
                    target_id: id.clone(),
                    value: Some(value_snapshot),
                    actions: on_submit.clone(),
                });
            }
        }
        UiNode::Spacer => ui.add_space(8.0),
        UiNode::TopBar { children } => {
            ui.horizontal(|ui| {
                for c in children {
                    render_in_ui(c, ui, state);
                }
            });
        }
        UiNode::Window { .. } => {}
        UiNode::Ui { children } => {
            for c in children {
                render_in_ui(c, ui, state);
            }
        }
        UiNode::Unknown { tag, children } => {
            *state.unknown_tags.entry(tag.clone()).or_insert(0) += 1;
            for c in children {
                render_in_ui(c, ui, state);
            }
        }
    }
}

fn substitute_vars<'a>(src: &'a str, vars: &AHashMap<String, String>) -> Cow<'a, str> {
    if !src.contains('$') {
        return Cow::Borrowed(src);
    }

    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let b = src.as_bytes();

    while i < b.len() {
        if b[i] == b'$' {
            i += 1;
            let start = i;
            while i < b.len() && is_var_char(b[i]) {
                i += 1;
            }
            let key = &src[start..i];
            if let Some(v) = vars.get(key) {
                out.push_str(v);
            } else {
                out.push('$');
                out.push_str(key);
            }
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }

    Cow::Owned(out)
}

#[inline]
fn is_var_char(c: u8) -> bool {
    matches!(c, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'.' | b'-')
}

fn parse_ui_root(doc: &Document) -> Result<UiNode, String> {
    let root = doc.root_element();

    let tag = root.tag_name().name();
    if tag != "ui" {
        return Err(format!("root tag must be <ui>, got <{tag}>"));
    }

    Ok(UiNode::Ui {
        children: parse_children(root)?,
    })
}

fn parse_theme(doc: &Document) -> UiThemeDesc {
    let root = doc.root_element();

    let mut theme = UiThemeDesc::default();

    let visuals = attr_any(root, &["visuals", "theme"]).unwrap_or("auto");
    theme.visuals = match visuals.trim().to_ascii_lowercase().as_str() {
        "dark" => UiVisuals::Dark,
        "light" => UiVisuals::Light,
        _ => UiVisuals::Auto,
    };

    theme.scale = attr_f32(root, "scale").unwrap_or(1.0).clamp(0.25, 4.0);
    theme.font_size = attr_f32(root, "font_size").unwrap_or(14.0).clamp(8.0, 40.0);

    let density = attr_str(root, "density").unwrap_or("default");
    theme.density = match density.trim().to_ascii_lowercase().as_str() {
        "compact" => UiDensity::Compact,
        "dense" => UiDensity::Dense,
        "tight" => UiDensity::Tight,
        _ => UiDensity::Default,
    };

    theme
}

fn parse_children(parent: Node) -> Result<Vec<UiNode>, String> {
    let mut out = Vec::new();
    for n in parent.children().filter(|n| n.is_element()) {
        out.push(parse_node(n)?);
    }
    Ok(out)
}

fn parse_node(n: Node) -> Result<UiNode, String> {
    let tag = n.tag_name().name();
    match tag {
        "topbar" => Ok(UiNode::TopBar {
            children: parse_children(n)?,
        }),
        "window" => {
            let title = attr(n, "title").unwrap_or_else(|| "Window".to_string());
            let open = attr(n, "open")
                .map(|v| v == "true" || v == "1" || v == "yes")
                .unwrap_or(true);

            Ok(UiNode::Window {
                title,
                open,
                children: parse_children(n)?,
            })
        }
        "row" | "div" => {
            if tag == "div" {
                let class = attr(n, "class").unwrap_or_default();
                if !class.split_whitespace().any(|c| c == "row") {
                    return Ok(UiNode::Unknown {
                        tag: tag.to_string(),
                        children: parse_children(n)?,
                    });
                }
            }
            Ok(UiNode::Row {
                children: parse_children(n)?,
            })
        }
        "col" | "column" => Ok(UiNode::Column {
            children: parse_children(n)?,
        }),
        "label" => Ok(UiNode::Label {
            id: attr_opt(n, "id"),
            text: attr(n, "text").unwrap_or_default(),
        }),
        "button" => {
            let id = attr(n, "id").ok_or_else(|| "button requires id".to_string())?;
            let text = attr(n, "text").unwrap_or_else(|| "Button".to_string());

            let mut on_click = SmallVec::<[String; 2]>::new();
            parse_actions_for(&n, UiEventKind::Click, &mut on_click);

            Ok(UiNode::Button { id, text, on_click })
        }
        "textbox" | "input" => {
            let id = attr(n, "id").unwrap_or_else(|| "textbox".to_string());
            let bind = attr(n, "bind").unwrap_or_else(|| id.clone());
            let hint = attr(n, "hint").unwrap_or_default();
            let multiline = attr(n, "multiline")
                .map(|v| v == "true" || v == "1" || v == "yes")
                .unwrap_or(false);

            let mut on_change = SmallVec::<[String; 2]>::new();
            let mut on_submit = SmallVec::<[String; 2]>::new();
            parse_actions_for(&n, UiEventKind::Change, &mut on_change);
            parse_actions_for(&n, UiEventKind::Submit, &mut on_submit);

            Ok(UiNode::TextBox {
                id,
                hint,
                bind,
                multiline,
                on_change,
                on_submit,
            })
        }
        "spacer" => Ok(UiNode::Spacer),
        _ => Ok(UiNode::Unknown {
            tag: tag.to_string(),
            children: parse_children(n)?,
        }),
    }
}

fn attr(n: Node, key: &str) -> Option<String> {
    n.attribute(key).map(|s| s.to_string())
}

fn attr_opt(n: Node, key: &str) -> Option<String> {
    n.attribute(key).map(|s| s.to_string())
}

#[inline]
fn attr_str<'a>(n: Node<'a, 'a>, key: &str) -> Option<&'a str> {
    n.attribute(key).map(|s| s.trim()).filter(|s| !s.is_empty())
}

#[inline]
fn attr_any<'a>(n: Node<'a, 'a>, keys: &[&str]) -> Option<&'a str> {
    for k in keys {
        if let Some(v) = attr_str(n, k) {
            return Some(v);
        }
    }
    None
}

#[inline]
fn attr_f32(n: Node<'_, '_>, key: &str) -> Option<f32> {
    attr_str(n, key).and_then(|s| s.parse::<f32>().ok())
}

/* =============================================================================================
Declarative actions
============================================================================================= */

fn parse_actions_for(node: &Node<'_, '_>, kind: UiEventKind, out: &mut SmallVec<[String; 2]>) {
    // Explicit attrs:
    match kind {
        UiEventKind::Click => {
            if let Some(v) = node.attribute("on_click") {
                split_actions_into(v, out);
            }
        }
        UiEventKind::Change => {
            if let Some(v) = node.attribute("on_change") {
                split_actions_into(v, out);
            }
        }
        UiEventKind::Submit => {
            if let Some(v) = node.attribute("on_submit") {
                split_actions_into(v, out);
            }
        }
    }

    // Compact attr:
    // on="click:a,b; change:c; submit:d"
    if let Some(v) = node.attribute("on") {
        for chunk in v.split(';') {
            let chunk = chunk.trim();
            if chunk.is_empty() {
                continue;
            }
            let Some((ev, acts)) = chunk.split_once(':') else {
                continue;
            };
            let ev = ev.trim().to_ascii_lowercase();
            let acts = acts.trim();

            let match_kind = match ev.as_str() {
                "click" | "on_click" => UiEventKind::Click,
                "change" | "on_change" => UiEventKind::Change,
                "submit" | "on_submit" => UiEventKind::Submit,
                _ => continue,
            };

            if match_kind == kind {
                split_actions_into(acts, out);
            }
        }
    }
}

#[inline]
fn split_actions_into(s: &str, out: &mut SmallVec<[String; 2]>) {
    for part in s.split(|c| c == ',' || c == '|') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        out.push(p.to_string());
    }
}

/* =============================================================================================
Theme application (egui only)
============================================================================================= */

#[cfg(feature = "egui")]
fn apply_theme(ctx: &egui::Context, theme: &UiThemeDesc) {
    let mut style = (*ctx.style()).clone();

    match theme.visuals {
        UiVisuals::Auto => {}
        UiVisuals::Dark => style.visuals = egui::Visuals::dark(),
        UiVisuals::Light => style.visuals = egui::Visuals::light(),
    }

    let s = theme.scale;
    style.spacing.item_spacing *= s;
    style.spacing.window_margin *= s;
    style.spacing.button_padding *= s;
    style.spacing.indent *= s;
    style.spacing.interact_size *= s;

    match theme.density {
        UiDensity::Default => {}
        UiDensity::Compact => {
            style.spacing.item_spacing *= 0.85;
            style.spacing.button_padding *= 0.90;
        }
        UiDensity::Dense => {
            style.spacing.item_spacing *= 0.75;
            style.spacing.button_padding *= 0.85;
        }
        UiDensity::Tight => {
            style.spacing.item_spacing *= 0.65;
            style.spacing.button_padding *= 0.80;
        }
    }

    style.override_font_id = Some(egui::FontId::proportional(theme.font_size));
    ctx.set_style(style);
}