use crate::ids::AudioEntityId;

#[cfg(feature = "abi")]
use abi_stable::{sabi_trait, std_types::RString, StableAbi};

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoicePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for VoicePriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[cfg(feature = "abi")]
#[repr(C)]
#[derive(StableAbi, Clone, Debug, PartialEq, Eq)]
pub struct VoiceLineDesc {
    pub key: RString,
    pub priority: VoicePriority,
}

#[cfg_attr(feature = "abi", sabi_trait)]
pub trait VoiceSystemV1: Send + Sync {
    fn play_voice_line(&self, speaker: AudioEntityId, line: VoiceLineDesc) -> u64;
    fn stop_voice_instance(&self, instance_id: u64);
}
