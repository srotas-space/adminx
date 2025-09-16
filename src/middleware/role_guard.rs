// adminx/src/middleware/role_guard.rs - Fixed version
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, web,
};
use futures_util::future::LocalBoxFuture;
use std::{
    collections::HashSet,
    rc::Rc,
};
use actix_session::SessionExt;
use crate::utils::{
    auth::{
        extract_claims_from_session
    },
    structs::{
        RoleGuard
    },
};
use crate::configs::initializer::AdminxConfig;
use tracing::{info, warn};

impl<S, B> Transform<S, ServiceRequest> for RoleGuard
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RoleGuardMiddleware<S>;
    type InitError = ();
    type Future = LocalBoxFuture<'static, Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let allowed_roles = self.allowed_roles.clone();
        Box::pin(async move {
            Ok(RoleGuardMiddleware {
                service: Rc::new(service),
                allowed_roles,
            })
        })
    }
}

pub struct RoleGuardMiddleware<S> {
    service: Rc<S>,
    allowed_roles: Vec<String>,
}

impl<S, B> Service<ServiceRequest> for RoleGuardMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = Rc::clone(&self.service);
        let allowed_roles = self.allowed_roles.clone();

        Box::pin(async move {
            let session = req.get_session();
            let uri = req.uri().to_string();

            // Get config from app data instead of environment variables
            let config = req.app_data::<web::Data<AdminxConfig>>()
                .ok_or_else(|| {
                    warn!("⚠️  AdminX config not found in app data for request: {}", uri);
                    actix_web::error::ErrorInternalServerError("AdminX config not found")
                })?;

            match extract_claims_from_session(&session, config.as_ref()).await {
                Ok(claims) => {
                    let user_roles: HashSet<String> = {
                        let mut roles = claims.roles.clone();
                        roles.push(claims.role.clone());
                        roles.into_iter().collect()
                    };

                    // Check if user has any of the allowed roles
                    let has_permission = allowed_roles.iter().any(|role| user_roles.contains(role));

                    if has_permission {
                        info!("✅ Access granted to {} for {} (roles: {:?})", 
                              claims.email, uri, user_roles);
                        
                        // Insert claims into request extensions for use by handlers
                        req.extensions_mut().insert(claims);
                        return svc.call(req).await;
                    } else {
                        warn!("🚫 Access denied to {} for {} - insufficient roles (user: {:?}, required: {:?})", 
                              claims.email, uri, user_roles, allowed_roles);
                        return Err(actix_web::error::ErrorForbidden(format!(
                            "Access denied. Required roles: {:?}, User roles: {:?}", 
                            allowed_roles, user_roles
                        )));
                    }
                }
                Err(auth_error) => {
                    warn!("🔐 Authentication failed for request: {} - {:?}", uri, auth_error);
                    return Err(actix_web::error::ErrorUnauthorized("Authentication required"));
                }
            }
        })
    }
}

// Helper functions for common role checks
impl RoleGuard {
    /// Create a role guard that allows only admins
    pub fn admin_only() -> Self {
        Self {
            allowed_roles: vec!["admin".to_string(), "superadmin".to_string()],
        }
    }

    /// Create a role guard for moderators and above
    pub fn moderator_and_above() -> Self {
        Self {
            allowed_roles: vec!["admin".to_string(), "superadmin".to_string(), "moderator".to_string()],
        }
    }

    /// Create a role guard that allows all authenticated users
    pub fn authenticated_users() -> Self {
        Self {
            allowed_roles: vec!["admin".to_string(), "moderator".to_string(), "user".to_string()],
        }
    }

    /// Create a custom role guard with specific roles
    pub fn custom_roles(roles: Vec<&str>) -> Self {
        Self {
            allowed_roles: roles.iter().map(|&s| s.to_string()).collect(),
        }
    }
}