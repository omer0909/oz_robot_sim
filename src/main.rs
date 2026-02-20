use bevy::{
    camera::Exposure,
    core_pipeline::{Skybox, tonemapping::Tonemapping},
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    light::{CascadeShadowConfigBuilder, light_consts::lux},
    post_process::bloom::Bloom,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use std::f32::consts::PI;

use avian3d::spatial_query::SpatialQueryFilter;
use avian3d::{math::Quaternion, prelude::*};
#[derive(Component)]
struct LookCamera {
    clicked_forward_distance: f32,
    clicked_pos: Vec3,
    camera_angle: Vec3,
}

impl LookCamera {
    fn default() -> Self {
        LookCamera {
            clicked_forward_distance: 0.0,
            clicked_pos: Vec3::ZERO,
            camera_angle: Vec3::ZERO,
        }
    }
}

fn spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let texture_handle = asset_server.load("Ryfjallet_cubemap_bc7.ktx2");
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0),
        Skybox {
            image: texture_handle.clone(),
            brightness: 1000.0,
            ..default()
        },
        Exposure::BLENDER,
        Tonemapping::BlenderFilmic,
        Bloom::NATURAL,
        LookCamera::default(),
    ));

    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 0.3,
        maximum_distance: 3.0,
        ..default()
    }
    .build();

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: lux::OFFICE,
            ..default()
        },
        Transform::from_xyz(2.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Z),
        cascade_shadow_config,
    ));

    commands.spawn((
        Transform::default(),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        RigidBody::Static,
        Collider::cuboid(1.0, 1.0, 1.0),
    ));
}

fn test_system(
    cam: Single<(
        &Camera,
        &Projection,
        &mut Transform,
        &GlobalTransform,
        &mut LookCamera,
    )>,
    window: Single<&Window>,
    spatial_query: SpatialQuery,
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

    let distance = spatial_query
        .cast_ray(
            ray.origin,
            ray.direction,
            1000.0,
            true,
            &SpatialQueryFilter::default(),
        )
        .map(|hit| hit.distance)
        .unwrap_or(2.0)
        .max(0.5);

    if mouse_button_input.just_pressed(MouseButton::Left) {
        cursor_options.grab_mode = CursorGrabMode::Locked;
        look_camera.clicked_forward_distance =
            distance * ray.direction.dot(*camera_global_transform.forward());
        look_camera.clicked_pos = camera_global_transform.translation() + ray.direction * distance;
    }

    if mouse_button_input.just_released(MouseButton::Left) {
        cursor_options.grab_mode = CursorGrabMode::None;
    }

    if accumulated_mouse_scroll.delta != Vec2::ZERO {
        let delta = accumulated_mouse_scroll.delta;
        camera_transform.translation += ray.direction * delta.y * distance * 0.1;
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
        let target = Quaternion::from_euler(
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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_systems(Update, test_system)
        .add_systems(Startup, spawn)
        .run();
}
