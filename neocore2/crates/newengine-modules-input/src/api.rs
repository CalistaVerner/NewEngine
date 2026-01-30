use std::sync::{Arc, RwLock};

use newengine_core::host_events::{KeyCode, MouseButton};

use crate::state::{GamepadEvent, InputState};

/// Public input API exposed via `Resources`.
///
/// This API is intentionally low-level (keys, buttons, pointers, text) and stable.
/// Higher-level gameplay bindings (actions, axes, contexts) live in separate layers.
///
/// Threading:
/// - The producer is the engine thread (Input module).
/// - Consumers may read from any thread; reads are snapshot-based.
pub trait InputApi: Send + Sync {
    fn key_down(&self, key: KeyCode) -> bool;
    fn key_pressed(&self, key: KeyCode) -> bool;
    fn key_released(&self, key: KeyCode) -> bool;

    fn mouse_pos(&self) -> (f32, f32);
    fn mouse_delta(&self) -> (f32, f32);
    fn wheel_delta(&self) -> (f32, f32);

    fn mouse_down(&self, btn: MouseButton) -> bool;
    fn mouse_pressed(&self, btn: MouseButton) -> bool;
    fn mouse_released(&self, btn: MouseButton) -> bool;

    /// Snapshot of text chars produced since the last publish.
    ///
    /// Returned as an `Arc<[char]>` to avoid allocations on hot paths.
    fn text_chars(&self) -> Arc<[char]>;

    /// IME preedit string snapshot.
    fn ime_preedit(&self) -> Arc<str>;

    /// IME commit string snapshot.
    fn ime_commit(&self) -> Arc<str>;

    /// Drains gamepad events produced since the last publish.
    /// Caller should reuse `out` to avoid allocations.
    fn drain_gamepad_events(&self, out: &mut Vec<GamepadEvent>);
}

#[derive(Clone)]
pub struct InputApiImpl {
    snap: Arc<RwLock<Snapshot>>,
}

#[derive(Clone)]
struct Snapshot {
    keys_down: Vec<bool>,
    keys_pressed: Vec<bool>,
    keys_released: Vec<bool>,

    mouse_x: f32,
    mouse_y: f32,
    mouse_dx: f32,
    mouse_dy: f32,
    wheel_dx: f32,
    wheel_dy: f32,

    mouse_down_bits: u32,
    mouse_pressed_bits: u32,
    mouse_released_bits: u32,

    text: Arc<[char]>,
    ime_preedit: Arc<str>,
    ime_commit: Arc<str>,

    gamepad_events: Vec<GamepadEvent>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            keys_down: Vec::new(),
            keys_pressed: Vec::new(),
            keys_released: Vec::new(),

            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            wheel_dx: 0.0,
            wheel_dy: 0.0,

            mouse_down_bits: 0,
            mouse_pressed_bits: 0,
            mouse_released_bits: 0,

            text: Arc::from([]),
            ime_preedit: Arc::from(""),
            ime_commit: Arc::from(""),

            gamepad_events: Vec::new(),
        }
    }
}

impl InputApiImpl {
    #[inline]
    pub fn new(key_count: usize) -> Self {
        let mut s = Snapshot::default();
        s.keys_down.resize(key_count, false);
        s.keys_pressed.resize(key_count, false);
        s.keys_released.resize(key_count, false);

        Self {
            snap: Arc::new(RwLock::new(s)),
        }
    }

    #[inline]
    pub fn publish_from_state(&self, st: &InputState) {
        let mut g = self.snap.write().expect("InputApi snapshot poisoned");

        g.keys_down.clone_from(&st.keys_down);
        g.keys_pressed.clone_from(&st.keys_pressed);
        g.keys_released.clone_from(&st.keys_released);

        g.mouse_x = st.mouse_x;
        g.mouse_y = st.mouse_y;
        g.mouse_dx = st.mouse_dx;
        g.mouse_dy = st.mouse_dy;
        g.wheel_dx = st.wheel_dx;
        g.wheel_dy = st.wheel_dy;

        g.mouse_down_bits = st.mouse_down_bits;
        g.mouse_pressed_bits = st.mouse_pressed_bits;
        g.mouse_released_bits = st.mouse_released_bits;

        g.text = Arc::from(st.text.as_slice());
        g.ime_preedit = Arc::from(st.ime_preedit.as_str());
        g.ime_commit = Arc::from(st.ime_commit.as_str());

        g.gamepad_events.clone_from(&st.gamepad_events);
    }

    #[inline]
    pub fn as_dyn(self) -> Arc<dyn InputApi> {
        Arc::new(self)
    }
}

impl InputApi for InputApiImpl {
    #[inline(always)]
    fn key_down(&self, key: KeyCode) -> bool {
        let g = self.snap.read().expect("InputApi snapshot poisoned");
        g.keys_down.get(key.to_index()).copied().unwrap_or(false)
    }

    #[inline(always)]
    fn key_pressed(&self, key: KeyCode) -> bool {
        let g = self.snap.read().expect("InputApi snapshot poisoned");
        g.keys_pressed.get(key.to_index()).copied().unwrap_or(false)
    }

    #[inline(always)]
    fn key_released(&self, key: KeyCode) -> bool {
        let g = self.snap.read().expect("InputApi snapshot poisoned");
        g.keys_released.get(key.to_index()).copied().unwrap_or(false)
    }

