// adminx/src/controllers/auth_controller.rs
use actix_session::Session;
use actix_web::{web, HttpResponse, Responder};
use tera::Context;
use tracing::{error, info, warn};
use crate::helpers::template_helper::render_template;
use crate::models::adminx_model::get_admin_by_email;
use crate::registry::get_registered_menus;
use crate::utils::jwt::create_jwt_token;
use crate::utils::structs::LoginForm;
use crate::configs::initializer::AdminxConfig;
use crate::utils::auth::{is_rate_limited, reset_rate_limit, extract_claims_from_session};
use std::time::Duration;
use crate::helpers::auth_helper::{
    create_base_template_context_with_auth,
};


/// GET /adminx/login - Show login page
pub async fn login_form(
    session: Session,
    config: web::Data<AdminxConfig>,
) -> impl Responder {
    // Check if user is already authenticated
    if let Ok(_claims) = extract_claims_from_session(&session, &config).await {
        // User is already logged in, redirect to dashboard
        return HttpResponse::Found()
            .append_header(("Location", "/adminx"))
            .finish();
    }
    
    let mut ctx = Context::new();
    // Important: Set authentication status to false for login page
    ctx.insert("is_authenticated", &false);
    ctx.insert("page_title", "Login");
    // Don't insert menus for unauthenticated users
    render_template("login.html.tera", ctx).await
}

/// POST /adminx/login - Authenticate and store token in session
pub async fn login_action(
    form: web::Form<LoginForm>,
    session: Session,
    config: web::Data<AdminxConfig>,
) -> impl Responder {
    let email = form.email.trim();
    let password = form.password.trim();
    
    info!("Attempting login for: {}", email);
    
    // Input validation
    if email.is_empty() || password.is_empty() {
        warn!("Empty email or password for login attempt");
        let mut ctx = Context::new();
        ctx.insert("is_authenticated", &false);
        ctx.insert("error", "Email and password are required");
        return render_template("login.html.tera", ctx).await;
    }
    
    if !email.contains('@') {
        warn!("Invalid email format: {}", email);
        let mut ctx = Context::new();
        ctx.insert("is_authenticated", &false);
        ctx.insert("error", "Invalid email format");
        return render_template("login.html.tera", ctx).await;
    }
    
    // Rate limiting check
    if is_rate_limited(email, 5, Duration::from_secs(900)) {
        warn!("Rate limit exceeded for: {}", email);
        let mut ctx = Context::new();
        ctx.insert("is_authenticated", &false);
        ctx.insert("error", "Too many login attempts. Please try again later.");
        return render_template("login.html.tera", ctx).await;
    }
    
    // Dummy hash to prevent timing attacks
    let dummy_hash = "$2b$12$dummy.hash.to.prevent.timing.attacks.abcdefghijklmnopqrstuvwxy";
    
    match get_admin_by_email(email).await {
        Some(admin) => {
            if admin.verify_password(password) {
                let admin_id = match &admin.id {
                    Some(id) => id.to_string(),
                    None => {
                        error!("Admin has no ID: {}", email);
                        let mut ctx = Context::new();
                        ctx.insert("is_authenticated", &false);
                        ctx.insert("error", "Authentication failed - missing admin ID");
                        return render_template("login.html.tera", ctx).await;
                    }
                };
                
                // Use config for JWT creation
                match create_jwt_token(&admin_id, email, "admin", &config) {
                    Ok(token) => {
                        info!("Login successful for: {}", email);
                        
                        // Reset rate limit on successful login
                        reset_rate_limit(email);
                        
                        if let Err(err) = session.insert("admintoken", &token) {
                            error!("Session insertion failed: {}", err);
                            let mut ctx = Context::new();
                            ctx.insert("is_authenticated", &false);
                            ctx.insert("error", "Session creation failed");
                            return render_template("login.html.tera", ctx).await;
                        }

                        HttpResponse::Found()
                            .append_header(("Location", "/adminx"))
                            .finish()
                    }
                    Err(err) => {
                        error!("JWT generation failed for {}: {}", email, err);
                        let mut ctx = Context::new();
                        ctx.insert("is_authenticated", &false);
                        ctx.insert("error", "Authentication failed - token generation error");
                        render_template("login.html.tera", ctx).await
                    }
                }
            } else {
                // Perform dummy verification to maintain consistent timing
                bcrypt::verify(password, dummy_hash).ok();
                warn!("Invalid password for: {}", email);
                let mut ctx = Context::new();
                ctx.insert("is_authenticated", &false);
                ctx.insert("error", "Invalid email or password");
                render_template("login.html.tera", ctx).await
            }
        }
        None => {
            // Perform dummy verification to maintain consistent timing
            bcrypt::verify(password, dummy_hash).ok();
            warn!("Admin not found: {}", email);
            let mut ctx = Context::new();
            ctx.insert("is_authenticated", &false);
            ctx.insert("error", "Invalid email or password");
            render_template("login.html.tera", ctx).await
        }
    }
}

/// GET/POST /adminx/logout - Clear session and redirect
pub async fn logout_action(session: Session) -> impl Responder {
    // Get user info before clearing session for logging
    let user_info = session.get::<String>("admintoken")
        .unwrap_or_default()
        .unwrap_or_else(|| "unknown".to_string());
    
    // Clear the session
    session.clear();
    
    info!("User logged out successfully: {}", if user_info == "unknown" { "session_token_unavailable" } else { "user_had_valid_session" });
    
    HttpResponse::Found()
        .append_header(("Location", "/adminx/login"))
        .finish()
}

