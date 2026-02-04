use crate::ids::{AudioBusId, AudioSnapshotId};

#[cfg(feature = "abi")]
use abi_stable::{sabi_trait, StableAbi};

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MixerCurve {
    Linear = 0,
    SmoothStep = 1,
    Exponential = 2,
}

impl Default for MixerCurve {
    fn default() -> Self {
        Self::Linear
    }
}

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct SnapshotBlendDesc {
    pub id: AudioSnapshotId,
    pub intensity: f32,
    pub curve: MixerCurve,
}

#[cfg_attr(feature = "abi", sabi_trait)]
pub trait MixerSystemV1: Send + Sync {
    fn set_bus_gain(&self, bus: AudioBusId, gain: f32);
    fn set_snapshot_blend(&self, desc: SnapshotBlendDesc);
}
