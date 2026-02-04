use crate::math::{Quatf, Vec3f};
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};

#[cfg(feature = "abi")]
use abi_stable::{std_types::RString, StableAbi};

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioEntityKind {
    Generic = 0,
    Ambience = 1,
    Voice = 2,
    MusicEmitter = 3,
    Vehicle = 4,
    Ui = 5,
}

impl Default for AudioEntityKind {
    fn default() -> Self {
        Self::Generic
    }
}

bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct AudioEntityFlags: u32 {
        const NONE = 0;

        const SPATIALIZED = 1 << 0;
        const DOPPLER     = 1 << 1;
        const OCCLUSION   = 1 << 2;
        const REVERB_SEND = 1 << 3;

        const STREAMING   = 1 << 8;
        const ONE_SHOT    = 1 << 9;
        const LOOPING     = 1 << 10;
    }
}

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Default, Debug, PartialEq, Zeroable, Pod)]
pub struct AudioTransform {
    pub position: Vec3f,
    pub rotation: Quatf,
    pub velocity: Vec3f,
}

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Default, Debug, PartialEq, Zeroable, Pod)]
pub struct AudioListenerDesc {
    pub transform: AudioTransform,
    pub up: Vec3f,
    pub forward: Vec3f,
}

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioEntityDesc {
    pub kind: AudioEntityKind,
    pub transform: AudioTransform,
    pub flags: AudioEntityFlags,
    pub gain: f32,
    pub pitch: f32,
}

impl Default for AudioEntityDesc {
    fn default() -> Self {
        Self {
            kind: AudioEntityKind::Generic,
            transform: AudioTransform::default(),
            flags: AudioEntityFlags::SPATIALIZED,
            gain: 1.0,
            pitch: 1.0,
        }
    }
}

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistanceModel {
    Inverse = 0,
    Linear = 1,
    Exponential = 2,
}

impl Default for DistanceModel {
    fn default() -> Self {
        Self::Inverse
    }
}

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpatializationDesc {
    pub distance_model: DistanceModel,
    pub min_distance: f32,
    pub max_distance: f32,
    pub rolloff: f32,
}

impl Default for SpatializationDesc {
    fn default() -> Self {
        Self {
            distance_model: DistanceModel::Inverse,
            min_distance: 1.0,
            max_distance: 100.0,
            rolloff: 1.0,
        }
    }
}

#[repr(C)]
#[cfg_attr(feature = "abi", derive(StableAbi))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioLogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

#[cfg(feature = "abi")]
#[repr(C)]
#[derive(StableAbi, Clone, Debug, PartialEq, Eq)]
pub struct AudioDebugLabel {
    pub name: RString,
}
