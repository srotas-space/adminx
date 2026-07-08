// adminx/src/middleware/role_guard.rs

use actix_session::SessionExt;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, web,
};
use actix_web::http::header;
use base64::Engine as _;
use futures_util::future::LocalBoxFuture;
use std::{collections::HashSet, rc::Rc};
use tracing::{info, warn};

use crate::configs::initializer::{AdminxConfig, BasicAuthConfig};
use crate::utils::{
    auth::extract_claims_from_session,
    structs::{Claims, RoleGuard},
};

/// Transform implementation for RoleGuard -> installs RoleGuardMiddleware.
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

            // Load AdminxConfig from app data
            let config = req
                .app_data::<web::Data<AdminxConfig>>()
                .ok_or_else(|| {
                    warn!("⚠️  AdminX config not found in app data for request: {}", uri);
                    actix_web::error::ErrorInternalServerError("AdminX config not found")
                })?;

            // 1️⃣ JWT / session is MANDATORY
            let claims: Claims = match extract_claims_from_session(&session, config.as_ref()).await {
                Ok(c) => {
                    info!("🔐 Session/JWT auth succeeded for {}", c.email);
                    c
                }
                Err(auth_error) => {
                    warn!(
                        "🔐 Authentication (JWT/session) failed for {} - {:?}",
                        uri, auth_error
                    );
                    return Err(actix_web::error::ErrorUnauthorized("Authentication required"));
                }
            };

            // 2️⃣ Optional Basic Auth (extra gate)
            //
            // If ADMINX_BASIC_AUTH is configured AND an Authorization: Basic header is present,
            // we validate it. If header is missing, we simply skip the check.
            //
            // ❗ Basic Auth does NOT create Claims, JWT is still the source of truth.
            if let Some(basic_cfg) = &config.basic_auth {
                if let Err(e) = check_optional_basic_auth(&req, basic_cfg) {
                    warn!("🔑 Basic Auth failed for {} - {}", uri, e);
                    return Err(actix_web::error::ErrorUnauthorized("Basic authentication failed"));
                }
            }

            // 3️⃣ Role-based authorization using Claims
            let user_roles: HashSet<String> = {
                let mut roles = claims.roles.clone();
                roles.push(claims.role.clone());
                roles.into_iter().collect()
            };

            let has_permission = allowed_roles.iter().any(|role| user_roles.contains(role));

            if has_permission {
                info!(
                    "✅ Access granted to {} for {} (roles: {:?})",
                    claims.email, uri, user_roles
                );

                // Make Claims available to handlers
                req.extensions_mut().insert(claims);

                return svc.call(req).await;
            } else {
                warn!(
                    "🚫 Access denied to {} for {} - insufficient roles (user: {:?}, required: {:?})",
                    claims.email, uri, user_roles, allowed_roles
                );

                // Don't disclose required/held role names to the client; the detail
                // is already in the server-side warn! log above.
                return Err(actix_web::error::ErrorForbidden("Access denied"));
            }
        })
    }
}

// =======================
// RoleGuard helper APIs
// =======================

impl RoleGuard {
    /// Create a role guard that allows only admins.
    pub fn admin_only() -> Self {
        Self {
            allowed_roles: vec!["admin".to_string(), "superadmin".to_string()],
        }
    }

    /// Create a role guard for moderators and above.
    pub fn moderator_and_above() -> Self {
        Self {
            allowed_roles: vec![
                "admin".to_string(),
                "superadmin".to_string(),
                "moderator".to_string(),
            ],
        }
    }

    /// Create a role guard that allows all authenticated users.
    pub fn authenticated_users() -> Self {
        Self {
            allowed_roles: vec![
                "admin".to_string(),
                "moderator".to_string(),
                "user".to_string(),
            ],
        }
    }

    /// Create a custom role guard with specific roles.
    pub fn custom_roles(roles: Vec<&str>) -> Self {
        Self {
            allowed_roles: roles.iter().map(|&s| s.to_string()).collect(),
        }
    }
}

