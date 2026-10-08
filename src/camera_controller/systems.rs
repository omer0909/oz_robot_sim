use super::components::LookCamera;
use bevy::picking::prelude::*;
use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use std::f32::consts::PI;

pub fn camera_controller_system(
    cam: Single<(
        &Camera,
        &Projection,
        &mut Transform,
        &GlobalTransform,
        &mut LookCamera,
    )>,
    window: Single<&Window>,
    mut ray_cast: MeshRayCast,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    accumulated_mouse_scroll: Res<AccumulatedMouseScroll>,
    mut cursor_options: Single<&mut CursorOptions>,
) {
    let (
        camera,
        Projection::Perspective(perspective),
        mut camera_transform,
        camera_global_transform,
        mut look_camera,
    ) = cam.into_inner()
    else {
        return;
    };

    let ray = window
        .cursor_position()
        .and_then(|cursor_pos| {
            camera
                .viewport_to_world(camera_global_transform, cursor_pos)
                .ok()
        })
        .unwrap_or(Ray3d::new(
            camera_global_transform.translation(),
            camera_global_transform.forward(),
        ));

    let distance = ray_cast
        .cast_ray(ray, &MeshRayCastSettings::default())
        .first()
        .map(|(_entity, hit)| hit.distance)
        .unwrap_or(2.0);

    if mouse_button_input.just_pressed(MouseButton::Left) {
        cursor_options.grab_mode = CursorGrabMode::Locked;
        look_camera.clicked_forward_distance =
            distance * ray.direction.dot(*camera_global_transform.forward());
        look_camera.clicked_pos = ray.get_point(distance);
    }

    if mouse_button_input.just_released(MouseButton::Left) {
        cursor_options.grab_mode = CursorGrabMode::None;
    }

    if accumulated_mouse_scroll.delta != Vec2::ZERO {
        let delta = accumulated_mouse_scroll.delta;
        camera_transform.translation += ray.direction * delta.y * distance.max(0.5) * 0.01;
    }

    if accumulated_mouse_motion.delta != Vec2::ZERO
        && mouse_button_input.pressed(MouseButton::Left)
        && (keyboard_input.pressed(KeyCode::ShiftLeft)
            || keyboard_input.pressed(KeyCode::ShiftRight))
    {
        let delta = accumulated_mouse_motion.delta;

        let factor = ((perspective.fov * 0.5).tan() / window.height())
            * 2.
            * look_camera.clicked_forward_distance;
        camera_transform.translation += camera_global_transform.left() * delta.x * factor;
        camera_transform.translation += camera_global_transform.up() * delta.y * factor;
    }

    if accumulated_mouse_motion.delta != Vec2::ZERO
        && mouse_button_input.pressed(MouseButton::Left)
        && !(keyboard_input.pressed(KeyCode::ShiftLeft)
            || keyboard_input.pressed(KeyCode::ShiftRight))
    {
        let sensitivity = 0.002;
        let delta = accumulated_mouse_motion.delta;
        look_camera.camera_angle.x =
            (look_camera.camera_angle.x - delta.y * sensitivity).clamp(-PI / 2.0, PI / 2.0);
        look_camera.camera_angle.y += -delta.x * sensitivity;
        let target = Quat::from_euler(
            EulerRot::YXZ,
            look_camera.camera_angle.y,
            look_camera.camera_angle.x,
            look_camera.camera_angle.z,
        );
        let to_pivot_world = look_camera.clicked_pos - camera_transform.translation;
        let to_pivot_local = camera_transform.rotation.inverse() * to_pivot_world;
        camera_transform.rotation = target;
        let to_pivot_world_new = target * to_pivot_local;
        camera_transform.translation = look_camera.clicked_pos - to_pivot_world_new;
    }
}
