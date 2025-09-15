//! Multi-Factor Authentication (MFA) using TOTP

use crate::error::{SecurityError, Result};
use crate::config::MfaConfig;
use qrcode::QrCode;
use qrcode::render::svg;
use totp_rs::{Algorithm, Secret, TOTP};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::Rng;

/// MFA secret and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaSecret {
    pub secret: String,
    pub algorithm: MfaAlgorithm,
    pub digits: usize,
    pub skew: u8,
    pub step: u64,
    pub issuer: String,
    pub account_name: String,
}

/// MFA code for verification
#[derive(Debug, Clone)]
pub struct MfaCode {
    pub code: String,
    pub timestamp: u64,
}

/// MFA algorithm types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MfaAlgorithm {
    SHA1,
    SHA256,
    SHA512,
}

impl From<MfaAlgorithm> for Algorithm {
    fn from(alg: MfaAlgorithm) -> Self {
        match alg {
            MfaAlgorithm::SHA1 => Algorithm::SHA1,
            MfaAlgorithm::SHA256 => Algorithm::SHA256,
            MfaAlgorithm::SHA512 => Algorithm::SHA512,
        }
    }
}

/// MFA service for managing TOTP-based authentication
pub struct MfaService {
    config: MfaConfig,
}

impl MfaService {
    /// Create new MFA service
    pub fn new() -> Self {
        Self {
            config: MfaConfig::default(),
        }
    }

    /// Create MFA service with custom configuration
    pub fn with_config(config: MfaConfig) -> Self {
        Self { config }
    }

    /// Generate MFA secret and QR code for user
    pub fn generate_secret(&self, account_name: &str) -> Result<(String, String)> {
        let mut rng = rand::thread_rng();

        // Generate random secret (20 bytes = 32 hex characters)
        let secret_bytes: [u8; 20] = rng.gen();
        let secret = Secret::Encoded(hex::encode(secret_bytes));

        // Create TOTP instance
        let _totp = TOTP::new(
            Algorithm::SHA1,
            self.config.digits.into(),
            self.config.skew,
            self.config.step,
            secret.to_bytes()
                .map_err(|e| SecurityError::Mfa(format!("Failed to decode secret: {}", e)))?,
        ).map_err(|e| SecurityError::Mfa(format!("Failed to create TOTP: {}", e)))?;

        // Generate TOTP URI manually
        let issuer_encoded = self.config.issuer.replace(" ", "%20").replace(":", "%3A");
        let account_encoded = account_name.replace(" ", "%20").replace(":", "%3A");

        let url = format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm={}&digits={}&period={}",
            issuer_encoded,
            account_encoded,
            hex::encode(&secret_bytes),
            issuer_encoded,
            "SHA1",
            self.config.digits,
            self.config.step
        );

        // Generate QR code
        let qr_code = QrCode::new(url.as_bytes())
            .map_err(|e| SecurityError::Mfa(format!("Failed to generate QR code: {}", e)))?;

        let qr_svg = qr_code
            .render()
            .min_dimensions(self.config.qr_code_size, self.config.qr_code_size)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#FFFFFF"))
            .build();

        let secret_hex = hex::encode(secret_bytes);

