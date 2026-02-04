use crate::math::Vec3f;

#[cfg(feature = "abi")]
use abi_stable::{sabi_trait, StableAbi};

/// Request describing an occlusion probe ray.
/// The host (physics / scene) may compute results and push them into audio.
#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct OcclusionRayDesc {
    pub origin: Vec3f,
    pub direction: Vec3f,
    pub max_distance: f32,
}

/// Occlusion query result.
/// - `occlusion`: attenuation caused by solid geometry in [0..1]
/// - `obstruction`: partial blockage (e.g. foliage) in [0..1]
#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct OcclusionResult {
    pub occlusion: f32,
    pub obstruction: f32,
}

/// Optional: defines a portal opening for indoor/outdoor transitions.
/// This is intentionally minimal and can be expanded later.
#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct AudioPortalDesc {
    pub id: u32,
    pub position: Vec3f,
    pub normal: Vec3f,
    pub width: f32,
    pub height: f32,
    pub openness: f32, // [0..1]
}

#[cfg_attr(feature = "abi", sabi_trait)]
pub trait AudioOcclusionV1: Send + Sync {
    /// Push an externally computed occlusion result.
    fn submit_occlusion_result(&self, ray: OcclusionRayDesc, result: OcclusionResult);

    /// Optional portal feed (indoor/outdoor acoustics). If unsupported, backend may ignore.
    fn set_portal(&self, portal: AudioPortalDesc);
}
