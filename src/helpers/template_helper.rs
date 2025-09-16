// adminx/src/helpers/template_helper.rs
use actix_web::{HttpResponse};
use actix_session::Session;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tera::{Context, Tera};
use crate::configs::initializer::AdminxConfig;
use crate::utils::auth::extract_claims_from_session;
use tracing::{error, warn};
use chrono::Datelike;


// Centralized template list to keep code clean and DRY
const TEMPLATE_FILES: &[(&str, &str)] = &[
    ("layout.html.tera", include_str!("../templates/layout.html.tera")),
    ("header.html.tera", include_str!("../templates/header.html.tera")),
    ("footer.html.tera", include_str!("../templates/footer.html.tera")),
    ("list.html.tera", include_str!("../templates/list.html.tera")),
    ("new.html.tera", include_str!("../templates/new.html.tera")),
    ("edit.html.tera", include_str!("../templates/edit.html.tera")),
    ("view.html.tera", include_str!("../templates/view.html.tera")),
    ("login.html.tera", include_str!("../templates/login.html.tera")),
    ("profile.html.tera", include_str!("../templates/profile.html.tera")),
    ("stats.html.tera", include_str!("../templates/stats.html.tera")),
    ("errors/404.html.tera", include_str!("../templates/errors/404.html.tera")),
    ("errors/500.html.tera", include_str!("../templates/errors/500.html.tera")),
];

pub static ADMINX_TEMPLATES: Lazy<Arc<Tera>> = Lazy::new(|| {
    let mut tera = Tera::default();

    for (name, content) in TEMPLATE_FILES {
        tera.add_raw_template(name, content)
            .unwrap_or_else(|e| panic!("Failed to add {}: {}", name, e));
    }

    tera.autoescape_on(vec![]); // Disable autoescaping if rendering raw HTML
    Arc::new(tera)
});

pub async fn render_template(template_name: &str, ctx: Context) -> HttpResponse {
    let tera = Arc::clone(&ADMINX_TEMPLATES);
    match tera.render(template_name, &ctx) {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(err) => {
            error!("Template render error for {}: {:?}", template_name, err);
            let mut error_ctx = Context::new();
            error_ctx.insert("error", &err.to_string());
            error_ctx.insert("template_name", template_name);
            
            let fallback_html = tera
                .render("errors/500.html.tera", &error_ctx)
                .unwrap_or_else(|_| format!(
                    "<h1>Internal Server Error</h1><p>Failed to render template: {}</p><p>Error: {}</p>", 
                    template_name, 
                    err
                ));
            HttpResponse::InternalServerError()
                .content_type("text/html")
                .body(fallback_html)
        }
    }
}

// Template rendering with authentication context
pub async fn render_template_with_auth(
    template_name: &str,
    mut context: Context,
    session: &Session,
    config: &AdminxConfig,
) -> HttpResponse {
    // Add authentication context to templates
    match extract_claims_from_session(session, config).await {
        Ok(claims) => {
            context.insert("current_user", &claims);
            context.insert("is_authenticated", &true);
            context.insert("user_email", &claims.email);
            context.insert("user_role", &claims.role);
            context.insert("user_roles", &claims.roles);
        }
        Err(_) => {
            context.insert("is_authenticated", &false);
            context.insert("current_user", &serde_json::Value::Null);
        }
    }
    
    render_template(template_name, context).await
}

// Protected template rendering (redirects if not authenticated)
pub async fn render_protected_template(
    template_name: &str,
    mut context: Context,
    session: &Session,
    config: &AdminxConfig,
    redirect_url: Option<&str>,
) -> HttpResponse {
    match extract_claims_from_session(session, config).await {
        Ok(claims) => {
            context.insert("current_user", &claims);
            context.insert("is_authenticated", &true);
            context.insert("user_email", &claims.email);
            context.insert("user_role", &claims.role);
            context.insert("user_roles", &claims.roles);
            
            render_template(template_name, context).await
        }
        Err(_) => {
            let redirect_to = redirect_url.unwrap_or("/adminx/login");
            HttpResponse::Found()
                .append_header(("Location", redirect_to))
                .finish()
        }
    }
}

// Template rendering with role-based access control
pub async fn render_role_protected_template(
    template_name: &str,
    mut context: Context,
    session: &Session,
    config: &AdminxConfig,
    required_roles: Vec<&str>,
    redirect_url: Option<&str>,
) -> HttpResponse {
    match extract_claims_from_session(session, config).await {
        Ok(claims) => {
            // Check if user has any of the required roles
            let user_roles: std::collections::HashSet<String> = {
                let mut roles = claims.roles.clone();
                roles.push(claims.role.clone());
                roles.into_iter().collect()
            };
            
            let has_required_role = required_roles.iter()
                .any(|role| user_roles.contains(&role.to_string()));
            
            if has_required_role {
                context.insert("current_user", &claims);
                context.insert("is_authenticated", &true);
                context.insert("user_email", &claims.email);
                context.insert("user_role", &claims.role);
                context.insert("user_roles", &claims.roles);
                
                render_template(template_name, context).await
            } else {
                warn!("Access denied for user {} to template {}", claims.email, template_name);
                render_403().await
            }
        }
        Err(_) => {
            let redirect_to = redirect_url.unwrap_or("/adminx/login");
            HttpResponse::Found()
                .append_header(("Location", redirect_to))
                .finish()
        }
    }
}

// Error page renderers
pub async fn render_404() -> HttpResponse {
    let tera = Arc::clone(&ADMINX_TEMPLATES);
    let ctx = Context::new();
    let html = tera
        .render("errors/404.html.tera", &ctx)
        .unwrap_or_else(|_| "<h1>404 - Page Not Found</h1>".to_string());
    HttpResponse::NotFound()
        .content_type("text/html")
        .body(html)
}

pub async fn render_403() -> HttpResponse {
    let tera = Arc::clone(&ADMINX_TEMPLATES);
    let mut ctx = Context::new();
    ctx.insert("error_message", "You don't have permission to access this resource.");
    
    let html = tera
        .render("errors/403.html.tera", &ctx)
        .unwrap_or_else(|_| "<h1>403 - Access Forbidden</h1><p>You don't have permission to access this resource.</p>".to_string());
    HttpResponse::Forbidden()
        .content_type("text/html")
        .body(html)
}

pub async fn render_500(error_message: Option<&str>) -> HttpResponse {
    let tera = Arc::clone(&ADMINX_TEMPLATES);
    let mut ctx = Context::new();
    ctx.insert("error_message", &error_message.unwrap_or("An internal server error occurred."));
    
    let html = tera
        .render("errors/500.html.tera", &ctx)
        .unwrap_or_else(|_| "<h1>500 - Internal Server Error</h1>".to_string());
    HttpResponse::InternalServerError()
        .content_type("text/html")
        .body(html)
}

// Template context helpers
pub fn create_base_context() -> Context {
    let mut ctx = Context::new();
    ctx.insert("app_name", "AdminX");
    ctx.insert("app_version", env!("CARGO_PKG_VERSION"));
    ctx.insert("current_year", &chrono::Utc::now().year());
    ctx
}

pub fn add_flash_messages(mut context: Context, messages: Vec<(&str, &str)>) -> Context {
    // messages is Vec<(level, message)> where level is "success", "error", "warning", "info"
    context.insert("flash_messages", &messages);
    context
}