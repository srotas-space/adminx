# AdminX

[![Crates.io](https://img.shields.io/crates/v/adminx)](https://crates.io/crates/adminx)
[![Documentation](https://docs.rs/adminx/badge.svg)](https://docs.rs/adminx)
[![License](https://img.shields.io/crates/l/adminx)](LICENSE)
[![Build Status](https://github.com/xsmmaurya/adminx/workflows/CI/badge.svg)](https://github.com/xsmmaurya/adminx/actions)

**AdminX** is a powerful, modern admin panel framework for Rust built on top of Actix Web and MongoDB. It provides a complete solution for creating administrative interfaces with minimal boilerplate code, featuring automatic CRUD operations, role-based access control, and a beautiful responsive UI.

## ✨ Features

### 🚀 **Core Functionality**
- **Zero-Config CRUD Operations** - Automatic Create, Read, Update, Delete with sensible defaults
- **Schema-Driven Forms** - Auto-generate forms from JSON Schema using `schemars`
- **Resource-Centric Architecture** - Define resources once, get full admin interface
- **Hybrid API/UI** - Both REST API and web interface from same resource definitions
- **Dynamic Menu Generation** - Automatic navigation based on registered resources

### 🔐 **Security First**
- **JWT + Session Authentication** - Secure token-based auth with session management
- **Role-Based Access Control (RBAC)** - Fine-grained permissions per resource
- **Rate Limiting** - Built-in protection against brute force attacks
- **Timing Attack Prevention** - Secure password verification
- **CSRF Protection** - Form-based submission security

### 🎨 **Modern UI/UX**
- **Responsive Design** - Mobile-first TailwindCSS-based interface
- **Dark/Light Mode** - Built-in theme switching
- **Toast Notifications** - User feedback with auto-dismiss
- **Real-time Validation** - Client-side form validation
- **Accessibility** - WCAG compliant with proper ARIA labels

### 🛠️ **Developer Experience**
- **Minimal Boilerplate** - Resources work out-of-the-box
- **Type Safety** - Full Rust type safety throughout
- **Embedded Templates** - Zero external dependencies
- **Comprehensive Logging** - Built-in tracing and debugging
- **Hot Reload Support** - Fast development iteration

## 🚀 Quick Start

Add AdminX to your `Cargo.toml`:

```toml
[dependencies]
adminx = "0.1.0"
actix-web = "4"
mongodb = { version = "2.4", features = ["tokio-runtime"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
schemars = { version = "0.8", features = ["derive"] }
```

### 1. Define Collections
```rust
// src/models/image_model.rs
use actix_web::web;
use mongodb::{
    bson::{doc, oid::ObjectId, to_bson, DateTime as BsonDateTime},
    Collection, Database,
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use crate::services::redis_service::get_redis_connection;
use strum_macros::EnumIter; 


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, EnumIter)]
#[serde(rename_all = "lowercase")]
pub enum ImageStatus {
    Active,
    Inactive,
}

impl ToString for ImageStatus {
    fn to_string(&self) -> String {
        match self {
            ImageStatus::Active => "active".to_string(),
            ImageStatus::Inactive => "inactive".to_string(),
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Image {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub title: String,
    pub image_url: String,
    pub status: ImageStatus,
    pub deleted: bool,
    pub created_at: BsonDateTime,
    pub updated_at: BsonDateTime,
}


impl Default for Image {
    fn default() -> Self {
        Self {
            id: None,
            title: String::from(""),
            image_url: String::from(""),
            status: ImageStatus::Active,
            deleted: false,
            created_at: BsonDateTime::now(),
            updated_at: BsonDateTime::now(),
        }
    }
}
```


### 2. Define adminx initializer
```rust
// src/admin/initializer.rs
use mongodb::Database;
use adminx::{
    adminx_initialize, 
    get_adminx_config, 
    setup_adminx_logging, 
    get_adminx_session_middleware,
    register_all_admix_routes,
    registry::register_resource,
    AdmixResource,
    AdminxConfig,
};
use actix_session::SessionMiddleware;

// Import your resources
// use crate::admin::resources::config_resource::ConfigResource;
use crate::admin::resources::image_resource::ImageResource;

pub struct AdminxInitializer;

impl AdminxInitializer {
    /// Initialize all AdminX components and return the configuration
    pub async fn initialize(db: Database) -> AdminxConfig {
        println!("Initializing AdminX components...");
        
        // Get AdminX configuration
        let adminx_config = get_adminx_config();
        
        // Setup logging
        setup_adminx_logging(&adminx_config);
        
        // Initialize AdminX with database
        let _adminx_instance = adminx_initialize(db.clone()).await;
        
        // Register resources
        Self::register_resources();
        
        // Print debug information
        Self::print_debug_info();
        
        adminx_config
    }
    
    /// Register all AdminX resources
    fn register_resources() {
        println!("📝 Registering AdminX resources...");
        // Register your resources with AdminX
        // register_resource(Box::new(ConfigResource::new()));
        register_resource(Box::new(ImageResource::new()));
        println!("All resources registered successfully!");
    }
    
    /// Print debug information about registered resources
    fn print_debug_info() {
        // Debug: Check if resources were registered
        let resources = adminx::registry::all_resources();
        println!("📋 Total resources registered: {}", resources.len());
        
        for resource in &resources {
            println!("   - Resource: '{}' at path: '{}'", 
                     resource.resource_name(), 
                     resource.base_path());
        }
    }
    
    /// Get the AdminX session middleware
    pub fn get_session_middleware(config: &AdminxConfig) -> SessionMiddleware<impl actix_session::storage::SessionStore> {
        get_adminx_session_middleware(config)
    }
    
    /// Get the AdminX routes service
    pub fn get_routes_service() -> actix_web::Scope {
        register_all_admix_routes()
    }
}
```


### 3. Define Resources
```rust
// src/admin/resources/image_resource.rs
use crate::dbs::mongo::get_collection;
use adminx::{AdmixResource, error::AdminxError};
use async_trait::async_trait;
use mongodb::{Collection, bson::Document};
use serde_json::{json, Value};
use crate::models::image_model::ImageStatus;
use futures::future::BoxFuture;
use std::collections::HashMap;
use convert_case::{Case, Casing};
use strum::IntoEnumIterator;

pub struct ImageOptions;

impl ImageOptions {
    pub fn statuses_options() -> Vec<Value> {
        let mut options = vec![];
        for variant in ImageStatus::iter() {
            let value = serde_json::to_string(&variant).unwrap().replace('"', "");
            let label = value.to_case(Case::Title);
            options.push(json!({ "value": value, "label": label }));
        }
        options
    }

    pub fn boolean_options() -> Vec<Value> {
        vec![
            json!({ "value": "true",  "label": "True"  }),
            json!({ "value": "false", "label": "False" }),
        ]
    }
}

#[derive(Debug, Clone)]
pub struct ImageResource;

#[async_trait]
impl AdmixResource for ImageResource {
    fn new() -> Self {
        ImageResource
    }

    fn resource_name(&self) -> &'static str {
        "Images"
    }

    fn base_path(&self) -> &'static str {
        "images"
    }

    fn collection_name(&self) -> &'static str {
        "images"
    }

    fn get_collection(&self) -> Collection<Document> {
        get_collection::<Document>("images")
    }

    fn clone_box(&self) -> Box<dyn AdmixResource> {
        Box::new(Self::new())
    }

    fn menu_group(&self) -> Option<&'static str> {
        Some("Management")
    }

    fn menu(&self) -> &'static str {
        "Images"
    }

    fn allowed_roles(&self) -> Vec<String> {
        vec!["admin".to_string(), "superadmin".to_string()]
    }

    fn supports_file_upload(&self) -> bool {
        true
    }
    
    fn max_file_size(&self) -> usize {
        5 * 1024 * 1024 // 5MB for images
    }
    
    fn allowed_file_extensions(&self) -> Vec<&'static str> {
        vec!["jpg", "jpeg", "png", "gif", "webp", "bmp", "pdf"]
    }
    
    fn permit_keys(&self) -> Vec<&'static str> {
        vec!["title", "image_url", "status", "deleted"]
    }
    
    // FIXED: Remove 'async' keyword and correct method signature
    fn process_file_upload(&self, field_name: &str, file_data: &[u8], filename: &str) -> BoxFuture<'static, Result<HashMap<String, String>, AdminxError>> {
        let filename = filename.to_string();
        let field_name = field_name.to_string();
        let file_data = file_data.to_vec();
        let data_size = file_data.len();
        
        Box::pin(async move {
            tracing::info!("Processing file upload for field: {}, filename: {}, size: {} bytes", 
                          field_name, filename, data_size);
            
            // Generate unique filename to avoid conflicts
            let timestamp = chrono::Utc::now().timestamp();
            let file_extension = filename.split('.').last().unwrap_or("jpg");
            let unique_filename = format!("images/{}_{}.{}", timestamp, field_name, file_extension);
            
            // Use your actual S3 upload utility
            match crate::utils::s3_util::upload_image_to_s3(unique_filename.clone(), file_data).await {
                Ok(public_url) => {
                    let mut urls = HashMap::new();
                    urls.insert("image_url".to_string(), public_url);
                    
                    tracing::info!("File uploaded successfully to S3: {}", unique_filename);
                    Ok(urls)
                }
                Err(e) => {
                    tracing::error!("S3 upload failed for {}: {}", unique_filename, e);
                    Err(AdminxError::InternalError)
                }
            }
        })
    }

    

    // ===========================
    // UI STRUCTURE OVERRIDES
    // ===========================
    fn form_structure(&self) -> Option<Value> {
        Some(json!({
            "groups": [
                {
                    "title": "Image Details",
                    "fields": [
                        {
                            "name": "title",
                            "field_type": "text",
                            "label": "Image Title",
                            "value": "",
                            "required": true,
                            "help_text": "Enter a descriptive title for the image"
                        },
                        {
                            "name": "image_file",
                            "field_type": "file",
                            "label": "Upload Image",
                            "accept": "image/*",
                            "required": true,
                            "help_text": "Upload an image file (JPG, PNG, GIF, WebP). Maximum size: 5MB."
                        },
                        {
                            "name": "status",
                            "field_type": "select", 
                            "label": "Status",
                            "value": "active",
                            "required": true,
                            "options": ImageOptions::statuses_options(),
                            "help_text": "Set the image status"
                        },
                        {
                            "name": "deleted",
                            "field_type": "boolean", 
                            "label": "Mark as Deleted",
                            "value": "false",
                            "required": false,
                            "options": ImageOptions::boolean_options(),
                            "help_text": "Mark this image as deleted (soft delete)"
                        }
                    ]
                }
            ]
        }))
    }

    fn list_structure(&self) -> Option<Value> {
        Some(json!({
            "columns": [
                {
                    "field": "title",
                    "label": "Title",
                    "sortable": true
                },
                {
                    "field": "image_url", 
                    "label": "Image URL",
                    "sortable": false,
                    "type": "url"
                },
                {
                    "field": "status",
                    "label": "Status",
                    "sortable": true,
                    "type": "badge"
                },
                {
                    "field": "deleted",
                    "label": "Deleted",
                    "sortable": true,
                    "type": "boolean"
                },
                {
                    "field": "created_at",
                    "label": "Created At",
                    "type": "datetime",
                    "sortable": true
                }
            ],
            "actions": ["view", "edit", "delete"]
        }))
    }

    fn view_structure(&self) -> Option<Value> {
        Some(json!({
            "sections": [
                {
                    "title": "Image Information",
                    "fields": [
                        {
                            "field": "title",
                            "label": "Title"
                        },
                        {
                            "field": "image_url",
                            "label": "Image URL",
                            "type": "url"
                        },
                        {
                            "field": "status",
                            "label": "Status",
                            "type": "badge"
                        },
                        {
                            "field": "deleted",
                            "label": "Deleted",
                            "type": "boolean"
                        }
                    ]
                },
                {
                    "title": "System Information",
                    "fields": [
                        {
                            "field": "_id",
                            "label": "Image ID"
                        },
                        {
                            "field": "created_at",
                            "label": "Created At",
                            "type": "datetime"
                        },
                        {
                            "field": "updated_at", 
                            "label": "Updated At",
                            "type": "datetime"
                        }
                    ]
                }
            ]
        }))
    }

    fn filters(&self) -> Option<Value> {
        Some(json!({
            "title": "Image Filters",
            "filters": [
                {
                    "field": "title",
                    "type": "text",
                    "label": "Title",
                    "placeholder": "Search by title..."
                },
                {
                    "field": "status",
                    "type": "select",
                    "label": "Status",
                    "options": ImageOptions::statuses_options(),
                },
                {
                    "field": "deleted",
                    "type": "boolean",
                    "label": "Show Deleted",
                    "options": ImageOptions::boolean_options(),
                },
                {
                    "field": "created_at",
                    "type": "date_range",
                    "label": "Created Date"
                }
            ]
        }))
    }

    // ===========================
    // CUSTOM ACTIONS (Optional)
    // ===========================
    fn custom_actions(&self) -> Vec<adminx::actions::CustomAction> {
        vec![
            adminx::actions::CustomAction {
                name: "toggle_status",
                method: "POST",
                handler: |req, _path, _body| {
                    let image_id = req.match_info().get("id").unwrap_or("unknown").to_string();

                    Box::pin(async move {
                        tracing::info!("Toggling status for image: {}", image_id);
                        
                        // TODO: Implement actual status toggle logic
                        actix_web::HttpResponse::Ok().json(serde_json::json!({
                            "success": true,
                            "message": format!("Image {} status toggled", image_id)
                        }))
                    })
                },
            },
        ]
    }
}
```


### 4. Set up Your Application

```rust
// src/main.rs
use actix_web::{web, App, HttpServer, middleware::Logger};
use dotenv::dotenv;
use std::env;
use crate::dbs::mongo::init_mongo_client;
use crate::admin::initializer::AdminxInitializer;

mod dbs;
mod admin;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    println!("Initializing database connection...");
    let db = init_mongo_client().await;
    
    // Initialize AdminX components using the initializer
    let adminx_config = AdminxInitializer::initialize(db.clone()).await;
    
    let server_address = env::var("SERVER_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(adminx_config.clone()))
            .wrap(Logger::default())
            .wrap(AdminxInitializer::get_session_middleware(&adminx_config))
            .service(AdminxInitializer::get_routes_service())
    })
    .bind(server_address)?
    .run()
    .await
}
```

### 5. Environment Variables

Create a `.env` file:

```env
JWT_SECRET=your-super-secret-jwt-key-minimum-32-characters
SESSION_SECRET=your-session-secret-key-must-be-at-least-64-characters-long
ENVIRONMENT=development
RUST_LOG=debug
```


### 5. Create admin username and password

```bash
cargo install adminx

# Patch Environment Variables
export MONGODB_URL="mongodb://localhost:27017"
export ADMINX_DB_NAME="adminx"
adminx create -u admin -e admin@example.com -y
```


### 6. Start your application

```bash
cargo run

Visit `http://localhost:8080/adminx` and log in with credentails created in step 5:
```



## 📖 Documentation

### Resource Customization

```rust
impl AdmixResource for UserResource {
    // ... basic implementation ...
    
    // Custom validation before create
    fn create(&self, req: &HttpRequest, mut payload: Value) -> BoxFuture<'static, HttpResponse> {
        let collection = self.get_collection();
        
        Box::pin(async move {
            // Custom validation
            if let Some(email) = payload.get("email").and_then(|e| e.as_str()) {
                if email.is_empty() {
                    return HttpResponse::BadRequest().json(json!({
                        "error": "Email is required"
                    }));
                }
            }
            
            // Add timestamp
            payload["created_at"] = json!(mongodb::bson::DateTime::now());
            
            // Call default create logic or implement custom logic
            // ... your custom create logic here
        })
    }
    
    // Custom search filters
    fn filters(&self) -> Option<Value> {
        Some(json!({
            "filters": [
                {"field": "name", "label": "Name", "type": "text"},
                {"field": "email", "label": "Email", "type": "text"},
                {"field": "age", "label": "Age", "type": "range"}
            ]
        }))
    }
    
    // Custom actions
    fn custom_actions(&self) -> Vec<CustomAction> {
        vec![
            CustomAction {
                name: "activate",
                method: "POST",
                handler: |_req, path, _body| {
                    Box::pin(async move {
                        let id = path.into_inner();
                        // Custom activation logic
                        HttpResponse::Ok().json(json!({
                            "message": format!("User {} activated", id)
                        }))
                    })
                }
            }
        ]
    }
}
```

### Advanced Configuration

```rust
// Custom middleware and advanced setup
use adminx::middleware::RoleGuardMiddleware;

HttpServer::new(move || {
    App::new()
        .app_data(web::Data::new(config.clone()))
        .wrap(get_adminx_session_middleware(&config))
        .wrap(Logger::default())
        // Custom routes before AdminX
        .route("/api/health", web::get().to(health_check))
        // AdminX routes with custom middleware
        .service(
            web::scope("/admin")
                .wrap(RoleGuard::admin_only())
                .service(register_all_admix_routes())
        )
        // Your app routes
        .service(web::scope("/api").service(your_api_routes()))
})
```


### Cli Configuration

# Use environment variables
```rust
export MONGODB_URL="mongodb://localhost:27017"
export ADMINX_DB_NAME="adminx"
adminx create -u admin -e admin@example.com -y
```

# Use command line arguments
```rust
adminx --mongodb-url "mongodb://localhost:27017" --database-name "adminx" list
```

# Interactive mode (will prompt for connection details)
```rust
adminx create -u newuser -e user@example.com
```

# Quick setup with defaults (localhost:27017, database: adminx)
```rust
adminx --mongodb-url "mongodb+srv://username:password@mongo-atlas-cluster.mongodb.net/?retryWrites=true&w=majority&appName=cluster-name" --database-name "dbname" create -u admin -e admin@srotas.space -p password -y
```


## 🎯 Examples

Check out the `examples/` directory for complete working examples:

- **[Basic CRUD](examples/basic-crud/)** - Simple blog with posts and users
- **[E-commerce Admin](examples/ecommerce/)** - Products, orders, and customers
- **[Multi-tenant SaaS](examples/saas/)** - Organizations and user management
- **[Custom Authentication](examples/custom-auth/)** - OAuth integration
- **[File Uploads](examples/file-uploads/)** - Image and document management

## 🔧 Available Features

### Resource Trait Methods

| Method | Purpose | Required |
|--------|---------|----------|
| `resource_name()` | Display name | ✅ |
| `base_path()` | URL path segment | ✅ |
| `collection_name()` | MongoDB collection | ✅ |
| `get_collection()` | Database connection | ✅ |
| `clone_box()` | Resource cloning | ✅ |
| `permit_params()` | Allowed fields | ⚪ |
| `allowed_roles()` | RBAC permissions | ⚪ |
| `form_structure()` | Custom forms | ⚪ |
| `list_structure()` | Table customization | ⚪ |
| `custom_actions()` | Additional endpoints | ⚪ |

### Built-in Routes

Each registered resource automatically gets:

| Route | Method | Purpose |
|-------|--------|---------|
| `/adminx/{resource}/list` | GET | List view (HTML) |
| `/adminx/{resource}/new` | GET | Create form (HTML) |
| `/adminx/{resource}/view/{id}` | GET | Detail view (HTML) |
| `/adminx/{resource}/edit/{id}` | GET | Edit form (HTML) |
| `/adminx/{resource}/create` | POST | Create handler |
| `/adminx/{resource}/update/{id}` | POST | Update handler |
| `/adminx/{resource}` | GET | List API (JSON) |
| `/adminx/{resource}` | POST | Create API (JSON) |
| `/adminx/{resource}/{id}` | GET | Get API (JSON) |
| `/adminx/{resource}/{id}` | PUT | Update API (JSON) |
| `/adminx/{resource}/{id}` | DELETE | Delete API (JSON) |

## 🔒 Security

AdminX includes comprehensive security features:

### Authentication & Authorization

```rust
// Role-based access control
fn allowed_roles(&self) -> Vec<String> {
    vec!["admin".to_string(), "moderator".to_string()]
}

// Fine-grained permissions
fn allowed_roles_with_permissions(&self) -> Value {
    json!({
        "admin": ["create", "read", "update", "delete"],
        "moderator": ["create", "read", "update"],
        "user": ["read"]
    })
}
```

### Rate Limiting

Built-in rate limiting protects against brute force attacks:

```rust
// Automatic rate limiting in auth controller
// 5 attempts per 15 minutes per email
if is_rate_limited(email, 5, Duration::from_secs(900)) {
    return HttpResponse::TooManyRequests()
        .body("Too many login attempts. Please try again later.");
}
```

## 🎨 UI Customization

### Themes and Styling

AdminX uses TailwindCSS with built-in dark mode support:

```html
<!-- Automatic dark mode toggle in header -->
<div class="flex gap-2">
  <label><input type="radio" name="theme" value="light" onchange="setTheme(this.value)" /> Light</label>
  <label><input type="radio" name="theme" value="dark" onchange="setTheme(this.value)" /> Dark</label>
</div>
```

### Custom Templates

Override default templates by providing your own:

```rust
// Custom template helper
pub async fn render_custom_template(template_name: &str, ctx: Context) -> HttpResponse {
    // Your custom template logic
}
```

## 🧪 Testing

AdminX includes comprehensive test utilities:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use adminx::test_utils::*;
    
    #[tokio::test]
    async fn test_user_resource_crud() {
        let resource = UserResource::new();
        let test_db = setup_test_database().await;
        
        // Test create
        let user_data = json!({
            "name": "Test User",
            "email": "test@example.com",
            "age": 25
        });
        
        let response = resource.create(&test_request(), user_data).await;
        assert!(response.status().is_success());
        
        // Test list
        let response = resource.list(&test_request(), "".to_string()).await;
        assert!(response.status().is_success());
    }
}
```

## 📊 Performance

### Database Optimization

```rust
// Automatic indexing for common queries
impl AdmixResource for UserResource {
    async fn setup_indexes(&self) -> Result<(), mongodb::error::Error> {
        let collection = self.get_collection();
        
        collection.create_index(
            mongodb::IndexModel::builder()
                .keys(doc! { "email": 1 })
                .options(mongodb::options::IndexOptions::builder()
                    .unique(true)
                    .build())
                .build(),
            None
        ).await?;
        
        Ok(())
    }
}
```

### Caching

```rust
// Built-in response caching (optional)
fn cache_duration(&self) -> Option<Duration> {
    Some(Duration::from_secs(300)) // 5 minutes
}
```


## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
git clone https://github.com/xsmmaurya/adminx.git
cd adminx
cargo build
cargo test
```

### Running Examples

```bash
cd examples/basic-crud
cargo run
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- Built with [Actix Web](https://actix.rs/) - Fast, powerful web framework
- UI powered by [TailwindCSS](https://tailwindcss.com/) - Utility-first CSS framework
- Templates with [Tera](https://tera.netlify.app/) - Jinja2-inspired template engine
- Database with [MongoDB](https://www.mongodb.com/) - Document database
- Schemas with [Schemars](https://crates.io/crates/schemars) - JSON Schema generation

---

## 📞 Support

- 📖 [How it works](https://adminx.srotas.space/get-started)
- 📖 [Support](https://adminx.srotas.space/support)
- 📖 [Documentation](https://docs.rs/adminx)
- 💬 [Discussions](https://github.com/srotas-space/adminx/discussions)
- 🐛 [Issues](https://github.com/srotas-space/adminx/issues)
- 📧 Email: xsmmaurya@gmail.com
- 📧 Email: deepxmaurya@gmail.com

---

Made with ❤️ by the Rustacean360 Team

## 👥 Contributors

- **[Snm Maurya](https://github.com/xsmmaurya)** - Creator & Lead Developer  
  <img src="https://srotas-space.s3.ap-south-1.amazonaws.com/snm.jpg" alt="Snm Maurya" width="80" height="80" style="border-radius: 50%;">  
  [LinkedIn](https://www.linkedin.com/in/xsmmaurya/)

- **[Deepak Maurya](https://github.com/deepxmaurya)** - Core Developer & Contributor  
  <img src="https://srotas-space.s3.ap-south-1.amazonaws.com/srotas-icon-1024.png" alt="Deepak Maurya" width="80" height="80" style="border-radius: 50%;">  
  [LinkedIn](https://www.linkedin.com/in/deepxmaurya/)


[![GitHub stars](https://img.shields.io/github/stars/srotas-space/adminx?style=social)](https://github.com/srotas-space/adminx)



## 🗺️ Roadmap

We are actively building AdminX step by step.  
The roadmap includes phases like core CRUD foundation, extended resource features, authentication & RBAC, export/import, custom pages, UI themes, and optional extensions.

👉 See the full roadmap here: [ROADMAP.md](./ROADMAP.md)

[![Project Status](https://img.shields.io/badge/status-actively--developed-brightgreen.svg)](https://github.com/srotas-space/adminx)
[![Contributions Welcome](https://img.shields.io/badge/contributions-welcome-blue.svg)](https://github.com/srotas-space/adminx/issues)