// =======================
// Basic Auth helper
// =======================

/// Optional Basic Auth:
/// - If header is missing -> Ok(()) (no restriction).
/// - If header is present and valid -> Ok(()),
/// - If header is present and invalid -> Err(...)
fn check_optional_basic_auth(
    req: &ServiceRequest,
    cfg: &BasicAuthConfig,
) -> Result<(), &'static str> {
    let header_value = match req.headers().get(header::AUTHORIZATION) {
        None => {
            // No Basic Auth header - optional, so allow.
            return Ok(());
        }
        Some(h) => h,
    };

    let header_str = header_value
        .to_str()
        .map_err(|_| "Invalid header encoding")?;

    if !header_str.starts_with("Basic ") {
        // Some other auth scheme, ignore (treat as not provided)
        return Ok(());
    }

    let encoded = &header_str[6..]; // strip "Basic "
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| "Invalid base64 in Authorization header")?;

    let decoded = String::from_utf8(decoded_bytes).map_err(|_| "Invalid UTF-8 in credentials")?;
    let parts: Vec<&str> = decoded.splitn(2, ':').collect();

    if parts.len() != 2 {
        return Err("Invalid Basic auth format");
    }

    let (user, pass) = (parts[0], parts[1]);

    if user == cfg.username && pass == cfg.password {
        Ok(())
    } else {
        Err("Invalid Basic auth credentials")
    }
}




// // adminx/src/middleware/role_guard.rs - Fixed version
// use actix_web::{
//     dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
//     Error, HttpMessage, web,
// };
// use actix_web::http::header;
// use futures_util::future::LocalBoxFuture;
// use base64::Engine as _;
// use crate::utils::structs::Claims;
// use std::{
//     collections::HashSet,
//     rc::Rc,
// };
// use actix_session::SessionExt;
// use crate::utils::{
//     auth::{
//         extract_claims_from_session
//     },
//     structs::{
//         RoleGuard
//     },
// };
// use crate::configs::initializer::{AdminxConfig, BasicAuthConfig};
// use tracing::{info, warn};

// impl<S, B> Transform<S, ServiceRequest> for RoleGuard
// where
//     S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
//     B: 'static,
// {
//     type Response = ServiceResponse<B>;
//     type Error = Error;
//     type Transform = RoleGuardMiddleware<S>;
//     type InitError = ();
//     type Future = LocalBoxFuture<'static, Result<Self::Transform, Self::InitError>>;

//     fn new_transform(&self, service: S) -> Self::Future {
//         let allowed_roles = self.allowed_roles.clone();
//         Box::pin(async move {
//             Ok(RoleGuardMiddleware {
//                 service: Rc::new(service),
//                 allowed_roles,
//             })
//         })
//     }
// }

// pub struct RoleGuardMiddleware<S> {
//     service: Rc<S>,
//     allowed_roles: Vec<String>,
// }

// impl<S, B> Service<ServiceRequest> for RoleGuardMiddleware<S>
// where
//     S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
//     B: 'static,
// {
//     type Response = ServiceResponse<B>;
//     type Error = Error;
//     type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

//     forward_ready!(service);

//     fn call(&self, req: ServiceRequest) -> Self::Future {
//         let svc = Rc::clone(&self.service);
//         let allowed_roles = self.allowed_roles.clone();

//         Box::pin(async move {
//             let session = req.get_session();
//             let uri = req.uri().to_string();

//             // Get config from app data instead of environment variables
//             let config = req.app_data::<web::Data<AdminxConfig>>()
//                 .ok_or_else(|| {
//                     warn!("⚠️  AdminX config not found in app data for request: {}", uri);
//                     actix_web::error::ErrorInternalServerError("AdminX config not found")
//                 })?;

//             // Optional Basic Auth
//             if let Some(basic_cfg) = &config.basic_auth {
//                 match authenticate_basic(&req, basic_cfg) {
//                     Ok(c) => {
//                         info!("🔑 Basic Auth succeeded for {}", c.email);
//                         c
//                     }
//                     Err(e) => {
//                         warn!("🔑 Basic Auth failed for {} - {}", uri, e);
//                         return Err(actix_web::error::ErrorUnauthorized("Basic Authentication failed"));
//                     }
//                 }
//             }

