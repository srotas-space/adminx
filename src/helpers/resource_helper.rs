// crates/adminx/src/helpers/resource_helper.rs - Complete Fixed Version
use actix_web::{web, HttpRequest, HttpResponse, Scope};
use serde_json::Value;
use std::sync::Arc;
use tera::Context;
use tracing::{info, warn, error};
use std::collections::HashSet;
use actix_session::Session;
use futures::TryStreamExt;

use crate::AdmixResource;
use crate::configs::initializer::AdminxConfig;
use crate::utils::auth::extract_claims_from_session;
use crate::utils::structs::Claims;
use crate::registry::get_registered_menus;

/// Check authentication and return user claims or redirect response
pub async fn check_authentication(
    session: &Session,
    config: &AdminxConfig,
    resource_name: &str,
    action: &str,
) -> Result<Claims, HttpResponse> {
    match extract_claims_from_session(session, config).await {
        Ok(claims) => {
            info!("🔐 Authenticated user {} accessing {} action on resource {}", 
                  claims.email, action, resource_name);
            Ok(claims)
        }
        Err(_) => {
            warn!("⚠️  Unauthenticated access attempt to {} action on resource {}", action, resource_name);
            Err(HttpResponse::Found()
                .append_header(("Location", "/adminx/login"))
                .finish())
        }
    }
}

/// Check if user has permission for resource action - Enhanced version
pub async fn check_resource_permission(
    session: &Session,
    config: &AdminxConfig,
    resource: &dyn AdmixResource,
    action: &str,
) -> Result<Claims, HttpResponse> {
    match extract_claims_from_session(session, config).await {
        Ok(claims) => {
            let user_roles: HashSet<String> = {
                let mut roles = claims.roles.clone();
                roles.push(claims.role.clone());
                roles.into_iter().collect()
            };
            
            let allowed_roles: HashSet<String> = 
                resource.allowed_roles().into_iter().collect();
            
            if user_roles.intersection(&allowed_roles).next().is_some() {
                info!("User {} has permission for {} action on resource {}", 
                      claims.email, action, resource.resource_name());
                Ok(claims)
            } else {
                warn!("User {} lacks permission for {} action on resource {} (user roles: {:?}, required: {:?})", 
                      claims.email, action, resource.resource_name(), claims.roles, resource.allowed_roles());
                Err(HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "Insufficient permissions",
                    "required_roles": resource.allowed_roles(),
                    "user_roles": claims.roles,
                    "action": action,
                    "resource": resource.resource_name()
                })))
            }
        }
        Err(_) => {
            Err(HttpResponse::Found()
                .append_header(("Location", "/adminx/login"))
                .finish())
        }
    }
}

/// Create template context for UI routes with common data
pub fn create_base_template_context(
    resource_name: &str,
    base_path: &str,
    claims: &Claims,
) -> Context {
    let mut ctx = Context::new();
    ctx.insert("resource_name", resource_name);
    ctx.insert("base_path", &format!("/adminx/{}", base_path));
    ctx.insert("menus", &get_registered_menus());
    ctx.insert("current_user", claims);
    ctx.insert("is_authenticated", &true);
    ctx
}


pub fn handle_delete_response(
    response: HttpResponse,
    base_path: &str,
    resource_name: &str,
) -> HttpResponse {
    if response.status().is_success() {
        info!("✅ Resource '{}' item deleted successfully, redirecting to list", resource_name);
        let location = format!("/adminx/{}/list?success=deleted", base_path);
        HttpResponse::Found()
            .append_header(("Location", location))
            .finish()
    } else {
        error!("❌ Resource '{}' item deletion failed with status: {}", resource_name, response.status());
        let location = format!("/adminx/{}/list?error=delete_failed", base_path);
        HttpResponse::Found()
            .append_header(("Location", location))
            .finish()
    }
}

