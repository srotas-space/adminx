// crates/adminx/src/lib.rs - Fixed version

pub mod resource;
pub mod filters;
pub mod pagination;
pub mod error;
pub mod router;
pub mod menu;
pub mod registry;
pub mod health;
pub mod middleware;
pub mod nested;
pub mod utils;
pub mod actions;
pub mod helpers;
pub mod controllers;
pub mod configs;
pub mod models;
pub mod schemas;
pub mod errors;

// Re-export main types for easier importing
pub use schemas::adminx_schema::AdminxSchema;

// Export configuration and app creation functions
pub use configs::initializer::{
    get_adminx_config,
    setup_adminx_logging, 
    get_adminx_session_middleware,
    adminx_initialize,
    AdminxConfig
};

// Export commonly used utilities - ✅ FIXED: Use Claims from structs only
pub use utils::{
    jwt::create_jwt_token, // ✅ Don't export Claims from jwt
    auth::{extract_claims_from_session, AdminxStatus, NewAdminxUser, InitOutcome},
    structs::{LoginForm, RoleGuard, Claims}, // ✅ Export Claims from structs
};

// Export core traits and types
pub use resource::AdmixResource;

// Export models
pub use models::adminx_model::{AdminxUser, AdminxUserPublic};

// Export controllers for custom route registration
pub use controllers::{
    auth_controller::{login_form, login_action, logout_action},
    dashboard_controller::{adminx_home, adminx_stats, adminx_profile},
};

// Export router for custom integration
pub use router::register_all_admix_routes;

// Export template helpers
pub use helpers::template_helper::{
    render_template, 
    render_template_with_auth, 
    render_protected_template,
    render_404,
    render_403,
    render_500,
};

// Export middleware
pub use middleware::role_guard::RoleGuardMiddleware;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

// Convenience functions for common operations
pub mod prelude {
    pub use crate::{
        AdminxConfig,
        adminx_initialize,
        Claims, // ✅ This now comes from structs
        AdminxUser,
        LoginForm,
        RoleGuard,
        render_template,
        extract_claims_from_session,
        AdmixResource, // ✅ Added this for convenience
    };
}

// Configuration validation
pub fn validate_config() -> Result<(), Box<dyn std::error::Error>> {
    use std::env;
    
    // Check required environment variables
    let required_vars = vec![
        "JWT_SECRET",
        // Add other required vars here
    ];
    
    for var in required_vars {
        if env::var(var).is_err() {
            return Err(format!("Required environment variable {} is not set", var).into());
        }
    }
    
    Ok(())
}

// Health check function
pub async fn health_check() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use crate::utils::database::check_database_health;
    
    let db_healthy = check_database_health().await.unwrap_or(false);
    
    Ok(serde_json::json!({
        "status": if db_healthy { "healthy" } else { "unhealthy" },
        "version": VERSION,
        "name": NAME,
        "database": if db_healthy { "connected" } else { "disconnected" },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_version_info() {
        assert!(!VERSION.is_empty());
        assert_eq!(NAME, "adminx");
    }

    #[test]
    fn test_config_validation() {
        // Set test environment variables
        env::set_var("JWT_SECRET", "test_secret_that_is_long_enough_for_testing");
        
        // Should pass with required vars set
        assert!(validate_config().is_ok());
        
        // Clean up
        env::remove_var("JWT_SECRET");
        
        // Should fail without required vars
        assert!(validate_config().is_err());
    }

    #[test]
    fn test_claims_consistency() {
        // Test that Claims struct is accessible and consistent
        use crate::utils::structs::Claims;
        
        let claims = Claims {
            sub: "test_user".to_string(),
            exp: 1234567890,
            email: "test@example.com".to_string(),
            role: "admin".to_string(),
            roles: vec!["admin".to_string()],
        };
        
        assert_eq!(claims.role, "admin");
        assert_eq!(claims.email, "test@example.com");
        assert!(claims.roles.contains(&"admin".to_string()));
    }

    #[test]
    fn test_prelude_imports() {
        // Test that prelude imports work correctly
        use crate::prelude::*;
        
        // This should compile without errors, proving all exports are consistent
        let _config_exists = std::marker::PhantomData::<AdminxConfig>;
        let _claims_exists = std::marker::PhantomData::<Claims>;
        let _resource_exists = std::marker::PhantomData::<Box<dyn AdmixResource>>;
    }
}