//             match extract_claims_from_session(&session, config.as_ref()).await {
//                 Ok(claims) => {
//                     let user_roles: HashSet<String> = {
//                         let mut roles = claims.roles.clone();
//                         roles.push(claims.role.clone());
//                         roles.into_iter().collect()
//                     };

//                     // Check if user has any of the allowed roles
//                     let has_permission = allowed_roles.iter().any(|role| user_roles.contains(role));

//                     if has_permission {
//                         info!("✅ Access granted to {} for {} (roles: {:?})", 
//                               claims.email, uri, user_roles);
                        
//                         // Insert claims into request extensions for use by handlers
//                         req.extensions_mut().insert(claims);
//                         return svc.call(req).await;
//                     } else {
//                         warn!("🚫 Access denied to {} for {} - insufficient roles (user: {:?}, required: {:?})", 
//                               claims.email, uri, user_roles, allowed_roles);
//                         return Err(actix_web::error::ErrorForbidden(format!(
//                             "Access denied. Required roles: {:?}, User roles: {:?}", 
//                             allowed_roles, user_roles
//                         )));
//                     }
//                 }
//                 Err(auth_error) => {
//                     warn!("🔐 Authentication failed for request: {} - {:?}", uri, auth_error);
//                     return Err(actix_web::error::ErrorUnauthorized("Authentication required"));
//                 }
//             }
//         })
//     }
// }

// // Helper functions for common role checks
// impl RoleGuard {
//     /// Create a role guard that allows only admins
//     pub fn admin_only() -> Self {
//         Self {
//             allowed_roles: vec!["admin".to_string(), "superadmin".to_string()],
//         }
//     }

//     /// Create a role guard for moderators and above
//     pub fn moderator_and_above() -> Self {
//         Self {
//             allowed_roles: vec!["admin".to_string(), "superadmin".to_string(), "moderator".to_string()],
//         }
//     }

//     /// Create a role guard that allows all authenticated users
//     pub fn authenticated_users() -> Self {
//         Self {
//             allowed_roles: vec!["admin".to_string(), "moderator".to_string(), "user".to_string()],
//         }
//     }

//     /// Create a custom role guard with specific roles
//     pub fn custom_roles(roles: Vec<&str>) -> Self {
//         Self {
//             allowed_roles: roles.iter().map(|&s| s.to_string()).collect(),
//         }
//     }
// }



// fn authenticate_basic(req: &ServiceRequest, cfg: &BasicAuthConfig) -> Result<Claims, &'static str> {
//     let header_value = req
//         .headers()
//         .get(header::AUTHORIZATION)
//         .ok_or("Missing Authorization header")?;

//     let header_str = header_value
//         .to_str()
//         .map_err(|_| "Invalid header encoding")?;

//     if !header_str.starts_with("Basic ") {
//         return Err("Not Basic auth");
//     }

//     let encoded = &header_str[6..]; // after "Basic "
//     let decoded_bytes = base64::engine::general_purpose::STANDARD
//         .decode(encoded)
//         .map_err(|_| "Invalid base64 in Authorization header")?;

//     let decoded = String::from_utf8(decoded_bytes).map_err(|_| "Invalid UTF-8 in credentials")?;
//     let parts: Vec<&str> = decoded.splitn(2, ':').collect();
//     if parts.len() != 2 {
//         return Err("Invalid Basic auth format");
//     }

//     let (user, pass) = (parts[0], parts[1]);

//     if user == cfg.username && pass == cfg.password {
//         Ok(Claims {
//             sub: user.to_string(),
//             exp: 0,
//             email: format!("{}@basic-auth.local", user),
//             role: "admin".to_string(),
//             roles: vec!["admin".to_string()],
//         })
//     } else {
//         Err("Invalid Basic auth credentials")
//     }
// }
