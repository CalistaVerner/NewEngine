#![forbid(unsafe_op_in_unsafe_fn)]

use abi_stable::std_types::{RResult, RString, RVec};

use super::Provider;

pub(crate) struct GltfProvider;

impl GltfProvider {
    fn detect_container(bytes: &[u8]) -> Option<&'static str> {
        if bytes.len() >= 4 && &bytes[0..4] == b"glTF" {
            return Some("glb");
        }

        let prefix = &bytes[..bytes.len().min(4096)];
        let Ok(s) = std::str::from_utf8(prefix) else { return None; };
        let t = s.trim_start();
        if t.starts_with('{') && (t.contains("\"asset\"") || t.contains("\"scenes\"")) {
            return Some("gltf");
        }

        None
    }

    fn validate(bytes: &[u8]) -> Result<(String, Vec<u8>), String> {
        let container = Self::detect_container(bytes).ok_or_else(|| "gltf: not a gltf/glb".to_owned())?;

        let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| format!("gltf: parse failed: {e}"))?;

        // NOTE: This importer operates on a single blob.
        // For .gltf we only support data: URIs (embedded buffers/images). External references are rejected.
        if container == "gltf" {
            let v: serde_json::Value = serde_json::from_slice(bytes)
                .map_err(|e| format!("gltf: json parse failed: {e}"))?;

            fn uri_is_external(uri: &str) -> bool {
                let u = uri.trim();
                !u.is_empty() && !u.starts_with("data:")
            }

            if let Some(buffers) = v.get("buffers").and_then(|x| x.as_array()) {
                for b in buffers {
                    if let Some(uri) = b.get("uri").and_then(|x| x.as_str()) {
                        if uri_is_external(uri) {
                            return Err(
                                "gltf: external buffer URIs are not supported (use .glb or embed data: URIs)".to_owned(),
                            );
                        }
                    }
                }
            }
            if let Some(images) = v.get("images").and_then(|x| x.as_array()) {
                for img in images {
                    if let Some(uri) = img.get("uri").and_then(|x| x.as_str()) {
                        if uri_is_external(uri) {
                            return Err(
                                "gltf: external image URIs are not supported (use .glb or embed data: URIs)".to_owned(),
                            );
                        }
                    }
                }
            }
        }

        let doc = &gltf.document;
        let scenes = doc.scenes().len();
        let nodes = doc.nodes().len();
        let meshes = doc.meshes().len();
        let materials = doc.materials().len();
        let textures = doc.textures().len();
        let images = doc.images().len();

        let meta = format!(
            "{{\"schema\":\"kalitech.model3d.meta.v1\",\"container\":\"{}\",\"format\":\"gltf\",\"gltf\":{{\"scenes\":{},\"nodes\":{},\"meshes\":{},\"materials\":{},\"textures\":{},\"images\":{}}}}}",
            container, scenes, nodes, meshes, materials, textures, images
        );

        Ok((meta, bytes.to_vec()))
    }
}

impl Provider for GltfProvider {
    fn name(&self) -> &'static str {
        "gltf"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["glb", "gltf"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        Self::detect_container(bytes).is_some()
    }

    fn import(&self, bytes: &[u8]) -> RResult<RVec<u8>, RString> {
        match Self::validate(bytes) {
            Ok((meta, payload)) => {
                let packed = super::super::module::pack_wire(&meta, &payload);
                RResult::ROk(RVec::from(packed))
            }
            Err(e) => RResult::RErr(RString::from(e)),
        }
    }

    fn describe_json(&self) -> &'static str {
        r#"{"name":"gltf","container":"glb|gltf","notes":"Validates and packs source bytes. .gltf requires embedded data URIs."}"#
    }
}
