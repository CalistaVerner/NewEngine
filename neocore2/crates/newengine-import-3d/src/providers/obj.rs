#![forbid(unsafe_op_in_unsafe_fn)]

use abi_stable::std_types::{RResult, RString, RVec};

use super::Provider;

pub(crate) struct ObjProvider;

impl ObjProvider {
    fn parse_mesh(bytes: &[u8]) -> Result<(String, Vec<u8>), String> {
        let s = std::str::from_utf8(bytes).map_err(|_| "obj: input is not valid utf-8".to_owned())?;

        let mut reader = std::io::Cursor::new(s.as_bytes());
        let (models, _materials) = tobj::load_obj_buf(
            &mut reader,
            &tobj::LoadOptions {
                triangulate: true,
                // Produce a single index domain so positions/normals/uvs are vertex-aligned.
                // This avoids the OBJ multi-index complexity at the importer boundary.
                single_index: true,
                ignore_points: true,
                ignore_lines: true,
            },
            |_| Ok((Vec::new(), std::collections::HashMap::new())),
        )
            .map_err(|e| format!("obj: parse failed: {e}"))?;

        let mut pos: Vec<[f32; 3]> = Vec::new();
        let mut nrm: Vec<[f32; 3]> = Vec::new();
        let mut uv: Vec<[f32; 2]> = Vec::new();
        let mut idx: Vec<u32> = Vec::new();

        let mut has_normals = false;
        let mut has_uvs = false;

        let mut bb_min = [f32::INFINITY; 3];
        let mut bb_max = [f32::NEG_INFINITY; 3];

        for m in models {
            let mesh = m.mesh;

            let vtx_count = mesh.positions.len() / 3;
            if vtx_count == 0 || mesh.indices.is_empty() {
                continue;
            }

            has_normals |= !mesh.normals.is_empty();
            has_uvs |= !mesh.texcoords.is_empty();

            let base = pos.len() as u32;

            for v in 0..vtx_count {
                let p0 = mesh.positions[v * 3];
                let p1 = mesh.positions[v * 3 + 1];
                let p2 = mesh.positions[v * 3 + 2];

                bb_min[0] = bb_min[0].min(p0);
                bb_min[1] = bb_min[1].min(p1);
                bb_min[2] = bb_min[2].min(p2);
                bb_max[0] = bb_max[0].max(p0);
                bb_max[1] = bb_max[1].max(p1);
                bb_max[2] = bb_max[2].max(p2);

                pos.push([p0, p1, p2]);

                if has_normals {
                    let n0 = mesh.normals.get(v * 3).copied().unwrap_or(0.0);
                    let n1 = mesh.normals.get(v * 3 + 1).copied().unwrap_or(0.0);
                    let n2 = mesh.normals.get(v * 3 + 2).copied().unwrap_or(0.0);
                    nrm.push([n0, n1, n2]);
                }

                if has_uvs {
                    let t0 = mesh.texcoords.get(v * 2).copied().unwrap_or(0.0);
                    let t1 = mesh.texcoords.get(v * 2 + 1).copied().unwrap_or(0.0);
                    uv.push([t0, t1]);
                }
            }

            idx.extend(mesh.indices.iter().map(|&i| base + i));
        }

        if pos.is_empty() || idx.is_empty() {
            return Err("obj: no geometry".to_owned());
        }

        let flags: u32 = (has_normals as u32) | ((has_uvs as u32) << 1);

        let mut out = Vec::new();
        out.extend_from_slice(b"NE3D");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&(pos.len() as u32).to_le_bytes());
        out.extend_from_slice(&(idx.len() as u32).to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());

        for p in &pos {
            out.extend_from_slice(&p[0].to_le_bytes());
            out.extend_from_slice(&p[1].to_le_bytes());
            out.extend_from_slice(&p[2].to_le_bytes());
        }
        if has_normals {
            for n in &nrm {
                out.extend_from_slice(&n[0].to_le_bytes());
                out.extend_from_slice(&n[1].to_le_bytes());
                out.extend_from_slice(&n[2].to_le_bytes());
            }
        }
        if has_uvs {
            for t in &uv {
                out.extend_from_slice(&t[0].to_le_bytes());
                out.extend_from_slice(&t[1].to_le_bytes());
            }
        }
        for i in &idx {
            out.extend_from_slice(&i.to_le_bytes());
        }

        let meta = format!(
            "{{\"schema\":\"kalitech.model3d.meta.v1\",\"container\":\"obj\",\"format\":\"ne3d_mesh\",\"mesh\":{{\"vertex_count\":{},\"index_count\":{},\"has_normals\":{},\"has_uvs\":{},\"bbox_min\":[{:.6},{:.6},{:.6}],\"bbox_max\":[{:.6},{:.6},{:.6}]}}}}",
            pos.len(),
            idx.len(),
            has_normals,
            has_uvs,
            bb_min[0],
            bb_min[1],
            bb_min[2],
            bb_max[0],
            bb_max[1],
            bb_max[2]
        );

        Ok((meta, out))
    }
}

impl Provider for ObjProvider {
    fn name(&self) -> &'static str {
        "obj"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["obj"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        // A very tolerant sniff: OBJ is text and usually contains "v " early.
        let prefix = &bytes[..bytes.len().min(2048)];
        let Ok(s) = std::str::from_utf8(prefix) else { return false; };
        let s = s.trim_start();
        s.starts_with('#') || s.starts_with('v') || s.contains("\nv ") || s.contains("\nvn ") || s.contains("\nf ")
    }

    fn import(&self, bytes: &[u8]) -> RResult<RVec<u8>, RString> {
        match Self::parse_mesh(bytes) {
            Ok((meta, payload)) => {
                let packed = super::super::module::pack_wire(&meta, &payload);
                RResult::ROk(RVec::from(packed))
            }
            Err(e) => RResult::RErr(RString::from(e)),
        }
    }

    fn describe_json(&self) -> &'static str {
        r#"{"name":"obj","container":"obj","notes":"Converted to NE3D mesh (little-endian)."}"#
    }
}
