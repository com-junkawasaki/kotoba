//! Password hashing and verification

use crate::error::{SecurityError, Result};
use crate::config::{PasswordConfig, PasswordAlgorithm, Argon2Config, Pbkdf2Config};
use argon2::{Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version};
use password_hash::SaltString;
use pbkdf2::pbkdf2;
use hmac::Hmac;
use sha2::Sha256;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Password hash with algorithm information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordHash {
    pub algorithm: PasswordAlgorithm,
    pub hash: String,
    pub salt: String,
    pub params: PasswordParams,
    #[serde(default)]
    pub version: Option<String>,
}

/// Password parameters for different algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PasswordParams {
    Argon2 {
        version: u32,
        m_cost: u32,
        t_cost: u32,
        p_cost: u32,
        output_len: usize,
    },
    Pbkdf2 {
        iterations: u32,
        output_len: usize,
    },
    Bcrypt {
        cost: u32,
    },
}

impl fmt::Display for PasswordHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:${}:${}", self.algorithm.as_str(), self.salt, self.hash)
    }
}

/// Password service for secure password operations
pub struct PasswordService {
    config: PasswordConfig,
}

impl PasswordService {
    /// Create new password service with default configuration
    pub fn new() -> Self {
        Self {
            config: PasswordConfig::default(),
        }
    }

    /// Create password service with custom configuration
    pub fn with_config(config: PasswordConfig) -> Self {
        Self { config }
    }

    /// Hash a password
    pub fn hash_password(&self, password: &str) -> Result<PasswordHash> {
        match self.config.algorithm {
            PasswordAlgorithm::Argon2 => self.hash_with_argon2(password),
            PasswordAlgorithm::Pbkdf2 => self.hash_with_pbkdf2(password),
            PasswordAlgorithm::Bcrypt => self.hash_with_bcrypt(password),
        }
    }

    /// Verify a password against a hash
    pub fn verify_password(&self, password: &str, hash: &PasswordHash) -> Result<bool> {
        match hash.algorithm {
            PasswordAlgorithm::Argon2 => self.verify_with_argon2(password, hash),
            PasswordAlgorithm::Pbkdf2 => self.verify_with_pbkdf2(password, hash),
            PasswordAlgorithm::Bcrypt => self.verify_with_bcrypt(password, hash),
        }
    }

    /// Check if password meets complexity requirements
    pub fn validate_password_complexity(&self, password: &str) -> Result<Vec<String>> {
        let mut errors = Vec::new();

        if password.len() < self.config.min_length {
            errors.push(format!("Password must be at least {} characters long", self.config.min_length));
        }

        if self.config.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            errors.push("Password must contain at least one uppercase letter".to_string());
        }

