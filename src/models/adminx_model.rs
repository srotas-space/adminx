// adminx/src/models/adminx_model.rs
use serde::{Deserialize, Serialize};
use mongodb::bson::{doc, oid::ObjectId, DateTime as BsonDateTime};
use bcrypt::verify;
use anyhow::Result;

use crate::{
    utils::{
        database::{
            get_adminx_database
        },
        auth::{
            AdminxStatus
        },
        jwt::create_jwt_token,
    },
    configs::initializer::AdminxConfig,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdminxUser {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub username: String,
    pub email: String,
    pub password: String, // hashed
    pub delete: bool,
    pub status: AdminxStatus,
    pub created_at: BsonDateTime,
    pub updated_at: BsonDateTime,
}

impl AdminxUser {
    pub fn verify_password(&self, plain: &str) -> bool {
        verify(plain, &self.password).unwrap_or(false)
    }
    
    /// Create a JWT token for this user
    pub fn create_session_token(&self, config: &AdminxConfig) -> Result<String, Box<dyn std::error::Error>> {
        let admin_id = self.id.as_ref()
            .ok_or("Missing admin ID")?
            .to_string();
        
        // Convert anyhow::Error to Box<dyn std::error::Error>
        create_jwt_token(&admin_id, &self.email, "admin", config)
            .map_err(|e| {
                let error: Box<dyn std::error::Error> = Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string()
                ));
                error
            })
    }
    
    /// Create a JWT token with custom roles
    pub fn create_session_token_with_roles(
        &self, 
        config: &AdminxConfig,
        additional_roles: Vec<String>
    ) -> Result<String, Box<dyn std::error::Error>> {
        let admin_id = self.id.as_ref()
            .ok_or("Missing admin ID")?
            .to_string();
        
        // Convert anyhow::Error to Box<dyn std::error::Error>
        crate::utils::jwt::create_jwt_token_with_roles(
            &admin_id, 
            &self.email, 
            "admin", 
            additional_roles,
            config
        ).map_err(|e| {
            let error: Box<dyn std::error::Error> = Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            ));
            error
        })
    }
    
    /// Check if user is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, AdminxStatus::Active) && !self.delete
    }
    
    /// Get user display name
    pub fn display_name(&self) -> &str {
        if self.username.is_empty() {
            &self.email
        } else {
            &self.username
        }
    }
    
    /// Update user's last login time (you might want to add this field to the struct)
    pub async fn update_last_login(&mut self) -> Result<(), mongodb::error::Error> {
        let db = get_adminx_database();
        let collection = db.collection::<AdminxUser>("adminxs");
        
        let now = BsonDateTime::now();
        self.updated_at = now;
        
        if let Some(id) = &self.id {
            collection.update_one(
                doc! { "_id": id },
                doc! { "$set": { "updated_at": now } },
                None,
            ).await?;
        }
        
        Ok(())
    }
    
    /// Sanitized version for API responses (no password)
    pub fn to_public(&self) -> AdminxUserPublic {
        AdminxUserPublic {
            id: self.id,
            username: self.username.clone(),
            email: self.email.clone(),
            delete: self.delete,
            status: self.status.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }



    pub async fn create_new_user(
        username: String,
        email: String,
        plain_password: String,
    ) -> Result<ObjectId, Box<dyn std::error::Error>> {
        // Validate inputs
        if email.trim().is_empty() {
            return Err("Email cannot be empty".into());
        }
        if plain_password.len() < 8 {
            return Err("Password must be at least 8 characters long".into());
        }
        
        // Check if user already exists
        if get_admin_by_email(&email).await.is_some() {
            return Err(format!("User with email {} already exists", email).into());
        }

        // Hash the password
        let hashed_password = bcrypt::hash(plain_password, bcrypt::DEFAULT_COST)
            .map_err(|e| format!("Failed to hash password: {}", e))?;

        let now = BsonDateTime::now();
        let new_user = AdminxUser {
            id: None, // Will be set by MongoDB
            username,
            email: email.to_lowercase(), // Normalize email
            password: hashed_password,
            delete: false,
            status: AdminxStatus::Active,
            created_at: now,
            updated_at: now,
        };

        // Insert into database
        let db = get_adminx_database();
        let collection = db.collection::<AdminxUser>("adminxs");
        
        let result = collection.insert_one(&new_user, None).await
            .map_err(|e| format!("Failed to create user: {}", e))?;

        Ok(result.inserted_id.as_object_id().unwrap())
    }

    /// Create a new AdminX user with custom status
    pub async fn create_new_user_with_status(
        username: String,
        email: String,
        plain_password: String,
        status: AdminxStatus,
    ) -> Result<ObjectId, Box<dyn std::error::Error>> {
        // Validate inputs
        if email.trim().is_empty() {
            return Err("Email cannot be empty".into());
        }
        if plain_password.len() < 8 {
            return Err("Password must be at least 8 characters long".into());
        }
        
        // Check if user already exists
        if get_admin_by_email(&email).await.is_some() {
            return Err(format!("User with email {} already exists", email).into());
        }

        // Hash the password
        let hashed_password = bcrypt::hash(plain_password, bcrypt::DEFAULT_COST)
            .map_err(|e| format!("Failed to hash password: {}", e))?;

        let now = BsonDateTime::now();
        let new_user = AdminxUser {
            id: None,
            username,
            email: email.to_lowercase(),
            password: hashed_password,
            delete: false,
            status,
            created_at: now,
            updated_at: now,
        };

        // Insert into database
        let db = get_adminx_database();
        let collection = db.collection::<AdminxUser>("adminxs");
        
        let result = collection.insert_one(&new_user, None).await
            .map_err(|e| format!("Failed to create user: {}", e))?;

        Ok(result.inserted_id.as_object_id().unwrap())
    }

    /// Bulk create multiple AdminX users
    pub async fn create_multiple_users(
        users: Vec<(String, String, String)>, // (username, email, password) tuples
    ) -> Result<Vec<ObjectId>, Box<dyn std::error::Error>> {
        let mut user_ids = Vec::new();
        
        for (username, email, password) in users {
            match Self::create_new_user(username, email, password).await {
                Ok(id) => user_ids.push(id),
                Err(e) => {
                    tracing::error!("Failed to create user: {}", e);
                    return Err(e);
                }
            }
        }
        
        Ok(user_ids)
    }

    /// Update user password (requires current password verification)
    pub async fn update_password(
        &mut self,
        current_password: &str,
        new_password: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Verify current password
        if !self.verify_password(current_password) {
            return Err("Current password is incorrect".into());
        }

        // Validate new password
        if new_password.len() < 8 {
            return Err("New password must be at least 8 characters long".into());
        }

        // Hash new password
        let hashed_password = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| format!("Failed to hash password: {}", e))?;

        // Update in database
        let db = get_adminx_database();
        let collection = db.collection::<AdminxUser>("adminxs");
        
        if let Some(id) = &self.id {
            let result = collection.update_one(
                doc! { "_id": id },
                doc! { 
                    "$set": { 
                        "password": &hashed_password,
                        "updated_at": BsonDateTime::now()
                    }
                },
                None,
            ).await
            .map_err(|e| format!("Failed to update password: {}", e))?;

            if result.modified_count > 0 {
                self.password = hashed_password;
                self.updated_at = BsonDateTime::now();
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Err("User ID is missing".into())
        }
    }

    /// Update user profile information
    pub async fn update_profile(
        &mut self,
        new_username: Option<String>,
        new_email: Option<String>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let mut update_doc = doc! {};
        
        // Validate and update username if provided
        if let Some(username) = new_username {
            if !username.trim().is_empty() {
                update_doc.insert("username", username.clone());
                self.username = username;
            }
        }

        // Validate and update email if provided
        if let Some(email) = new_email {
            if email.trim().is_empty() {
                return Err("Email cannot be empty".into());
            }
            
            let normalized_email = email.to_lowercase();
            
            // Check if email is already taken by another user
            if let Some(existing_user) = get_admin_by_email(&normalized_email).await {
                if Some(existing_user.id.unwrap()) != self.id {
                    return Err(format!("Email {} is already taken", normalized_email).into());
                }
            }
            
            update_doc.insert("email", normalized_email.clone());
            self.email = normalized_email;
        }

        if update_doc.is_empty() {
            return Ok(false); // Nothing to update
        }

        update_doc.insert("updated_at", BsonDateTime::now());

        // Update in database
        let db = get_adminx_database();
        let collection = db.collection::<AdminxUser>("adminxs");
        
        if let Some(id) = &self.id {
            let result = collection.update_one(
                doc! { "_id": id },
                doc! { "$set": update_doc },
                None,
            ).await
            .map_err(|e| format!("Failed to update profile: {}", e))?;

            if result.modified_count > 0 {
                self.updated_at = BsonDateTime::now();
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Err("User ID is missing".into())
        }
    }


    
}

/// Public version of AdminxUser without sensitive data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdminxUserPublic {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub username: String,
    pub email: String,
    pub delete: bool,
    pub status: AdminxStatus,
    pub created_at: BsonDateTime,
    pub updated_at: BsonDateTime,
}

