// crates/adminx/src/resource.rs - Enhanced with file upload support
use actix_web::{HttpRequest, HttpResponse, ResponseError};
use async_trait::async_trait;
use futures::future::BoxFuture;
use serde_json::{json, Value};
use crate::menu::{MenuItem, MenuAction};
use crate::actions::CustomAction;
use crate::nested::AdmixNestedResource;
use crate::error::AdminxError;
use crate::filters::parse_query;
use crate::pagination::PaginatedResponse;
use mongodb::{Collection, bson::{doc, oid::ObjectId, Document}};
use futures::TryStreamExt;
use std::collections::HashMap;
use crate::helpers::resource_helper::convert_form_data_to_json;

#[async_trait]
pub trait AdmixResource: Send + Sync {
    // ===========================
    // REQUIRED METHODS (Must be implemented)
    // ===========================
    fn new() -> Self where Self: Sized;
    fn resource_name(&self) -> &'static str;
    fn base_path(&self) -> &'static str;
    fn collection_name(&self) -> &'static str; 
    fn get_collection(&self) -> Collection<Document>;
    fn clone_box(&self) -> Box<dyn AdmixResource>;

    // ===========================
    // CONFIGURATION (Optional - with defaults)
    // ===========================

    /// Optional parent/super menu name to group this resource under.
    fn menu_group(&self) -> Option<&'static str> {
        None
    }

    /// Menu label for this resource (default: same as resource_name)
    fn menu(&self) -> &'static str {
        self.resource_name()
    }

    fn allowed_roles(&self) -> Vec<String> {
        vec!["admin".to_string()]
    }

    fn allowed_roles_with_permissions(&self) -> Value {
        json!({})
    }

    fn visible_fields_for_role(&self, _roles: &[String]) -> Vec<String> {
        vec![]
    }

    fn nested_resources(&self) -> Vec<Box<dyn AdmixNestedResource>> {
        vec![]
    }

    fn custom_actions(&self) -> Vec<CustomAction> {
        vec![]
    }

    fn allowed_actions(&self) -> Option<Vec<MenuAction>> {
        None // None means all actions are allowed
    }

    fn permit_keys(&self) -> Vec<&'static str> {
        vec![] // Override this to specify which fields can be created/updated
    }

    fn readonly_keys(&self) -> Vec<&'static str> {
        vec!["_id", "created_at", "updated_at"]
    }

    // ===========================
    // FILE UPLOAD CONFIGURATION (New)
    // ===========================
    
    /// Return true if this resource supports file uploads
    fn supports_file_upload(&self) -> bool {
        false
    }
    
    /// Maximum file size in bytes (default: 10MB)
    fn max_file_size(&self) -> usize {
        10 * 1024 * 1024 // 10MB
    }
    
    /// Allowed file extensions
    fn allowed_file_extensions(&self) -> Vec<&'static str> {
        vec!["jpg", "jpeg", "png", "gif", "webp"]
    }
    
    /// File upload configuration
    fn file_upload_config(&self) -> Option<Value> {
        None
    }
    
    /* -----------------------------------------------------------
    START - Image specific resource
    ------------------------------------------------------------ */
    /// Handle file upload processing - override this for custom file handling
    fn process_file_upload(&self, _field_name: &str, _file_data: &[u8], _filename: &str) -> BoxFuture<'static, Result<HashMap<String, String>, AdminxError>> {
        Box::pin(async move {
            Err(AdminxError::BadRequest("File upload not implemented for this resource".into()))
        })
    }


    // In your adminx crate: crates/adminx/src/resource.rs