/// Handle form data conversion from HTML form to JSON - Enhanced version
pub fn convert_form_data_to_json(
    form_data: std::collections::HashMap<String, String>
) -> Value {
    let mut json_data = serde_json::Map::new();
    
    for (key, value) in form_data {
        // Skip editor mode fields (they're just for UI state)
        if key.ends_with("_mode") {
            continue;
        }
        
        if !value.is_empty() {
            // Handle boolean fields
            if key == "deleted" || key == "active" || key == "enabled" || key.ends_with("_flag") {
                match value.as_str() {
                    "true" | "1" | "on" => {
                        json_data.insert(key, serde_json::Value::Bool(true));
                    }
                    "false" | "0" | "off" => {
                        json_data.insert(key, serde_json::Value::Bool(false));
                    }
                    _ => {
                        // If it's not a clear boolean, treat as string
                        json_data.insert(key, serde_json::Value::String(value));
                    }
                }
            }
            // Handle numeric fields
            else if key.ends_with("_id") || key.ends_with("_count") || key.ends_with("_number") {
                if let Ok(num) = value.parse::<i64>() {
                    json_data.insert(key, serde_json::Value::Number(serde_json::Number::from(num)));
                } else if let Ok(num) = value.parse::<f64>() {
                    if let Some(num_val) = serde_json::Number::from_f64(num) {
                        json_data.insert(key, serde_json::Value::Number(num_val));
                    } else {
                        json_data.insert(key, serde_json::Value::String(value));
                    }
                } else {
                    json_data.insert(key, serde_json::Value::String(value));
                }
            }
            // Handle JSON fields (try to parse as JSON first)
            else if key == "data" || key.ends_with("_json") || key.ends_with("_config") {
                // Try to parse as JSON first to validate, but store as string
                match serde_json::from_str::<serde_json::Value>(&value) {
                    Ok(_) => {
                        // If it parsed successfully, store as string (most APIs expect JSON fields as strings)
                        json_data.insert(key, serde_json::Value::String(value));
                    }
                    Err(_) => {
                        // If it's not valid JSON, store as-is
                        json_data.insert(key, serde_json::Value::String(value));
                    }
                }
            }
            // Default: treat as string
            else {
                json_data.insert(key, serde_json::Value::String(value));
            }
        }
    }
    
    serde_json::Value::Object(json_data)
}


/*-------------------------------------------------------------------------
/// START Handle resource creation response and return appropriate redirect
--------------------------------------------------------------------------*/
pub fn handle_create_response(
    response: HttpResponse,
    base_path: &str,
    resource_name: &str,
) -> HttpResponse {
    if response.status().is_success() {
        info!("✅ Resource '{}' created successfully, redirecting to list", resource_name);
        let location = format!("/adminx/{}/list?success=created", base_path);
        HttpResponse::Found()
            .append_header(("Location", location))
            .finish()
    } else {
        error!("❌ Resource '{}' creation failed with status: {}", resource_name, response.status());
        let location = format!("/adminx/{}/new?error=create_failed", base_path);
        HttpResponse::Found()
            .append_header(("Location", location))
            .finish()
    }
}

pub fn handle_update_response(
    response: HttpResponse,
    base_path: &str,
    item_id: &str,
    resource_name: &str,
) -> HttpResponse {
    if response.status().is_success() {
        info!("✅ Resource '{}' item '{}' updated successfully, redirecting to view", resource_name, item_id);
        let location = format!("/adminx/{}/view/{}?success=updated", base_path, item_id);
        HttpResponse::Found()
            .append_header(("Location", location))
            .finish()
    } else {
        error!("❌ Resource '{}' item '{}' update failed with status: {}", resource_name, item_id, response.status());
        let location = format!("/adminx/{}/edit/{}?error=update_failed", base_path, item_id);
        HttpResponse::Found()
            .append_header(("Location", location))
            .finish()
    }
}
/*-------------------------------------------------------------------------
/// END Handle resource creation response and return appropriate redirect
--------------------------------------------------------------------------*/



