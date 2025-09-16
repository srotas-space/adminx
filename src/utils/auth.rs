// adminx/src/utils/auth.rs
use crate::models::adminx_model::{AdminxUser};
use crate::configs::initializer::AdminxConfig;
use mongodb::{
    bson::{doc, DateTime as BsonDateTime},
};
use bcrypt::{hash, DEFAULT_COST};
use anyhow::{Result};
use crate::{custom_error_expression};
use serde::{Serialize, Deserialize};
use actix_session::Session;
use actix_web::{Error, web};
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::{
    utils::{
        database::{
            get_adminx_database
        },
        ubson::{
            convert_to_bson
        },
        structs::{
            Claims // ✅ Use Claims from structs.rs, not jwt.rs
        },
    }
};

// Updated to use config instead of env::var
pub async fn extract_claims_from_session(
    session: &Session,
    config: &AdminxConfig,
) -> Result<Claims, Error> {
    let token = session
        .get::<String>("admintoken")
        .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid session"))?
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing token in session"))?;
    
    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid token"))?;
    
    Ok(token_data.claims)
}

// Convenience function for extracting claims from request context
pub async fn extract_claims_from_request(
    session: &Session,
    config: &web::Data<AdminxConfig>,
) -> Result<Claims, Error> {
    extract_claims_from_session(session, config.as_ref()).await
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AdminxStatus {
    Active,
    Inactive,
    Suspended,
}

pub enum InitOutcome {
    Created,
    Updated,
}

#[derive(Debug)]
pub struct NewAdminxUser {
    pub username: String,
    pub email: String,
    pub password: String,
    pub status: AdminxStatus,
    pub delete: bool,
}

pub async fn initiate_auth(adminx: NewAdminxUser) -> Result<InitOutcome, actix_web::Error> {
    let db = get_adminx_database();
    let collection = db.collection::<AdminxUser>("adminxs");
    
    let now = BsonDateTime::now();
    let hashed_pwd = hash(&adminx.password, DEFAULT_COST)
        .map_err(|e| custom_error_expression!(bad_request, 400, format!("Failed to hash password: {e}")))?;
        
    match collection.find_one(doc! { "email": &adminx.email }, None).await {
        Ok(Some(_exist)) => {
            let status_bson = convert_to_bson(&adminx.status)?;
            let update_doc = doc! {
                "username": adminx.username,
                "password": hashed_pwd,
                "delete": adminx.delete,
                "status": status_bson,
                "updated_at": now,
            };
            collection.update_one(
                doc! { "email": &adminx.email },
                doc! { "$set": update_doc },
                None,
            )
            .await
            .map_err(|e| custom_error_expression!(bad_request, 400, e.to_string()))?;
            Ok(InitOutcome::Updated)
        }
        Ok(None) => {
            let new_user = AdminxUser {
                id: None,
                username: adminx.username,
                email: adminx.email,
                password: hashed_pwd,
                delete: adminx.delete,
                status: adminx.status,
                created_at: now,
                updated_at: now,
            };
            collection.insert_one(new_user, None)
                .await
                .map_err(|e| custom_error_expression!(invalid_request, 422, format!("User creation failed: {e}")))?;
            Ok(InitOutcome::Created)
        }
        Err(e) => {
            return Err(custom_error_expression!(internal_error, 500, format!("DB error: {e}")))?;
        }
    }
}

// Additional security utilities that can use config
pub fn validate_session_config(config: &AdminxConfig) -> Result<(), String> {
    if config.jwt_secret.len() < 32 {
        return Err("JWT_SECRET must be at least 32 characters long".to_string());
    }
    
    if !config.session_secret.is_empty() && config.session_secret.len() < 64 {
        return Err("SESSION_SECRET must be at least 64 characters long".to_string());
    }
    
    Ok(())
}

// Rate limiting helper (optional enhancement)
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref LOGIN_ATTEMPTS: Mutex<HashMap<String, (u32, Instant)>> = Mutex::new(HashMap::new());
}

pub fn is_rate_limited(email: &str, max_attempts: u32, window: Duration) -> bool {
    let mut attempts = LOGIN_ATTEMPTS.lock().unwrap();
    let now = Instant::now();
    
    match attempts.get_mut(email) {
        Some((count, last_attempt)) => {
            if now.duration_since(*last_attempt) > window {
                // Reset counter if outside window
                *count = 1;
                *last_attempt = now;
                false
            } else if *count >= max_attempts {
                // Still within rate limit
                true
            } else {
                // Increment counter
                *count += 1;
                *last_attempt = now;
                false
            }
        }
        None => {
            // First attempt
            attempts.insert(email.to_string(), (1, now));
            false
        }
    }
}

pub fn reset_rate_limit(email: &str) {
    let mut attempts = LOGIN_ATTEMPTS.lock().unwrap();
    attempts.remove(email);
}