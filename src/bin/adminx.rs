// crates/adminx/src/bin/adminx.rs

use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::env;
use adminx::{
    models::adminx_model::{AdminxUser, get_admin_by_email, get_all_admins},
    utils::{
    	auth::{
    		AdminxStatus,
    	},
    	database::{
    		initiate_mongo_client,
    		initiate_database,
    		get_adminx_database,
    	},
	}
};
use mongodb::{bson::oid::ObjectId};

#[derive(Parser)]
#[command(name = "adminx")]
#[command(about = "AdminX CLI tool for managing admin users")]
#[command(version = "1.0")]
struct Cli {
    /// MongoDB connection URL
    #[arg(long, env = "MONGODB_URL")]
    mongodb_url: Option<String>,
    
    /// Database name
    #[arg(long, env = "ADMINX_DB_NAME")]
    database_name: Option<String>,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new admin user
    Create {
        /// Username for the admin
        #[arg(short, long)]
        username: String,
        /// Email address for the admin
        #[arg(short, long)]
        email: String,
        /// Password (will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
        /// User status (active, inactive, suspended)
        #[arg(short, long, default_value = "active")]
        status: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// List all admin users
    List {
        /// Include deleted users
        #[arg(short, long)]
        deleted: bool,
        /// Output format (table, json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },
    /// Show details of a specific admin user
    Show {
        /// User email or ID
        identifier: String,
    },
    /// Delete an admin user (soft delete)
    Delete {
        /// User email or ID
        identifier: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Update admin user status
    Status {
        /// User email or ID
        identifier: String,
        /// New status (active, inactive, suspended)
        status: String,
    },
    /// Reset admin user password
    ResetPassword {
        /// User email or ID
        identifier: String,
        /// New password (will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Get MongoDB URL and database name
    let mongodb_url = get_mongodb_url(&cli)?;
    let db_name = get_database_name(&cli)?;
    
    // Initialize database connection
    let db = initiate_mongo_client(&mongodb_url, &db_name).await;
    let _ = initiate_database(db);

    
    println!("Connected to MongoDB: {} (database: {})", mongodb_url, db_name);
    
    match cli.command {
        Commands::Create { username, email, password, status, yes } => {
            create_user(username, email, password, status, yes).await?;
        }
        Commands::List { deleted, format } => {
            list_users(deleted, format).await?;
        }
        Commands::Show { identifier } => {
            show_user(identifier).await?;
        }
        Commands::Delete { identifier, yes } => {
            delete_user(identifier, yes).await?;
        }
        Commands::Status { identifier, status } => {
            update_status(identifier, status).await?;
        }
        Commands::ResetPassword { identifier, password } => {
            reset_password(identifier, password).await?;
        }
    }
    
    Ok(())
}

fn get_mongodb_url(cli: &Cli) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(url) = &cli.mongodb_url {
        return Ok(url.clone());
    }
    
    // Try environment variable
    if let Ok(url) = env::var("MONGODB_URL") {
        return Ok(url);
    }
    
    // Prompt user for MongoDB URL
    print!("Enter MongoDB URL (default: mongodb://localhost:27017): ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    
    if input.is_empty() {
        Ok("mongodb://localhost:27017".to_string())
    } else {
        Ok(input.to_string())
    }
}

fn get_database_name(cli: &Cli) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(name) = &cli.database_name {
        return Ok(name.clone());
    }
    
    // Try environment variable
    if let Ok(name) = env::var("ADMINX_DB_NAME") {
        return Ok(name);
    }
    
    // Prompt user for database name
    print!("Enter database name (default: adminx): ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    
    if input.is_empty() {
        Ok("adminx".to_string())
    } else {
        Ok(input.to_string())
    }
}


async fn create_user(
    username: String,
    email: String,
    password: Option<String>,
    status_str: String,
    skip_confirm: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse status
    let status = match status_str.to_lowercase().as_str() {
        "active" => AdminxStatus::Active,
        "inactive" => AdminxStatus::Inactive,
        "suspended" => AdminxStatus::Suspended,
        _ => {
            eprintln!("Invalid status. Must be one of: active, inactive, suspended");
            return Ok(());
        }
    };
    
    // Get password if not provided
    let password = match password {
        Some(p) => p,
        None => {
            print!("Enter password: ");
            io::stdout().flush()?;
            let password = rpassword::read_password()?;
            if password.len() < 8 {
                eprintln!("Password must be at least 8 characters long");
                return Ok(());
            }
            password
        }
    };
    
    // Check if user already exists
    if let Some(_) = get_admin_by_email(&email).await {
        eprintln!("User with email {} already exists", email);
        return Ok(());
    }
    
    // Show confirmation
    if !skip_confirm {
        println!("Creating admin user:");
        println!("  Username: {}", username);
        println!("  Email: {}", email);
        println!("  Status: {:?}", status);
        print!("Continue? (y/N): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().to_lowercase().starts_with('y') {
            println!("Cancelled");
            return Ok(());
        }
    }
    
    // Create user
    match AdminxUser::create_new_user_with_status(username, email.clone(), password, status).await {
        Ok(user_id) => {
            println!("✓ Successfully created admin user");
            println!("  ID: {}", user_id);
            println!("  Email: {}", email);
        }
        Err(e) => {
            eprintln!("Failed to create user: {}", e);
        }
    }
    
    Ok(())
}

async fn list_users(include_deleted: bool, format: String) -> Result<(), Box<dyn std::error::Error>> {
    let users = get_all_admins(include_deleted).await?;
    
    if users.is_empty() {
        println!("No users found");
        return Ok(());
    }
    
    match format.as_str() {
        "json" => {
            let public_users: Vec<_> = users.iter().map(|u| u.to_public()).collect();
            println!("{}", serde_json::to_string_pretty(&public_users)?);
        }
        "table" | _ => {
            println!("{:<25} {:<30} {:<15} {:<10} {:<20}", "ID", "Email", "Username", "Status", "Created");
            println!("{}", "-".repeat(100));
            
            for user in users {
                println!(
                    "{:<25} {:<30} {:<15} {:<10} {:<20}",
                    user.id.map_or("N/A".to_string(), |id| id.to_string()),
                    user.email,
                    user.username,
                    format!("{:?}", user.status),
                    user.created_at.to_chrono().format("%Y-%m-%d %H:%M").to_string()
                );
            }
        }
    }
    
    Ok(())
}

async fn show_user(identifier: String) -> Result<(), Box<dyn std::error::Error>> {
    let user = find_user_by_identifier(&identifier).await?;
    
    match user {
        Some(user) => {
            println!("Admin User Details:");
            println!("  ID: {}", user.id.map_or("N/A".to_string(), |id| id.to_string()));
            println!("  Username: {}", user.username);
            println!("  Email: {}", user.email);
            println!("  Status: {:?}", user.status);
            println!("  Deleted: {}", user.delete);
            println!("  Created: {}", user.created_at.to_chrono().format("%Y-%m-%d %H:%M:%S"));
            println!("  Updated: {}", user.updated_at.to_chrono().format("%Y-%m-%d %H:%M:%S"));
        }
        None => {
            println!("User not found: {}", identifier);
        }
    }
    
    Ok(())
}

async fn delete_user(identifier: String, skip_confirm: bool) -> Result<(), Box<dyn std::error::Error>> {
    let user = find_user_by_identifier(&identifier).await?;
    
    let user = match user {
        Some(user) => user,
        None => {
            println!("User not found: {}", identifier);
            return Ok(());
        }
    };
    
    if user.delete {
        println!("User is already deleted");
        return Ok(());
    }
    
    if !skip_confirm {
        println!("Delete user:");
        println!("  Email: {}", user.email);
        println!("  Username: {}", user.username);
        print!("Continue? (y/N): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().to_lowercase().starts_with('y') {
            println!("Cancelled");
            return Ok(());
        }
    }
    
    if let Some(user_id) = user.id {
        match adminx::models::adminx_model::delete_admin_by_id(&user_id).await {
            Ok(true) => println!("✓ User deleted successfully"),
            Ok(false) => println!("User not found or already deleted"),
            Err(e) => eprintln!("Failed to delete user: {}", e),
        }
    }
    
    Ok(())
}

async fn update_status(identifier: String, status_str: String) -> Result<(), Box<dyn std::error::Error>> {
    let status = match status_str.to_lowercase().as_str() {
        "active" => AdminxStatus::Active,
        "inactive" => AdminxStatus::Inactive,
        "suspended" => AdminxStatus::Suspended,
        _ => {
            eprintln!("Invalid status. Must be one of: active, inactive, suspended");
            return Ok(());
        }
    };
    
    let user = find_user_by_identifier(&identifier).await?;
    
    let user = match user {
        Some(user) => user,
        None => {
            println!("User not found: {}", identifier);
            return Ok(());
        }
    };
    
    if let Some(user_id) = user.id {
        match adminx::models::adminx_model::update_admin_status(&user_id, status).await {
            Ok(true) => println!("✓ User status updated successfully"),
            Ok(false) => println!("Failed to update user status"),
            Err(e) => eprintln!("Error updating status: {}", e),
        }
    }
    
    Ok(())
}

async fn reset_password(identifier: String, password: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let user = find_user_by_identifier(&identifier).await?;
    
    let user = match user {
        Some(user) => user,
        None => {
            println!("User not found: {}", identifier);
            return Ok(());
        }
    };
    
    let new_password = match password {
        Some(p) => p,
        None => {
            print!("Enter new password: ");
            io::stdout().flush()?;
            let password = rpassword::read_password()?;
            if password.len() < 8 {
                eprintln!("Password must be at least 8 characters long");
                return Ok(());
            }
            password
        }
    };
    
    // For password reset, we'll directly hash and update (bypass current password check)
    let hashed_password = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| format!("Failed to hash password: {}", e))?;
    
    // Update in database directly
    if let Some(user_id) = user.id {
        let db = get_adminx_database();
        let collection = db.collection::<AdminxUser>("adminxs");
        
        let result = collection.update_one(
            mongodb::bson::doc! { "_id": user_id },
            mongodb::bson::doc! { 
                "$set": { 
                    "password": hashed_password,
                    "updated_at": mongodb::bson::DateTime::now()
                }
            },
            None,
        ).await?;
        
        if result.modified_count > 0 {
            println!("✓ Password reset successfully");
        } else {
            println!("Failed to reset password");
        }
    }
    
    Ok(())
}

async fn find_user_by_identifier(identifier: &str) -> Result<Option<AdminxUser>, Box<dyn std::error::Error>> {
    // First try to find by email
    if let Some(user) = get_admin_by_email(identifier).await {
        return Ok(Some(user));
    }
    
    // Then try to parse as ObjectId and find by ID
    if let Ok(object_id) = ObjectId::parse_str(identifier) {
        if let Some(user) = adminx::models::adminx_model::get_admin_by_id(&object_id).await {
            return Ok(Some(user));
        }
    }
    
    Ok(None)
}