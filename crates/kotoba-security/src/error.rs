//! Security error types and handling

use thiserror::Error;
use totp_rs::SecretParseError;

/// Result type for security operations
pub type Result<T> = std::result::Result<T, SecurityError>;

/// Comprehensive security error types
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Authorization failed: {0}")]
    Authorization(String),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("OAuth2 error: {0}")]
    OAuth2(String),

    #[error("MFA error: {0}")]
    Mfa(String),

    #[error("Password hashing error: {0}")]
    Password(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Time error: {0}")]
    Time(String),

    #[error("Cryptography error: {0}")]
    Crypto(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Token invalid")]
    TokenInvalid,

    #[error("User not found")]
    UserNotFound,

    #[error("User already exists")]
    UserExists,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("MFA required")]
    MfaRequired,

    #[error("MFA setup required")]
    MfaSetupRequired,

    #[error("Account locked")]
    AccountLocked,

    #[error("Account disabled")]
    AccountDisabled,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("Provider not supported")]
    ProviderNotSupported,

    #[error("State mismatch")]
    StateMismatch,

    #[error("CSRF token invalid")]
    CsrfTokenInvalid,

    #[error("Session expired")]
    SessionExpired,

    #[error("Session invalid")]
    SessionInvalid,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("External service error: {0}")]
    ExternalService(String),
}

impl SecurityError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            SecurityError::Http(_)
                | SecurityError::Io(_)
                | SecurityError::ExternalService(_)
                | SecurityError::Database(_)
                | SecurityError::Cache(_)
        )
    }

    /// Check if error is client error (4xx)
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            SecurityError::Authentication(_)
                | SecurityError::Authorization(_)
                | SecurityError::InvalidInput(_)
                | SecurityError::TokenExpired
                | SecurityError::TokenInvalid
                | SecurityError::UserNotFound
                | SecurityError::InvalidCredentials
                | SecurityError::InsufficientPermissions
                | SecurityError::CsrfTokenInvalid
                | SecurityError::StateMismatch
                | SecurityError::RateLimitExceeded
        )
    }

    /// Check if error is server error (5xx)
    pub fn is_server_error(&self) -> bool {
        matches!(
            self,
            SecurityError::Configuration(_)
                | SecurityError::Jwt(_)
                | SecurityError::OAuth2(_)
                | SecurityError::Mfa(_)
                | SecurityError::Password(_)
                | SecurityError::Session(_)
                | SecurityError::Json(_)
                | SecurityError::Url(_)
                | SecurityError::Time(_)
                | SecurityError::Crypto(_)
                | SecurityError::Database(_)
                | SecurityError::Cache(_)
                | SecurityError::ExternalService(_)
        )
    }

    /// Get HTTP status code for error
    pub fn http_status_code(&self) -> u16 {
        match self {
            // 4xx Client Errors
            SecurityError::Authentication(_) => 401,
            SecurityError::Authorization(_) => 403,
            SecurityError::InvalidInput(_) => 400,
            SecurityError::TokenExpired => 401,
            SecurityError::TokenInvalid => 401,
            SecurityError::UserNotFound => 404,
            SecurityError::InvalidCredentials => 401,
            SecurityError::InsufficientPermissions => 403,
            SecurityError::CsrfTokenInvalid => 403,
            SecurityError::StateMismatch => 400,
            SecurityError::RateLimitExceeded => 429,
            SecurityError::MfaRequired => 401,
            SecurityError::MfaSetupRequired => 401,
            SecurityError::AccountLocked => 423,
            SecurityError::AccountDisabled => 401,

            // 5xx Server Errors
            _ => 500,
        }
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> &'static str {
        match self {
            SecurityError::Authentication(_) => "Authentication failed. Please check your credentials.",
            SecurityError::Authorization(_) => "You don't have permission to access this resource.",
            SecurityError::InvalidInput(_) => "Invalid input provided.",
            SecurityError::TokenExpired => "Your session has expired. Please log in again.",
            SecurityError::TokenInvalid => "Invalid authentication token.",
            SecurityError::UserNotFound => "User not found.",
            SecurityError::InvalidCredentials => "Invalid username or password.",
            SecurityError::InsufficientPermissions => "Insufficient permissions for this action.",
            SecurityError::CsrfTokenInvalid => "Security token is invalid.",
            SecurityError::StateMismatch => "Request state mismatch. Please try again.",
            SecurityError::RateLimitExceeded => "Too many requests. Please try again later.",
            SecurityError::MfaRequired => "Multi-factor authentication is required.",
            SecurityError::MfaSetupRequired => "Multi-factor authentication setup is required.",
            SecurityError::AccountLocked => "Account is locked. Please contact support.",
            SecurityError::AccountDisabled => "Account is disabled. Please contact support.",
            _ => "An internal error occurred. Please try again later.",
        }
    }
}

/// Convert from anyhow::Error for compatibility
impl From<anyhow::Error> for SecurityError {
    fn from(err: anyhow::Error) -> Self {
        SecurityError::Configuration(err.to_string())
    }
}

/// Convert from chrono errors
impl From<chrono::ParseError> for SecurityError {
    fn from(err: chrono::ParseError) -> Self {
        SecurityError::Time(err.to_string())
    }
}

/// Convert from TOTP secret parse errors
impl From<SecretParseError> for SecurityError {
    fn from(err: SecretParseError) -> Self {
        SecurityError::Mfa(format!("TOTP secret parsing failed: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        assert!(SecurityError::InvalidCredentials.is_client_error());
        assert!(!SecurityError::InvalidCredentials.is_server_error());
        assert!(!SecurityError::InvalidCredentials.is_retryable());

        assert!(SecurityError::Jwt(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken
        )).is_server_error());
        assert!(!SecurityError::Jwt(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken
        )).is_client_error());

        assert!(SecurityError::Http(reqwest::Error::from(
            reqwest::ErrorKind::Request
        )).is_retryable());
    }

    #[test]
    fn test_http_status_codes() {
        assert_eq!(SecurityError::InvalidCredentials.http_status_code(), 401);
        assert_eq!(SecurityError::Authorization("test".to_string()).http_status_code(), 403);
        assert_eq!(SecurityError::InvalidInput("test".to_string()).http_status_code(), 400);
        assert_eq!(SecurityError::Configuration("test".to_string()).http_status_code(), 500);
    }

    #[test]
    fn test_user_messages() {
        assert_eq!(
            SecurityError::InvalidCredentials.user_message(),
            "Invalid username or password."
        );
        assert_eq!(
            SecurityError::TokenExpired.user_message(),
            "Your session has expired. Please log in again."
        );
    }
}
