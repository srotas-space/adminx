// /crates/adminx/src/helpers/form_helper.rs
use convert_case::{Case, Casing};
use serde_json::{Value, json, Map as JsonMap};

/// Converts the dynamic form_structure JSON into a flat list of fields with name/label
pub fn extract_fields_for_form(form_structure: &JsonMap<String, Value>) -> Vec<Value> {
    let mut fields = vec![];
    if let Some(groups) = form_structure.get("groups").and_then(|g| g.as_array()) {
        for group in groups {
            if let Some(group_fields) = group.get("fields").and_then(|f| f.as_array()) {
                for field in group_fields {
                    // accept "field": "name" or { name: "...", ... }
                    if let Some(name) = field.as_str() {
                        fields.push(json!({
                            "name": name,
                            "label": name.to_case(Case::Title),
                        }));
                    } else if let Some(name) = field.get("name").and_then(|n| n.as_str()) {
                        fields.push(json!({
                            "name": name,
                            "label": name.to_case(Case::Title),
                        }));
                    }
                }
            }
        }
    }
    fields
}

/// Convert a form Value into a map
pub fn to_map(form: &Value) -> JsonMap<String, Value> {
    match form {
        Value::Object(map) => map.clone(),
        _ => JsonMap::new(),
    }
}