fn create(&self, _req: &HttpRequest, payload: Value) -> BoxFuture<'static, HttpResponse> {
    // Extract everything we need BEFORE the async block
    let collection = self.get_collection();
    let permitted = self.permit_keys().into_iter().collect::<std::collections::HashSet<_>>();
    let resource_name = self.resource_name().to_string();
    
    Box::pin(async move {
        // Now _req is not captured in this async block
        tracing::info!("Default create implementation for resource: {} with payload: {:?}", resource_name, payload);
        
        let mut clean_map = serde_json::Map::new();
        if let Value::Object(map) = payload {
            for (key, value) in map {
                if permitted.contains(key.as_str()) {
                    clean_map.insert(key, value);
                }
            }
        }

        let now = mongodb::bson::DateTime::now();
        clean_map.insert("created_at".to_string(), json!(now));
        clean_map.insert("updated_at".to_string(), json!(now));

        if permitted.contains("deleted") && !clean_map.contains_key("deleted") {
            clean_map.insert("deleted".to_string(), json!(false));
        }

        tracing::debug!("Cleaned payload for {}: {:?}", resource_name, clean_map);

        match mongodb::bson::to_document(&Value::Object(clean_map)) {
            Ok(document) => {
                match collection.insert_one(document, None).await {
                    Ok(insert_result) => {
                        tracing::info!("Document created successfully for {}: {:?}", resource_name, insert_result.inserted_id);
                        HttpResponse::Created().json(json!({
                            "success": true,
                            "message": format!("{} created successfully", resource_name),
                            "id": insert_result.inserted_id
                        }))
                    },
                    Err(e) => {
                        tracing::error!("Error inserting document for {}: {}", resource_name, e);
                        AdminxError::InternalError.error_response()
                    }
                }
            },
            Err(e) => {
                tracing::error!("Error converting payload to BSON for {}: {}", resource_name, e);
                AdminxError::BadRequest("Invalid input data".into()).error_response()
            }
        }
    })
}

fn update(&self, _req: &HttpRequest, id: String, payload: Value) -> BoxFuture<'static, HttpResponse> {
    // Extract everything we need BEFORE the async block
    let collection = self.get_collection();
    let permitted = self.permit_keys().into_iter().collect::<std::collections::HashSet<_>>();
    let resource_name = self.resource_name().to_string();
    
    Box::pin(async move {
        // Now _req is not captured in this async block
        tracing::info!("Default update implementation for resource: {} with id: {} and payload: {:?}", 
                     resource_name, id, payload);
        
        match ObjectId::parse_str(&id) {
            Ok(oid) => {
                let mut clean_map = serde_json::Map::new();
                if let Value::Object(map) = payload {
                    for (key, value) in map {
                        if permitted.contains(key.as_str()) {
                            clean_map.insert(key, value);
                        }
                    }
                }

                clean_map.insert("updated_at".to_string(), json!(mongodb::bson::DateTime::now()));

                let bson_payload: Document = match mongodb::bson::to_document(&Value::Object(clean_map)) {
                    Ok(doc) => doc,
                    Err(e) => {
                        tracing::error!("Error converting payload to BSON for {}: {}", resource_name, e);
                        return AdminxError::BadRequest("Invalid payload format".into()).error_response();
                    }
                };

                let update_doc = doc! { "$set": bson_payload };

                match collection.update_one(doc! { "_id": oid }, update_doc, None).await {
                    Ok(result) => {
                        if result.modified_count > 0 {
                            tracing::info!("Document {} updated successfully for {}", id, resource_name);
                            HttpResponse::Ok().json(json!({
                                "success": true,
                                "message": format!("{} updated successfully", resource_name),
                                "modified_count": result.modified_count
                            }))
                        } else {
                            tracing::warn!("No document found to update with id: {} for {}", id, resource_name);
                            AdminxError::NotFound.error_response()
                        }
                    },
                    Err(e) => {
                        tracing::error!("Error updating document {} for {}: {}", id, resource_name, e);
                        AdminxError::InternalError.error_response()
                    }
                }
            }
            Err(e) => {
                tracing::error!("Invalid ObjectId {} for {}: {}", id, resource_name, e);
                AdminxError::BadRequest("Invalid ID format".into()).error_response()
            }
        }
    })
}


fn create_with_files(
    &self,
    _req: &HttpRequest,
    mut form_data: std::collections::HashMap<String, String>,
    files: std::collections::HashMap<String, (String, Vec<u8>)>,
) -> futures::future::BoxFuture<'static, actix_web::HttpResponse> {
    let resource = self.clone_box();

    Box::pin(async move {
        // 1) पहले फाइल अपलोड प्रोसेस कर लें
        for (field_name, (filename, file_data)) in files {
            match resource.process_file_upload(&field_name, &file_data, &filename).await {
                Ok(upload_results) => {
                    for (k, v) in upload_results {
                        form_data.insert(k, v);
                    }
                }
                Err(e) => {
                    tracing::error!("File upload failed for field {}: {:?}", field_name, e);
                    return actix_web::HttpResponse::BadRequest().json(serde_json::json!({
                        "error": format!("File upload failed: {:?}", e)
                    }));
                }
            }
        }

        // 2) form_data → JSON
        let json_payload = convert_form_data_to_json(form_data);

        // 3) ⬇️ HttpRequest को inner scope में बनाइए; future निकालिए; फिर outer में await कीजिए
        let fut = {
            let test_req = actix_web::test::TestRequest::default().to_http_request();
            resource.create(&test_req, json_payload)
        };

        // अब यहाँ HttpRequest drop हो चुका होगा, इसलिए future `Send` रहेगा
        fut.await
    })
}

