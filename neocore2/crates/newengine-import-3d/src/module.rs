#![forbid(unsafe_op_in_unsafe_fn)]

use abi_stable::sabi_trait::TD_Opaque;
use abi_stable::std_types::{RResult, RString, RVec};
use abi_stable::StableAbi;

use newengine_plugin_api::{
    Blob, HostApiV1, MethodName, PluginInfo, PluginModule, ServiceV1, ServiceV1Dyn, ServiceV1_TO,
};

use std::sync::OnceLock;

use crate::providers;

/* =============================================================================================
Wire: [u32 meta_len_le][meta_json utf8][payload bytes]
============================================================================================= */

#[inline]
pub(crate) fn pack_wire(meta_json: &str, payload: &[u8]) -> Vec<u8> {
    let meta = meta_json.as_bytes();
    let meta_len: u32 = meta.len().min(u32::MAX as usize) as u32;

    let mut out = Vec::with_capacity(4 + meta.len() + payload.len());
    out.extend_from_slice(&meta_len.to_le_bytes());
    out.extend_from_slice(meta);
    out.extend_from_slice(payload);
    out
}

#[inline]
fn err(msg: impl Into<String>) -> RResult<RVec<u8>, RString> {
    RResult::RErr(RString::from(msg.into()))
}

fn import_auto(bytes: &[u8]) -> RResult<RVec<u8>, RString> {
    for p in providers::iter_providers() {
        if p.sniff(bytes) {
            return p.import(bytes);
        }
    }

    // Fallback: try parsers even if sniffing failed (helps with edge cases).
    for p in providers::iter_providers() {
        let r = p.import(bytes);
        if r.is_ok() {
            return r;
        }
    }

    err("3d: unsupported container")
}

#[derive(StableAbi)]
#[repr(C)]
struct ThreeDImporterService;

impl ThreeDImporterService {
    #[inline]
    fn describe_cached() -> &'static str {
        static CACHED: OnceLock<String> = OnceLock::new();

        CACHED
            .get_or_init(|| {
                let mut exts: Vec<&'static str> = Vec::new();
                let mut formats: Vec<&'static str> = Vec::new();

                for p in providers::iter_providers() {
                    for &e in p.extensions() {
                        if !exts.iter().any(|&x| x == e) {
                            exts.push(e);
                        }
                    }
                    formats.push(p.describe_json());
                }

                exts.sort_unstable();

                let mut exts_json = String::with_capacity(2 + exts.len() * 8);
                exts_json.push('[');
                for (i, e) in exts.iter().enumerate() {
                    if i != 0 {
                        exts_json.push(',');
                    }
                    exts_json.push('"');
                    exts_json.push_str(e);
                    exts_json.push('"');
                }
                exts_json.push(']');

                let mut formats_json = String::with_capacity(2 + formats.len() * 64);
                formats_json.push('[');
                for (i, f) in formats.iter().enumerate() {
                    if i != 0 {
                        formats_json.push(',');
                    }
                    formats_json.push_str(f);
                }
                formats_json.push(']');

                format!(
                    r#"{{
  "id":"kalitech.import.3d.v1",
  "kind":"asset_importer",
  "asset_importer":{{
    "priority":120,
    "extensions":{exts_json},
    "output_type_id":"kalitech.asset.model3d",
    "format":"3d",
    "method":"import_3d_v1",
    "wire":"u32_meta_len_le + meta_utf8 + payload",
    "formats":{formats_json}
  }},
  "methods":{{
    "import_3d_v1":{{"in":"3d bytes (auto sniff)","out":"[u32 meta_len_le][meta_json][payload]"}}
  }},
  "meta_schema":"kalitech.model3d.meta.v1"
}}"#,
                    exts_json = exts_json,
                    formats_json = formats_json,
                )
            })
            .as_str()
    }
}

impl ServiceV1 for ThreeDImporterService {
    fn id(&self) -> RString {
        RString::from("kalitech.import.3d.v1")
    }

    fn describe(&self) -> RString {
        RString::from(Self::describe_cached())
    }

    fn call(&self, method: MethodName, payload: Blob) -> RResult<Blob, RString> {
        let bytes: Vec<u8> = payload.into_vec();
        match method.as_str() {
            "import_3d_v1" => import_auto(&bytes).map(|v| v),
            _ => RResult::RErr(RString::from(format!(
                "3d-importer: unknown method '{}'",
                method
            ))),
        }
    }
}

#[derive(Default)]
pub struct ThreeDImporterPlugin;

impl PluginModule for ThreeDImporterPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: RString::from("import.3d"),
            name: RString::from("3D Importer (.obj/.fbx/.glb/.gltf)"),
            version: RString::from(env!("CARGO_PKG_VERSION")),
        }
    }

    fn init(&mut self, host: HostApiV1) -> RResult<(), RString> {
        let svc: ServiceV1Dyn<'static> = ServiceV1_TO::from_value(ThreeDImporterService, TD_Opaque);

        let r = (host.register_service_v1)(svc);
        if let Err(e) = r.clone().into_result() {
            (host.log_warn)(RString::from(format!(
                "3d-importer: register service failed: {}",
                e
            )));
            return r;
        }

        (host.log_info)(RString::from("3d-importer: service registered (kalitech.import.3d.v1)"));
        RResult::ROk(())
    }

    fn start(&mut self) -> RResult<(), RString> {
        RResult::ROk(())
    }

    fn fixed_update(&mut self, _dt: f32) -> RResult<(), RString> {
        RResult::ROk(())
    }

    fn update(&mut self, _dt: f32) -> RResult<(), RString> {
        RResult::ROk(())
    }

    fn render(&mut self, _dt: f32) -> RResult<(), RString> {
        RResult::ROk(())
    }

    fn shutdown(&mut self) {}
}
