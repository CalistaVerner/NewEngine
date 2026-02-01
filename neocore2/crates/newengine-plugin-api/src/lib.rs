#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(non_local_definitions)]
#![allow(non_camel_case_types)]

use abi_stable::{
    library::RootModule,
    sabi_trait,
    std_types::{RBox, RResult, RString},
    sabi_types::VersionStrings,
    StableAbi,
};


/* =============================================================================================
   ABI-safe small structs (no tuples)
   ============================================================================================= */

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, StableAbi)]
pub struct Vec2fAbi {
    pub x: f32,
    pub y: f32,
}

impl Vec2fAbi {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, StableAbi)]
pub struct Vec2uAbi {
    pub x: u32,
    pub y: u32,
}

impl Vec2uAbi {
    #[inline]
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

/* =============================================================================================
   ABI-safe input / window event types
   ============================================================================================= */

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, StableAbi)]
pub enum KeyCodeAbi {
    Escape,
    Enter,
    Space,
    Tab,
    Backspace,

    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,

    Digit0, Digit1, Digit2, Digit3, Digit4,
    Digit5, Digit6, Digit7, Digit8, Digit9,

    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,

    Unknown,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, StableAbi)]
pub enum KeyStateAbi {
    Pressed,
    Released,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, StableAbi)]
pub enum MouseButtonAbi {
    Left,
    Right,
    Middle,
    Other(u16),
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub enum HostEventAbi {
    Window(WindowHostEventAbi),
    Input(InputHostEventAbi),
    Text(TextHostEventAbi),
}

#[repr(C)]
#[derive(Debug, Clone, Copy, StableAbi)]
pub enum WindowHostEventAbi {
    Ready { size: Vec2uAbi },
    Resized { size: Vec2uAbi },
    Focused(bool),
    CloseRequested,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, StableAbi)]
pub enum InputHostEventAbi {
    Key {
        code: KeyCodeAbi,
        state: KeyStateAbi,
        repeat: bool,
    },
    MouseMove { pos: Vec2fAbi },
    MouseDelta { delta: Vec2fAbi },
    MouseButton {
        button: MouseButtonAbi,
        state: KeyStateAbi,
    },
    MouseWheel { delta: Vec2fAbi },
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub enum TextHostEventAbi {
    /// Unicode scalar value (0..=0x10FFFF). Avoids `char` which is not `StableAbi`.
    CharU32(u32),
    ImePreedit(RString),
    ImeCommit(RString),
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct PluginInfo {
    pub id: RString,
    pub version: RString,
}

/* =============================================================================================
   ABI traits
   ============================================================================================= */

#[sabi_trait]
pub trait HostEventSink: Send {
    fn on_host_event(&mut self, ev: HostEventAbi);
}

pub type HostEventSinkDyn<'a> = HostEventSink_TO<'a, RBox<()>>;

#[sabi_trait]
pub trait InputApiV1: Send + Sync {
    fn key_down(&self, key: KeyCodeAbi) -> bool;
    fn key_pressed(&self, key: KeyCodeAbi) -> bool;
    fn key_released(&self, key: KeyCodeAbi) -> bool;

    fn mouse_pos(&self) -> Vec2fAbi;
    fn mouse_delta(&self) -> Vec2fAbi;
    fn wheel_delta(&self) -> Vec2fAbi;

    fn mouse_down(&self, btn: MouseButtonAbi) -> bool;
    fn mouse_pressed(&self, btn: MouseButtonAbi) -> bool;
    fn mouse_released(&self, btn: MouseButtonAbi) -> bool;

    fn text_take(&self) -> RString;
    fn ime_preedit(&self) -> RString;
    fn ime_commit_take(&self) -> RString;
}

pub type InputApiV1Dyn<'a> = InputApiV1_TO<'a, RBox<()>>;

#[repr(C)]
#[derive(abi_stable::StableAbi, Clone)]
pub struct HostApiV1 {
    pub log_info: extern "C" fn(RString),
    pub log_warn: extern "C" fn(RString),
    pub log_error: extern "C" fn(RString),

    pub request_exit: extern "C" fn(),
    pub monotonic_time_ns: extern "C" fn() -> u64,

    pub subscribe_host_events:
        extern "C" fn(HostEventSinkDyn<'static>) -> RResult<(), RString>,

    pub provide_input_api_v1:
        extern "C" fn(InputApiV1Dyn<'static>) -> RResult<(), RString>,
}

#[sabi_trait]
pub trait PluginModule: Send {
    fn info(&self) -> PluginInfo;

    fn init(&mut self, host: HostApiV1) -> RResult<(), RString>;
    fn start(&mut self) -> RResult<(), RString>;

    fn fixed_update(&mut self, dt: f32) -> RResult<(), RString>;
    fn update(&mut self, dt: f32) -> RResult<(), RString>;
    fn render(&mut self, dt: f32) -> RResult<(), RString>;

    fn shutdown(&mut self);
}

pub type PluginModuleDyn<'a> = PluginModule_TO<'a, RBox<()>>;

/* =============================================================================================
   RootModule prefix (creates PluginRootV1_Ref + load_from_file)
   ============================================================================================= */

/* =============================================================================================
   RootModule prefix (creates PluginRootV1_Ref + load_from_file)
   ============================================================================================= */

#[repr(C)]
#[derive(StableAbi)]
#[allow(non_camel_case_types)]
#[sabi(kind(Prefix(prefix_ref = PluginRootV1_Ref)))]
pub struct PluginRootV1 {
    #[sabi(last_prefix_field)]
    pub create: extern "C" fn() -> PluginModuleDyn<'static>,
}

impl RootModule for PluginRootV1_Ref {
    abi_stable::declare_root_module_statics! { PluginRootV1_Ref }

    const BASE_NAME: &'static str = "newengine_plugin";
    const NAME: &'static str = "newengine_plugin";
    const VERSION_STRINGS: VersionStrings = abi_stable::package_version_strings!();
}