fn update_with_files(
    &self,
    _req: &HttpRequest,
    id: String,
    mut form_data: std::collections::HashMap<String, String>,
    files: std::collections::HashMap<String, (String, Vec<u8>)>,
) -> futures::future::BoxFuture<'static, actix_web::HttpResponse> {
    let resource = self.clone_box();

    Box::pin(async move {
        for (field_name, (filename, file_data)) in files {
            if !file_data.is_empty() {
                match resource.process_file_upload(&field_name, &file_data, &filename).await {
                    Ok(upload_results) => {
                        for (k, v) in upload_results {
                            form_data.insert(k, v);
                        }
                    }
                    Err(e) => {
                        tracing::error!("File upload failed for field {}: {:?}", field_name, e);
                        return actix_web::HttpResponse::BadRequest().json(serde_json::json!({
                            "error": format!("File upload failed: {:?}", e)
                        }));
                    }
                }
            }
        }

        let json_payload = convert_form_data_to_json(form_data);

        let fut = {
            let test_req = actix_web::test::TestRequest::default().to_http_request();
            resource.update(&test_req, id, json_payload)
        };

        fut.await
    })
}

// fn create_with_files(
//     &self,
//     _req: &HttpRequest,
//     mut form_data: HashMap<String, String>,
//     files: HashMap<String, (String, Vec<u8>)>,
// ) -> BoxFuture<'static, HttpResponse> {
//     let resource = self.clone_box();
    
//     Box::pin(async move {
//         // Process each uploaded file
//         for (field_name, (filename, file_data)) in files {
//             match resource.process_file_upload(&field_name, &file_data, &filename).await {
//                 Ok(upload_results) => {
//                     for (key, value) in upload_results {
//                         form_data.insert(key, value);
//                     }
//                 }
//                 Err(e) => {
//                     tracing::error!("File upload failed for field {}: {:?}", field_name, e);
//                     return HttpResponse::BadRequest().json(serde_json::json!({
//                         "error": format!("File upload failed: {:?}", e)
//                     }));
//                 }
//             }
//         }
        
//         let json_payload = crate::helpers::resource_helper::convert_form_data_to_json(form_data);
//         let test_req = actix_web::test::TestRequest::default().to_http_request();
//         resource.create(&test_req, json_payload).await
//     })
// }

// fn update_with_files(
//     &self,
//     _req: &HttpRequest,
//     id: String,
//     mut form_data: HashMap<String, String>,
//     files: HashMap<String, (String, Vec<u8>)>,
// ) -> BoxFuture<'static, HttpResponse> {
//     let resource = self.clone_box();
    
//     Box::pin(async move {
//         for (field_name, (filename, file_data)) in files {
//             if !file_data.is_empty() {
//                 match resource.process_file_upload(&field_name, &file_data, &filename).await {
//                     Ok(upload_results) => {
//                         for (key, value) in upload_results {
//                             form_data.insert(key, value);
//                         }
//                     }
//                     Err(e) => {
//                         tracing::error!("File upload failed for field {}: {:?}", field_name, e);
//                         return HttpResponse::BadRequest().json(serde_json::json!({
//                             "error": format!("File upload failed: {:?}", e)
//                         }));
//                     }
//                 }
//             }
//         }
        
