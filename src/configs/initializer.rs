// src/configs/initializer.rs
use log::{info, debug, warn};
use mongodb::Database;
use anyhow::{Error as AnyhowError};
use actix_web::{web};
use actix_session::{SessionMiddleware, storage::CookieSessionStore, config::PersistentSession};
use actix_web::cookie::{Key, SameSite};
use env_logger::Env;
use std::{env, time::Duration};
use crate::router::register_all_admix_routes;
use crate::utils::{
    database::{
        initiate_database
    },
};

#[derive(Debug, Clone)]
pub struct AdminxConfig {
    pub jwt_secret: String,
    pub session_secret: String,
    pub environment: String,
    pub log_level: String,
    pub session_timeout: Duration,
}

impl AdminxConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            jwt_secret: env::var("JWT_SECRET")
                .map_err(|_| "JWT_SECRET is required")?,
            session_secret: env::var("SESSION_SECRET")
                .unwrap_or_else(|_| {
                    if cfg!(debug_assertions) {
                        warn!("⚠️  SESSION_SECRET not set, using generated key - NOT suitable for production!");
                        String::new() // Will trigger key generation
                    } else {
                        panic!("SESSION_SECRET is required in production");
                    }
                }),
            environment: env::var("ENVIRONMENT")
                .unwrap_or_else(|_| "development".to_string()),
            log_level: env::var("RUST_LOG")
                .unwrap_or_else(|_| "debug".to_string()),
            session_timeout: Duration::from_secs(
                env::var("SESSION_TIMEOUT")
                    .unwrap_or_else(|_| "86400".to_string())
                    .parse()
                    .unwrap_or(86400)
            ),
        })
    }
    
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}

fn load_session_key(config: &AdminxConfig) -> Key {
    if config.session_secret.is_empty() {
        if cfg!(debug_assertions) {
            warn!("⚠️  Using generated session key - NOT suitable for production!");
            Key::generate()
        } else {
            panic!("SESSION_SECRET environment variable is required in production");
        }
    } else {
        if config.session_secret.len() < 64 {
            panic!("SESSION_SECRET must be at least 64 characters long");
        }
        Key::from(config.session_secret.as_bytes())
    }
}

fn create_session_middleware(config: &AdminxConfig) -> SessionMiddleware<CookieSessionStore> {
    let secret_key = load_session_key(config);
    
    // Convert std::time::Duration to actix_web::cookie::time::Duration
    let session_ttl = actix_web::cookie::time::Duration::seconds(config.session_timeout.as_secs() as i64);
    
    SessionMiddleware::builder(
        CookieSessionStore::default(),
        secret_key
    )
    .cookie_name("adminx_session".to_string())
    .cookie_secure(config.is_production())
    .cookie_http_only(true)
    .cookie_same_site(if config.is_production() { 
        SameSite::Strict 
    } else { 
        SameSite::Lax 
    })
    .session_lifecycle(
        PersistentSession::default()
            .session_ttl(session_ttl)
    )
    .build()
}

// Component-based approach - much cleaner!
pub fn get_adminx_config() -> AdminxConfig {
    AdminxConfig::from_env().unwrap_or_else(|e| {
        eprintln!("❌ AdminX configuration error: {}", e);
        std::process::exit(1);
    })
}

pub fn setup_adminx_logging(config: &AdminxConfig) {
    if env::var("ADMINX_LOGGING_INITIALIZED").is_err() {
        let _ = env_logger::Builder::from_env(Env::default().default_filter_or(&config.log_level))
            .format_timestamp_millis()
            .try_init();
        
        env::set_var("ADMINX_LOGGING_INITIALIZED", "true");
        info!("✅ AdminX logging initialized");
        info!("🔧 AdminX environment: {}", config.environment);
        debug!("🔍 AdminX debug logging active");
    }
}

pub fn get_adminx_session_middleware(config: &AdminxConfig) -> SessionMiddleware<CookieSessionStore> {
    create_session_middleware(config)
}

// Alternative using service configuration
pub fn configure_adminx_services(cfg: &mut web::ServiceConfig) {
    let config = get_adminx_config();
    cfg.app_data(web::Data::new(config));
    cfg.service(register_all_admix_routes());

}

pub async fn adminx_initialize(db: Database) -> Result<(), AnyhowError> {
    let _ = initiate_database(db);
    // let _ = ADMINX_TEMPLATES.len();
    info!("AdminX initialized successfully");
    Ok(())
}