        Ok((secret_hex, qr_svg))
    }

    /// Generate MFA secret and detailed information
    pub fn generate_secret_detailed(&self, account_name: &str) -> Result<MfaSecret> {
        let mut rng = rand::thread_rng();
        let secret_bytes: [u8; 20] = rng.gen();
        let secret_hex = hex::encode(secret_bytes);

        let secret = MfaSecret {
            secret: secret_hex,
            algorithm: MfaAlgorithm::SHA1,
            digits: self.config.digits as usize,
            skew: self.config.skew,
            step: self.config.step,
            issuer: self.config.issuer.clone(),
            account_name: account_name.to_string(),
        };

        Ok(secret)
    }

    /// Generate QR code from existing secret
    pub fn generate_qr_code(&self, secret: &MfaSecret) -> Result<String> {
        let _secret_bytes = hex::decode(&secret.secret)
            .map_err(|e| SecurityError::Mfa(format!("Invalid secret hex: {}", e)))?;

        let totp_secret = Secret::Encoded(secret.secret.clone());

        let _totp = TOTP::new(
            secret.algorithm.clone().into(),
            secret.digits,
            secret.skew,
            secret.step,
            totp_secret.to_bytes()
                .map_err(|e| SecurityError::Mfa(format!("Failed to decode secret: {}", e)))?,
        ).map_err(|e| SecurityError::Mfa(format!("Failed to create TOTP: {}", e)))?;

        // Generate TOTP URI manually
        let issuer_encoded = secret.issuer.replace(" ", "%20").replace(":", "%3A");
        let account_encoded = secret.account_name.replace(" ", "%20").replace(":", "%3A");

        let url = format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm={}&digits={}&period={}",
            issuer_encoded,
            account_encoded,
            secret.secret,
            issuer_encoded,
            match secret.algorithm {
                MfaAlgorithm::SHA1 => "SHA1",
                MfaAlgorithm::SHA256 => "SHA256",
                MfaAlgorithm::SHA512 => "SHA512",
            },
            secret.digits,
            secret.step
        );

        let qr_code = QrCode::new(url.as_bytes())
            .map_err(|e| SecurityError::Mfa(format!("Failed to generate QR code: {}", e)))?;

        let qr_svg = qr_code
            .render()
            .min_dimensions(self.config.qr_code_size, self.config.qr_code_size)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#FFFFFF"))
            .build();

        Ok(qr_svg)
    }

    /// Verify MFA code
    pub fn verify_code(&self, secret_hex: &str, code: &str) -> Result<bool> {
        let _secret_bytes = hex::decode(secret_hex)
            .map_err(|e| SecurityError::Mfa(format!("Invalid secret hex: {}", e)))?;

        let secret = Secret::Encoded(secret_hex.to_string());

        let secret_bytes = secret.to_bytes()
            .map_err(|e| SecurityError::Mfa(format!("Failed to decode secret: {}", e)))?;
        let totp = TOTP::new(
            Algorithm::SHA1,
            self.config.digits.into(),
            self.config.skew,
            self.config.step,
            secret_bytes,
        ).map_err(|e| SecurityError::Mfa(format!("Failed to create TOTP: {}", e)))?;

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| SecurityError::Time(e.to_string()))?
            .as_secs();

        Ok(totp.check(code, current_time))
    }

    /// Verify MFA code with detailed secret
    pub fn verify_code_detailed(&self, secret: &MfaSecret, code: &str) -> Result<bool> {
        let secret_obj = Secret::Encoded(secret.secret.clone());

        let totp = TOTP::new(
            secret.algorithm.clone().into(),
            secret.digits,
            secret.skew,
            secret.step,
            secret_obj.to_bytes().map_err(|e| SecurityError::Mfa(format!("Failed to decode secret: {}", e)))?,
        ).map_err(|e| SecurityError::Mfa(format!("Failed to create TOTP: {}", e)))?;

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| SecurityError::Time(e.to_string()))?
            .as_secs();

        Ok(totp.check(code, current_time))
    }

    /// Generate backup codes
    pub fn generate_backup_codes(&self, count: usize) -> Vec<String> {
        let mut rng = rand::thread_rng();
        let mut codes = Vec::with_capacity(count);

        for _ in 0..count {
            let code: u32 = rng.gen_range(100000..999999);
            codes.push(format!("{:06}", code));
        }

        codes
    }

    /// Get current TOTP code for a secret (for testing purposes)
    pub fn get_current_code(&self, secret_hex: &str) -> Result<String> {
        let secret = Secret::Encoded(secret_hex.to_string());
        let secret_bytes = secret.to_bytes()
            .map_err(|e| SecurityError::Mfa(format!("Failed to decode secret: {}", e)))?;

        let totp = TOTP::new(
            Algorithm::SHA1,
            self.config.digits.into(),
            self.config.skew,
            self.config.step,
            secret_bytes,
        ).map_err(|e| SecurityError::Mfa(format!("Failed to create TOTP: {}", e)))?;

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| SecurityError::Time(e.to_string()))?
            .as_secs();

        Ok(totp.generate(current_time))
    }

    /// Validate secret format
    pub fn validate_secret(&self, secret_hex: &str) -> Result<()> {
        // Check if it's valid hex
        hex::decode(secret_hex)
            .map_err(|_| SecurityError::InvalidInput("Invalid secret format".to_string()))?;

        // Check length (should be 40 hex characters for 20 bytes)
        if secret_hex.len() != 40 {
            return Err(SecurityError::InvalidInput("Secret must be 40 hex characters".to_string()));
        }

        Ok(())
    }

    /// Get remaining time until next code
    pub fn get_remaining_time(&self, secret: &MfaSecret) -> Result<u64> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| SecurityError::Time(e.to_string()))?
            .as_secs();

        let remaining = secret.step - (current_time % secret.step);
        Ok(remaining)
    }

    /// Create provisioning URI for manual entry
    pub fn create_provisioning_uri(&self, secret: &MfaSecret) -> Result<String> {
        let secret_obj = Secret::Encoded(secret.secret.clone());

        let _totp = TOTP::new(
            secret.algorithm.clone().into(),
            secret.digits,
            secret.skew,
            secret.step,
            secret_obj.to_bytes().map_err(|e| SecurityError::Mfa(format!("Failed to decode secret: {}", e)))?,
        ).map_err(|e| SecurityError::Mfa(format!("Failed to create TOTP: {}", e)))?;

        // Generate TOTP URI manually since get_url method may not be available
        // Note: For production, consider using a proper URL encoding library
        let issuer_encoded = secret.issuer.replace(" ", "%20").replace(":", "%3A");
        let account_encoded = secret.account_name.replace(" ", "%20").replace(":", "%3A");

        let uri = format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm={}&digits={}&period={}",
            issuer_encoded,
            account_encoded,
            secret.secret,
            issuer_encoded,
            match secret.algorithm {
                MfaAlgorithm::SHA1 => "SHA1",
                MfaAlgorithm::SHA256 => "SHA256",
                MfaAlgorithm::SHA512 => "SHA512",
            },
            secret.digits,
            secret.step
        );

        Ok(uri)
    }
}

