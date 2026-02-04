use crate::ids::{AudioBusId, AudioEntityId, AudioEventId, AudioSnapshotId};
use crate::types::{AudioEntityDesc, AudioListenerDesc, SpatializationDesc};

#[cfg(feature = "abi")]
use abi_stable::sabi_trait;

#[cfg_attr(feature = "abi", sabi_trait)]
pub trait AudioSystemV1: Send + Sync {
    /// Creates an audio entity handle. The backend decides the internal allocation strategy.
    fn create_entity(&self, desc: AudioEntityDesc) -> AudioEntityId;

    /// Destroys a previously created entity. It is valid to pass an invalid id.
    fn destroy_entity(&self, id: AudioEntityId);

    /// Updates transforms for an existing entity.
    fn set_entity_desc(&self, id: AudioEntityId, desc: AudioEntityDesc);

    /// Sets the main listener parameters.
    fn set_listener(&self, listener: AudioListenerDesc);

    /// Sets global spatialization defaults.
    fn set_spatialization_defaults(&self, desc: SpatializationDesc);

    /// Advances the simulation and executes the mixing graph.
    fn update(&self, dt_sec: f32);

    /// Plays a backend-defined event (one-shot or stateful). Returns an event instance id if supported.
    fn post_event(&self, event: AudioEventId, target: AudioEntityId) -> u64;

    /// Stops a previously posted event instance.
    fn stop_event_instance(&self, instance_id: u64);

    /// Sets a bus gain (linear).
    fn set_bus_gain(&self, bus: AudioBusId, gain: f32);

    /// Pushes a mixer snapshot with a normalized intensity [0..1].
    fn set_snapshot(&self, snapshot: AudioSnapshotId, intensity: f32);
}