// Database operations
pub async fn get_admin_by_email(email: &str) -> Option<AdminxUser> {
    let db = get_adminx_database();
    let collection = db.collection::<AdminxUser>("adminxs");
    
    tracing::debug!("Searching for admin with email: {}", email);
    
    match collection.find_one(doc! { 
        "email": email,
        "delete": false // Only return non-deleted users
    }, None).await {
        Ok(user) => {
            if user.is_some() {
                tracing::debug!("Admin found for email: {}", email);
            } else {
                tracing::debug!("No admin found for email: {}", email);
            }
            user
        }
        Err(e) => {
            tracing::error!("Database error while searching for admin {}: {}", email, e);
            None
        }
    }
}

pub async fn get_admin_by_id(id: &ObjectId) -> Option<AdminxUser> {
    let db = get_adminx_database();
    let collection = db.collection::<AdminxUser>("adminxs");
    
    match collection.find_one(doc! { 
        "_id": id,
        "delete": false
    }, None).await {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("Database error while searching for admin by ID {}: {}", id, e);
            None
        }
    }
}

pub async fn get_all_admins(include_deleted: bool) -> Result<Vec<AdminxUser>, mongodb::error::Error> {
    let db = get_adminx_database();
    let collection = db.collection::<AdminxUser>("adminxs");
    
    let filter = if include_deleted {
        doc! {}
    } else {
        doc! { "delete": false }
    };
    
    let mut cursor = collection.find(filter, None).await?;
    let mut users = Vec::new();
    
    use futures::stream::TryStreamExt;
    while let Some(user) = cursor.try_next().await? {
        users.push(user);
    }
    
    Ok(users)
}

