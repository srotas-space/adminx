// adminx/src/utils/jwt.rs - Fixed version
use jsonwebtoken::{encode, EncodingKey, Header};
use anyhow::{Result, Context};
use crate::configs::initializer::AdminxConfig;
use crate::utils::structs::Claims; // ✅ Use centralized Claims from structs.rs

pub fn create_jwt_token(
    user_id: &str, 
    email: &str, 
    role: &str,
    config: &AdminxConfig,
) -> Result<String> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(config.session_timeout.as_secs() as i64))
        .expect("valid timestamp")
        .timestamp() as usize;
    
    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration,
        email: email.to_owned(),
        role: role.to_owned(),
        roles: vec![role.to_owned()], // Include primary role in roles array
    };
    
    let token = encode(
        &Header::default(), 
        &claims, 
        &EncodingKey::from_secret(config.jwt_secret.as_ref())
    )
    .context("Failed to encode JWT")?;
    
    Ok(token)
}

// Additional utility functions for JWT management
pub fn create_jwt_token_with_roles(
    user_id: &str, 
    email: &str, 
    role: &str,
    additional_roles: Vec<String>,
    config: &AdminxConfig,
) -> Result<String> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(config.session_timeout.as_secs() as i64))
        .expect("valid timestamp")
        .timestamp() as usize;
    
    let mut all_roles = additional_roles;
    if !all_roles.contains(&role.to_string()) {
        all_roles.push(role.to_owned());
    }
    
    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration,
        email: email.to_owned(),
        role: role.to_owned(),
        roles: all_roles,
    };
    
    let token = encode(
        &Header::default(), 
        &claims, 
        &EncodingKey::from_secret(config.jwt_secret.as_ref())
    )
    .context("Failed to encode JWT")?;
    
    Ok(token)
}

// Create token with custom expiration
pub fn create_jwt_token_with_expiration(
    user_id: &str, 
    email: &str, 
    role: &str,
    config: &AdminxConfig,
    duration: chrono::Duration,
) -> Result<String> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(duration)
        .expect("valid timestamp")
        .timestamp() as usize;
    
    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration,
        email: email.to_owned(),
        role: role.to_owned(),
        roles: vec![role.to_owned()],
    };
    
    let token = encode(
        &Header::default(), 
        &claims, 
        &EncodingKey::from_secret(config.jwt_secret.as_ref())
    )
    .context("Failed to encode JWT")?;
    
    Ok(token)
}

// Validate JWT token structure (without signature verification)
pub fn validate_token_structure(token: &str) -> Result<Claims> {
    use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
    
    // This is just for structure validation - we use a dummy key
    let dummy_key = DecodingKey::from_secret(b"dummy");
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false; // Don't validate expiration for structure check
    
    match decode::<Claims>(token, &dummy_key, &validation) {
        Ok(token_data) => Ok(token_data.claims),
        Err(_) => {
            // Try to decode just the payload without verification
            let parts: Vec<&str> = token.split('.').collect();
            if parts.len() != 3 {
                return Err(anyhow::anyhow!("Invalid token format"));
            }
            
            use base64::{Engine as _, engine::general_purpose};
            let payload = general_purpose::STANDARD_NO_PAD
                .decode(parts[1])
                .context("Failed to decode token payload")?;
            
            let claims: Claims = serde_json::from_slice(&payload)
                .context("Failed to parse token claims")?;
            
            Ok(claims)
        }
    }
}

// Check if token is expired
pub fn is_token_expired(claims: &Claims) -> bool {
    let now = chrono::Utc::now().timestamp() as usize;
    claims.exp < now
}

// Get time until token expires
pub fn time_until_expiration(claims: &Claims) -> Option<chrono::Duration> {
    let now = chrono::Utc::now().timestamp() as usize;
    if claims.exp > now {
        Some(chrono::Duration::seconds((claims.exp - now) as i64))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    fn test_config() -> AdminxConfig {
        AdminxConfig {
            jwt_secret: "test_secret_key_that_is_long_enough_for_testing_purposes".to_string(),
            session_secret: "test_session_secret_that_is_definitely_long_enough_for_secure_testing".to_string(),
            environment: "test".to_string(),
            log_level: "debug".to_string(),
            session_timeout: Duration::from_secs(3600),
        }
    }
    
    #[test]
    fn test_create_jwt_token() {
        let config = test_config();
        let token = create_jwt_token("123", "test@example.com", "admin", &config);
        assert!(token.is_ok());
    }
    
    #[test]
    fn test_token_structure_validation() {
        let config = test_config();
        let token = create_jwt_token("123", "test@example.com", "admin", &config).unwrap();
        let claims = validate_token_structure(&token);
        assert!(claims.is_ok());
        
        let claims = claims.unwrap();
        assert_eq!(claims.sub, "123");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.role, "admin");
    }
    
    #[test]
    fn test_token_expiration_check() {
        let config = test_config();
        let token = create_jwt_token("123", "test@example.com", "admin", &config).unwrap();
        let claims = validate_token_structure(&token).unwrap();
        
        // Token should not be expired immediately after creation
        assert!(!is_token_expired(&claims));
        
        // Should have time until expiration
        assert!(time_until_expiration(&claims).is_some());
    }
}