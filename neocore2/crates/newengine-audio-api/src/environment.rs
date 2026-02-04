#[cfg(feature = "abi")]
use abi_stable::{sabi_trait, StableAbi};

/// Global acoustic environment parameters (simple reverb-like model).
/// Backends can map this to their own reverb buses / sends.
#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct EnvironmentDesc {
    pub reverb_room: f32,
    pub reverb_decay: f32,
    pub reverb_damping: f32,
    pub hf_reference: f32,
    pub wet: f32,
}

#[cfg_attr(feature = "abi", sabi_trait)]
pub trait AudioEnvironmentV1: Send + Sync {
    fn set_environment(&self, env: EnvironmentDesc);
}
