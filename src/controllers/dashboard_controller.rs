// adminx/src/controllers/dashboard_controller.rs

use actix_web::{HttpResponse, Responder, web, HttpRequest};
use actix_session::Session;
use tera::Context;
use crate::registry::get_registered_menus;
use crate::helpers::template_helper::{render_template, render_template_with_auth};
use crate::configs::initializer::AdminxConfig;
use crate::utils::auth::extract_claims_from_session;
use tracing::info;

pub async fn adminx_home(
    session: Session,
    config: web::Data<AdminxConfig>,
    _req: HttpRequest,
) -> impl Responder {
    // Check if user is authenticated
    match extract_claims_from_session(&session, &config).await {
        Ok(claims) => {
            info!("Dashboard accessed by: {}", claims.email);
            
            let mut ctx = Context::new();
            ctx.insert("menus", &get_registered_menus());
            ctx.insert("current_user", &claims);
            ctx.insert("is_authenticated", &true);
            ctx.insert("user_email", &claims.email);
            ctx.insert("user_role", &claims.role);
            ctx.insert("user_roles", &claims.roles);
            
            render_template("layout.html.tera", ctx).await
        }
        Err(_) => {
            // User not authenticated, redirect to login
            HttpResponse::Found()
                .append_header(("Location", "/adminx/login"))
                .finish()
        }
    }
}

// Alternative version that uses the helper function
pub async fn adminx_home_with_helper(
    session: Session,
    config: web::Data<AdminxConfig>,
) -> impl Responder {
    let mut ctx = Context::new();
    ctx.insert("menus", &get_registered_menus());
    
    render_template_with_auth("layout.html.tera", ctx, &session, &config).await
}

// Additional dashboard endpoints
pub async fn adminx_stats(
    session: Session,
    config: web::Data<AdminxConfig>,
) -> impl Responder {
    match extract_claims_from_session(&session, &config).await {
        Ok(claims) => {
            let mut ctx = Context::new();
            ctx.insert("menus", &get_registered_menus());
            ctx.insert("current_user", &claims);
            
            // Add some stats data
            ctx.insert("total_users", &42); // Replace with actual data
            ctx.insert("total_resources", &get_registered_menus().len());
            
            render_template("stats.html.tera", ctx).await
        }
        Err(_) => {
            HttpResponse::Found()
                .append_header(("Location", "/adminx/login"))
                .finish()
        }
    }
}

pub async fn adminx_profile(
    session: Session,
    config: web::Data<AdminxConfig>,
) -> impl Responder {
    match extract_claims_from_session(&session, &config).await {
        Ok(claims) => {
            let mut ctx = Context::new();
            ctx.insert("menus", &get_registered_menus());
            ctx.insert("current_user", &claims);
            ctx.insert("profile_user", &claims); // For profile-specific data
            
            render_template("profile.html.tera", ctx).await
        }
        Err(_) => {
            HttpResponse::Found()
                .append_header(("Location", "/adminx/login"))
                .finish()
        }
    }
}