// crates/adminx/src/helpers/downloads/json_download.rs
use actix_web::{HttpRequest, HttpResponse};
use std::sync::Arc;
use tracing::{info};
use std::collections::HashSet;
use futures::TryStreamExt;
use crate::AdmixResource;
use chrono::Utc;
use crate::utils::constants::{
    DEFAULT_PAGE,
    DEFAULT_PER_PAGE,
};

pub async fn export_data_as_json(
    resource: &Arc<Box<dyn AdmixResource>>,
    req: &HttpRequest,
    _query_string: String,
) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
    let collection = resource.get_collection();
    
    // Parse query parameters for filters and pagination
    let query_params: std::collections::HashMap<String, String> = 
        serde_urlencoded::from_str(req.query_string()).unwrap_or_default();
    
    // Extract pagination parameters
    let page = query_params.get("page")
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(DEFAULT_PAGE);
    
    let per_page = query_params.get("per_page")
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(DEFAULT_PER_PAGE);
    
    let complete_export = query_params.get("complete")
        .map(|v| v == "true")
        .unwrap_or(false);
    
    // Build filter document from query parameters
    let mut filter_doc = mongodb::bson::doc! {};
    let permitted_fields: HashSet<&str> = resource.permit_keys().into_iter().collect();
    
    // Apply the same filters as the list view
    for (key, value) in &query_params {
        if !value.is_empty() && 
           (permitted_fields.contains(key.as_str()) || key == "search") && 
           !["download", "page", "per_page", "complete"].contains(&key.as_str()) {
            match key.as_str() {
                "name" | "email" | "username" | "key" | "title" | "description" | "search" => {
                    if key == "search" {
                        let search_fields = vec!["name", "email", "username", "key", "title", "description"];
                        let mut search_conditions = Vec::new();
                        
                        for field in search_fields {
                            if permitted_fields.contains(field) {
                                search_conditions.push(mongodb::bson::doc! {
                                    field: {
                                        "$regex": value,
                                        "$options": "i"
                                    }
                                });
                            }
                        }
                        
                        if !search_conditions.is_empty() {
                            filter_doc.insert("$or", search_conditions);
                        }
                    } else {
                        filter_doc.insert(key, mongodb::bson::doc! {
                            "$regex": value,
                            "$options": "i"
                        });
                    }
                }
                "status" | "data_type" | "deleted" | "active" | "enabled" => {
                    if value == "true" || value == "false" {
                        let bool_val = value == "true";
                        filter_doc.insert(key, bool_val);
                    } else {
                        filter_doc.insert(key, value);
                    }
                }
                _ => {
                    filter_doc.insert(key, value);
                }
            }
        }
    }
    
    info!("Exporting JSON with filters: {:?}", filter_doc);
    
    // Configure find options with conditional pagination
    let mut find_options = mongodb::options::FindOptions::default();
    find_options.sort = Some(mongodb::bson::doc! { "created_at": -1 });
    
    if complete_export {
        // Export all records (no pagination limits)
        info!("Exporting complete JSON dataset (all records)");
        // Don't set skip or limit - fetch everything
    } else {
        // Apply pagination for current page only
        let skip = (page - 1) * per_page;
        find_options.skip = Some(skip);
        find_options.limit = Some(per_page as i64);
        info!("Exporting JSON page {} ({} records per page)", page, per_page);
    }
    
    let mut cursor = collection.find(filter_doc, find_options).await
        .map_err(|e| format!("Database query failed: {}", e))?;
    
    let mut documents = Vec::new();
    while let Some(doc) = cursor.try_next().await.unwrap_or(None) {
        // Convert MongoDB document to JSON-friendly format
        let mut json_doc = serde_json::Map::new();
        
        // Handle MongoDB ObjectId
        if let Ok(oid) = doc.get_object_id("_id") {
            json_doc.insert("id".to_string(), serde_json::Value::String(oid.to_hex()));
        }
        
        // Convert all fields to JSON
        for field_name in resource.permit_keys() {
            if let Some(bson_val) = doc.get(field_name) {
                match bson_val {
                    mongodb::bson::Bson::String(s) => {
                        json_doc.insert(field_name.to_string(), serde_json::Value::String(s.clone()));
                    }
                    mongodb::bson::Bson::Boolean(b) => {
                        json_doc.insert(field_name.to_string(), serde_json::Value::Bool(*b));
                    }
                    mongodb::bson::Bson::Int32(i) => {
                        json_doc.insert(field_name.to_string(), serde_json::Value::Number(serde_json::Number::from(*i)));
                    }
                    mongodb::bson::Bson::Int64(i) => {
                        json_doc.insert(field_name.to_string(), serde_json::Value::Number(serde_json::Number::from(*i)));
                    }
                    mongodb::bson::Bson::Double(d) => {
                        if let Some(num) = serde_json::Number::from_f64(*d) {
                            json_doc.insert(field_name.to_string(), serde_json::Value::Number(num));
                        }
                    }
                    mongodb::bson::Bson::DateTime(dt) => {
                        let timestamp_ms = dt.timestamp_millis();
                        if let Some(datetime) = chrono::DateTime::from_timestamp_millis(timestamp_ms) {
                            json_doc.insert(field_name.to_string(), 
                                         serde_json::Value::String(datetime.to_rfc3339()));
                        }
                    }
                    mongodb::bson::Bson::Null => {
                        json_doc.insert(field_name.to_string(), serde_json::Value::Null);
                    }
                    _ => {
                        json_doc.insert(field_name.to_string(), serde_json::Value::String(format!("{:?}", bson_val)));
                    }
                }
            }
        }
        
        // Add standard timestamp fields
        if let Ok(created_at) = doc.get_datetime("created_at") {
            let timestamp_ms = created_at.timestamp_millis();
            if let Some(datetime) = chrono::DateTime::from_timestamp_millis(timestamp_ms) {
                json_doc.insert("created_at".to_string(), 
                             serde_json::Value::String(datetime.to_rfc3339()));
            }
        }
        
        if let Ok(updated_at) = doc.get_datetime("updated_at") {
            let timestamp_ms = updated_at.timestamp_millis();
            if let Some(datetime) = chrono::DateTime::from_timestamp_millis(timestamp_ms) {
                json_doc.insert("updated_at".to_string(), 
                             serde_json::Value::String(datetime.to_rfc3339()));
            }
        }
        
        documents.push(serde_json::Value::Object(json_doc));
    }
    
    // Enhanced JSON response with pagination info
    let json_data = if complete_export {
        serde_json::json!({
            "data": documents,
            "total": documents.len(),
            "exported_at": Utc::now().to_rfc3339(),
            "resource": resource.resource_name(),
            "export_type": "complete"
        })
    } else {
        serde_json::json!({
            "data": documents,
            "total": documents.len(),
            "exported_at": Utc::now().to_rfc3339(),
            "resource": resource.resource_name(),
            "export_type": "paginated",
            "page": page,
            "per_page": per_page
        })
    };
    
    let json_string = serde_json::to_string_pretty(&json_data)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    
    // Generate filename with pagination info
    let filename = if complete_export {
        format!("{}_{}_complete.json", 
                resource.resource_name(), 
                Utc::now().format("%Y%m%d_%H%M%S"))
    } else {
        format!("{}_page{}_{}.json", 
                resource.resource_name(),
                page,
                Utc::now().format("%Y%m%d_%H%M%S"))
    };
    
    if complete_export {
        info!("✅ Exported {} records as complete JSON", documents.len());
    } else {
        info!("✅ Exported {} records as JSON (page {})", documents.len(), page);
    }
    
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .append_header(("Content-Disposition", format!("attachment; filename=\"{}\"", filename)))
        .body(json_string))
}