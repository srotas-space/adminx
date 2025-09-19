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

### 1. Define Your Resource

```rust
use adminx::prelude::*;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use mongodb::{Collection, bson::Document};

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<mongodb::bson::oid::ObjectId>,
    
    #[schemars(regex = "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$")]
    pub email: String,
    
    pub name: String,
    
    #[schemars(range(min = 18, max = 120))]
    pub age: Option<u32>,
    
    pub created_at: Option<mongodb::bson::DateTime>,
}

pub struct UserResource;

impl AdmixResource for UserResource {
    fn new() -> Self { Self }
    
    fn resource_name(&self) -> &'static str { "User" }
    fn base_path(&self) -> &'static str { "users" }
    fn collection_name(&self) -> &'static str { "users" }
    
    fn get_collection(&self) -> Collection<Document> {
        use adminx::utils::database::get_adminx_database;
        get_adminx_database().collection(self.collection_name())
    }
    
    fn clone_box(&self) -> Box<dyn AdmixResource> {
        Box::new(self.clone())
    }
    
    // Specify which fields can be created/updated
    fn permit_keys(&self) -> Vec<&'static str> {
        vec!["email", "name", "age"]
    }
    
    // Define allowed roles
    fn allowed_roles(&self) -> Vec<String> {
        vec!["admin".to_string(), "moderator".to_string()]
    }
    
    // Optional: Custom form structure
    fn form_structure(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "groups": [
                {
                    "title": "User Details",
                    "fields": [
                        {"name": "name", "label": "Full Name", "type": "text", "required": true},
                        {"name": "email", "label": "Email Address", "type": "email", "required": true},
                        {"name": "age", "label": "Age", "type": "number", "required": false}
                    ]
                }
            ]
        }))
    }
}
```

### 2. Set up Your Application

```rust
use actix_web::{web, App, HttpServer, middleware::Logger};
use adminx::prelude::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load configuration
    let config = get_adminx_config();
    setup_adminx_logging(&config);
    
    // Connect to MongoDB
    let client = mongodb::Client::with_uri_str("mongodb://localhost:27017").await.unwrap();
    let database = client.database("adminx_demo");
    
    // Initialize AdminX
    adminx_initialize(database.clone()).await.unwrap();
    
    // Register your resources
    adminx::register_resource(Box::new(UserResource::new()));
    
    // Create admin user (optional)
    use adminx::utils::auth::{initiate_auth, NewAdminxUser, AdminxStatus};
    let admin_user = NewAdminxUser {
        username: "admin".to_string(),
        email: "admin@example.com".to_string(),
        password: "secure_password".to_string(),
        status: AdminxStatus::Active,
        delete: false,
    };
    initiate_auth(admin_user).await.ok();
    
    println!("🚀 Starting AdminX server at http://localhost:8080");
    println!("📱 Admin panel: http://localhost:8080/adminx");
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .wrap(get_adminx_session_middleware(&config))
            .wrap(Logger::default())
            .service(register_all_admix_routes())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### 3. Environment Variables

Create a `.env` file:

```env
JWT_SECRET=your-super-secret-jwt-key-minimum-32-characters
SESSION_SECRET=your-session-secret-key-must-be-at-least-64-characters-long
ENVIRONMENT=development
RUST_LOG=debug
```

### 4. Run Your Application

```bash
cargo run
```

Visit `http://localhost:8080/adminx` and log in with:
- **Email**: `admin@example.com`
- **Password**: `secure_password`

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

