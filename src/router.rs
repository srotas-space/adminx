// crates/adminx/src/router.rs - Complete Fixed Version
use actix_web::{web, Scope};
use tracing::{info, warn};
use crate::registry::all_resources;
use crate::controllers::{
    resource_controller::{
        register_admix_resource_routes
    }
};
use crate::controllers::auth_controller::{
    login_form, 
    login_action, 
    logout_action, 
    dashboard_view,
    profile_view,
    api_login_action,
    check_auth_status
};
use crate::utils::{
    structs::{
        RoleGuard
    },
};


pub fn register_all_admix_routes() -> Scope {
    info!("🔧 Starting AdminX route registration...");
    
    let mut scope = web::scope("/adminx")
        // ===========================
        // AUTHENTICATION ROUTES
        // ===========================
        .route("/login", web::get().to(login_form))
        .route("/login", web::post().to(login_action))
        .route("/logout", web::get().to(logout_action))     // FIXED: Added GET support
        .route("/logout", web::post().to(logout_action))    // Keep POST support too
        
        // ===========================
        // DASHBOARD ROUTES
        // ===========================
        .route("", web::get().to(dashboard_view))           // FIXED: Use dashboard_view instead of adminx_home
        .route("/", web::get().to(dashboard_view))          // FIXED: Use dashboard_view instead of adminx_home
        .route("/dashboard", web::get().to(dashboard_view))
        
        // ===========================
        // PROFILE ROUTES
        // ===========================
        .route("/profile", web::get().to(profile_view))
        
        // ===========================
        // API ROUTES
        // ===========================
        .route("/api/login", web::post().to(api_login_action))
        .route("/api/auth/status", web::get().to(check_auth_status));

    // Debug: Check if we have any resources
    let resources = all_resources();
    info!("📋 Found {} resources to register", resources.len());
    
    if resources.is_empty() {
        warn!("⚠️  No resources found! Make sure you've called register_resource() before starting the server.");
        return scope;
    }

    // Register resource routes with role guards
    for resource in resources {
        let resource_name = resource.resource_name();
        let base_path = resource.base_path();
        let allowed_roles = resource.allowed_roles();
        
        info!("📝 Registering resource: '{}' at path: '{}'", resource_name, base_path);
        info!("🔐 Allowed roles for {}: {:?}", resource_name, allowed_roles);
        
        // Create the resource scope with the base path
        let resource_scope = web::scope(&format!("/{}", base_path))
            .service(register_admix_resource_routes(resource))
            .wrap(RoleGuard { allowed_roles });
        
        scope = scope.service(resource_scope);
        
        info!("✅ Successfully registered resource: '{}'", resource_name);
        info!("🌐 Available URLs:");
        info!("   - GET  /adminx/{}/list", base_path);
        info!("   - GET  /adminx/{}/new", base_path);
        info!("   - GET  /adminx/{}/view/{{id}}", base_path);
        info!("   - GET  /adminx/{}/edit/{{id}}", base_path);
        info!("   - GET  /adminx/{} (API list)", base_path);
        info!("   - POST /adminx/{} (API create)", base_path);
        info!("   - GET  /adminx/{}/{{id}} (API get)", base_path);
        info!("   - PUT  /adminx/{}/{{id}} (API update)", base_path);
        info!("   - DELETE /adminx/{}/{{id}} (API delete)", base_path);
    }
    
    info!("🎉 AdminX route registration completed!");
    scope
}

// Alternative version WITHOUT authentication middleware — for local testing only.
// Gated to debug builds so it cannot be compiled into (and accidentally shipped in)
// a release binary, where it would expose the entire admin surface unauthenticated.
#[cfg(debug_assertions)]
pub fn register_all_admix_routes_debug() -> Scope {
    warn!("🔧 AdminX route registration in DEBUG MODE - NO AUTH. Never use this in production.");
    
    let mut scope = web::scope("/adminx")
        // ===========================
        // AUTHENTICATION ROUTES (DEBUG)
        // ===========================
        .route("/login", web::get().to(login_form))
        .route("/login", web::post().to(login_action))
        .route("/logout", web::get().to(logout_action))     // FIXED: Added GET support
        .route("/logout", web::post().to(logout_action))    // Keep POST support too
        
        // ===========================
        // DASHBOARD ROUTES (DEBUG)
        // ===========================
        .route("", web::get().to(dashboard_view))           // FIXED: Use dashboard_view
        .route("/", web::get().to(dashboard_view))          // FIXED: Use dashboard_view
        .route("/dashboard", web::get().to(dashboard_view))
        
        // ===========================
        // PROFILE ROUTES (DEBUG)
        // ===========================
        .route("/profile", web::get().to(profile_view))
        
        // ===========================
        // API ROUTES (DEBUG)
        // ===========================
        .route("/api/login", web::post().to(api_login_action))
        .route("/api/auth/status", web::get().to(check_auth_status));

    // Debug: Check if we have any resources
    let resources = all_resources();
    info!("📋 Found {} resources to register", resources.len());
    
    if resources.is_empty() {
        warn!("⚠️  No resources found! Make sure you've called register_resource() before starting the server.");
        return scope;
    }

    // Register resource routes WITHOUT role guards for debugging
    for resource in resources {
        let resource_name = resource.resource_name();
        let base_path = resource.base_path();
        
        info!("📝 Registering resource: '{}' at path: '{}'", resource_name, base_path);
        
        // Create the resource scope with the base path - NO MIDDLEWARE
        let resource_scope = web::scope(&format!("/{}", base_path))
            .service(register_admix_resource_routes(resource));
        
        scope = scope.service(resource_scope);
        
        info!("✅ Successfully registered resource: '{}'", resource_name);
    }
    
    info!("🎉 AdminX route registration completed (DEBUG MODE)!");
    scope
}