    #[inline(always)]
    fn mouse_pos(&self) -> (f32, f32) {
        let g = self.snap.read().expect("InputApi snapshot poisoned");
        (g.mouse_x, g.mouse_y)
    }

    #[inline(always)]
    fn mouse_delta(&self) -> (f32, f32) {
        let g = self.snap.read().expect("InputApi snapshot poisoned");
        (g.mouse_dx, g.mouse_dy)
    }

    #[inline(always)]
    fn wheel_delta(&self) -> (f32, f32) {
        let g = self.snap.read().expect("InputApi snapshot poisoned");
        (g.wheel_dx, g.wheel_dy)
    }

    #[inline(always)]
    fn mouse_down(&self, btn: MouseButton) -> bool {
        let g = self.snap.read().expect("InputApi snapshot poisoned");
        (g.mouse_down_bits & crate::state::mouse_bit(btn)) != 0
    }

    #[inline(always)]
    fn mouse_pressed(&self, btn: MouseButton) -> bool {
        let g = self.snap.read().expect("InputApi snapshot poisoned");
        (g.mouse_pressed_bits & crate::state::mouse_bit(btn)) != 0
    }

    #[inline(always)]
    fn mouse_released(&self, btn: MouseButton) -> bool {
        let g = self.snap.read().expect("InputApi snapshot poisoned");
        (g.mouse_released_bits & crate::state::mouse_bit(btn)) != 0
    }

    #[inline]
    fn text_chars(&self) -> Arc<[char]> {
        self.snap.read().expect("InputApi snapshot poisoned").text.clone()
    }

    #[inline]
    fn ime_preedit(&self) -> Arc<str> {
        self.snap
            .read()
            .expect("InputApi snapshot poisoned")
            .ime_preedit
            .clone()
    }

    #[inline]
    fn ime_commit(&self) -> Arc<str> {
        self.snap
            .read()
            .expect("InputApi snapshot poisoned")
            .ime_commit
            .clone()
    }

    #[inline]
    fn drain_gamepad_events(&self, out: &mut Vec<GamepadEvent>) {
        let mut g = self.snap.write().expect("InputApi snapshot poisoned");
        out.clear();
        out.extend(g.gamepad_events.drain(..));
    }
}

/* =============================================================================================
   Action mapping layer (optional)
   ============================================================================================= */

/// Identifier for a gameplay action.
pub type ActionId = u16;

/// Snapshot of resolved gameplay actions.
#[derive(Clone, Default)]
pub struct ActionSnapshot {
    down: Vec<bool>,
    pressed: Vec<bool>,
    released: Vec<bool>,
}

impl ActionSnapshot {
    #[inline]
    pub fn new(action_count: usize) -> Self {
        let mut s = Self::default();
        s.down.resize(action_count, false);
        s.pressed.resize(action_count, false);
        s.released.resize(action_count, false);
        s
    }

    #[inline(always)]
    pub fn down(&self, id: ActionId) -> bool {
        self.down.get(id as usize).copied().unwrap_or(false)
    }

    #[inline(always)]
    pub fn pressed(&self, id: ActionId) -> bool {
        self.pressed.get(id as usize).copied().unwrap_or(false)
    }

    #[inline(always)]
    pub fn released(&self, id: ActionId) -> bool {
        self.released.get(id as usize).copied().unwrap_or(false)
    }
}

/// Binding source for an action.
///
/// `Hash` is intentionally NOT derived to avoid requiring `KeyCode/MouseButton: Hash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionBind {
    Key(KeyCode),
    Mouse(MouseButton),
}

/// Simple action map that resolves low-level input into gameplay actions.
///
/// This does not attempt to solve contexts, chords, or rebinding persistence.
/// It is deliberately small and can be composed with higher layers.
pub struct ActionMap {
    binds: Vec<Vec<ActionBind>>,
    snap: ActionSnapshot,
}

impl ActionMap {
    #[inline]
    pub fn new(action_count: usize) -> Self {
        Self {
            binds: vec![Vec::new(); action_count],
            snap: ActionSnapshot::new(action_count),
        }
    }

    #[inline]
    pub fn bind(&mut self, action: ActionId, bind: ActionBind) {
        if let Some(v) = self.binds.get_mut(action as usize) {
            v.push(bind);
        }
    }

    /// Resolves actions from the given `InputApi` snapshot.
    /// Call once per frame (typically after input module publishes).
    #[inline]
    pub fn update(&mut self, input: &dyn InputApi) {
        for i in 0..self.binds.len() {
            let mut down = false;
            let mut pressed = false;
            let mut released = false;

            for b in self.binds[i].iter().copied() {
                match b {
                    ActionBind::Key(k) => {
                        down |= input.key_down(k);
                        pressed |= input.key_pressed(k);
                        released |= input.key_released(k);
                    }
                    ActionBind::Mouse(m) => {
                        down |= input.mouse_down(m);
                        pressed |= input.mouse_pressed(m);
                        released |= input.mouse_released(m);
                    }
                }
            }

            self.snap.down[i] = down;
            self.snap.pressed[i] = pressed;
            self.snap.released[i] = released;
        }
    }

    #[inline]
    pub fn snapshot(&self) -> &ActionSnapshot {
        &self.snap
    }
}