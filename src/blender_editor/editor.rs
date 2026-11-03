use avian3d::prelude::*;
use bevy::{prelude::*, world_serialization::WorldInstanceReady};
use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde::Deserialize;
use std::fs;

#[derive(Component)]
pub struct Robot {
    pub ray_count: usize,
    pub lidar: Entity,
    pub radius: f32,
    pub slam_active: bool,
}

impl Robot {
    fn new(ray_count: usize, lidar: Entity, radius: f32) -> Self {
        Robot {
            ray_count,
            lidar,
            radius,
            slam_active: true,
        }
    }
}

#[derive(JsonSchema, Deserialize, Debug)]
#[schemars(extend("blender_object_ref" = "full_path"))]
struct ObjectRef(String);

#[derive(Deserialize, PhysicsLayer, JsonSchema, Default, Debug)]
pub enum GameLayer {
    #[default]
    Default,
    IgnoreLidar,
}

#[derive(Deserialize, Debug)]
struct BlenderBevyData {
    bevy_data: String,
}

#[derive(Deserialize, JsonSchema, Debug)]
struct BlenderObjectData {
    components: BlenderComponentData,
}

#[derive(Deserialize, JsonSchema, Debug)]
struct BlenderMeshColliderData {
    layer: GameLayer,
}

#[derive(Deserialize, JsonSchema, Debug)]
struct RobotComponent {
    lidar: ObjectRef,
    footprint_circle_radius: f32,
}

#[derive(Deserialize, JsonSchema, Debug)]
struct BlenderComponentData {
    mesh_collider_fom_self: Option<BlenderMeshColliderData>,
    robot: Option<RobotComponent>,
}

pub fn export_schema(path: &str) {
    let mut settings = SchemaSettings::draft07();
    settings.inline_subschemas = true;

    let generator = settings.into_generator();
    let sema = generator.into_root_schema_for::<BlenderObjectData>();

    let sema_json = serde_json::to_string_pretty(&sema).unwrap();
    fs::write(path, &sema_json).expect(&format!("{path} can not write!"));
}

pub fn on_scene_spawn(
    trigger: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    extras: Query<&GltfExtras>,
    mesh_query: Query<Entity, With<Mesh3d>>,
    name_query: Query<(Entity, &Name)>,
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
            for child in mesh_query.iter_many(entity_children).flatten() {
                commands.entity(child).insert((
                    RigidBody::Static,
                    ColliderConstructor::TrimeshFromMesh,
                    CollisionLayers::new(mesh_collider_data.layer.to_bits(), LayerMask::ALL),
                ));
            }
        }

        if let Some(robot) = object_data.components.robot {
            let Some(target_entity) = name_query
                .iter_many(children.iter_descendants(trigger.entity))
                .flatten()
                .find(|(_, name)| name.as_str() == robot.lidar.0)
                .map(|(entity, _)| entity)
            else {
                continue;
            };

            commands.entity(entity).insert(Robot::new(
                500,
                target_entity,
                robot.footprint_circle_radius,
            ));
        }
    }
}
