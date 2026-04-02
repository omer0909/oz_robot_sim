use bevy::prelude::*;

pub mod components;
mod systems;

pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, systems::camera_controller_system);
    }
}