/// MFA recovery codes manager
pub struct MfaRecoveryManager {
    codes: HashMap<String, Vec<String>>, // user_id -> recovery codes
}

impl MfaRecoveryManager {
    pub fn new() -> Self {
        Self {
            codes: HashMap::new(),
        }
    }

    /// Generate recovery codes for user
    pub fn generate_recovery_codes(&mut self, user_id: &str, count: usize) -> Vec<String> {
        let codes = (0..count)
            .map(|_| {
                let mut rng = rand::thread_rng();
                let code: u64 = rng.gen();
                hex::encode(&code.to_be_bytes()[..6]) // 12 hex characters
            })
            .collect::<Vec<_>>();

        self.codes.insert(user_id.to_string(), codes.clone());
        codes
    }

    /// Verify and consume recovery code
    pub fn verify_recovery_code(&mut self, user_id: &str, code: &str) -> bool {
        if let Some(codes) = self.codes.get_mut(user_id) {
            if let Some(pos) = codes.iter().position(|c| c == code) {
                codes.remove(pos);
                return true;
            }
        }
        false
    }

    /// Get remaining recovery codes count for user
    pub fn get_remaining_codes_count(&self, user_id: &str) -> usize {
        self.codes.get(user_id).map(|codes| codes.len()).unwrap_or(0)
    }

