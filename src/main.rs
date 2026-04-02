use avian3d::prelude::*;
use avian3d::spatial_query::SpatialQueryFilter;
use bevy::ecs::prelude::*;

use bevy::scene::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    camera::Exposure,
    core_pipeline::{Skybox, tonemapping::Tonemapping},
    gltf::GltfLoaderSettings,
    light::{CascadeShadowConfigBuilder, light_consts::lux},
    post_process::bloom::Bloom,
    prelude::*,
};

mod blender_editor;
mod camera_controller;

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
        camera_controller::components::LookCamera::default(),
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
        .observe(blender_editor::editor::on_scene_spawn);

    commands
        .spawn((
            SceneRoot(asset_server.load_with_settings(
                "robot.glb#Scene0",
                |settings: &mut GltfLoaderSettings| {
                    settings.load_meshes = RenderAssetUsages::all();
                },
            )),
            Robot,
            MapGenerator::new(),
        ))
        .observe(blender_editor::editor::on_scene_spawn);
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
            let filter =
                SpatialQueryFilter::from_mask(blender_editor::editor::GameLayer::default());
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

fn main() {
    blender_editor::editor::export_schema("schema.json");

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(camera_controller::CameraControllerPlugin)
        .add_systems(Startup, spawn)
        .add_systems(Update, map_generator)
        .run();
}