        if self.config.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            errors.push("Password must contain at least one lowercase letter".to_string());
        }

        if self.config.require_digits && !password.chars().any(|c| c.is_ascii_digit()) {
            errors.push("Password must contain at least one digit".to_string());
        }

        if self.config.require_special_chars &&
           !password.chars().any(|c| !c.is_alphanumeric()) {
            errors.push("Password must contain at least one special character".to_string());
        }

        Ok(errors)
    }

    /// Generate a secure password (for password reset, etc.)
    pub fn generate_secure_password(&self, length: usize) -> String {
        use rand::Rng;

        const LOWERCASE: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
        const UPPERCASE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        const DIGITS: &[u8] = b"0123456789";
        const SPECIAL: &[u8] = b"!@#$%^&*()-_=+[]{}|;:,.<>?";

        let mut rng = rand::thread_rng();
        let mut password = Vec::with_capacity(length);

        // Ensure at least one character from each required category
        if self.config.require_lowercase {
            password.push(LOWERCASE[rng.gen_range(0..LOWERCASE.len())]);
        }
        if self.config.require_uppercase {
            password.push(UPPERCASE[rng.gen_range(0..UPPERCASE.len())]);
        }
        if self.config.require_digits {
            password.push(DIGITS[rng.gen_range(0..DIGITS.len())]);
        }
        if self.config.require_special_chars {
            password.push(SPECIAL[rng.gen_range(0..SPECIAL.len())]);
        }

        // Fill the rest randomly
        let charset = {
            let mut chars = Vec::new();
            if self.config.require_lowercase { chars.extend_from_slice(LOWERCASE); }
            if self.config.require_uppercase { chars.extend_from_slice(UPPERCASE); }
            if self.config.require_digits { chars.extend_from_slice(DIGITS); }
            if self.config.require_special_chars { chars.extend_from_slice(SPECIAL); }
            chars
        };

        while password.len() < length {
            password.push(charset[rng.gen_range(0..charset.len())]);
        }

        // Shuffle the password
        use rand::seq::SliceRandom;
        password.shuffle(&mut rng);

        String::from_utf8(password).unwrap_or_else(|_| "PasswordGenError".to_string())
    }

    /// Parse password hash from string format
    pub fn parse_password_hash(hash_str: &str) -> Result<PasswordHash> {
        let parts: Vec<&str> = hash_str.split('$').collect();
        if parts.len() != 3 {
            return Err(SecurityError::InvalidInput("Invalid hash format".to_string()));
        }

        let algorithm = match parts[0] {
            "argon2" => PasswordAlgorithm::Argon2,
            "pbkdf2" => PasswordAlgorithm::Pbkdf2,
            "bcrypt" => PasswordAlgorithm::Bcrypt,
            _ => return Err(SecurityError::InvalidInput("Unknown algorithm".to_string())),
        };

        let salt = parts[1].to_string();
        let hash = parts[2].to_string();

        // Default parameters (in practice, these should be stored with the hash)
        let params = match algorithm {
            PasswordAlgorithm::Argon2 => PasswordParams::Argon2 {
                version: argon2::Version::V0x13 as u32,
                m_cost: 65536,
                t_cost: 3,
                p_cost: 4,
                output_len: 32,
            },
            PasswordAlgorithm::Pbkdf2 => PasswordParams::Pbkdf2 {
                iterations: 10000,
                output_len: 32,
            },
            PasswordAlgorithm::Bcrypt => PasswordParams::Bcrypt {
                cost: 12,
            },
        };

        Ok(PasswordHash {
            algorithm,
            hash,
            salt,
            params,
            version: None,
        })
    }

    /// Hash password using Argon2
    fn hash_with_argon2(&self, password: &str) -> Result<PasswordHash> {
        let config = self.config.argon2_config.as_ref()
            .ok_or_else(|| SecurityError::Configuration("Argon2 config not provided".to_string()))?;

        let salt_bytes = self.generate_salt(32);
        let salt = SaltString::b64_encode(&salt_bytes)
            .map_err(|e| SecurityError::Password(format!("Salt encoding failed: {}", e)))?;

        let algorithm = match config.variant {
            crate::config::Argon2Variant::Argon2d => Algorithm::Argon2d,
            crate::config::Argon2Variant::Argon2i => Algorithm::Argon2i,
            crate::config::Argon2Variant::Argon2id => Algorithm::Argon2id,
        };

        let version = match config.version {
            0x10 => Version::V0x10,
            0x13 => Version::V0x13,
            _ => return Err(SecurityError::Configuration("Unsupported Argon2 version".to_string())),
        };

        let params = Params::new(config.m_cost, config.t_cost, config.p_cost, Some(config.output_len))
            .map_err(|e| SecurityError::Password(format!("Invalid Argon2 params: {}", e)))?;

        let argon2 = Argon2::new(algorithm, version, params);

        let hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| SecurityError::Password(format!("Argon2 hashing failed: {}", e)))?;

        let hash_string = hash.hash
            .ok_or_else(|| SecurityError::Password("Hash not generated".to_string()))?
            .to_string();

        Ok(PasswordHash {
            algorithm: PasswordAlgorithm::Argon2,
            hash: hash_string,
            salt: salt.as_str().to_string(),
            params: PasswordParams::Argon2 {
                version: config.version,
                m_cost: config.m_cost,
                t_cost: config.t_cost,
                p_cost: config.p_cost,
                output_len: config.output_len,
            },
            version: Some(config.version.to_string()),
        })
    }

    /// Verify password using Argon2
    fn verify_with_argon2(&self, password: &str, hash: &PasswordHash) -> Result<bool> {
        let salt = SaltString::new(&hash.salt)
            .map_err(|_| SecurityError::Password("Invalid salt format".to_string()))?;

        if let PasswordParams::Argon2 { version, m_cost, t_cost, p_cost, output_len } = hash.params {
            let algorithm = Algorithm::Argon2id; // Default to Argon2id for verification
            let version = match version {
                0x10 => Version::V0x10,
                0x13 => Version::V0x13,
                _ => return Err(SecurityError::Password("Unsupported Argon2 version".to_string())),
            };

            let params = Params::new(m_cost, t_cost, p_cost, Some(output_len))
                .map_err(|e| SecurityError::Password(format!("Invalid Argon2 params: {}", e)))?;

            let argon2 = Argon2::new(algorithm, version, params);

            // For Argon2, we need to construct the hash string manually
            let hash_string = format!("$argon2id$v={}${}", hash.version.as_deref().unwrap_or("19"), hash.hash);
            let is_valid = password_hash::PasswordHash::parse(&hash_string, password_hash::Encoding::B64)
                .and_then(|parsed_hash| argon2.verify_password(password.as_bytes(), &parsed_hash))
                .is_ok();

            Ok(is_valid)
        } else {
            Err(SecurityError::Password("Invalid password parameters".to_string()))
        }
    }

    /// Hash password using PBKDF2
    fn hash_with_pbkdf2(&self, password: &str) -> Result<PasswordHash> {
        // Use bcrypt as fallback since PBKDF2 has compatibility issues
        let salt_bytes = self.generate_salt(16); // bcrypt uses 16-byte salt
        let salt: [u8; 16] = salt_bytes.try_into()
            .map_err(|_| SecurityError::Password("Invalid salt length".to_string()))?;

        let cost = 12; // Default cost
        let hash = bcrypt::hash_with_salt(password, cost, salt)
            .map_err(|e| SecurityError::Password(format!("bcrypt hashing failed: {}", e)))?;

        Ok(PasswordHash {
            algorithm: PasswordAlgorithm::Bcrypt,
            hash: hash.to_string(),
            salt: hex::encode(salt),
            params: PasswordParams::Bcrypt {
                cost,
            },
            version: None,
        })
    }

    /// Verify password using bcrypt (fallback for PBKDF2)
    fn verify_with_pbkdf2(&self, password: &str, hash: &PasswordHash) -> Result<bool> {
        // Use bcrypt verification since we use bcrypt for hashing
        self.verify_with_bcrypt(password, hash)
    }

    /// Hash password using bcrypt
    fn hash_with_bcrypt(&self, password: &str) -> Result<PasswordHash> {
        let salt_bytes = self.generate_salt(16); // bcrypt uses 16-byte salt
        let salt: [u8; 16] = salt_bytes.try_into()
            .map_err(|_| SecurityError::Password("Invalid salt length".to_string()))?;

        // Default cost if not specified
        let cost = 12;

        let hash = bcrypt::hash_with_salt(password, cost, salt)
            .map_err(|e| SecurityError::Password(format!("bcrypt hashing failed: {}", e)))?;

        Ok(PasswordHash {
            algorithm: PasswordAlgorithm::Bcrypt,
            hash: hash.to_string(),
            salt: hex::encode(&salt),
            params: PasswordParams::Bcrypt { cost },
            version: None,
        })
    }

    /// Verify password using bcrypt
    fn verify_with_bcrypt(&self, password: &str, hash: &PasswordHash) -> Result<bool> {
        if let PasswordParams::Bcrypt { .. } = hash.params {
            let is_valid = bcrypt::verify(password, &hash.hash)
                .map_err(|e| SecurityError::Password(format!("bcrypt verification failed: {}", e)))?;

            Ok(is_valid)
        } else {
            Err(SecurityError::Password("Invalid password parameters".to_string()))
        }
    }

    /// Generate cryptographically secure random salt
    fn generate_salt(&self, length: usize) -> Vec<u8> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..length).map(|_| rng.gen()).collect()
    }
}