pub async fn count_active_admins() -> Result<u64, mongodb::error::Error> {
    let db = get_adminx_database();
    let collection = db.collection::<AdminxUser>("adminxs");
    
    collection.count_documents(doc! {
        "delete": false,
        "status": "active"
    }, None).await
}

pub async fn delete_admin_by_id(id: &ObjectId) -> Result<bool, mongodb::error::Error> {
    let db = get_adminx_database();
    let collection = db.collection::<AdminxUser>("adminxs");
    
    let result = collection.update_one(
        doc! { "_id": id },
        doc! { 
            "$set": { 
                "delete": true,
                "updated_at": BsonDateTime::now()
            }
        },
        None,
    ).await?;
    
    Ok(result.modified_count > 0)
}

pub async fn update_admin_status(id: &ObjectId, status: AdminxStatus) -> Result<bool, mongodb::error::Error> {
    let db = get_adminx_database();
    let collection = db.collection::<AdminxUser>("adminxs");
    
    let status_bson = crate::utils::ubson::convert_to_bson(&status)
        .map_err(|e| mongodb::error::Error::custom(format!("Serialization error: {}", e)))?;
    
    let result = collection.update_one(
        doc! { "_id": id },
        doc! { 
            "$set": { 
                "status": status_bson,
                "updated_at": BsonDateTime::now()
            }
        },
        None,
    ).await?;
    
    Ok(result.modified_count > 0)
}