/// GET /adminx - Dashboard/Home page
pub async fn dashboard_view(
    session: Session,
    config: web::Data<AdminxConfig>,
) -> impl Responder {
    match create_base_template_context_with_auth("Dashboard", "", &session, &config).await {
        Ok(mut ctx) => {
            ctx.insert("page_title", "Dashboard");
            render_template("stats.html.tera", ctx).await
        }
        Err(redirect_response) => redirect_response,
    }
}

/// GET /adminx/profile - Show user profile
pub async fn profile_view(
    session: Session,
    config: web::Data<AdminxConfig>,
) -> impl Responder {
    match extract_claims_from_session(&session, &config).await {
        Ok(claims) => {
            let mut ctx = Context::new();
            ctx.insert("is_authenticated", &true);
            ctx.insert("user_email", &claims.email);
            ctx.insert("user_role", &claims.role);
            ctx.insert("user_roles", &claims.roles);
            ctx.insert("current_user", &claims);
            ctx.insert("menus", &get_registered_menus());
            ctx.insert("page_title", "Profile");
            render_template("profile.html.tera", ctx).await
        }
        Err(_) => {
            HttpResponse::Found()
                .append_header(("Location", "/adminx/login"))
                .finish()
        }
    }
}

/// Helper function for error responses with consistent format
fn auth_error_response(message: &str, status: actix_web::http::StatusCode) -> HttpResponse {
    HttpResponse::build(status)
        .content_type("application/json")
        .json(serde_json::json!({
            "error": message,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
}

/// Enhanced JSON login endpoint for API calls
pub async fn api_login_action(
    form: web::Json<LoginForm>,
    session: Session,
    config: web::Data<AdminxConfig>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let email = form.email.trim();
    let password = form.password.trim();
    
    // Get request metadata for logging
    let connection_info = req.connection_info();
    let ip = connection_info.peer_addr().unwrap_or("unknown");
    let user_agent = req.headers().get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");
    
    info!(
        email = %email,
        ip = %ip,
        user_agent = %user_agent,
        "API login attempt"
    );
    
    // Input validation
    if email.is_empty() || password.is_empty() {
        return auth_error_response("Email and password are required", 
            actix_web::http::StatusCode::BAD_REQUEST);
    }
    
    if !email.contains('@') {
        return auth_error_response("Invalid email format", 
            actix_web::http::StatusCode::BAD_REQUEST);
    }
    
    // Rate limiting
    if is_rate_limited(email, 5, Duration::from_secs(900)) {
        warn!(
            email = %email,
            ip = %ip,
            "Rate limit exceeded"
        );
        return auth_error_response("Too many login attempts", 
            actix_web::http::StatusCode::TOO_MANY_REQUESTS);
    }
    
    // Dummy hash for timing attack prevention
    let dummy_hash = "$2b$12$dummy.hash.to.prevent.timing.attacks.abcdefghijklmnopqrstuvwxy";
    
    match get_admin_by_email(email).await {
        Some(admin) => {
            if admin.verify_password(password) {
                let admin_id = match &admin.id {
                    Some(id) => id.to_string(),
                    None => {
                        error!("Admin has no ID: {}", email);
                        return auth_error_response("Authentication failed", 
                            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
                    }
                };
                
                match create_jwt_token(&admin_id, email, "admin", &config) {
                    Ok(token) => {
                        info!(
                            email = %email,
                            ip = %ip,
                            "API login successful"
                        );
                        
                        reset_rate_limit(email);
                        
                        if let Err(err) = session.insert("admintoken", &token) {
                            error!("Session insertion failed: {}", err);
                            return auth_error_response("Session creation failed", 
                                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
                        }
                        
                        HttpResponse::Ok().json(serde_json::json!({
                            "success": true,
                            "redirect": "/adminx",
                            "message": "Login successful",
                            "user": {
                                "email": email,
                                "role": "admin"
                            }
                        }))
                    }
                    Err(err) => {
                        error!("JWT generation failed for {}: {}", email, err);
                        auth_error_response("Authentication failed", 
                            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            } else {
                bcrypt::verify(password, dummy_hash).ok();
                warn!(
                    email = %email,
                    ip = %ip,
                    "Invalid password"
                );
                auth_error_response("Invalid credentials", 
                    actix_web::http::StatusCode::UNAUTHORIZED)
            }
        }
        None => {
            bcrypt::verify(password, dummy_hash).ok();
            warn!(
                email = %email,
                ip = %ip,
                "Admin not found"
            );
            auth_error_response("Invalid credentials", 
                actix_web::http::StatusCode::UNAUTHORIZED)
        }
    }
}

/// API endpoint to check authentication status
pub async fn check_auth_status(
    session: Session,
    config: web::Data<AdminxConfig>,
) -> impl Responder {
    match extract_claims_from_session(&session, &config).await {
        Ok(claims) => {
            HttpResponse::Ok().json(serde_json::json!({
                "authenticated": true,
                "user": {
                    "email": claims.email,
                    "role": claims.role,
                    "roles": claims.roles
                }
            }))
        }
        Err(_) => {
            HttpResponse::Ok().json(serde_json::json!({
                "authenticated": false
            }))
        }
    }
}