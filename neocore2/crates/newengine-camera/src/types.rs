#![forbid(unsafe_op_in_unsafe_fn)]

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3, Vec4};

/// A compact representation of camera matrices required for rendering.
///
/// Matrices are column-major (GLSL convention).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct CameraMatrices {
    pub view: Mat4,
    pub proj: Mat4,
    pub view_proj: Mat4,
    pub inv_view: Mat4,
    pub inv_proj: Mat4,
    pub inv_view_proj: Mat4,
    pub world_pos: Vec3,
    pub _pad0: f32,
    pub viewport: Vec4, // (w, h, 1/w, 1/h)
    pub jitter: Vec2,
    pub _pad1: Vec2,
}

impl CameraMatrices {
    #[inline]
    pub fn new(view: Mat4, proj: Mat4, world_pos: Vec3, viewport_wh: Vec2, jitter: Vec2) -> Self {
        let view_proj = proj * view;
        let inv_view = view.inverse();
        let inv_proj = proj.inverse();
        let inv_view_proj = view_proj.inverse();

        let w = viewport_wh.x.max(1.0);
        let h = viewport_wh.y.max(1.0);

        Self {
            view,
            proj,
            view_proj,
            inv_view,
            inv_proj,
            inv_view_proj,
            world_pos,
            _pad0: 0.0,
            viewport: Vec4::new(w, h, 1.0 / w, 1.0 / h),
            jitter,
            _pad1: Vec2::ZERO,
        }
    }
}
