use crate::ids::AudioEntityId;

#[cfg(feature = "abi")]
use abi_stable::{sabi_trait, StableAbi};

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VehicleEngineState {
    Off = 0,
    Idle = 1,
    Running = 2,
    Redline = 3,
}

impl Default for VehicleEngineState {
    fn default() -> Self {
        Self::Off
    }
}

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct VehicleAudioFrame {
    pub engine_state: VehicleEngineState,
    pub rpm: f32,
    pub throttle: f32,
    pub load: f32,
    pub gear: i32,
    pub speed_mps: f32,
    pub turbo: f32,
}

#[cfg_attr(feature = "abi", sabi_trait)]
pub trait VehicleAudioV1: Send + Sync {
    fn bind_vehicle_entity(&self, vehicle: AudioEntityId);
    fn submit_frame(&self, vehicle: AudioEntityId, frame: VehicleAudioFrame);
}
