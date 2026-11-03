use avian3d::prelude::*;
use avian3d::spatial_query::SpatialQueryFilter;
use bevy::ecs::prelude::*;
use bevy::world_serialization::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    camera::Exposure,
    core_pipeline::tonemapping::Tonemapping,
    gltf::GltfLoaderSettings,
    light::{CascadeShadowConfigBuilder, light_consts::lux},
    post_process::bloom::Bloom,
    prelude::*,
};

mod blender_editor;
mod camera_controller;
mod mapping;
mod navigation;

fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    let load_glb = |path: &'static str| {
        asset_server
            .load_builder()
            .with_settings(|settings: &mut GltfLoaderSettings| {
                settings.load_meshes = RenderAssetUsages::all();
            })
            .load(path)
    };

    commands.spawn_scene_list(world(
        load_glb("scene.glb#Scene0"),
        load_glb("robot.glb#Scene0"),
    ));
}

fn world(scene: Handle<WorldAsset>, robot: Handle<WorldAsset>) -> impl SceneList {
    bsn_list! {
        Camera3d
        Transform::from_xyz(0.0, 0.0, 10.0)
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.25, 0.25, 0.25)),
        }
        EnvironmentMapLight {
            diffuse_map: "witsand_woolshop_4k_diffuse.ktx2",
            specular_map: "witsand_woolshop_4k_specular.ktx2",
            intensity: 1000.0,
        }
        Exposure::BLENDER
        Tonemapping::BlenderFilmic
        Bloom::NATURAL
        Msaa::Sample4
        camera_controller::components::LookCamera
        --
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: lux::AMBIENT_DAYLIGHT,
        }
        Transform::from_xyz(2.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Z)
        CascadeShadowConfigBuilder {
            maximum_distance: 30.0,
            ..default()
        }
        .build()
        --
        WorldAssetRoot({scene})
        on(blender_editor::editor::on_scene_spawn)
        --
        WorldAssetRoot({robot})
        on(blender_editor::editor::on_scene_spawn)
    }
}

fn move_system(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<blender_editor::editor::Robot>>,
    robot: Query<&blender_editor::editor::Robot>,
    spatial_query: SpatialQuery,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    for mut transform in &mut query {
        let rotation_factor = {
            let mut result = 0.0;
            if keyboard_input.pressed(KeyCode::ArrowLeft) {
                result += 1.0;
            }
            if keyboard_input.pressed(KeyCode::ArrowRight) {
                result -= 1.0;
            }
            result
        };
        transform.rotate_y(60.0_f32.to_radians() * time.delta_secs() * rotation_factor);

        let movement_factor = {
            let mut result = 0.0;
            if keyboard_input.pressed(KeyCode::ArrowUp) {
                result += 1.0;
            }
            if keyboard_input.pressed(KeyCode::ArrowDown) {
                result -= 1.0;
            }
            result
        };

        let movement = transform.forward() * 1.0 * movement_factor * time.delta_secs();

        let robot_height = 1.5;
        let Ok(robot) = robot.single() else {
            continue;
        };
        if let Ok(direction) = Dir3::new(movement) {
            let shape = Collider::cylinder(robot.radius, robot_height);
            let config = ShapeCastConfig::from_max_distance(movement.length() + 0.02);
            let filter =
                SpatialQueryFilter::from_mask(blender_editor::editor::GameLayer::default());
            let has_collision = spatial_query
                .cast_shape(
                    &shape,
                    transform.translation + Vec3::new(0.0, robot_height / 2.0 + 0.05, 0.0),
                    transform.rotation,
                    direction,
                    &config,
                    &filter,
                )
                .is_some();

            if !has_collision {
                transform.translation += movement;
            }
        }
    }
}

fn main() {
    #[cfg(debug_assertions)]
    blender_editor::editor::export_schema("schema.json");

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(camera_controller::CameraControllerPlugin)
        .add_systems(Startup, spawn)
        .add_systems(Update, mapping::systems::map_generator)
        .add_systems(Update, move_system)
        .add_systems(Update, navigation::systems::process_moving_target)
        .add_systems(Update, navigation::systems::check_go_target_comand)
        .insert_resource(mapping::resources::Map::new())
        .run();
}