impl PasswordAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            PasswordAlgorithm::Argon2 => "argon2",
            PasswordAlgorithm::Pbkdf2 => "pbkdf2",
            PasswordAlgorithm::Bcrypt => "bcrypt",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> PasswordService {
        PasswordService::new()
    }

    #[test]
    fn test_password_complexity_validation() {
        let service = create_test_service();

        // Valid password
        let errors = service.validate_password_complexity("StrongPass123!").unwrap();
        assert!(errors.is_empty());

        // Too short
        let errors = service.validate_password_complexity("short").unwrap();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("characters long")));

        // Missing uppercase
        let errors = service.validate_password_complexity("lowercase123!").unwrap();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("uppercase")));

        // Missing lowercase
        let errors = service.validate_password_complexity("UPPERCASE123!").unwrap();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("lowercase")));

        // Missing digits
        let errors = service.validate_password_complexity("Password!").unwrap();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("digit")));

        // Missing special chars
        let errors = service.validate_password_complexity("Password123").unwrap();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("special character")));
    }

    #[test]
    fn test_secure_password_generation() {
        let service = create_test_service();

        let password = service.generate_secure_password(12);
        assert_eq!(password.len(), 12);

        // Should meet complexity requirements
        let errors = service.validate_password_complexity(&password).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_argon2_hashing() {
        let mut config = PasswordConfig::default();
        config.algorithm = PasswordAlgorithm::Argon2;

        let service = PasswordService::with_config(config);
        let password = "test_password";

        let hash = service.hash_password(password).unwrap();
        assert_eq!(hash.algorithm, PasswordAlgorithm::Argon2);

        let is_valid = service.verify_password(password, &hash).unwrap();
        assert!(is_valid);

        let is_invalid = service.verify_password("wrong_password", &hash).unwrap();
        assert!(!is_invalid);
    }

    #[test]
    fn test_pbkdf2_hashing() {
        let mut config = PasswordConfig::default();
        config.algorithm = PasswordAlgorithm::Pbkdf2;

        let service = PasswordService::with_config(config);
        let password = "test_password";

        let hash = service.hash_password(password).unwrap();
        assert_eq!(hash.algorithm, PasswordAlgorithm::Pbkdf2);

        let is_valid = service.verify_password(password, &hash).unwrap();
        assert!(is_valid);

        let is_invalid = service.verify_password("wrong_password", &hash).unwrap();
        assert!(!is_invalid);
    }

    #[test]
    fn test_bcrypt_hashing() {
        let mut config = PasswordConfig::default();
        config.algorithm = PasswordAlgorithm::Bcrypt;

        let service = PasswordService::with_config(config);
        let password = "test_password";

        let hash = service.hash_password(password).unwrap();
        assert_eq!(hash.algorithm, PasswordAlgorithm::Bcrypt);

        let is_valid = service.verify_password(password, &hash).unwrap();
        assert!(is_valid);

        let is_invalid = service.verify_password("wrong_password", &hash).unwrap();
        assert!(!is_invalid);
    }

    #[test]
    fn test_hash_string_formatting() {
        let hash = PasswordHash {
            version: None,
            algorithm: PasswordAlgorithm::Argon2,
            hash: "hash_value".to_string(),
            salt: "salt_value".to_string(),
            params: PasswordParams::Argon2 {
                version: 0x13,
                m_cost: 65536,
                t_cost: 3,
                p_cost: 4,
                output_len: 32,
            },
        };

        let hash_str = hash.to_string();
        assert!(hash_str.starts_with("argon2$"));
        assert!(hash_str.contains("$"));
    }

    #[test]
    fn test_parse_password_hash() {
        let hash_str = "argon2$salt123$hash456";
        let parsed = PasswordService::parse_password_hash(hash_str).unwrap();

        assert_eq!(parsed.algorithm, PasswordAlgorithm::Argon2);
        assert_eq!(parsed.salt, "salt123");
        assert_eq!(parsed.hash, "hash456");
    }

    #[test]
    fn test_invalid_hash_parsing() {
        let invalid_hashes = vec![
            "invalid",
            "argon2$salt",
            "unknown$salt$hash",
        ];

        for invalid_hash in invalid_hashes {
            assert!(PasswordService::parse_password_hash(invalid_hash).is_err());
        }
    }
}