/// Get default list structure for resources that don't define one
pub fn get_default_list_structure() -> Value {
    serde_json::json!({
        "columns": [
            {
                "field": "id",
                "label": "ID",
                "sortable": false
            },
            {
                "field": "created_at",
                "label": "Created At",
                "type": "datetime",
                "sortable": true
            }
        ],
        "actions": ["view", "edit", "delete"]
    })
}

/// Fetch list data - Generic version that works with any resource
pub async fn fetch_list_data(
    resource: &Arc<Box<dyn AdmixResource>>,
    req: &HttpRequest,
    _query_string: String,
) -> Result<(Vec<String>, Vec<serde_json::Map<String, Value>>, Value), Box<dyn std::error::Error + Send + Sync>> {
    let collection = resource.get_collection();
    
    // Parse query parameters for pagination and filters
    let query_params: std::collections::HashMap<String, String> = 
        serde_urlencoded::from_str(req.query_string()).unwrap_or_default();
    
    let page: u64 = query_params.get("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1);
    let per_page: u64 = query_params.get("per_page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(10);
    
    let skip = (page - 1) * per_page;
    
    // Build filter document from query parameters
    let mut filter_doc = mongodb::bson::doc! {};
    
    // Get permitted query fields for security
    let permitted_fields: HashSet<&str> = resource.permit_keys().into_iter().collect();
    
    // Build filters based on query parameters
    for (key, value) in &query_params {
        if !value.is_empty() && (permitted_fields.contains(key.as_str()) || key == "search") {
            match key.as_str() {
                // Text fields that should use regex search
                "name" | "email" | "username" | "key" | "title" | "description" | "search" => {
                    if key == "search" {
                        // Global search across multiple fields
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
                // Exact match fields
                "status" | "data_type" | "deleted" | "active" | "enabled" => {
                    // Handle boolean fields properly
                    if value == "true" || value == "false" {
                        let bool_val = value == "true";
                        filter_doc.insert(key, bool_val);
                    } else {
                        filter_doc.insert(key, value);
                    }
                }
                // Date range filters
                key if key.ends_with("_from") => {
                    let base_field = key.trim_end_matches("_from");
                    if permitted_fields.contains(base_field) {
                        if let Ok(date) = chrono::DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", value)) {
                            let existing_filter = filter_doc.get_mut(base_field);
                            match existing_filter {
                                Some(mongodb::bson::Bson::Document(ref mut doc)) => {
                                    doc.insert("$gte", mongodb::bson::DateTime::from_chrono(date.with_timezone(&chrono::Utc)));
                                }
                                _ => {
                                    filter_doc.insert(base_field, mongodb::bson::doc! {
                                        "$gte": mongodb::bson::DateTime::from_chrono(date.with_timezone(&chrono::Utc))
                                    });
                                }
                            }
                        }
                    }
                }
                key if key.ends_with("_to") => {
                    let base_field = key.trim_end_matches("_to");
                    if permitted_fields.contains(base_field) {
                        if let Ok(date) = chrono::DateTime::parse_from_rfc3339(&format!("{}T23:59:59Z", value)) {
                            let existing_filter = filter_doc.get_mut(base_field);
                            match existing_filter {
                                Some(mongodb::bson::Bson::Document(ref mut doc)) => {
                                    doc.insert("$lte", mongodb::bson::DateTime::from_chrono(date.with_timezone(&chrono::Utc)));
                                }
                                _ => {
                                    filter_doc.insert(base_field, mongodb::bson::doc! {
                                        "$lte": mongodb::bson::DateTime::from_chrono(date.with_timezone(&chrono::Utc))
                                    });
                                }
                            }
                        }
                    }
                }
                // Number range filters
                key if key.ends_with("_min") => {
                    let base_field = key.trim_end_matches("_min");
                    if permitted_fields.contains(base_field) {
                        if let Ok(num) = value.parse::<f64>() {
                            let existing_filter = filter_doc.get_mut(base_field);
                            match existing_filter {
                                Some(mongodb::bson::Bson::Document(ref mut doc)) => {
                                    doc.insert("$gte", num);
                                }
                                _ => {
                                    filter_doc.insert(base_field, mongodb::bson::doc! {
                                        "$gte": num
                                    });
                                }
                            }
                        }
                    }
                }
                key if key.ends_with("_max") => {
                    let base_field = key.trim_end_matches("_max");
                    if permitted_fields.contains(base_field) {
                        if let Ok(num) = value.parse::<f64>() {
                            let existing_filter = filter_doc.get_mut(base_field);
                            match existing_filter {
                                Some(mongodb::bson::Bson::Document(ref mut doc)) => {
                                    doc.insert("$lte", num);
                                }
                                _ => {
                                    filter_doc.insert(base_field, mongodb::bson::doc! {
                                        "$lte": num
                                    });
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Default: exact match for other fields
                    filter_doc.insert(key, value);
                }
            }
        }
    }
    
    info!("Applied filters: {:?}", filter_doc);
    
    // Get total count with filters
    let total = collection.count_documents(filter_doc.clone(), None).await
        .unwrap_or(0);
    
    // Fetch documents with pagination and filters
    let mut find_options = mongodb::options::FindOptions::default();
    find_options.skip = Some(skip);
    find_options.limit = Some(per_page as i64);
    find_options.sort = Some(mongodb::bson::doc! { "created_at": -1 });
    
    let mut cursor = collection.find(filter_doc, find_options).await
        .map_err(|e| format!("Database query failed: {}", e))?;
    
    let mut documents = Vec::new();
    while let Some(doc) = cursor.try_next().await.unwrap_or(None) {
        documents.push(doc);
    }
    
    // Get column structure from resource's list_structure or use defaults
    let list_structure = resource.list_structure().unwrap_or_else(|| get_default_list_structure());
    let columns = list_structure.get("columns")
        .and_then(|c| c.as_array())
        .map(|cols| {
            cols.iter()
                .filter_map(|col| col.get("field").and_then(|f| f.as_str()))
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        })
        .unwrap_or_else(|| {
            // Default columns based on permitted fields
            let mut default_cols = vec!["id".to_string()];
            let permitted = resource.permit_keys();
            for field in permitted {
                if field != "_id" && field != "created_at" && field != "updated_at" {
                    default_cols.push(field.to_string());
                }
            }
            default_cols.push("created_at".to_string());
            default_cols
        });
    
    // Convert MongoDB documents to the format expected by the template
    let rows: Vec<serde_json::Map<String, Value>> = documents
        .into_iter()
        .map(|doc| {
            let mut row = serde_json::Map::new();
            
            // Handle MongoDB ObjectId
            if let Ok(oid) = doc.get_object_id("_id") {
                row.insert("id".to_string(), Value::String(oid.to_hex()));
            }
            
            // Extract fields based on permitted fields and columns
            for field_name in &columns {
                if field_name == "id" {
                    continue; // Already handled above
                }
                
                // Try different data types for each field
                if let Ok(string_val) = doc.get_str(field_name) {
                    row.insert(field_name.clone(), Value::String(string_val.to_string()));
                } else if let Ok(bool_val) = doc.get_bool(field_name) {
                    row.insert(field_name.clone(), Value::String(bool_val.to_string()));
                } else if let Ok(int_val) = doc.get_i32(field_name) {
                    row.insert(field_name.clone(), Value::String(int_val.to_string()));
                } else if let Ok(int64_val) = doc.get_i64(field_name) {
                    row.insert(field_name.clone(), Value::String(int64_val.to_string()));
                } else if let Ok(datetime_val) = doc.get_datetime(field_name) {
                    let timestamp_ms = datetime_val.timestamp_millis();
                    if let Some(datetime) = chrono::DateTime::from_timestamp_millis(timestamp_ms) {
                        row.insert(field_name.clone(), 
                                 Value::String(datetime.format("%Y-%m-%d %H:%M:%S").to_string()));
                    } else {
                        row.insert(field_name.clone(), Value::String("N/A".to_string()));
                    }
                } else if doc.contains_key(field_name) {
                    // Handle other BSON types
                    if let Some(bson_val) = doc.get(field_name) {
                        match bson_val {
                            mongodb::bson::Bson::String(s) => {
                                row.insert(field_name.clone(), Value::String(s.clone()));
                            }
                            mongodb::bson::Bson::Boolean(b) => {
                                row.insert(field_name.clone(), Value::String(b.to_string()));
                            }
                            mongodb::bson::Bson::Int32(i) => {
                                row.insert(field_name.clone(), Value::String(i.to_string()));
                            }
                            mongodb::bson::Bson::Int64(i) => {
                                row.insert(field_name.clone(), Value::String(i.to_string()));
                            }
                            mongodb::bson::Bson::Double(d) => {
                                row.insert(field_name.clone(), Value::String(d.to_string()));
                            }
                            mongodb::bson::Bson::Null => {
                                row.insert(field_name.clone(), Value::String("".to_string()));
                            }
                            _ => {
                                row.insert(field_name.clone(), Value::String(format!("{:?}", bson_val)));
                            }
                        }
                    }
                } else {
                    // Field doesn't exist in document
                    row.insert(field_name.clone(), Value::String("N/A".to_string()));
                }
            }
            
            row
        })
        .collect();
    
    let total_pages = if per_page > 0 { (total + per_page - 1) / per_page } else { 1 };
    
    // Build pagination with current filters
    let mut filter_params = Vec::new();
    for (key, value) in &query_params {
        if key != "page" && !value.is_empty() {
            filter_params.push(format!("{}={}", key, urlencoding::encode(value)));
        }
    }
    let filter_string = if filter_params.is_empty() {
        String::new()
    } else {
        format!("&{}", filter_params.join("&"))
    };
    
    let pagination = serde_json::json!({
        "current": page,
        "total": total_pages,
        "prev": if page > 1 { Some(page - 1) } else { None },
        "next": if page < total_pages { Some(page + 1) } else { None },
        "filter_params": filter_string
    });
    
    info!("Fetched {} items for list view (page {} of {}) with filters", rows.len(), page, total_pages);
    Ok((columns, rows, pagination))
}

/// Get filters data and current filter values for the template
pub fn get_filters_data(
    resource: &Arc<Box<dyn AdmixResource>>,
    query_params: &std::collections::HashMap<String, String>
) -> (Option<Value>, serde_json::Map<String, Value>) {
    let filters = resource.filters();
    let mut current_filters = serde_json::Map::new();
    
    // Extract current filter values from query parameters
    if let Some(filter_config) = &filters {
        if let Some(filter_array) = filter_config.get("filters").and_then(|f| f.as_array()) {
            for filter in filter_array {
                if let Some(field) = filter.get("field").and_then(|f| f.as_str()) {
                    if let Some(value) = query_params.get(field) {
                        if !value.is_empty() {
                            current_filters.insert(field.to_string(), Value::String(value.clone()));
                        }
                    }
                    
                    // Handle range filters (date_range, number_range)
                    let from_key = format!("{}_from", field);
                    let to_key = format!("{}_to", field);
                    let min_key = format!("{}_min", field);
                    let max_key = format!("{}_max", field);
                    
                    if let Some(from_value) = query_params.get(&from_key) {
                        if !from_value.is_empty() {
                            current_filters.insert(from_key, Value::String(from_value.clone()));
                        }
                    }
                    
                    if let Some(to_value) = query_params.get(&to_key) {
                        if !to_value.is_empty() {
                            current_filters.insert(to_key, Value::String(to_value.clone()));
                        }
                    }
                    
                    if let Some(min_value) = query_params.get(&min_key) {
                        if !min_value.is_empty() {
                            current_filters.insert(min_key, Value::String(min_value.clone()));
                        }
                    }
                    
                    if let Some(max_value) = query_params.get(&max_key) {
                        if !max_value.is_empty() {
                            current_filters.insert(max_key, Value::String(max_value.clone()));
                        }
                    }
                }
            }
        }
    }
    
    // Also handle global search
    if let Some(search_value) = query_params.get("search") {
        if !search_value.is_empty() {
            current_filters.insert("search".to_string(), Value::String(search_value.clone()));
        }
    }
    
    (filters, current_filters)
}

/// Fetch single item data for view/edit pages - Generic version that works with any resource
pub async fn fetch_single_item_data(
    resource: &Arc<Box<dyn AdmixResource>>,
    _req: &HttpRequest,
    id: &str,
) -> Result<serde_json::Map<String, Value>, Box<dyn std::error::Error + Send + Sync>> {
    let collection = resource.get_collection();
    
    // Parse ObjectId
    let oid = mongodb::bson::oid::ObjectId::parse_str(id)
        .map_err(|e| format!("Invalid ObjectId: {}", e))?;
    
    // Find the document
    let doc = collection.find_one(mongodb::bson::doc! { "_id": oid }, None).await
        .map_err(|e| format!("Database query failed: {}", e))?
        .ok_or("Document not found")?;
    
    // Convert to template-friendly format
    let mut record = serde_json::Map::new();
    
    // Handle MongoDB ObjectId first
    if let Ok(oid) = doc.get_object_id("_id") {
        record.insert("id".to_string(), Value::String(oid.to_hex()));
    }
    
    // Get all permitted fields from the resource and extract them from the document
    let permitted_fields = resource.permit_keys();
    
    for field_name in permitted_fields {
        // Try different data types for each field
        if let Ok(string_val) = doc.get_str(field_name) {
            record.insert(field_name.to_string(), Value::String(string_val.to_string()));
        } else if let Ok(bool_val) = doc.get_bool(field_name) {
            record.insert(field_name.to_string(), Value::String(bool_val.to_string()));
        } else if let Ok(int_val) = doc.get_i32(field_name) {
            record.insert(field_name.to_string(), Value::String(int_val.to_string()));
        } else if let Ok(int64_val) = doc.get_i64(field_name) {
            record.insert(field_name.to_string(), Value::String(int64_val.to_string()));
        } else if let Ok(float_val) = doc.get_f64(field_name) {
            record.insert(field_name.to_string(), Value::String(float_val.to_string()));
        } else if let Ok(datetime_val) = doc.get_datetime(field_name) {
            let timestamp_ms = datetime_val.timestamp_millis();
            if let Some(datetime) = chrono::DateTime::from_timestamp_millis(timestamp_ms) {
                // For date/datetime fields, format them appropriately
                if field_name.contains("date") || field_name.contains("time") || field_name == "created_at" || field_name == "updated_at" {
                    record.insert(field_name.to_string(), 
                                Value::String(datetime.format("%Y-%m-%d %H:%M:%S").to_string()));
                } else {
                    record.insert(field_name.to_string(), 
                                Value::String(datetime.to_rfc3339()));
                }
            } else {
                record.insert(field_name.to_string(), Value::String("N/A".to_string()));
            }
        }
        // If field exists but we can't parse it, try to get it as a generic BSON value
        else if doc.contains_key(field_name) {
            if let Some(bson_val) = doc.get(field_name) {
                match bson_val {
                    mongodb::bson::Bson::String(s) => {
                        record.insert(field_name.to_string(), Value::String(s.clone()));
                    }
                    mongodb::bson::Bson::Boolean(b) => {
                        record.insert(field_name.to_string(), Value::String(b.to_string()));
                    }
                    mongodb::bson::Bson::Int32(i) => {
                        record.insert(field_name.to_string(), Value::String(i.to_string()));
                    }
                    mongodb::bson::Bson::Int64(i) => {
                        record.insert(field_name.to_string(), Value::String(i.to_string()));
                    }
                    mongodb::bson::Bson::Double(d) => {
                        record.insert(field_name.to_string(), Value::String(d.to_string()));
                    }
                    mongodb::bson::Bson::Null => {
                        record.insert(field_name.to_string(), Value::String("".to_string()));
                    }
                    _ => {
                        // For complex types, convert to string representation
                        record.insert(field_name.to_string(), Value::String(format!("{:?}", bson_val)));
                    }
                }
            }
        }
    }
    
    // Always handle standard timestamp fields even if not in permit_keys
    if !record.contains_key("created_at") {
        if let Ok(created_at) = doc.get_datetime("created_at") {
            let timestamp_ms = created_at.timestamp_millis();
            if let Some(datetime) = chrono::DateTime::from_timestamp_millis(timestamp_ms) {
                record.insert("created_at".to_string(), 
                             Value::String(datetime.format("%Y-%m-%d %H:%M:%S").to_string()));
            }
        }
    }
    
    if !record.contains_key("updated_at") {
        if let Ok(updated_at) = doc.get_datetime("updated_at") {
            let timestamp_ms = updated_at.timestamp_millis();
            if let Some(datetime) = chrono::DateTime::from_timestamp_millis(timestamp_ms) {
                record.insert("updated_at".to_string(), 
                             Value::String(datetime.format("%Y-%m-%d %H:%M:%S").to_string()));
            }
        }
    }
    
    info!("Fetched single item with id: {} for resource: {} with fields: {:?}", 
          id, resource.resource_name(), record.keys().collect::<Vec<_>>());
    Ok(record)
}

pub fn get_default_form_structure() -> Value {
    serde_json::json!({
        "groups": [
            {
                "title": "Details",
                "fields": []
            }
        ]
    })
}

/// Get default view structure for resources that don't define one
pub fn get_default_view_structure() -> Value {
    serde_json::json!({
        "sections": [
            {
                "title": "Details",
                "fields": []
            }
        ]
    })
}

/// Register API-only routes without UI components
pub fn register_api_only_routes(resource: Box<dyn AdmixResource>) -> Scope {
    let resource_name = resource.resource_name().to_string();
    info!("Registering API-only routes for resource: {}", resource_name);
    
    let mut scope = web::scope("");

    // GET / - List all items
    let list_resource = resource.clone_box();
    scope = scope.route("", web::get().to(move |req: HttpRequest, query: web::Query<String>| {
        let resource = list_resource.clone_box();
        async move { 
            info!("📡 List API endpoint called for resource: {}", resource.resource_name());
            resource.list(&req, query.into_inner()).await 
        }
    }));

    // POST / - Create new item
    let create_resource = resource.clone_box();
    scope = scope.route("", web::post().to(move |req: HttpRequest, body: web::Json<Value>| {
        let resource = create_resource.clone_box();
        async move { 
            info!("📡 Create API endpoint called for resource: {}", resource.resource_name());
            resource.create(&req, body.into_inner()).await 
        }
    }));

    // GET /{id} - Get single item
    let get_resource = resource.clone_box();
    scope = scope.route("/{id}", web::get().to(move |req: HttpRequest, path: web::Path<String>| {
        let resource = get_resource.clone_box();
        async move { 
            let id = path.into_inner();
            info!("📡 Get API endpoint called for resource: {} with id: {}", resource.resource_name(), id);
            resource.get(&req, id).await 
        }
    }));

    // PUT /{id} - Update item
    let update_resource = resource.clone_box();
    scope = scope.route("/{id}", web::put().to(move |req: HttpRequest, path: web::Path<String>, body: web::Json<Value>| {
        let resource = update_resource.clone_box();
        async move { 
            let id = path.into_inner();
            info!("📡 Update API endpoint called for resource: {} with id: {}", resource.resource_name(), id);
            resource.update(&req, id, body.into_inner()).await 
        }
    }));

    // DELETE /{id} - Delete item
    let delete_resource = resource.clone_box();
    scope = scope.route("/{id}", web::delete().to(move |req: HttpRequest, path: web::Path<String>| {
        let resource = delete_resource.clone_box();
        async move { 
            let id = path.into_inner();
            info!("📡 Delete API endpoint called for resource: {} with id: {}", resource.resource_name(), id);
            resource.delete(&req, id).await 
        }
    }));

    // Add custom actions
    for action in resource.custom_actions() {
        let path = format!("/{{id}}/{}", action.name);
        info!("Adding custom action: {} {} for resource: {}", action.method, path, resource_name);
        
        match action.method {
            "POST" => {
                scope = scope.route(&path, web::post().to(action.handler));
            }
            "GET" => {
                scope = scope.route(&path, web::get().to(action.handler));
            }
            "PUT" => {
                scope = scope.route(&path, web::put().to(action.handler));
            }
            "DELETE" => {
                scope = scope.route(&path, web::delete().to(action.handler));
            }
            "PATCH" => {
                scope = scope.route(&path, web::patch().to(action.handler));
            }
            method => {
                error!("Unsupported HTTP method: {} for action: {} in resource: {}", method, action.name, resource_name);
            }
        }
    }

    scope
}

/// Register protected routes with role-based access control
pub fn register_protected_resource_routes(resource: Box<dyn AdmixResource>) -> Scope {
    let resource_name = resource.resource_name().to_string();
    let allowed_roles = resource.allowed_roles();
    
    info!("Registering protected routes for resource: {} with roles: {:?}", resource_name, allowed_roles);
    
    let mut scope = web::scope("");

    // GET / - List with role check
    let list_resource = resource.clone_box();
    scope = scope.route(
        "",
        web::get().to(move |req: HttpRequest, query: web::Query<String>, session: Session, config: web::Data<AdminxConfig>| {
            let resource = list_resource.clone_box();
            async move {
                match check_resource_permission(&session, &config, resource.as_ref(), "list").await {
                    Ok(_claims) => resource.list(&req, query.into_inner()).await,
                    Err(response) => response,
                }
            }
        }),
    );

    // POST / - Create with role check
    let create_resource = resource.clone_box();
    scope = scope.route(
        "",
        web::post().to(move |req: HttpRequest, body: web::Json<Value>, session: Session, config: web::Data<AdminxConfig>| {
            let resource = create_resource.clone_box();
            async move {
                match check_resource_permission(&session, &config, resource.as_ref(), "create").await {
                    Ok(_claims) => resource.create(&req, body.into_inner()).await,
                    Err(response) => response,
                }
            }
        }),
    );

    // GET /{id} - Get with role check
    let get_resource = resource.clone_box();
    scope = scope.route(
        "/{id}",
        web::get().to(move |req: HttpRequest, path: web::Path<String>, session: Session, config: web::Data<AdminxConfig>| {
            let resource = get_resource.clone_box();
            async move {
                let id = path.into_inner();
                match check_resource_permission(&session, &config, resource.as_ref(), "view").await {
                    Ok(_claims) => resource.get(&req, id).await,
                    Err(response) => response,
                }
            }
        }),
    );

    // PUT /{id} - Update with role check
    let update_resource = resource.clone_box();
    scope = scope.route(
        "/{id}",
        web::put().to(move |req: HttpRequest, path: web::Path<String>, body: web::Json<Value>, session: Session, config: web::Data<AdminxConfig>| {
            let resource = update_resource.clone_box();
            async move {
                let id = path.into_inner();
                match check_resource_permission(&session, &config, resource.as_ref(), "update").await {
                    Ok(_claims) => resource.update(&req, id, body.into_inner()).await,
                    Err(response) => response,
                }
            }
        }),
    );

    // DELETE /{id} - Delete with role check
    let delete_resource = resource.clone_box();
    scope = scope.route(
        "/{id}",
        web::delete().to(move |req: HttpRequest, path: web::Path<String>, session: Session, config: web::Data<AdminxConfig>| {
            let resource = delete_resource.clone_box();
            async move {
                let id = path.into_inner();
                match check_resource_permission(&session, &config, resource.as_ref(), "delete").await {
                    Ok(_claims) => resource.delete(&req, id).await,
                    Err(response) => response,
                }
            }
        }),
    );

    scope
}