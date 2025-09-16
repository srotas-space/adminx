// crates/adminx/src/helpers/downloads/csv_download.rs
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

/// Export data as CSV with pagination support
pub async fn export_data_as_csv(
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
    
    // Build filter document (same logic as before)
    let mut filter_doc = mongodb::bson::doc! {};
    let permitted_fields: HashSet<&str> = resource.permit_keys().into_iter().collect();
    
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
    
    info!("Exporting CSV with filters: {:?}", filter_doc);
    
    // Configure find options with conditional pagination
    let mut find_options = mongodb::options::FindOptions::default();
    find_options.sort = Some(mongodb::bson::doc! { "created_at": -1 });
    
    if complete_export {
        // Export all records (no pagination limits)
        info!("Exporting complete CSV dataset (all records)");
        // Don't set skip or limit - fetch everything
    } else {
        // Apply pagination for current page only
        let skip = (page - 1) * per_page;
        find_options.skip = Some(skip);
        find_options.limit = Some(per_page as i64);
        info!("Exporting CSV page {} ({} records per page)", page, per_page);
    }
    
    let mut cursor = collection.find(filter_doc, find_options).await
        .map_err(|e| format!("Database query failed: {}", e))?;
    
    // Build CSV headers
    let mut headers = vec!["id".to_string()];
    for field in resource.permit_keys() {
        headers.push(field.to_string());
    }
    headers.push("created_at".to_string());
    headers.push("updated_at".to_string());
    
    // Start building CSV content
    let mut csv_content = headers.join(",") + "\n";
    
    let mut record_count = 0;
    while let Some(doc) = cursor.try_next().await.unwrap_or(None) {
        let mut row = Vec::new();
        
        // Add ID
        if let Ok(oid) = doc.get_object_id("_id") {
            row.push(escape_csv_field(&oid.to_hex()));
        } else {
            row.push("".to_string());
        }
        
        // Add permitted fields
        for field_name in resource.permit_keys() {
            let field_value = if let Some(bson_val) = doc.get(field_name) {
                match bson_val {
                    mongodb::bson::Bson::String(s) => escape_csv_field(s),
                    mongodb::bson::Bson::Boolean(b) => b.to_string(),
                    mongodb::bson::Bson::Int32(i) => i.to_string(),
                    mongodb::bson::Bson::Int64(i) => i.to_string(),
                    mongodb::bson::Bson::Double(d) => d.to_string(),
                    mongodb::bson::Bson::DateTime(dt) => {
                        let timestamp_ms = dt.timestamp_millis();
                        if let Some(datetime) = chrono::DateTime::from_timestamp_millis(timestamp_ms) {
                            escape_csv_field(&datetime.format("%Y-%m-%d %H:%M:%S").to_string())
                        } else {
                            "".to_string()
                        }
                    }
                    mongodb::bson::Bson::Null => "".to_string(),
                    _ => escape_csv_field(&format!("{:?}", bson_val)),
                }
            } else {
                "".to_string()
            };
            row.push(field_value);
        }
        
        // Add timestamps
        if let Ok(created_at) = doc.get_datetime("created_at") {
            let timestamp_ms = created_at.timestamp_millis();
            if let Some(datetime) = chrono::DateTime::from_timestamp_millis(timestamp_ms) {
                row.push(escape_csv_field(&datetime.format("%Y-%m-%d %H:%M:%S").to_string()));
            } else {
                row.push("".to_string());
            }
        } else {
            row.push("".to_string());
        }
        
        if let Ok(updated_at) = doc.get_datetime("updated_at") {
            let timestamp_ms = updated_at.timestamp_millis();
            if let Some(datetime) = chrono::DateTime::from_timestamp_millis(timestamp_ms) {
                row.push(escape_csv_field(&datetime.format("%Y-%m-%d %H:%M:%S").to_string()));
            } else {
                row.push("".to_string());
            }
        } else {
            row.push("".to_string());
        }
        
        csv_content.push_str(&(row.join(",") + "\n"));
        record_count += 1;
    }
    
    // Generate filename with pagination info
    let filename = if complete_export {
        format!("{}_{}_complete.csv", 
                resource.resource_name(), 
                Utc::now().format("%Y%m%d_%H%M%S"))
    } else {
        format!("{}_page{}_{}.csv", 
                resource.resource_name(),
                page,
                Utc::now().format("%Y%m%d_%H%M%S"))
    };
    
    if complete_export {
        info!("✅ Exported {} records as complete CSV", record_count);
    } else {
        info!("✅ Exported {} records as CSV (page {})", record_count, page);
    }
    
    Ok(HttpResponse::Ok()
        .content_type("text/csv")
        .append_header(("Content-Disposition", format!("attachment; filename=\"{}\"", filename)))
        .body(csv_content))
}

/// Helper function to properly escape CSV fields
fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}