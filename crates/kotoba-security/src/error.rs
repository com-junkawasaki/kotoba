//! Security error types and handling

use totp_rs::SecretParseError;

/// Result type for security operations
pub type Result<T> = std::result::Result<T, SecurityError>;

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            SecurityError::Authentication(msg) => write!(f, "Authentication failed: {}", msg),
            SecurityError::Authorization(msg) => write!(f, "Authorization failed: {}", msg),
            SecurityError::Jwt(e) => write!(f, "JWT error: {}", e),
            SecurityError::OAuth2(msg) => write!(f, "OAuth2 error: {}", msg),
            SecurityError::Mfa(msg) => write!(f, "MFA error: {}", msg),
            SecurityError::Password(msg) => write!(f, "Password hashing error: {}", msg),
            SecurityError::Session(msg) => write!(f, "Session error: {}", msg),
            SecurityError::Http(e) => write!(f, "HTTP client error: {}", e),
            SecurityError::Json(e) => write!(f, "JSON parsing error: {}", e),
            SecurityError::Url(e) => write!(f, "URL parsing error: {}", e),
            SecurityError::Io(e) => write!(f, "IO error: {}", e),
            SecurityError::Utf8(e) => write!(f, "UTF-8 error: {}", e),
            SecurityError::Time(msg) => write!(f, "Time error: {}", msg),
            SecurityError::Crypto(msg) => write!(f, "Cryptography error: {}", msg),
            SecurityError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            SecurityError::TokenExpired => write!(f, "Token expired"),
            SecurityError::TokenInvalid => write!(f, "Token invalid"),
            SecurityError::UserNotFound => write!(f, "User not found"),
            SecurityError::UserExists => write!(f, "User already exists"),
            SecurityError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            SecurityError::MfaRequired => write!(f, "MFA required"),
            SecurityError::MfaSetupRequired => write!(f, "MFA setup required"),
            SecurityError::AccountLocked => write!(f, "Account locked"),
            SecurityError::AccountDisabled => write!(f, "Account disabled"),
            SecurityError::InvalidCredentials => write!(f, "Invalid credentials"),
            SecurityError::InsufficientPermissions => write!(f, "Insufficient permissions"),
            SecurityError::ProviderNotSupported => write!(f, "Provider not supported"),
            SecurityError::StateMismatch => write!(f, "State mismatch"),
            SecurityError::CsrfTokenInvalid => write!(f, "CSRF token invalid"),
            SecurityError::SessionExpired => write!(f, "Session expired"),
            SecurityError::SessionInvalid => write!(f, "Session invalid"),
            SecurityError::Database(msg) => write!(f, "Database error: {}", msg),
            SecurityError::Cache(msg) => write!(f, "Cache error: {}", msg),
            SecurityError::ExternalService(msg) => write!(f, "External service error: {}", msg),
        }
    }
}

impl std::error::Error for SecurityError {}

/// Security error types
#[derive(Debug)]
pub enum SecurityError {
    Configuration(String),
    Authentication(String),
    Authorization(String),
    Jwt(jsonwebtoken::errors::Error),
    OAuth2(String),
    Mfa(String),
    Password(String),
    Session(String),
    Http(reqwest::Error),
    Json(serde_json::Error),
    Url(url::ParseError),
    Io(std::io::Error),
    Utf8(std::string::FromUtf8Error),
    Time(String),
    Crypto(String),
    InvalidInput(String),
    TokenExpired,
    TokenInvalid,
    UserNotFound,
    UserExists,
    RateLimitExceeded,
    MfaRequired,
    MfaSetupRequired,
    AccountLocked,
    AccountDisabled,
    InvalidCredentials,
    InsufficientPermissions,
    ProviderNotSupported,
    StateMismatch,
    CsrfTokenInvalid,
    SessionExpired,
    SessionInvalid,
    Database(String),
    Cache(String),
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

        // Test retryable error - skip for now due to reqwest API complexity
        // TODO: Add proper reqwest error testing when API stabilizes
        // assert!(SecurityError::Http(some_retryable_error).is_retryable());
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