    /// Check if user has recovery codes
    pub fn has_recovery_codes(&self, user_id: &str) -> bool {
        self.codes.get(user_id).map(|codes| !codes.is_empty()).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn create_test_service() -> MfaService {
        MfaService::new()
    }

    #[test]
    fn test_generate_secret() {
        let service = create_test_service();
        let result = service.generate_secret("test@example.com");

        assert!(result.is_ok());
        let (secret, qr_code) = result.unwrap();

        assert_eq!(secret.len(), 40); // 20 bytes = 40 hex chars
        assert!(qr_code.contains("<svg")); // Should contain SVG markup
        assert!(qr_code.contains("test@example.com")); // Should contain account name
    }

    #[test]
    fn test_secret_validation() {
        let service = create_test_service();

        // Valid secret
        assert!(service.validate_secret("1234567890abcdef1234567890abcdef12345678").is_ok());

        // Invalid hex
        assert!(service.validate_secret("gggggggggggggggggggggggggggggggggggggggg").is_err());

        // Wrong length
        assert!(service.validate_secret("1234567890abcdef").is_err());
    }

    #[test]
    fn test_code_verification() {
        let service = create_test_service();

        // Generate secret
        let (secret, _) = service.generate_secret("test@example.com").unwrap();

        // Get current code
        let current_code = service.get_current_code(&secret).unwrap();

        // Verify the code (should work)
        let is_valid = service.verify_code(&secret, &current_code).unwrap();
        assert!(is_valid);

        // Verify wrong code (should fail)
        let is_valid_wrong = service.verify_code(&secret, "000000").unwrap();
        assert!(!is_valid_wrong);
    }

    #[test]
    fn test_code_expiration() {
        let service = create_test_service();

        // Generate secret
        let (secret, _) = service.generate_secret("test@example.com").unwrap();

        // Get current code
        let current_code = service.get_current_code(&secret).unwrap();

        // Wait for next time window
        thread::sleep(Duration::from_secs(31)); // TOTP step is 30 seconds

        // Old code should no longer be valid
        let is_valid_old = service.verify_code(&secret, &current_code).unwrap();
        assert!(!is_valid_old);
    }

    #[test]
    fn test_detailed_secret() {
        let service = create_test_service();
        let secret = service.generate_secret_detailed("test@example.com").unwrap();

        assert_eq!(secret.digits, 6);
        assert_eq!(secret.step, 30);
        assert_eq!(secret.algorithm, MfaAlgorithm::SHA1);
        assert_eq!(secret.issuer, "Kotoba");
        assert_eq!(secret.account_name, "test@example.com");

        // Generate QR code from detailed secret
        let qr_code = service.generate_qr_code(&secret).unwrap();
        assert!(qr_code.contains("<svg"));
    }

    #[test]
    fn test_backup_codes() {
        let service = create_test_service();
        let codes = service.generate_backup_codes(5);

        assert_eq!(codes.len(), 5);
        for code in &codes {
            assert_eq!(code.len(), 6); // 6-digit codes
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn test_recovery_manager() {
        let mut manager = MfaRecoveryManager::new();

        let user_id = "user123";
        let codes = manager.generate_recovery_codes(user_id, 5);

        assert_eq!(codes.len(), 5);
        assert_eq!(manager.get_remaining_codes_count(user_id), 5);

        // Verify a code
        let first_code = codes[0].clone();
        assert!(manager.verify_recovery_code(user_id, &first_code));
        assert_eq!(manager.get_remaining_codes_count(user_id), 4);

        // Try to use the same code again (should fail)
        assert!(!manager.verify_recovery_code(user_id, &first_code));
        assert_eq!(manager.get_remaining_codes_count(user_id), 4);

        // Try invalid code
        assert!(!manager.verify_recovery_code(user_id, "invalid"));
        assert_eq!(manager.get_remaining_codes_count(user_id), 4);
    }

    #[test]
    fn test_remaining_time() {
        let service = create_test_service();
        let secret = service.generate_secret_detailed("test@example.com").unwrap();

        let remaining = service.get_remaining_time(&secret).unwrap();
        assert!(remaining <= 30);
        assert!(remaining >= 0);
    }

    #[test]
    fn test_provisioning_uri() {
        let service = create_test_service();
        let secret = service.generate_secret_detailed("test@example.com").unwrap();

        let uri = service.create_provisioning_uri(&secret).unwrap();
        assert!(uri.starts_with("otpauth://totp/"));
        assert!(uri.contains("Kotoba"));
        assert!(uri.contains("test@example.com"));
    }
}
