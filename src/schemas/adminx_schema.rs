// adminx/src/schemas/adminx_schema.rs
use schemars::gen::SchemaGenerator;
use schemars::{schema::RootSchema, JsonSchema};
use schemars::schema::{InstanceType, Schema};
use serde_json::Value;

/// Field struct used to render forms
#[derive(serde::Serialize)]
pub struct Field {
    pub name: String,
    pub label: String,
    pub value: String,
    pub field_type: String,                // e.g., "text", "select", "date"
    pub options: Option<Vec<String>>,     // for enums or dropdowns
}


/// Blanket trait to unify schema derivation across models.
pub trait AdminxSchema: JsonSchema {}
/// Implement it for all types that implement `JsonSchema`
impl<T: JsonSchema> AdminxSchema for T {}




/// Detect field type from schema definition
fn detect_field_type(schema: &Schema) -> (String, Option<Vec<String>>) {
    if let Schema::Object(obj) = schema {
        // Enum detection → dropdown
        if let Some(enum_values) = &obj.enum_values {
            let options = enum_values
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>();

            return ("select".to_string(), Some(options));
        }

        // Type detection and name pattern matching
        if let Some(instance_types) = &obj.instance_type {
            if instance_types.contains(&InstanceType::Integer) {
                return ("number".to_string(), None);
            }
            if instance_types.contains(&InstanceType::Boolean) {
                return ("checkbox".to_string(), None);
            }
            if instance_types.contains(&InstanceType::String) {
                // name-based heuristics
                if let Some(meta) = &obj.metadata {
                    if let Some(title) = &meta.title {
                        if title.ends_with("_at") || title.contains("date") {
                            return ("date".to_string(), None);
                        }
                    }
                }
            }
        }
    }

    ("text".to_string(), None)
}

/// Generate Field metadata from any model
pub fn generate_fields_from_model<T: JsonSchema>(record: Option<&Value>) -> Vec<Field> {
    let gen = SchemaGenerator::default();
    let schema: RootSchema = gen.into_root_schema_for::<T>();

    let obj_props = match &schema.schema.object {
        Some(obj) => &obj.properties,
        None => return vec![],
    };

    obj_props
        .iter()
        .map(|(name, schema)| {
            let label = name.replace('_', " ").to_uppercase();
            let value = record
                .and_then(|r| r.get(name))
                .map(stringify_json_value)
                .unwrap_or_default();

            let (field_type, options) = detect_field_type(schema);

            Field {
                name: name.clone(),
                label,
                value,
                field_type,
                options,
            }
        })
        .collect()
}

/// Generate default grouped form structure with full field metadata
pub fn form_structure_from_model<T: JsonSchema>() -> Option<Value> {
    let fields: Vec<Field> = generate_fields_from_model::<T>(None);
    if fields.is_empty() {
        return None;
    }

    Some(serde_json::json!({
        "title": "Create",
        "groups": [
            {
                "title": "Details",
                "fields": fields
            }
        ]
    }))
}


fn stringify_json_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => "".to_string(),
    }
}
