use avian3d::prelude::*;
use bevy::{prelude::*, scene::SceneInstanceReady};
use schemars::{JsonSchema, r#gen::SchemaSettings};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize, PhysicsLayer, JsonSchema, Default, Debug)]
pub enum GameLayer {
    #[default]
    Default,
    RobotLayer,
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
struct BlenderComponentData {
    mesh_collider_fom_self: Option<BlenderMeshColliderData>,
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
