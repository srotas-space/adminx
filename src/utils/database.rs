// adminx/src/utils/database.rs
use mongodb::{Client, options::ClientOptions, Database};
use log::{info};
use once_cell::sync::OnceCell;
use crate::configs::initializer::AdminxConfig;
use anyhow::{Result, Context};
use std::sync::Arc;

pub static ADMINX_DATABASE: OnceCell<Database> = OnceCell::new();
pub static ADMINX_CONFIG: OnceCell<Arc<AdminxConfig>> = OnceCell::new();




pub async fn initiate_mongo_client(mongo_uri: &str, mongo_database_name: &str) -> Database {
    let client_options = ClientOptions::parse(&mongo_uri)
        .await
        .expect("Failed to parse MongoDB URI");

    let client = Client::with_options(client_options)
        .expect("Failed to initialize MongoDB client");

    let db = client.database(&mongo_database_name);

    info!("✅ Mongo client initialized: {}", mongo_uri);

    db
}



pub fn initiate_database(db: Database) {
    ADMINX_DATABASE.set(db).ok(); // ignore error if already set
}

pub fn get_adminx_database() -> &'static Database {
    ADMINX_DATABASE
        .get()
        .expect("ADMINX_DATABASE has not been initialized. Call initiate_database(db) first.")
}

// Optional: Store config globally if needed by database operations
pub fn set_adminx_config(config: AdminxConfig) {
    ADMINX_CONFIG.set(Arc::new(config)).ok();
}

pub fn get_adminx_config() -> Option<&'static Arc<AdminxConfig>> {
    ADMINX_CONFIG.get()
}

// Database health check function
pub async fn check_database_health() -> Result<bool> {
    let db = get_adminx_database();
    
    // Simple ping to check database connectivity
    match db.run_command(mongodb::bson::doc! {"ping": 1}, None).await {
        Ok(_) => Ok(true),
        Err(e) => {
            tracing::error!("Database health check failed: {}", e);
            Ok(false)
        }
    }
}

// Database initialization with error handling
pub async fn initialize_database_with_validation(db: Database) -> Result<()> {
    // Test the database connection
    db.run_command(mongodb::bson::doc! {"ping": 1}, None)
        .await
        .context("Failed to ping database")?;
    
    // Initialize the database
    initiate_database(db);
    
    tracing::info!("✅ AdminX database initialized successfully");
    Ok(())
}

// Optional: Database configuration validation
pub fn validate_database_config() -> Result<()> {
    let _db = get_adminx_database();
    
    // Check if required collections exist or create them
    // This is just an example - modify based on your needs
    tracing::info!("Database validation completed");
    Ok(())
}