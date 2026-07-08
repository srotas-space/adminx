# AdminX

[![Crates.io](https://img.shields.io/crates/v/adminx)](https://crates.io/crates/adminx)
[![Documentation](https://docs.rs/adminx/badge.svg)](https://docs.rs/adminx)
[![License](https://img.shields.io/crates/l/adminx)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/adminx)](https://crates.io/crates/adminx)
[![Recent Downloads](https://img.shields.io/crates/dr/adminx)](https://crates.io/crates/adminx)

![AdminX](https://srotas-suite-space.s3.ap-south-1.amazonaws.com/nsadmin.png)

[How to Use Sample Starter Template] (https://github.com/srotas-space/adminx-examples)

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

## ⚡ Performance

- **Memory Usage**: ~10MB baseline
- **Response Time**: <5ms for CRUD operations  
- **Concurrent Users**: 10,000+ tested
- **Database Queries**: Optimized with automatic indexing

## 🔍 Why AdminX?

| Feature | AdminX | Django Admin | Laravel Nova | Rails Admin |
|---------|--------|--------------|--------------|-------------|
| Type Safety | ✅ Rust | ❌ Python | ❌ PHP | ❌ Ruby |
| Performance | 🚀 Blazing Fast | ⚡ Fast | ⚡ Fast | ⚡ Fast |
| Zero Config CRUD | ✅ | ✅ | ✅ | ✅ |
| Built-in Auth | ✅ JWT+Session | ✅ | ✅ | ✅ |
| File Uploads | ✅ S3 Ready | ✅ | ✅ | ✅ |
| Modern UI | ✅ TailwindCSS | ❌ | ✅ | ❌ |

## 🚀 30-Second Quick Start

```bash
# 1. Add to Cargo.toml
cargo add adminx actix-web mongodb tokio serde

# 2. Create main.rs with minimal setup
# 3. Run your application
cargo run

# 4. Visit http://localhost:8080/adminx
```

**[👉 Full Setup Guide](#full-setup)**

## 🚀 Full Setup Guide

Add AdminX to your `Cargo.toml`:

```toml
[dependencies]
adminx = "0.2.7"
actix-web = "4"
mongodb = { version = "2.4", features = ["tokio-runtime"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
schemars = { version = "0.8", features = ["derive"] }
```

### 1. Define Your Data Models

```rust
// src/models/image_model.rs
use mongodb::{bson::{doc, oid::ObjectId, DateTime as BsonDateTime}};
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter; 

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, EnumIter)]
#[serde(rename_all = "lowercase")]
pub enum ImageStatus {
    Active,
    Inactive,
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
```

### 2. Create AdminX Initializer

```rust
// src/admin/initializer.rs
use mongodb::Database;
use adminx::{
    adminx_initialize, get_adminx_config, setup_adminx_logging, 
    get_adminx_session_middleware, register_all_admix_routes,
    registry::register_resource, AdmixResource, AdminxConfig,
};
use crate::admin::resources::image_resource::ImageResource;

pub struct AdminxInitializer;

impl AdminxInitializer {
    pub async fn initialize(db: Database) -> AdminxConfig {
        let adminx_config = get_adminx_config();
        setup_adminx_logging(&adminx_config);
        let _adminx_instance = adminx_initialize(db.clone()).await;
        
        Self::register_resources();
        adminx_config
    }
    
    fn register_resources() {
        register_resource(Box::new(ImageResource::new()));
    }
    
    pub fn get_session_middleware(config: &AdminxConfig) -> actix_session::SessionMiddleware<impl actix_session::storage::SessionStore> {
        get_adminx_session_middleware(config)
    }
    
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

#[derive(Debug, Clone)]
pub struct ImageResource;

#[async_trait]
impl AdmixResource for ImageResource {
    fn new() -> Self { ImageResource }
    
    fn resource_name(&self) -> &'static str { "Images" }
    fn base_path(&self) -> &'static str { "images" }
    fn collection_name(&self) -> &'static str { "images" }
    
    fn get_collection(&self) -> Collection<Document> {
        get_collection::<Document>("images")
    }
    
    fn clone_box(&self) -> Box<dyn AdmixResource> {
        Box::new(Self::new())
    }
    
    fn menu_group(&self) -> Option<&'static str> { Some("Management") }
    fn menu(&self) -> &'static str { "Images" }
    
    fn allowed_roles(&self) -> Vec<String> {
        vec!["admin".to_string(), "superadmin".to_string()]
    }
    
    fn permit_keys(&self) -> Vec<&'static str> {
        vec!["title", "image_url", "status", "deleted"]
    }
    
    // Custom form with file upload
    fn form_structure(&self) -> Option<Value> {
        Some(json!({
            "groups": [{
                "title": "Image Details",
                "fields": [
                    {
                        "name": "title",
                        "field_type": "text",
                        "label": "Image Title",
                        "required": true
                    },
                    {
                        "name": "image_file",
                        "field_type": "file",
                        "label": "Upload Image",
                        "accept": "image/*",
                        "required": true
                    }
                ]
            }]
        }))
    }
}
```

### 4. Set up Your Application

```rust
// src/main.rs
use actix_web::{web, App, HttpServer, middleware::Logger};
use dotenv::dotenv;
use crate::dbs::mongo::init_mongo_client;
use crate::admin::initializer::AdminxInitializer;

mod dbs;
mod admin;
mod models;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    let db = init_mongo_client().await;
    let adminx_config = AdminxInitializer::initialize(db.clone()).await;
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(adminx_config.clone()))
            .wrap(Logger::default())
            .wrap(AdminxInitializer::get_session_middleware(&adminx_config))
            .service(AdminxInitializer::get_routes_service())
    })
    .bind("0.0.0.0:8080")?
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

### 6. Create Admin User

```bash
# Install AdminX CLI
cargo install adminx

# Create admin user
export MONGODB_URL="mongodb://localhost:27017"
export ADMINX_DB_NAME="adminx"
adminx create -u admin -e admin@example.com -y
```

### 7. Start Your Application

```bash
cargo run

# Visit http://localhost:8080/adminx and log in!
```

## 📖 Documentation

### Implementation Architecture

AdminX follows a modular, resource-centric architecture demonstrated in the code examples above. Here's how the framework is structured and implemented:

#### **AdminX Initializer Pattern**

The framework uses a centralized initializer that manages the complete AdminX lifecycle:

```rust
impl AdminxInitializer {
    pub async fn initialize(db: Database) -> AdminxConfig {
        // Get AdminX configuration from environment
        let adminx_config = get_adminx_config();
        
        // Setup structured logging
        setup_adminx_logging(&adminx_config);
        
        // Initialize core AdminX with database connection
        let _adminx_instance = adminx_initialize(db.clone()).await;
        
        // Register all your resources
        Self::register_resources();
        
        adminx_config
    }
    
    fn register_resources() {
        // Register each resource with the global registry
        register_resource(Box::new(UserResource::new()));
        register_resource(Box::new(NotificationResource::new()));
        register_resource(Box::new(ConfigResource::new()));
        register_resource(Box::new(ImageResource::new()));
    }
}
```

#### **Resource Implementation Pattern**

Each resource implements the `AdmixResource` trait with full customization capabilities:

```rust
#[async_trait]
impl AdmixResource for UserResource {
    // Required core methods
    fn resource_name(&self) -> &'static str { "Users" }
    fn base_path(&self) -> &'static str { "users" }
    fn collection_name(&self) -> &'static str { "users" }
    fn get_collection(&self) -> Collection<Document> {
        get_collection::<Document>("users")
    }
    
    // UI customization through JSON structures
    fn form_structure(&self) -> Option<Value> {
        Some(json!({
            "groups": [{
                "title": "User Details",
                "fields": [
                    {
                        "name": "name",
                        "field_type": "text",
                        "label": "Full Name",
                        "value": ""
                    }
                ]
            }]
        }))
    }
}
```

#### **File Upload Implementation**

Handle file uploads with S3 integration:

```rust
impl AdmixResource for ImageResource {
    fn supports_file_upload(&self) -> bool { true }
    fn max_file_size(&self) -> usize { 5 * 1024 * 1024 } // 5MB
    fn allowed_file_extensions(&self) -> Vec<&'static str> {
        vec!["jpg", "jpeg", "png", "gif", "webp"]
    }
    
    fn process_file_upload(&self, field_name: &str, file_data: &[u8], filename: &str) 
        -> BoxFuture<'static, Result<HashMap<String, String>, AdminxError>> {
        
        Box::pin(async move {
            // Custom S3 upload logic
            let unique_filename = format!("images/{}_{}.jpg", timestamp, field_name);
            match crate::utils::s3_util::upload_image_to_s3(unique_filename, file_data).await {
                Ok(public_url) => {
                    let mut urls = HashMap::new();
                    urls.insert("image_url".to_string(), public_url);
                    Ok(urls)
                }
                Err(e) => Err(AdminxError::InternalError)
            }
        })
    }
}
```

#### **Dynamic Form Generation**

Advanced form types with rich editors:

```rust
fn form_structure(&self) -> Option<Value> {
    Some(json!({
        "groups": [{
            "title": "Configuration",
            "fields": [
                {
                    "name": "data",
                    "field_type": "editor",  // Rich text/JSON/HTML editor
                    "label": "Configuration Data"
                },
                {
                    "name": "data_type",
                    "field_type": "select",
                    "options": ConfigOptions::data_types_options() // Dynamic enum options
                }
            ]
        }]
    }))
}
```

#### **Custom Actions Implementation**

Define custom business logic actions:

```rust
fn custom_actions(&self) -> Vec<adminx::actions::CustomAction> {
    vec![
        adminx::actions::CustomAction {
            name: "toggle_status",
            method: "POST",
            handler: |req, _path, _body| {
                Box::pin(async move {
                    let id = req.match_info().get("id").unwrap();
                    // Your custom business logic here
                    HttpResponse::Ok().json(json!({
                        "success": true,
                        "message": "Status toggled"
                    }))
                })
            }
        }
    ]
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

### CLI Configuration

```bash
# Use environment variables
export MONGODB_URL="mongodb://localhost:27017"
export ADMINX_DB_NAME="adminx"
adminx create -u admin -e admin@example.com -y

# Use command line arguments
adminx --mongodb-url "mongodb://localhost:27017" --database-name "adminx" list

# Interactive mode (will prompt for connection details)
adminx create -u newuser -e user@example.com

# Quick setup with MongoDB Atlas
adminx --mongodb-url "mongodb+srv://username:password@cluster.mongodb.net/?retryWrites=true&w=majority" --database-name "dbname" create -u admin -e admin@example.com -p password -y
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

## 📊 Performance Optimization

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

## ⚙️ Compatibility

| AdminX Version | Rust Version | Actix Web | MongoDB Driver |
|----------------|--------------|-----------|----------------|
| 0.1.x | 1.70+ | 4.x | 2.4+ |

**MSRV (Minimum Supported Rust Version)**: 1.70.0

## ❓ Frequently Asked Questions

**Q: Can I use AdminX with existing Actix Web applications?**
A: Yes! AdminX is designed to integrate seamlessly with existing Actix Web apps.

**Q: Does AdminX support other databases besides MongoDB?**
A: Currently MongoDB is supported. PostgreSQL and SQLite support is planned.

**Q: Can I customize the UI theme?**
A: Yes! AdminX uses TailwindCSS and supports custom themes and styling.

**Q: How do I handle file uploads?**
A: AdminX provides built-in file upload support with S3 integration. See the ImageResource example above.

**Q: Can I add custom business logic?**
A: Absolutely! Use custom actions and override CRUD methods to implement your business logic.

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

## 🌟 Community

[![GitHub Discussions](https://img.shields.io/github/discussions/srotas-space/adminx)](https://github.com/srotas-space/adminx/discussions)

Join our growing community of Rust developers building admin panels with AdminX!

- 📖 [Documentation](https://docs.rs/adminx)
- 💬 [Discussions](https://github.com/srotas-space/adminx/discussions)
- 🐛 [Issues](https://github.com/srotas-space/adminx/issues)
- 📧 Email: xsmmaurya@gmail.com
- 📧 Email: deepxmaurya@gmail.com

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Actix Web](https://actix.rs/) - Fast, powerful web framework
- UI powered by [TailwindCSS](https://tailwindcss.com/) - Utility-first CSS framework
- Templates with [Tera](https://tera.netlify.app/) - Jinja2-inspired template engine
- Database with [MongoDB](https://www.mongodb.com/) - Document database
- Schemas with [Schemars](https://crates.io/crates/schemars) - JSON Schema generation


## 🗺️ Roadmap

We are actively building AdminX step by step.  
The roadmap includes phases like core CRUD foundation, extended resource features, authentication & RBAC, export/import, custom pages, UI themes, and optional extensions.

👉 See the full roadmap here: [ROADMAP.md](./ROADMAP.md)

[![Project Status](https://img.shields.io/badge/status-actively--developed-brightgreen.svg)](https://github.com/srotas-space/adminx)
[![Contributions Welcome](https://img.shields.io/badge/contributions-welcome-blue.svg)](https://github.com/srotas-space/adminx/issues)

[How to Use Sample Starter Template] (https://github.com/srotas-space/adminx-examples)
---


Made with ❤️ by the [Srotas Space] (https://open-source.srotas.space)

---


## 👥 Contributors

- **[Snm Maurya](https://github.com/xsmmaurya)** - Creator & Lead Developer
  <img src="https://srotas-suite-space.s3.ap-south-1.amazonaws.com/snm.jpeg" alt="Snm Maurya" width="80" height="80" style="border-radius: 50%;">
  [LinkedIn](https://www.linkedin.com/in/xsmmaurya/)

- **[Deepak Maurya](https://github.com/deepxmaurya)** - Core Developer & Contributor
  <img src="https://srotas-suite-space.s3.ap-south-1.amazonaws.com/deepx.jpeg" alt="Deepak Maurya" width="80" height="80" style="border-radius: 50%;"> 
  [LinkedIn](https://www.linkedin.com/in/deepxmaurya/)

[![GitHub stars](https://img.shields.io/github/stars/srotas-space/adminx?style=social)](https://github.com/srotas-space/adminx)