// Helper function to register auth routes only (for separate registration)
pub fn register_auth_routes_only() -> Scope {
    web::scope("/adminx")
        .route("/login", web::get().to(login_form))
        .route("/login", web::post().to(login_action))
        .route("/logout", web::get().to(logout_action))
        .route("/logout", web::post().to(logout_action))
        .route("", web::get().to(dashboard_view))
        .route("/", web::get().to(dashboard_view))
        .route("/dashboard", web::get().to(dashboard_view))
        .route("/profile", web::get().to(profile_view))
        .route("/api/login", web::post().to(api_login_action))
        .route("/api/auth/status", web::get().to(check_auth_status))
}

// Helper function to register only resource routes (for separate registration)
pub fn register_resource_routes_only() -> Scope {
    info!("🔧 Starting AdminX resource-only route registration...");
    
    let mut scope = web::scope("/adminx");
    let resources = all_resources();
    
    info!("📋 Found {} resources to register", resources.len());
    
    for resource in resources {
        let resource_name = resource.resource_name();
        let base_path = resource.base_path();
        let allowed_roles = resource.allowed_roles();
        
        info!("📝 Registering resource: '{}' at path: '{}'", resource_name, base_path);
        
        let resource_scope = web::scope(&format!("/{}", base_path))
            .service(register_admix_resource_routes(resource))
            .wrap(RoleGuard { allowed_roles });
        
        scope = scope.service(resource_scope);
        
        info!("✅ Successfully registered resource: '{}'", resource_name);
    }
    
    info!("🎉 AdminX resource route registration completed!");
    scope
}

// Enhanced router with better error handling
pub fn register_all_admix_routes_enhanced() -> Scope {
    info!("🔧 Starting Enhanced AdminX route registration...");
    
    // First register auth routes
    let mut scope = web::scope("/adminx")
        // Auth routes with better organization
        .service(
            web::scope("/auth")
                .route("/login", web::get().to(login_form))
                .route("/login", web::post().to(login_action))
                .route("/logout", web::get().to(logout_action))
                .route("/logout", web::post().to(logout_action))
                .route("/status", web::get().to(check_auth_status))
        )
        // Main app routes
        .route("", web::get().to(dashboard_view))
        .route("/", web::get().to(dashboard_view))
        .route("/dashboard", web::get().to(dashboard_view))
        .route("/profile", web::get().to(profile_view))
        // Legacy auth routes (for backward compatibility)
        .route("/login", web::get().to(login_form))
        .route("/login", web::post().to(login_action))
        .route("/logout", web::get().to(logout_action))
        .route("/logout", web::post().to(logout_action))
        // API routes
        .service(
            web::scope("/api")
                .route("/login", web::post().to(api_login_action))
                .route("/auth/status", web::get().to(check_auth_status))
        );

    // Register resources
    let resources = all_resources();
    info!("📋 Found {} resources to register", resources.len());
    
    if resources.is_empty() {
        warn!("⚠️  No resources found!");
        return scope;
    }

    for resource in resources {
        let resource_name = resource.resource_name();
        let base_path = resource.base_path();
        let allowed_roles = resource.allowed_roles();
        
        info!("📝 Registering resource: '{}' at path: '{}' with roles: {:?}", 
              resource_name, base_path, allowed_roles);
        
        let resource_scope = web::scope(&format!("/{}", base_path))
            .service(register_admix_resource_routes(resource))
            .wrap(RoleGuard { allowed_roles });
        
        scope = scope.service(resource_scope);
    }
    
    info!("🎉 Enhanced AdminX route registration completed!");
    scope
}