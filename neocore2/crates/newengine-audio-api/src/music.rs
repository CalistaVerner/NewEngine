use crate::ids::AudioTagId;

#[cfg(feature = "abi")]
use abi_stable::{sabi_trait, StableAbi};

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicState {
    None = 0,
    Exploration = 1,
    Combat = 2,
    Tension = 3,
    Cinematic = 4,
}

impl Default for MusicState {
    fn default() -> Self {
        Self::None
    }
}

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct MusicParams {
    pub intensity: f32,
    pub danger: f32,
    pub player_health: f32,
}

#[cfg_attr(feature = "abi", sabi_trait)]
pub trait MusicSystemV1: Send + Sync {
    fn set_state(&self, state: MusicState);
    fn set_params(&self, params: MusicParams);

    /// Tag-based triggers (e.g. biome, quest phase, faction).
    fn trigger_tag(&self, tag: AudioTagId, on: bool);
}
