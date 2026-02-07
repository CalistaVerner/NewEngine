#![forbid(unsafe_op_in_unsafe_fn)]

use abi_stable::std_types::{RResult, RString, RVec};

use super::Provider;

pub(crate) struct FbxProvider;

impl FbxProvider {
    fn sniff_fbx(bytes: &[u8]) -> bool {
        // Binary FBX starts with "Kaydara FBX Binary".
        if bytes.len() >= 18 && &bytes[0..18] == b"Kaydara FBX Binary" {
            return true;
        }

        // ASCII FBX commonly starts with ';' comments like "; FBX 7.4.0 project file".
        let prefix = &bytes[..bytes.len().min(256)];
        let Ok(s) = std::str::from_utf8(prefix) else { return false; };
        let t = s.trim_start();
        t.starts_with(';') && t.contains("FBX")
    }
}

impl Provider for FbxProvider {
    fn name(&self) -> &'static str {
        "fbx"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["fbx"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        Self::sniff_fbx(bytes)
    }

    fn import(&self, bytes: &[u8]) -> RResult<RVec<u8>, RString> {
        // Phase 1: pass-through container with metadata.
        // Later we can add conversion into NE3D mesh/scene without changing ABI.
        if !Self::sniff_fbx(bytes) {
            return RResult::RErr(RString::from("fbx: not an fbx container"));
        }

        let meta = format!(
            "{{\"schema\":\"kalitech.model3d.meta.v1\",\"container\":\"fbx\",\"format\":\"fbx\",\"notes\":\"Pass-through bytes (conversion TBD)\",\"size_bytes\":{}}}",
            bytes.len()
        );

        let packed = super::super::module::pack_wire(&meta, bytes);
        RResult::ROk(RVec::from(packed))
    }

    fn describe_json(&self) -> &'static str {
        r#"{"name":"fbx","container":"fbx","notes":"Pass-through container (conversion TBD)."}"#
    }
}
