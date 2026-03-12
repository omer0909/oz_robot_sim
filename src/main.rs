use bevy::ecs::prelude::*;
use bevy::scene::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    camera::Exposure,
    core_pipeline::{Skybox, tonemapping::Tonemapping},
    gltf::GltfLoaderSettings,
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    light::{CascadeShadowConfigBuilder, light_consts::lux},
    post_process::bloom::Bloom,
    prelude::*,
    scene::SceneInstanceReady,
    window::{CursorGrabMode, CursorOptions},
};
use serde::Deserialize;

use std::f32::consts::PI;

use avian3d::spatial_query::SpatialQueryFilter;
use avian3d::{math::Quaternion, prelude::*};

#[derive(Component, Default)]
struct LookCamera {
    clicked_forward_distance: f32,
    clicked_pos: Vec3,
    camera_angle: Vec3,
}

#[derive(Component, Default)]
struct Robot;

#[derive(Component, Default)]
struct MapGenerator {
    pub ray_count: usize,
}

impl MapGenerator {
    fn new() -> Self {
        MapGenerator { ray_count: 1000 }
    }
}

#[derive(Deserialize, PhysicsLayer, Default, Debug)]
enum GameLayer {
    #[default]
    Default,
    RobotLayer,
}

fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
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
        DirectionalLight::default(),
        Transform::from_xyz(5.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands
        .spawn(SceneRoot(asset_server.load_with_settings(
            "scene.glb#Scene0",
            |settings: &mut GltfLoaderSettings| {
                settings.load_meshes = RenderAssetUsages::all();
            },
        )))
        .observe(on_scene_spawn);

    commands.spawn((
        SceneRoot(asset_server.load_with_settings(
            "robot.glb#Scene0",
            |settings: &mut GltfLoaderSettings| {
                settings.load_meshes = RenderAssetUsages::all();
            },
        )),
        Robot,
        MapGenerator::new(),
    ));
}

fn map_generator(
    spatial_query: SpatialQuery,
    mut map_generator: Query<(&MapGenerator, &GlobalTransform)>,
    mut gizmos: Gizmos,
) {
    for (generator, transform) in map_generator.iter_mut() {
        let origin = transform.translation() + Vec3::Y * 0.5;
        let angle_step = (std::f32::consts::PI * 2.0) / generator.ray_count as f32;

        for i in 0..generator.ray_count {
            let angle = i as f32 * angle_step;
            let direction = {
                let dir = Vec2::from_angle(angle);
                Vec3::new(dir.x, 0.0, dir.y)
            };
            let filter = SpatialQueryFilter::from_mask(GameLayer::Default);
            if let Some(hit) =
                spatial_query.cast_ray(origin, Dir3::new_unchecked(direction), 10.0, true, &filter)
            {
                let hit_point = origin + direction * hit.distance;
                gizmos.line(origin, hit_point, Color::linear_rgb(0.0, 1.0, 0.0));
                gizmos.circle(hit_point, 0.1, Color::linear_rgb(1.0, 0.0, 0.0));
            } else {
                let end_point = origin + direction * 10.0;
                gizmos.line(origin, end_point, Color::srgba(0.0, 1.0, 1.0, 0.3));
            }
        }
    }
}

fn camera_controller_system(
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

#[derive(Deserialize, Debug)]
struct BlenderBevyData {
    bevy_data: String,
}

#[derive(Deserialize, Debug)]
struct BlenderObjectData {
    components: BlenderComponentData,
}

#[derive(Deserialize, Debug)]
struct BlenderMeshColliderData {
    layer: GameLayer,
}

#[derive(Deserialize, Debug)]
struct BlenderComponentData {
    mesh_collider_fom_self: Option<BlenderMeshColliderData>,
}

fn on_scene_spawn(
    trigger: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    extras: Query<&GltfExtras>,
    mesh_query: Query<Entity, With<Mesh3d>>,
) {
    for entity in children.iter_descendants(trigger.entity) {
        let Ok(gltf_extras) = extras.get(entity) else {
            continue;
        };

        let Ok(object_data) = serde_json::from_str::<BlenderBevyData>(&gltf_extras.value)
            .and_then(|bevy_data| serde_json::from_str::<BlenderObjectData>(&bevy_data.bevy_data))
        else {
            continue;
        };

        if let Some(mesh_collider_data) = object_data.components.mesh_collider_fom_self
            && let Ok(entity_children) = children.get(entity)
        {
            for child in mesh_query.iter_many(entity_children) {
                commands.entity(child).insert((
                    RigidBody::Static,
                    ColliderConstructor::TrimeshFromMesh,
                    CollisionLayers::new(mesh_collider_data.layer.to_bits(), LayerMask::ALL),
                ));
            }
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_systems(Update, camera_controller_system)
        .add_systems(Startup, spawn)
        .add_systems(Update, map_generator)
        .run();
}
