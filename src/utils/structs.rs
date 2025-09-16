// adminx/src/utils/structs.rs - Cleaned version with no duplicates
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,         // Subject (e.g., user ID)
    pub exp: usize,          // Expiration (as timestamp)
    pub email: String,       // Email address
    pub role: String,        // Primary role (e.g., "admin")
    pub roles: Vec<String>,  // Additional roles for fine-grained permissions
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct RoleGuard {
    pub allowed_roles: Vec<String>,
}

impl RoleGuard {
    pub fn new(roles: Vec<&str>) -> Self {
        Self {
            allowed_roles: roles.iter().map(|&s| s.to_string()).collect(),
        }
    }
    
    pub fn from_strings(roles: Vec<String>) -> Self {
        Self {
            allowed_roles: roles,
        }
    }
    
    // ✅ REMOVED DUPLICATE METHODS - These are now only in middleware/role_guard.rs
    // The middleware file has the full implementation with better methods
}

// Additional utility structs
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub errors: Option<Vec<String>>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            errors: None,
        }
    }
    
    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
            errors: None,
        }
    }
    
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
            errors: None,
        }
    }
    
    pub fn error_with_details(message: String, errors: Vec<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
            errors: Some(errors),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>, // "asc" or "desc"
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(10),
            sort_by: None,
            sort_order: Some("desc".to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub current_page: u32,
    pub per_page: u32,
    pub total_items: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

// Session management structs
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub user_id: String,
    pub email: String,
    pub role: String,
    pub roles: Vec<String>,
    pub expires_at: i64,
    pub created_at: i64,
}

impl From<Claims> for SessionInfo {
    fn from(claims: Claims) -> Self {
        Self {
            user_id: claims.sub,
            email: claims.email,
            role: claims.role,
            roles: claims.roles,
            expires_at: claims.exp as i64,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

// Form validation structs
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationErrors {
    pub errors: Vec<ValidationError>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }
    
    pub fn add(&mut self, field: &str, message: &str) {
        self.errors.push(ValidationError {
            field: field.to_string(),
            message: message.to_string(),
        });
    }
    
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
    
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

// Flash message support
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlashMessage {
    pub level: FlashLevel,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FlashLevel {
    Success,
    Info,
    Warning,
    Error,
}

impl FlashMessage {
    pub fn success(message: &str) -> Self {
        Self {
            level: FlashLevel::Success,
            message: message.to_string(),
        }
    }
    
    pub fn info(message: &str) -> Self {
        Self {
            level: FlashLevel::Info,
            message: message.to_string(),
        }
    }
    
    pub fn warning(message: &str) -> Self {
        Self {
            level: FlashLevel::Warning,
            message: message.to_string(),
        }
    }
    
    pub fn error(message: &str) -> Self {
        Self {
            level: FlashLevel::Error,
            message: message.to_string(),
        }
    }
}