//         let json_payload = crate::helpers::resource_helper::convert_form_data_to_json(form_data);
//         let test_req = actix_web::test::TestRequest::default().to_http_request();
//         resource.update(&test_req, id, json_payload).await
//     })
// }

    // fn create_with_files(
    //     &self,
    //     _req: &HttpRequest,
    //     mut form_data: HashMap<String, String>,
    //     files: HashMap<String, (String, Vec<u8>)>,
    // ) -> BoxFuture<'static, HttpResponse> {
    //     let resource = self.clone_box();
        
    //     Box::pin(async move {
    //         // Process each uploaded file
    //         for (field_name, (filename, file_data)) in files {
    //             match resource.process_file_upload(&field_name, &file_data, &filename).await {
    //                 Ok(upload_results) => {
    //                     for (key, value) in upload_results {
    //                         form_data.insert(key, value);
    //                     }
    //                 }
    //                 Err(e) => {
    //                     tracing::error!("File upload failed for field {}: {:?}", field_name, e);
    //                     return HttpResponse::BadRequest().json(serde_json::json!({
    //                         "error": format!("File upload failed: {:?}", e)
    //                     }));
    //                 }
    //             }
    //         }
            
    //         let json_payload = crate::helpers::resource_helper::convert_form_data_to_json(form_data);
    //         // Create the test request inside the async block, use it immediately, then drop it
    //         {
    //             let test_req = actix_web::test::TestRequest::default().to_http_request();
    //             resource.create(&test_req, json_payload).await
    //         }
    //     })
    // }

    // fn update_with_files(
    //     &self,
    //     _req: &HttpRequest,
    //     id: String,
    //     mut form_data: HashMap<String, String>,
    //     files: HashMap<String, (String, Vec<u8>)>,
    // ) -> BoxFuture<'static, HttpResponse> {
    //     let resource = self.clone_box();
        
    //     Box::pin(async move {
    //         for (field_name, (filename, file_data)) in files {
    //             if !file_data.is_empty() {
    //                 match resource.process_file_upload(&field_name, &file_data, &filename).await {
    //                     Ok(upload_results) => {
    //                         for (key, value) in upload_results {
    //                             form_data.insert(key, value);
    //                         }
    //                     }
    //                     Err(e) => {
    //                         tracing::error!("File upload failed for field {}: {:?}", field_name, e);
    //                         return HttpResponse::BadRequest().json(serde_json::json!({
    //                             "error": format!("File upload failed: {:?}", e)
    //                         }));
    //                     }
    //                 }
    //             }
    //         }
            
    //         let json_payload = crate::helpers::resource_helper::convert_form_data_to_json(form_data);
    //         let test_req = actix_web::test::TestRequest::default().to_http_request();
    //         resource.update(&test_req, id, json_payload).await
    //     })
    // }
    /* -----------------------------------------------------------
    END - Image specific resource
    ------------------------------------------------------------ */

    // ===========================
    // UI STRUCTURE METHODS (Optional)
    // ===========================
    fn form_structure(&self) -> Option<Value> {
        None // Override to customize create/edit forms
    }

    fn list_structure(&self) -> Option<Value> {
        None // Override to customize list view
    }

    fn view_structure(&self) -> Option<Value> {
        None // Override to customize detail view
    }

    fn filters(&self) -> Option<Value> {
        None // Override to add search/filter functionality
    }

    // ===========================
    // ENHANCED CRUD IMPLEMENTATIONS
    // ===========================
    
    fn list(&self, _req: &HttpRequest, query: String) -> BoxFuture<'static, HttpResponse> {
        let collection = self.get_collection();
        let resource_name = self.resource_name().to_string();
        
        Box::pin(async move {
            tracing::info!("Default list implementation for resource: {}", resource_name);
            
            let opts = parse_query(&query);
            
            let total = match collection.count_documents(opts.filter.clone(), None).await {
                Ok(count) => count,
                Err(e) => {
                    tracing::error!("Error counting documents for {}: {}", resource_name, e);
                    return AdminxError::InternalError.error_response();
                }
            };
            
            let mut find_options = mongodb::options::FindOptions::default();
            find_options.skip = Some(opts.skip);
            find_options.limit = Some(opts.limit as i64);
            if let Some(sort) = opts.sort {
                find_options.sort = Some(sort);
            }
            
            match collection.find(opts.filter, find_options).await {
                Ok(mut cursor) => {
                    let mut documents = Vec::new();
                    while let Some(doc) = cursor.try_next().await.unwrap_or(None) {
                        documents.push(doc);
                    }

                    tracing::info!("Found {} documents for {} out of {} total", 
                                 documents.len(), resource_name, total);
                    
                    HttpResponse::Ok().json(PaginatedResponse {
                        data: documents,
                        total,
                        page: (opts.skip / opts.limit) + 1,
                        per_page: opts.limit,
                    })
                }
                Err(e) => {
                    tracing::error!("Error executing find query for {}: {}", resource_name, e);
                    AdminxError::InternalError.error_response()
                }
            }
        })
    }

    fn get(&self, _req: &HttpRequest, id: String) -> BoxFuture<'static, HttpResponse> {
        let collection = self.get_collection();
        let resource_name = self.resource_name().to_string();
        
        Box::pin(async move {
            tracing::info!("Default get implementation for resource: {} with id: {}", resource_name, id);
            
            match ObjectId::parse_str(&id) {
                Ok(oid) => {
                    match collection.find_one(doc! { "_id": oid }, None).await {
                        Ok(Some(document)) => {
                            tracing::info!("Found document with id: {} for resource: {}", id, resource_name);
                            HttpResponse::Ok().json(document)
                        },
                        Ok(None) => {
                            tracing::warn!("Document not found with id: {} for resource: {}", id, resource_name);
                            AdminxError::NotFound.error_response()
                        },
                        Err(e) => {
                            tracing::error!("Database error getting document {} for {}: {}", id, resource_name, e);
                            AdminxError::InternalError.error_response()
                        }
                    }
                },
                Err(e) => {
                    tracing::error!("Invalid ObjectId {} for {}: {}", id, resource_name, e);
                    AdminxError::BadRequest("Invalid ID format".into()).error_response()
                }
            }
        })
    }

    // /// Enhanced create method that can handle both regular form data and file uploads
    // fn create(&self, _req: &HttpRequest, payload: Value) -> BoxFuture<'static, HttpResponse> {
    //     let collection = self.get_collection();
    //     let permitted = self.permit_keys().into_iter().collect::<std::collections::HashSet<_>>();
    //     let resource_name = self.resource_name().to_string();
        
    //     Box::pin(async move {
    //         tracing::info!("Default create implementation for resource: {} with payload: {:?}", resource_name, payload);
            
    //         let mut clean_map = serde_json::Map::new();
    //         if let Value::Object(map) = payload {
    //             for (key, value) in map {
    //                 if permitted.contains(key.as_str()) {
    //                     clean_map.insert(key, value);
    //                 }
    //             }
    //         }

    //         let now = mongodb::bson::DateTime::now();
    //         clean_map.insert("created_at".to_string(), json!(now));
    //         clean_map.insert("updated_at".to_string(), json!(now));

    //         // Add default values for file upload resources
    //         if permitted.contains("deleted") && !clean_map.contains_key("deleted") {
    //             clean_map.insert("deleted".to_string(), json!(false));
    //         }

    //         tracing::debug!("Cleaned payload for {}: {:?}", resource_name, clean_map);

    //         match mongodb::bson::to_document(&Value::Object(clean_map)) {
    //             Ok(document) => {
    //                 match collection.insert_one(document, None).await {
    //                     Ok(insert_result) => {
    //                         tracing::info!("Document created successfully for {}: {:?}", resource_name, insert_result.inserted_id);
    //                         HttpResponse::Created().json(json!({
    //                             "success": true,
    //                             "message": format!("{} created successfully", resource_name),
    //                             "id": insert_result.inserted_id
    //                         }))
    //                     },
    //                     Err(e) => {
    //                         tracing::error!("Error inserting document for {}: {}", resource_name, e);
    //                         AdminxError::InternalError.error_response()
    //                     }
    //                 }
    //             },
    //             Err(e) => {
    //                 tracing::error!("Error converting payload to BSON for {}: {}", resource_name, e);
    //                 AdminxError::BadRequest("Invalid input data".into()).error_response()
    //             }
    //         }
    //     })
    // }

    // /// Enhanced update method with soft delete support
    // fn update(&self, _req: &HttpRequest, id: String, payload: Value) -> BoxFuture<'static, HttpResponse> {
    //     let collection = self.get_collection();
    //     let permitted = self.permit_keys().into_iter().collect::<std::collections::HashSet<_>>();
    //     let resource_name = self.resource_name().to_string();
        
    //     Box::pin(async move {
    //         tracing::info!("Default update implementation for resource: {} with id: {} and payload: {:?}", 
    //                      resource_name, id, payload);
            
    //         match ObjectId::parse_str(&id) {
    //             Ok(oid) => {
    //                 let mut clean_map = serde_json::Map::new();
    //                 if let Value::Object(map) = payload {
    //                     for (key, value) in map {
    //                         if permitted.contains(key.as_str()) {
    //                             clean_map.insert(key, value);
    //                         }
    //                     }
    //                 }

    //                 clean_map.insert("updated_at".to_string(), json!(mongodb::bson::DateTime::now()));

    //                 let bson_payload: Document = match mongodb::bson::to_document(&Value::Object(clean_map)) {
    //                     Ok(doc) => doc,
    //                     Err(e) => {
    //                         tracing::error!("Error converting payload to BSON for {}: {}", resource_name, e);
    //                         return AdminxError::BadRequest("Invalid payload format".into()).error_response();
    //                     }
    //                 };

    //                 let update_doc = doc! { "$set": bson_payload };

    //                 match collection.update_one(doc! { "_id": oid }, update_doc, None).await {
    //                     Ok(result) => {
    //                         if result.modified_count > 0 {
    //                             tracing::info!("Document {} updated successfully for {}", id, resource_name);
    //                             HttpResponse::Ok().json(json!({
    //                                 "success": true,
    //                                 "message": format!("{} updated successfully", resource_name),
    //                                 "modified_count": result.modified_count
    //                             }))
    //                         } else {
    //                             tracing::warn!("No document found to update with id: {} for {}", id, resource_name);
    //                             AdminxError::NotFound.error_response()
    //                         }
    //                     },
    //                     Err(e) => {
    //                         tracing::error!("Error updating document {} for {}: {}", id, resource_name, e);
    //                         AdminxError::InternalError.error_response()
    //                     }
    //                 }
    //             }
    //             Err(e) => {
    //                 tracing::error!("Invalid ObjectId {} for {}: {}", id, resource_name, e);
    //                 AdminxError::BadRequest("Invalid ID format".into()).error_response()
    //             }
    //         }
    //     })
    // }

    /// Enhanced delete with soft delete support
    fn delete(&self, _req: &HttpRequest, id: String) -> BoxFuture<'static, HttpResponse> {
        let collection = self.get_collection();
        let resource_name = self.resource_name().to_string();
        let permitted = self.permit_keys().into_iter().collect::<std::collections::HashSet<_>>();
        
        Box::pin(async move {
            tracing::info!("Default delete implementation for resource: {} with id: {}", resource_name, id);
            
            match ObjectId::parse_str(&id) {
                Ok(oid) => {
                    // If resource supports soft delete (has "deleted" in permitted keys), use soft delete
                    if permitted.contains("deleted") {
                        let update_doc = doc! { 
                            "$set": {
                                "deleted": true,
                                "updated_at": mongodb::bson::DateTime::now()
                            }
                        };
                        
                        match collection.update_one(doc! { "_id": oid }, update_doc, None).await {
                            Ok(result) => {
                                if result.modified_count > 0 {
                                    tracing::info!("Document {} soft deleted successfully for {}", id, resource_name);
                                    HttpResponse::Ok().json(json!({
                                        "success": true,
                                        "message": format!("{} deleted successfully", resource_name),
                                        "soft_delete": true,
                                        "modified_count": result.modified_count
                                    }))
                                } else {
                                    tracing::warn!("No document found to soft delete with id: {} for {}", id, resource_name);
                                    AdminxError::NotFound.error_response()
                                }
                            },
                            Err(e) => {
                                tracing::error!("Error soft deleting document {} for {}: {}", id, resource_name, e);
                                AdminxError::InternalError.error_response()
                            }
                        }
                    } else {
                        // Hard delete
                        match collection.delete_one(doc! { "_id": oid }, None).await {
                            Ok(result) => {
                                if result.deleted_count > 0 {
                                    tracing::info!("Document {} hard deleted successfully for {}", id, resource_name);
                                    HttpResponse::Ok().json(json!({
                                        "success": true,
                                        "message": format!("{} deleted successfully", resource_name),
                                        "soft_delete": false,
                                        "deleted_count": result.deleted_count
                                    }))
                                } else {
                                    tracing::warn!("No document found to hard delete with id: {} for {}", id, resource_name);
                                    AdminxError::NotFound.error_response()
                                }
                            },
                            Err(e) => {
                                tracing::error!("Error hard deleting document {} for {}: {}", id, resource_name, e);
                                AdminxError::InternalError.error_response()
                            }
                        }
                    }
                },
                Err(e) => {
                    tracing::error!("Invalid ObjectId {} for {}: {}", id, resource_name, e);
                    AdminxError::BadRequest("Invalid ID format".into()).error_response()
                }
            }
        })
    }

    
    // ===========================
    // MENU GENERATION
    // ===========================
    fn generate_menu(&self) -> Option<MenuItem> {
        Some(MenuItem {
            title: self.menu().to_string(),
            path: self.base_path().to_string(),
            icon: Some(if self.supports_file_upload() { "image".to_string() } else { "users".to_string() }),
            order: Some(10),
            children: None,
        })
    }

    fn build_adminx_menus(&self) -> Option<MenuItem> {
        self.generate_menu()
    }
}

// Manual clone implementation
impl Clone for Box<dyn AdmixResource> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}



