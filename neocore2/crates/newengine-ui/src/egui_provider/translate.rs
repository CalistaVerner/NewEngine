use crate::draw::*;
use crate::texture::reserved;

/// Convert egui output into engine draw list.
/// This keeps renderer contract stable and isolates egui types inside this module.
pub fn egui_output_to_draw_list(
    ctx: &egui::Context,
    output: egui::FullOutput,
    out: &mut UiDrawList,
) {
    let pixels_per_point = ctx.pixels_per_point();
    out.pixels_per_point = pixels_per_point;

    // Viewport size in points; convert to physical pixels.
    let screen_rect = ctx.screen_rect();
    let w_px = (screen_rect.width() * pixels_per_point).round().max(0.0) as u32;
    let h_px = (screen_rect.height() * pixels_per_point).round().max(0.0) as u32;
    out.screen_size_px = [w_px, h_px];

    // Texture deltas (font atlas etc.)
    apply_texture_delta(&output.textures_delta, &mut out.texture_delta);

    // Tessellate shapes -> meshes
    let clipped_primitives = ctx.tessellate(output.shapes, output.pixels_per_point);

    // Flatten into one mesh with multiple draw commands.
    // This is optimal for a single dynamic VB/IB upload.
    for egui::ClippedPrimitive { clip_rect, primitive } in clipped_primitives {
        let clip = clip_rect_to_px(clip_rect, pixels_per_point);

        match primitive {
            egui::epaint::Primitive::Mesh(m) => {
                push_egui_mesh(&m, clip, pixels_per_point, &mut out.mesh);
            }
            egui::epaint::Primitive::Callback(_) => {
                // Callbacks are intentionally ignored in this backend-agnostic layer.
                // If you need them later, create an explicit engine callback command type.
            }
        }
    }
}

fn clip_rect_to_px(r: egui::Rect, ppp: f32) -> UiRect {
    // egui clip rect in points; convert to pixels.
    UiRect {
        min_x: (r.min.x * ppp).round(),
        min_y: (r.min.y * ppp).round(),
        max_x: (r.max.x * ppp).round(),
        max_y: (r.max.y * ppp).round(),
    }
}

fn push_egui_mesh(mesh: &egui::epaint::Mesh, clip: UiRect, ppp: f32, out: &mut UiMesh) {
    if mesh.indices.is_empty() || mesh.vertices.is_empty() {
        return;
    }

    let base_v = out.vertices.len() as u32;
    let base_i = out.indices.len() as u32;

    out.vertices.reserve(mesh.vertices.len());
    out.indices.reserve(mesh.indices.len());

    // Positions in egui are in points; convert to pixels.
    for v in &mesh.vertices {
        let pos_px = [v.pos.x * ppp, v.pos.y * ppp];
        let uv = [v.uv.x, v.uv.y];
        let color = egui_color_to_rgba8(v.color);

        out.vertices.push(UiVertex {
            pos: pos_px,
            uv,
            color,
        });
    }

    // Indices are u32 already.
    for &i in &mesh.indices {
        out.indices.push(base_v + i);
    }

    let tex = egui_texid_to_engine(mesh.texture_id);
    let cmd = UiDrawCmd {
        texture: tex,
        clip_rect: clip,
        index_range: base_i..(base_i + mesh.indices.len() as u32),
    };
    out.cmds.push(cmd);
}

#[inline]
fn egui_color_to_rgba8(c: egui::Color32) -> u32 {
    // egui::Color32 is sRGBA8
    let [r, g, b, a] = c.to_array();
    u32::from_le_bytes([r, g, b, a])
}

#[inline]
fn egui_texid_to_engine(id: egui::TextureId) -> UiTexId {
    match id {
        egui::TextureId::Managed(mid) => {
            // Reserve FONT_ATLAS for managed 0 by convention.
            // Egui's font atlas is typically managed; we map deterministically.
            // Other managed ids are offset into user range.
            if mid == 0 {
                reserved::FONT_ATLAS
            } else {
                UiTexId::new(reserved::USER_BEGIN + mid)
            }
        }
        egui::TextureId::User(u) => UiTexId::new(u as u32),
    }
}

fn apply_texture_delta(delta: &egui::TexturesDelta, out: &mut UiTextureDelta) {
    // Full sets
    for (id, image_delta) in &delta.set {
        let tex_id = egui_texid_to_engine(*id);

        // If pos is Some, this is a patch (sub-rect update).
        if let Some([x, y]) = image_delta.pos {
            let (w, h, rgba8) = image_delta_to_rgba8(&image_delta.image);
            out.patches.push(UiTexturePatch {
                id: tex_id,
                origin: [x as u32, y as u32],
                size: [w, h],
                rgba8,
            });
        } else {
            let (w, h, rgba8) = image_delta_to_rgba8(&image_delta.image);
            out.set.insert(
                tex_id,
                UiTexture {
                    size: [w, h],
                    rgba8,
                },
            );
        }
    }

    // Frees
    for id in &delta.free {
        out.free.push(egui_texid_to_engine(*id));
    }
}

fn image_delta_to_rgba8(img: &egui::ImageData) -> (u32, u32, Vec<u8>) {
    match img {
        egui::ImageData::Color(cimg) => {
            let w = cimg.size[0] as u32;
            let h = cimg.size[1] as u32;
            let mut rgba8 = Vec::with_capacity((w * h * 4) as usize);
            for p in &cimg.pixels {
                rgba8.extend_from_slice(&p.to_array());
            }
            (w, h, rgba8)
        }
        egui::ImageData::Font(fimg) => {
            let w = fimg.size[0] as u32;
            let h = fimg.size[1] as u32;
            // Font image is alpha8; expand to RGBA where RGB=255, A=alpha.
            let mut rgba8 = Vec::with_capacity((w * h * 4) as usize);
            for &a in &fimg.pixels {
                rgba8.push(255);
                rgba8.push(255);
                rgba8.push(255);
                rgba8.push(a);
            }
            (w, h, rgba8)
        }
    }
}