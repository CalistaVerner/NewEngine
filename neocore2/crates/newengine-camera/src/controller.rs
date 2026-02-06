#![forbid(unsafe_op_in_unsafe_fn)]

use glam::{Quat, Vec2, Vec3};

use crate::rig::CameraRig;

/// Raw camera input for a single frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct CameraInput {
    /// Mouse delta in pixels (or any consistent unit).
    pub look_delta: Vec2,
    /// Movement axes: x = right, y = up, z = forward (positive forward).
    pub move_axis: Vec3,
    /// Sprint multiplier (e.g. 2.0 when shift is pressed).
    pub speed_mul: f32,
}

/// A simple, stable free-fly controller.
///
/// This is intentionally small and deterministic. It is a baseline for editor camera and debug flycam.
/// Game camera rigs (TLOU/CP-style) will build on top (constraints, collisions, cinematic layers).
#[derive(Clone, Copy, Debug)]
pub struct FreeFlyController {
    pub yaw: f32,
    pub pitch: f32,

    pub look_sens: f32,
    pub move_speed: f32,

    pub pitch_limit: f32,
}

impl Default for FreeFlyController {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            look_sens: 0.0025,
            move_speed: 6.0,
            pitch_limit: 1.54, // ~88 deg
        }
    }
}

impl FreeFlyController {
    #[inline]
    pub fn apply(&mut self, rig: &mut CameraRig, input: CameraInput, dt: f32) {
        let speed_mul = if input.speed_mul > 0.0 { input.speed_mul } else { 1.0 };

        self.yaw += input.look_delta.x * self.look_sens;
        self.pitch += input.look_delta.y * self.look_sens;
        self.pitch = self.pitch.clamp(-self.pitch_limit, self.pitch_limit);

        let rot_yaw = Quat::from_rotation_y(self.yaw);
        let rot_pitch = Quat::from_rotation_x(self.pitch);
        rig.rotation = rot_yaw * rot_pitch;

        // move_axis.z is forward; rig.forward() is -Z in RH, so we use local -Z for forward.
        let local = Vec3::new(input.move_axis.x, input.move_axis.y, -input.move_axis.z);
        let len = local.length();
        if len > 1e-6 {
            let dir = local / len;
            let delta = dir * (self.move_speed * speed_mul * dt);
            rig.translate_local(delta);
        }
    }
}