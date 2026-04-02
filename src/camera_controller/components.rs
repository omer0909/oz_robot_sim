use bevy::prelude::*;

#[derive(Component, Default)]
pub struct LookCamera {
    pub(super) clicked_forward_distance: f32,
    pub(super) clicked_pos: Vec3,
    pub(super) camera_angle: Vec3,
}
