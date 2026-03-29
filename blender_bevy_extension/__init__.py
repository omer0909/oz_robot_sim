import json
import os

import bpy


@bpy.app.handlers.persistent
def on_blend_load(dummy):
    global schema, SchemaProps
    path = get_schema_path()
    if not path:
        return
    with open(path, "r", encoding="utf-8") as f:
        schema = json.load(f)
    # Props'u yeniden oluştur
    try:
        del bpy.types.Object.schema_props
        bpy.utils.unregister_class(SchemaProps)
    except Exception:
        pass
    components_schema = schema["properties"]["components"]["properties"]
    props = build_props(components_schema)
    for comp_name in components_schema.keys():
        props[f"enabled__{comp_name}"] = bpy.props.BoolProperty(
            name=comp_name,
            description=f"'{comp_name}' componentini aktif et",
            default=False,
            update=save_json,
        )
    SchemaProps = type(
        "SchemaProps", (bpy.types.PropertyGroup,), {"__annotations__": props}
    )
    bpy.utils.register_class(SchemaProps)
    bpy.types.Object.schema_props = bpy.props.PointerProperty(type=SchemaProps)


def get_schema_path():
    try:
        blend_path = bpy.data.filepath
    except AttributeError:
        return None  # --command modu

    if not blend_path:
        return None

    parent_dir = os.path.dirname(os.path.dirname(blend_path))
    schema_path = os.path.join(parent_dir, "schema.json")
    return schema_path if os.path.exists(schema_path) else None


schema = None
SchemaProps = None


def get_type(data):
    if not isinstance(data, dict):
        return str(data)
    if "type" not in data:
        return None
    if isinstance(data["type"], list):
        return next((t for t in data["type"] if t != "null"), None)
    return data["type"]


# ✅ EKSİK OLAN FONKSİYON — eklendi
def props_to_dict(schema_props, properties, prefix=""):
    result = {}
    for key, value in properties.items():
        data_type = get_type(value)
        prop_name = f"{prefix}{key}"
        if data_type == "object":
            result[key] = props_to_dict(
                schema_props, value.get("properties", {}), f"{prop_name}__"
            )
        else:
            result[key] = getattr(schema_props, prop_name, None)
    return result


def save_json(self, context):
    if schema is None:
        return
    obj = context.object
    if not obj or not hasattr(obj, "schema_props"):
        return
    schema_props = obj.schema_props
    components = schema["properties"]["components"]["properties"]
    output = {"components": {}}
    for comp_name, comp_data in components.items():
        enabled_key = f"enabled__{comp_name}"
        if getattr(schema_props, enabled_key, False):  # sadece aktifler
            output["components"][comp_name] = props_to_dict(
                schema_props, comp_data.get("properties", {}), f"{comp_name}__"
            )
    if output["components"]:
        obj["bevy_data"] = json.dumps(output, ensure_ascii=False)
    elif "bevy_data" in obj:
        del obj["bevy_data"]


def build_props(properties, prefix=""):
    annotations = {}
    for key, value in properties.items():
        data_type = get_type(value)
        prop_name = f"{prefix}{key}"
        if data_type == "object":
            annotations.update(
                build_props(value.get("properties", {}), f"{prop_name}__")
            )
        elif data_type == "number":
            annotations[prop_name] = bpy.props.FloatProperty(name=key, update=save_json)
        elif data_type == "integer":
            annotations[prop_name] = bpy.props.IntProperty(name=key, update=save_json)
        elif data_type == "boolean":
            annotations[prop_name] = bpy.props.BoolProperty(name=key, update=save_json)
        elif data_type == "string":
            if "enum" in value:
                items = [(e, e, "") for e in value["enum"]]
                annotations[prop_name] = bpy.props.EnumProperty(
                    name=key, items=items, update=save_json
                )
            else:
                annotations[prop_name] = bpy.props.StringProperty(
                    name=key, update=save_json
                )
    return annotations


class SCHEMA_PT_Panel(bpy.types.Panel):
    bl_label = "Components"
    bl_idname = "SCHEMA_PT_panel"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Bevy"

    def draw(self, context):
        if schema is None:
            self.layout.label(text="schema.json bulunamadı", icon="ERROR")
            return

        layout = self.layout
        obj = context.object
        if not obj:
            layout.label(text="Obje seçilmedi", icon="INFO")
            return

        schema_props = obj.schema_props  # ✅ scene → object
        components = schema["properties"]["components"]["properties"]

        for comp_name, comp_data in components.items():
            enabled_key = f"enabled__{comp_name}"
            is_enabled = getattr(schema_props, enabled_key, False)

            box = layout.box()
            row = box.row(align=True)
            row.prop(schema_props, enabled_key, text="")  # ✅ toggle checkbox
            row.label(
                text=comp_name, icon="CHECKMARK" if is_enabled else "RADIOBUT_OFF"
            )

            if is_enabled:  # sadece aktifse field'ları göster
                col = box.column(align=True)
                for child_key, child_value in comp_data.get("properties", {}).items():
                    self.draw_item(
                        col, child_key, child_value, schema_props, f"{comp_name}__"
                    )

    def draw_item(self, layout, key, data, schema_props, prefix):
        data_type = get_type(data)
        prop_name = f"{prefix}{key}"
        if data_type == "object":
            box = layout.box()
            box.label(text=key, icon="TRIA_RIGHT")
            for child_key, child_value in data.get("properties", {}).items():
                self.draw_item(
                    box, child_key, child_value, schema_props, f"{prop_name}__"
                )
        else:
            layout.prop(schema_props, prop_name)


def register():
    global schema, SchemaProps

    path = get_schema_path()
    if path:
        with open(path, "r", encoding="utf-8") as f:
            schema = json.load(f)

    components_schema = (
        schema["properties"]["components"]["properties"] if schema else {}
    )
    props = build_props(components_schema)

    for comp_name in components_schema.keys():
        props[f"enabled__{comp_name}"] = bpy.props.BoolProperty(
            name=comp_name,
            description=f"'{comp_name}' componentini aktif et",
            default=False,
            update=save_json,
        )

    SchemaProps = type(
        "SchemaProps", (bpy.types.PropertyGroup,), {"__annotations__": props}
    )
    bpy.app.handlers.load_post.append(on_blend_load)
    bpy.utils.register_class(SchemaProps)
    bpy.types.Object.schema_props = bpy.props.PointerProperty(type=SchemaProps)
    bpy.utils.register_class(SCHEMA_PT_Panel)


def unregister():
    bpy.app.handlers.load_post.remove(on_blend_load)
    bpy.utils.unregister_class(SCHEMA_PT_Panel)
    del bpy.types.Object.schema_props
    bpy.utils.unregister_class(SchemaProps)


if __name__ == "__main__":
    register()
