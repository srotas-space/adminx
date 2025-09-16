// crates/adminx/src/controllers/resource_controller.rs
use actix_web::{web, HttpRequest, HttpResponse, Scope};
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, warn, error};
use actix_session::Session;
use actix_multipart::Multipart;
use futures::TryStreamExt;
use std::collections::HashMap;

use crate::configs::initializer::AdminxConfig;
use crate::AdmixResource;
use crate::helpers::{
    form_helper::{
        extract_fields_for_form,
        to_map,
    },
    template_helper::{
        render_template,
    },
    resource_helper::{
        check_authentication,
        create_base_template_context,
        convert_form_data_to_json,
        handle_create_response,
        handle_update_response,
        handle_delete_response,
        get_default_form_structure,
        get_default_view_structure,
        fetch_list_data,
        fetch_single_item_data,
    }
};

/// Register all UI + API routes for a resource
pub fn register_admix_resource_routes(resource: Box<dyn AdmixResource>) -> Scope {
    let base_path = resource.base_path().to_string();
    let resource_name = resource.resource_name().to_string();
    
    info!("Registering routes for resource: {} at path: {}", resource_name, base_path);
    
    let mut scope = web::scope("");

    // ========================
    // UI Routes (HTML pages) - REGISTER THESE FIRST!
    // ========================

    let resource_arc = Arc::new(resource.clone_box());
    let ui_resource_name = resource_arc.resource_name().to_string();
    let ui_base_path = resource_arc.base_path().to_string();

    // GET /list - List view with download support
    scope = scope.route("/list", web::get().to({
        let resource = Arc::clone(&resource_arc);
        let resource_name = ui_resource_name.clone();
        move |req: HttpRequest, session: Session, config: web::Data<AdminxConfig>| {
            let query_string = req.query_string().to_string();
            let resource = Arc::clone(&resource);
            let resource_name = resource_name.clone();
            async move {
                match check_authentication(&session, &config, &resource_name, "list").await {
                    Ok(claims) => {
                        // Parse query parameters directly from the request
                        let query_params: std::collections::HashMap<String, String> = 
                            serde_urlencoded::from_str(&query_string).unwrap_or_default();
                        
                        // CHECK FOR DOWNLOAD REQUESTS FIRST
                        if let Some(download_format) = query_params.get("download") {
                            info!("📥 Download request for {} in format: {} by user: {}", 
                                  resource_name, download_format, claims.email);
                            
                            match download_format.as_str() {
                                "json" => {
                                    match crate::helpers::downloads::json_download::export_data_as_json(&resource, &req, query_string).await {
                                        Ok(response) => {
                                            info!("✅ JSON export successful for {} by {}", resource_name, claims.email);
                                            return response;
                                        }
                                        Err(e) => {
                                            error!("❌ Failed to export JSON for {}: {}", resource_name, e);
                                            return HttpResponse::InternalServerError()
                                                .content_type("text/plain")
                                                .body(format!("Failed to export JSON data: {}", e));
                                        }
                                    }
                                }
                                "csv" => {
                                    match crate::helpers::downloads::csv_download::export_data_as_csv(&resource, &req, query_string).await {
                                        Ok(response) => {
                                            info!("✅ CSV export successful for {} by {}", resource_name, claims.email);
                                            return response;
                                        }
                                        Err(e) => {
                                            error!("❌ Failed to export CSV for {}: {}", resource_name, e);
                                            return HttpResponse::InternalServerError()
                                                .content_type("text/plain")
                                                .body(format!("Failed to export CSV data: {}", e));
                                        }
                                    }
                                }
                                _ => {
                                    warn!("⚠️ Unsupported download format requested: {}", download_format);
                                    return HttpResponse::BadRequest()
                                        .content_type("text/plain")
                                        .body(format!("Unsupported download format: {}. Supported formats: json, csv", download_format));
                                }
                            }
                        }
                        
                        // REGULAR LIST VIEW (No download request)
                        info!("✅ List UI accessed by: {} for resource: {}", claims.email, resource_name);
                        
                        let mut ctx = create_base_template_context(&resource_name, &resource.base_path(), &claims);
                        
                        // Check for success/error messages from query parameters
                        if query_params.contains_key("success") {
                            match query_params.get("success").unwrap().as_str() {
                                "created" => ctx.insert("toast_message", &"Successfully created new item!"),
                                "updated" => ctx.insert("toast_message", &"Successfully updated item!"),
                                "deleted" => ctx.insert("toast_message", &"Successfully deleted item!"),
                                _ => {}
                            }
                            ctx.insert("toast_type", &"success");
                        }
                        
                        if query_params.contains_key("error") {
                            match query_params.get("error").unwrap().as_str() {
                                "create_failed" => ctx.insert("toast_message", &"Failed to create item. Please try again."),
                                "update_failed" => ctx.insert("toast_message", &"Failed to update item. Please try again."),
                                "delete_failed" => ctx.insert("toast_message", &"Failed to delete item. Please try again."),
                                _ => {}
                            }
                            ctx.insert("toast_type", &"error");
                        }
                        
                        // Get filters configuration and current values
                        let (filters, current_filters) = crate::helpers::resource_helper::get_filters_data(&resource, &query_params);
                        ctx.insert("filters", &filters);
                        ctx.insert("current_filters", &current_filters);
                        ctx.insert("has_active_filters", &(!current_filters.is_empty()));
                        
                        // Fetch actual data from the resource (with filters applied)
                        match fetch_list_data(&resource, &req, query_string).await {
                            Ok((headers, rows, pagination)) => {
                                ctx.insert("headers", &headers);
                                ctx.insert("rows", &rows);
                                ctx.insert("pagination", &pagination);
                                
                                info!("📊 Loaded {} items for {} list view", rows.len(), resource_name);
                            }
                            Err(e) => {
                                error!("❌ Failed to fetch list data for {}: {}", resource_name, e);
                                // Provide empty data as fallback
                                let headers = vec!["id", "name", "email", "created_at"];
                                let rows: Vec<serde_json::Map<String, serde_json::Value>> = vec![];
                                let pagination = serde_json::json!({
                                    "current": 1,
                                    "total": 1,
                                    "prev": null,
                                    "next": null,
                                    "filter_params": ""
                                });
                                
                                ctx.insert("headers", &headers);
                                ctx.insert("rows", &rows);
                                ctx.insert("pagination", &pagination);
                                ctx.insert("toast_message", &"Failed to load data. Please refresh the page.");
                                ctx.insert("toast_type", &"error");
                            }
                        }

                        render_template("list.html.tera", ctx).await
                    }
                    Err(response) => response
                }
            }
        }
    }));

    // GET /new - New item form page
    scope = scope.route("/new", web::get().to({
        let resource = Arc::clone(&resource_arc);
        let resource_name = ui_resource_name.clone();
        let base_path = ui_base_path.clone();
        move |_req: HttpRequest, session: Session, config: web::Data<AdminxConfig>| {
            let resource = Arc::clone(&resource);
            let resource_name = resource_name.clone();
            let base_path = base_path.clone();
            async move {
                match check_authentication(&session, &config, &resource_name, "create").await {
                    Ok(claims) => {
                        info!("✅ New form UI accessed by: {} for resource: {}", claims.email, resource_name);
                        
                        let form = resource.form_structure()
                            .unwrap_or_else(|| {
                                warn!("No form structure defined for resource: {}", resource_name);
                                get_default_form_structure()
                            });

                        let mut ctx = create_base_template_context(&resource_name, &base_path, &claims);
                        let form_map = to_map(&form);
                        ctx.insert("fields", &extract_fields_for_form(&form_map));
                        ctx.insert("form_structure", &form);
                        ctx.insert("form", &form);
                        ctx.insert("is_edit_mode", &false);
                        let supports_upload = resource.supports_file_upload();
                        ctx.insert("supports_upload", &supports_upload);

                        render_template("new.html.tera", ctx).await
                    }
                    Err(response) => response
                }
            }
        }
    }));

    // GET /view/{id} - View single item page
    scope = scope.route("/view/{id}", web::get().to({
        let resource = Arc::clone(&resource_arc);
        let resource_name = ui_resource_name.clone();
        move |req: HttpRequest, id: web::Path<String>, session: Session, config: web::Data<AdminxConfig>| {
            let resource = Arc::clone(&resource);
            let resource_name = resource_name.clone();
            async move {
                match check_authentication(&session, &config, &resource_name, "view").await {
                    Ok(claims) => {
                        let item_id = id.into_inner();
                        info!("✅ View UI accessed by: {} for resource: {} item: {}", claims.email, resource_name, item_id);
                        
                        let mut ctx = create_base_template_context(&resource_name, &resource.base_path(), &claims);
                        
                        // Check for success messages from query parameters
                        let query_params: std::collections::HashMap<String, String> = 
                            serde_urlencoded::from_str(&req.query_string()).unwrap_or_default();
                        
                        if query_params.contains_key("success") {
                            match query_params.get("success").unwrap().as_str() {
                                "updated" => ctx.insert("toast_message", &"Successfully updated item!"),
                                _ => {}
                            }
                            ctx.insert("toast_type", &"success");
                        }
                        
                        // Fetch the actual record data
                        match fetch_single_item_data(&resource, &req, &item_id).await {
                            Ok(record) => {
                                let view_structure = resource.view_structure()
                                    .unwrap_or_else(|| get_default_view_structure());
                                ctx.insert("view_structure", &view_structure);
                                ctx.insert("item_id", &item_id);
                                ctx.insert("record", &record);

                                render_template("view.html.tera", ctx).await
                            }
                            Err(e) => {
                                error!("❌ Failed to fetch item {} for {}: {}", item_id, resource_name, e);
                                HttpResponse::NotFound().body(format!("Item not found: {}", e))
                            }
                        }
                    }
                    Err(response) => response
                }
            }
        }
    }));

    // GET /edit/{id} - Edit item form page
    scope = scope.route("/edit/{id}", web::get().to({
        let resource = Arc::clone(&resource_arc);
        let resource_name = ui_resource_name.clone();
        let base_path = ui_base_path.clone();
        move |_req: HttpRequest, id: web::Path<String>, session: Session, config: web::Data<AdminxConfig>| {
            let resource = Arc::clone(&resource);
            let resource_name = resource_name.clone();
            let base_path = base_path.clone();
            async move {
                match check_authentication(&session, &config, &resource_name, "edit").await {
                    Ok(claims) => {
                        let item_id = id.into_inner();
                        info!("✅ Edit form UI accessed by: {} for resource: {} item: {}", claims.email, resource_name, item_id);
                        
                        let mut ctx = create_base_template_context(&resource_name, &base_path, &claims);
                        
                        // Fetch the actual record data for editing
                        let req = actix_web::test::TestRequest::get().to_http_request();
                        match fetch_single_item_data(&resource, &req, &item_id).await {
                            Ok(record) => {
                                let form = resource.form_structure()
                                    .unwrap_or_else(|| get_default_form_structure());

                                let form_map = to_map(&form);

                                // let mut cleaned_record = serde_json::Value::Object(raw_record.clone());
                                // coerce_editor_json_fields(&mut cleaned_record, &form_map);
                                // // Transform the raw MongoDB data using form structure
                                // // let cleaned_record = coerce_editor_json_fields(&raw_record, &form_map);

                                // println!("cleaned_record: {:?}", cleaned_record);
                                ctx.insert("fields", &extract_fields_for_form(&form_map));
                                ctx.insert("form_structure", &form);
                                ctx.insert("form", &form);
                                ctx.insert("item_id", &item_id);
                                ctx.insert("is_edit_mode", &true);
                                ctx.insert("record", &record);
                                let supports_upload = resource.supports_file_upload();
                                ctx.insert("supports_upload", &supports_upload);

                                render_template("edit.html.tera", ctx).await
                            }
                            Err(e) => {
                                error!("❌ Failed to fetch item {} for edit: {}", item_id, e);
                                HttpResponse::NotFound().body(format!("Item not found: {}", e))
                            }
                        }
                    }
                    Err(response) => response
                }
            }
        }
    }));

    // POST /create - Handle HTML form submission for new items
    scope = scope.route("/create", web::post().to({
        let resource = Arc::clone(&resource_arc);
        let resource_name = ui_resource_name.clone();
        move |req: HttpRequest, form_data: web::Form<std::collections::HashMap<String, String>>, session: Session, config: web::Data<AdminxConfig>| {
            let resource = Arc::clone(&resource);
            let resource_name = resource_name.clone();
            async move {
                match check_authentication(&session, &config, &resource_name, "create").await {
                    Ok(claims) => {
                        info!("✅ Create form submitted by: {} for resource: {}", claims.email, resource_name);
                        
                        let json_payload = convert_form_data_to_json(form_data.into_inner());
                        tracing::debug!("Converted form data to JSON: {:?}", json_payload);
                        
                        let create_response = resource.create(&req, json_payload).await;
                        handle_create_response(create_response, &resource.base_path(), &resource_name)
                    }
                    Err(response) => response
                }
            }
        }
    }));

    // POST /create-with-files - Handle multipart form submission for new items
    scope = scope.route("/create-with-files", web::post().to({
        let resource = Arc::clone(&resource_arc);
        let resource_name = ui_resource_name.clone();
        move |req: HttpRequest, mut payload: Multipart, session: Session, config: web::Data<AdminxConfig>| {
            let resource = Arc::clone(&resource);
            let resource_name = resource_name.clone();
            async move {
                if !resource.supports_file_upload() {
                    return HttpResponse::BadRequest().body("File upload not supported for this resource");
                }
                
                match check_authentication(&session, &config, &resource_name, "create").await {
                    Ok(_claims) => {
                        let mut form_data = HashMap::new();
                        let mut files = HashMap::new();
                        
                        while let Some(mut field) = payload.try_next().await.unwrap_or(None) {
                            let name = field.name().unwrap_or("").to_string();
                            
                            // Extract filename first and clone it to avoid borrow issues
                            let filename = field
                                .content_disposition()
                                .and_then(|cd| cd.get_filename())
                                .map(|f| f.to_string()); // Convert to owned String
                            
                            let mut data = Vec::new();
                            while let Some(chunk) = field.try_next().await.unwrap_or(None) {
                                data.extend_from_slice(&chunk);
                            }
                            
                            if let Some(filename) = filename {
                                files.insert(name, (filename, data));
                            } else {
                                form_data.insert(name, String::from_utf8_lossy(&data).to_string());
                            }
                        }
                        
                        resource.create_with_files(&req, form_data, files).await
                    }
                    Err(response) => response
                }
            }
        }
    }));

    // POST /update/{id}/with-files - Handle multipart form submission for updates
    scope = scope.route("/update/{id}/with-files", web::post().to({
        let resource = Arc::clone(&resource_arc);
        let resource_name = ui_resource_name.clone();
        move |req: HttpRequest, id: web::Path<String>, mut payload: Multipart, session: Session, config: web::Data<AdminxConfig>| {
            let resource = Arc::clone(&resource);
            let resource_name = resource_name.clone();
            async move {
                if !resource.supports_file_upload() {
                    return HttpResponse::BadRequest().body("File upload not supported for this resource");
                }
                
                match check_authentication(&session, &config, &resource_name, "update").await {
                    Ok(claims) => {
                        let item_id = id.into_inner();
                        info!("✅ Update with files form submitted by: {} for resource: {} item: {}", 
                              claims.email, resource_name, item_id);
                        
                        let mut form_data = HashMap::new();
                        let mut files = HashMap::new();
                        
                        while let Some(mut field) = payload.try_next().await.unwrap_or(None) {
                            let name = field.name().unwrap_or("").to_string();
                            
                            let filename = field
                                .content_disposition()
                                .and_then(|cd| cd.get_filename())
                                .map(|f| f.to_string());
                            
                            let mut data = Vec::new();
                            while let Some(chunk) = field.try_next().await.unwrap_or(None) {
                                data.extend_from_slice(&chunk);
                            }
                            
                            if let Some(filename) = filename {
                                // Only process non-empty files for updates
                                if !data.is_empty() {
                                    files.insert(name, (filename, data));
                                }
                            } else {
                                form_data.insert(name, String::from_utf8_lossy(&data).to_string());
                            }
                        }
                        
                        resource.update_with_files(&req, item_id, form_data, files).await
                    }
                    Err(response) => response
                }
            }
        }
    }));

    // POST /update/{id} - Handle HTML form submission for updates
    scope = scope.route("/update/{id}", web::post().to({
        let resource = Arc::clone(&resource_arc);
        let resource_name = ui_resource_name.clone();
        move |req: HttpRequest, id: web::Path<String>, form_data: web::Form<std::collections::HashMap<String, String>>, session: Session, config: web::Data<AdminxConfig>| {
            let resource = Arc::clone(&resource);
            let resource_name = resource_name.clone();
            async move {
                match check_authentication(&session, &config, &resource_name, "update").await {
                    Ok(claims) => {
                        let item_id = id.into_inner();
                        info!("✅ Update form submitted by: {} for resource: {} item: {}", claims.email, resource_name, item_id);
                        
                        let json_payload = convert_form_data_to_json(form_data.into_inner());
                        tracing::debug!("Converted form data to JSON: {:?}", json_payload);
                        
                        let update_response = resource.update(&req, item_id.clone(), json_payload).await;
                        handle_update_response(update_response, &resource.base_path(), &item_id, &resource_name)
                    }
                    Err(response) => response
                }
            }
        }
    }));

    // POST /{id}/delete - Handle HTML form submission for deletes
    scope = scope.route("/{id}/delete", web::post().to({
        let resource = Arc::clone(&resource_arc);
        let resource_name = ui_resource_name.clone();
        move |req: HttpRequest, id: web::Path<String>, session: Session, config: web::Data<AdminxConfig>| {
            let resource = Arc::clone(&resource);
            let resource_name = resource_name.clone();
            async move {
                match check_authentication(&session, &config, &resource_name, "delete").await {
                    Ok(claims) => {
                        let item_id = id.into_inner();
                        info!("✅ Delete form submitted by: {} for resource: {} item: {}", claims.email, resource_name, item_id);
                        
                        let delete_response = resource.delete(&req, item_id.clone()).await;
                        handle_delete_response(delete_response, &resource.base_path(), &resource_name)
                    }
                    Err(response) => response
                }
            }
        }
    }));

    

    // ========================
    // API Routes (JSON endpoints) - MOVED TO /api PREFIX TO AVOID CONFLICTS
    // ========================
    
    // GET /api - List all items (JSON API)
    let list_resource = resource.clone_box();
    scope = scope.route(
        "/api",
        web::get().to(move |req: HttpRequest| {
            let resource = list_resource.clone_box();
            async move {
                info!("📡 List API endpoint called for resource: {}", resource.resource_name());
                let query_string = req.query_string().to_string();
                resource.list(&req, query_string).await
            }
        }),
    );

    // POST /api - Create new item (JSON API)
    let create_resource = resource.clone_box();
    scope = scope.route(
        "/api",
        web::post().to(move |req: HttpRequest, body: web::Json<Value>| {
            let resource = create_resource.clone_box();
            async move {
                info!("📡 Create API endpoint called for resource: {}", resource.resource_name());
                resource.create(&req, body.into_inner()).await
            }
        }),
    );

    // GET /api/{id} - Get single item (JSON API)
    let get_resource = resource.clone_box();
    scope = scope.route(
        "/api/{id}",
        web::get().to(move |req: HttpRequest, path: web::Path<String>| {
            let resource = get_resource.clone_box();
            async move {
                let id = path.into_inner();
                info!("📡 Get API endpoint called for resource: {} with id: {}", resource.resource_name(), id);
                resource.get(&req, id).await
            }
        }),
    );

    // PUT /api/{id} - Update item (JSON API)
    let update_resource = resource.clone_box();
    scope = scope.route(
        "/api/{id}",
        web::put().to(move |req: HttpRequest, path: web::Path<String>, body: web::Json<Value>| {
            let resource = update_resource.clone_box();
            async move {
                let id = path.into_inner();
                info!("📡 Update API endpoint called for resource: {} with id: {}", resource.resource_name(), id);
                resource.update(&req, id, body.into_inner()).await
            }
        }),
    );

    // DELETE /api/{id} - Delete item (JSON API)
    let delete_resource = resource.clone_box();
    scope = scope.route(
        "/api/{id}",
        web::delete().to(move |req: HttpRequest, path: web::Path<String>| {
            let resource = delete_resource.clone_box();
            async move {
                let id = path.into_inner();
                info!("📡 Delete API endpoint called for resource: {} with id: {}", resource.resource_name(), id);
                resource.delete(&req, id).await
            }
        }),
    );

    // ========================
    // Custom Actions
    // ========================
    for action in resource_arc.custom_actions() {
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

    info!("✅ Successfully registered all routes for resource: {}", resource_name);
    scope
}