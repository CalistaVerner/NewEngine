use ahash::AHashMap;
use bytemuck::{Pod, Zeroable};
use smallvec::SmallVec;

/// A stable UI texture handle owned by the UI system.
/// Renderer side maps this to an actual GPU texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct UiTexId(pub u32);

impl UiTexId {
    #[inline]
    pub const fn new(v: u32) -> Self {
        Self(v)
    }
}

/// 2D rectangle in physical pixels (top-left origin).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct UiRect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl UiRect {
    #[inline]
    pub fn empty() -> Self {
        Self {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.max_x <= self.min_x || self.max_y <= self.min_y
    }
}

/// Vertex format designed for fast GPU upload.
/// Color is RGBA8 in sRGB UI space (renderer decides conversion).
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct UiVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: u32,
}

/// A single draw call: indexed triangle list with a clip rect and texture.
#[derive(Debug, Clone)]
pub struct UiDrawCmd {
    pub texture: UiTexId,
    pub clip_rect: UiRect,
    pub index_range: std::ops::Range<u32>,
}

/// One mesh batch: vertices + indices + commands referencing them.
#[derive(Debug, Clone)]
pub struct UiMesh {
    pub vertices: Vec<UiVertex>,
    pub indices: Vec<u32>,
    pub cmds: SmallVec<[UiDrawCmd; 8]>,
}

impl UiMesh {
    #[inline]
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            cmds: SmallVec::new(),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.cmds.clear();
    }
}

/// A full draw list for the current frame.
#[derive(Debug, Clone)]
pub struct UiDrawList {
    /// Physical screen size in pixels.
    pub screen_size_px: [u32; 2],
    /// DPI scale factor (physical / logical).
    pub pixels_per_point: f32,
    /// Mesh batches.
    pub mesh: UiMesh,
    /// Texture updates required by UI (font atlas etc.).
    pub texture_delta: UiTextureDelta,
}

impl UiDrawList {
    #[inline]
    pub fn new() -> Self {
        Self {
            screen_size_px: [0, 0],
            pixels_per_point: 1.0,
            mesh: UiMesh::new(),
            texture_delta: UiTextureDelta::new(),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.mesh.clear();
        self.texture_delta.clear();
    }
}

/// A CPU-side texture used to update renderer-owned textures.
#[derive(Debug, Clone)]
pub struct UiTexture {
    pub size: [u32; 2],
    /// RGBA8 pixels row-major; length must be size.x * size.y * 4.
    pub rgba8: Vec<u8>,
}

/// Incremental texture updates for the frame.
///
/// Renderer contract:
/// - Apply `set` first (create/replace full textures).
/// - Then apply `patches` (sub-rect updates).
/// - Then process `free`.
#[derive(Debug, Clone)]
pub struct UiTextureDelta {
    pub set: AHashMap<UiTexId, UiTexture>,
    pub patches: Vec<UiTexturePatch>,
    pub free: Vec<UiTexId>,
}

impl UiTextureDelta {
    #[inline]
    pub fn new() -> Self {
        Self {
            set: AHashMap::new(),
            patches: Vec::new(),
            free: Vec::new(),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.set.clear();
        self.patches.clear();
        self.free.clear();
    }
}

/// Sub-rect patch into an existing texture.
#[derive(Debug, Clone)]
pub struct UiTexturePatch {
    pub id: UiTexId,
    pub origin: [u32; 2],
    pub size: [u32; 2],
    /// RGBA8 pixels row-major; length must be size.x * size.y * 4.
    pub rgba8: Vec<u8>,
}