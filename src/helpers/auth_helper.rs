// adminx/src/helpers/auth_helper.rs
use actix_web::HttpResponse;
use actix_session::Session;
use tera::Context;
use crate::configs::initializer::AdminxConfig;
use crate::utils::auth::extract_claims_from_session;
use crate::registry::get_registered_menus;

pub async fn create_base_template_context_with_auth(
    resource_name: &str,
    base_path: &str,
    session: &Session,
    config: &AdminxConfig,
) -> Result<Context, HttpResponse> {
    match extract_claims_from_session(session, config).await {
        Ok(claims) => {
            let mut ctx = Context::new();
            ctx.insert("resource_name", resource_name);
            ctx.insert("base_path", &format!("/adminx/{}", base_path));
            ctx.insert("menus", &get_registered_menus());
            ctx.insert("current_user", &claims);
            ctx.insert("is_authenticated", &true);
            Ok(ctx)
        }
        Err(_) => {
            Err(HttpResponse::Found()
                .append_header(("Location", "/adminx/login"))
                .finish())
        }
    }
}