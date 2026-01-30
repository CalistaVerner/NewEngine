#![forbid(unsafe_op_in_unsafe_fn)]

use abi_stable::derive_macro_reexports::PrefixTypeTrait;
use abi_stable::erased_types::TD_Opaque;
use abi_stable::std_types::{RBox, ROption, RResult, RString};
use abi_stable::{export_root_module, sabi_extern_fn, StableAbi};

use newengine_plugin_api::{
    HostApiV1, PluginInfo, PluginModule, PluginModule_TO, PluginRootV1, PluginRootV1_Ref,
};

#[derive(StableAbi)]
#[repr(C)]
pub struct InputPlugin {
    host: ROption<RBox<HostApiV1>>,
    tick: u64,
    last_log_ns: u64,
}

impl Default for InputPlugin {
    fn default() -> Self {
        Self {
            host: ROption::RNone,
            tick: 0,
            last_log_ns: 0,
        }
    }
}

impl InputPlugin {
    #[inline(always)]
    fn host(&self) -> Option<&HostApiV1> {
        match &self.host {
            ROption::RSome(h) => Some(h.as_ref()),
            ROption::RNone => None,
        }
    }

    #[inline(always)]
    fn log_info(&self, msg: impl Into<RString>) {
        if let Some(h) = self.host() {
            (h.log_info)(msg.into());
        }
    }

    #[inline(always)]
    fn now_ns(&self) -> Option<u64> {
        self.host().map(|h| (h.monotonic_time_ns)())
    }
}

impl PluginModule for InputPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: RString::from("input"),
            version: RString::from(env!("CARGO_PKG_VERSION")),
        }
    }

    fn init(&mut self, host: HostApiV1) -> RResult<(), RString> {
        self.host = ROption::RSome(RBox::new(host));
        self.tick = 0;
        self.last_log_ns = self.now_ns().unwrap_or(0);

        self.log_info("input plugin: init, HELLO from Kayla <3");
        RResult::ROk(())
    }

    fn start(&mut self) -> RResult<(), RString> {
        if matches!(self.host, ROption::RNone) {
            return RResult::RErr(RString::from("input plugin: start called before init"));
        }

        self.log_info("input plugin: start");
        RResult::ROk(())
    }

    fn fixed_update(&mut self, _dt: f32) -> RResult<(), RString> {
        RResult::ROk(())
    }

    fn update(&mut self, _dt: f32) -> RResult<(), RString> {
        self.tick = self.tick.wrapping_add(1);

        let now = match self.now_ns() {
            Some(v) => v,
            None => return RResult::ROk(()),
        };

        const LOG_INTERVAL_NS: u64 = 500_000_000;

        if now < self.last_log_ns + LOG_INTERVAL_NS {
            return RResult::ROk(());
        }

        self.last_log_ns = now;

        let host = match self.host() {
            Some(h) => h,
            None => return RResult::ROk(()),
        };

        (host.log_info)(RString::from(format!(
            "input plugin: update tick={} time_ns={}",
            self.tick, now
        )));

        RResult::ROk(())
    }

    fn render(&mut self, _dt: f32) -> RResult<(), RString> {
        RResult::ROk(())
    }

    fn shutdown(&mut self) {
        self.log_info("input plugin: shutdown");
        self.host = ROption::RNone;
    }
}

#[sabi_extern_fn]
fn create_plugin() -> PluginModule_TO<'static, RBox<()>> {
    PluginModule_TO::from_value(InputPlugin::default(), TD_Opaque)
}

#[export_root_module]
pub fn newengine_plugin_root() -> PluginRootV1_Ref {
    PluginRootV1 {
        create: create_plugin,
    }
    .leak_into_prefix()
}
