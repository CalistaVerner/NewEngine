use crate::capability::{AudioCapabilityMask, AUDIO_API_VERSION_V1};
use crate::occlusion::AudioOcclusionV1;

#[cfg(feature = "abi")]
use abi_stable::sabi_trait;

/// High-level entry point for typed usage (optional).
/// NOTE: This is NOT what gets registered into `newengine-plugin-api`.
/// The plugin must register a `ServiceV1` and expose this API through method calls.
#[cfg_attr(feature = "abi", sabi_trait)]
pub trait AudioApiV1: Send + Sync {
    #[inline]
    fn api_version(&self) -> u32 {
        AUDIO_API_VERSION_V1
    }

    fn capabilities(&self) -> AudioCapabilityMask;

    fn system(&self) -> AudioSystemV1Dyn<'_>;
    fn ambience(&self) -> AmbienceSystemV1Dyn<'_>;
    fn environment(&self) -> AudioEnvironmentV1Dyn<'_>;
    fn occlusion(&self) -> AudioOcclusionV1Dyn<'_>;
    fn mixer(&self) -> MixerSystemV1Dyn<'_>;
    fn music(&self) -> MusicSystemV1Dyn<'_>;
    fn voice(&self) -> VoiceSystemV1Dyn<'_>;
    fn vehicle(&self) -> VehicleAudioV1Dyn<'_>;
}
