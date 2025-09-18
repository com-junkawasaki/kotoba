//! Authentication Handler Module
//!
//! このモジュールは認証・認可機能を提供します。
//! JWTトークン、パスワードハッシュ、セッション管理などを含みます。

use crate::{HandlerError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 認証設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiration_hours: u64,
    pub bcrypt_cost: u32,
    pub session_timeout_minutes: u64,
    pub allowed_origins: Vec<String>,
    pub enable_cors: bool,
}

/// ユーザークレデンシャル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCredentials {
    pub username: String,
    pub password: String,
}

/// JWTクレーム
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,        // Subject (user ID)
    pub exp: u64,          // Expiration time
    pub iat: u64,          // Issued at
    pub iss: String,       // Issuer
    pub aud: String,       // Audience
    pub role: Option<String>, // User role
    pub permissions: Vec<String>, // User permissions
}

/// ユーザーロール
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    User,
    Guest,
    Custom(String),
}

/// 認証ミドルウェア
pub struct AuthMiddleware {
    config: AuthConfig,
}

impl AuthMiddleware {
    /// 新しい認証ミドルウェアを作成
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    /// JWTトークンを生成
    #[cfg(feature = "jsonwebtoken")]
    pub fn generate_jwt(&self, user_id: &str, role: Option<&str>) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| HandlerError::Jsonnet(format!("Time error: {}", e)))?
            .as_secs();

        let claims = JwtClaims {
            sub: user_id.to_string(),
            exp: now + (self.config.jwt_expiration_hours * 3600),
            iat: now,
            iss: "kotoba-auth".to_string(),
            aud: "kotoba-app".to_string(),
            role: role.map(|s| s.to_string()),
            permissions: vec![], // TODO: 実装する
        };

        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(|e| HandlerError::Jsonnet(format!("JWT encoding error: {}", e)))
    }

    /// JWTトークンを検証
    #[cfg(feature = "jsonwebtoken")]
    pub fn verify_jwt(&self, token: &str) -> Result<JwtClaims> {
        let validation = jsonwebtoken::Validation::default();

        let token_data = jsonwebtoken::decode::<JwtClaims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| HandlerError::Jsonnet(format!("JWT decoding error: {}", e)))?;

        // トークンの有効期限をチェック
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| HandlerError::Jsonnet(format!("Time error: {}", e)))?
            .as_secs();

        if token_data.claims.exp < now {
            return Err(HandlerError::Jsonnet("JWT token has expired".to_string()));
        }

        Ok(token_data.claims)
    }

    /// パスワードをハッシュ化
    #[cfg(feature = "bcrypt")]
    pub fn hash_password(&self, password: &str) -> Result<String> {
        bcrypt::hash(password, self.config.bcrypt_cost)
            .map_err(|e| HandlerError::Jsonnet(format!("Password hashing error: {}", e)))
    }

    /// パスワードを検証
    #[cfg(feature = "bcrypt")]
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        bcrypt::verify(password, hash)
            .map_err(|e| HandlerError::Jsonnet(format!("Password verification error: {}", e)))
    }

    /// CORSヘッダーを追加
    pub fn add_cors_headers(&self, response_headers: &mut HashMap<String, String>, origin: Option<&str>) {
        if !self.config.enable_cors {
            return;
        }

        response_headers.insert("Access-Control-Allow-Origin".to_string(),
            origin.unwrap_or("*").to_string());
        response_headers.insert("Access-Control-Allow-Methods".to_string(),
            "GET, POST, PUT, DELETE, OPTIONS".to_string());
        response_headers.insert("Access-Control-Allow-Headers".to_string(),
            "Content-Type, Authorization, X-Requested-With".to_string());
        response_headers.insert("Access-Control-Max-Age".to_string(), "86400".to_string());
    }

    /// Originが許可されているかチェック
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        if self.config.allowed_origins.is_empty() {
            return true; // すべてのオリジンを許可
        }

        self.config.allowed_origins.iter().any(|allowed| allowed == origin)
    }

    /// セッションを検証
    pub fn validate_session(&self, session_id: &str, user_id: &str) -> Result<bool> {
        // TODO: 実際のセッションストアとの統合
        // 現在はプレースホルダー

        // セッションIDの形式チェック
        if session_id.len() < 32 {
            return Ok(false);
        }

        // ユーザーIDの形式チェック
        if user_id.is_empty() {
            return Ok(false);
        }

        // TODO: セッションストアから有効性をチェック
        Ok(true)
    }

    /// 権限をチェック
    pub fn check_permission(&self, user_role: &UserRole, required_permission: &str) -> bool {
        match user_role {
            UserRole::Admin => true, // Adminはすべての権限を持つ
            UserRole::User => {
                // 一般ユーザーの権限
                matches!(required_permission,
                    "read_profile" | "update_profile" | "read_posts" | "create_posts")
            }
            UserRole::Guest => {
                // ゲストユーザーの権限
                matches!(required_permission, "read_posts" | "read_public")
            }
            UserRole::Custom(role) => {
                // カスタムロールの権限チェック
                match role.as_str() {
                    "editor" => matches!(required_permission,
                        "read_posts" | "create_posts" | "update_posts" | "delete_posts"),
                    "moderator" => matches!(required_permission,
                        "read_posts" | "update_posts" | "moderate_comments"),
                    _ => false,
                }
            }
        }
    }

    /// 設定を取得
    pub fn config(&self) -> &AuthConfig {
        &self.config
    }
}

/// セッションマネージャー
pub struct SessionManager {
    sessions: HashMap<String, SessionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub user_id: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub data: HashMap<String, serde_json::Value>,
}

impl SessionManager {
    /// 新しいセッションマネージャーを作成
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// 新しいセッションを作成
    pub fn create_session(&mut self, user_id: &str, timeout_minutes: u64) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| HandlerError::Jsonnet(format!("Time error: {}", e)))?
            .as_secs();

        let session_data = SessionData {
            user_id: user_id.to_string(),
            created_at: now,
            expires_at: now + (timeout_minutes * 60),
            data: HashMap::new(),
        };

        self.sessions.insert(session_id.clone(), session_data);
        Ok(session_id)
    }

    /// セッションを取得
    pub fn get_session(&self, session_id: &str) -> Option<&SessionData> {
        self.sessions.get(session_id)
    }

    /// セッションを検証
    pub fn validate_session(&self, session_id: &str) -> Result<bool> {
        if let Some(session) = self.sessions.get(session_id) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| HandlerError::Jsonnet(format!("Time error: {}", e)))?
                .as_secs();

            Ok(session.expires_at > now)
        } else {
            Ok(false)
        }
    }

    /// セッションを更新
    pub fn update_session(&mut self, session_id: &str, data: HashMap<String, serde_json::Value>) -> Result<()> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.data = data;
            Ok(())
        } else {
            Err(HandlerError::Jsonnet("Session not found".to_string()))
        }
    }

    /// セッションを削除
    pub fn delete_session(&mut self, session_id: &str) -> Result<()> {
        if self.sessions.remove(session_id).is_some() {
            Ok(())
        } else {
            Err(HandlerError::Jsonnet("Session not found".to_string()))
        }
    }

    /// 期限切れのセッションをクリーンアップ
    pub fn cleanup_expired_sessions(&mut self) -> Result<usize> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| HandlerError::Jsonnet(format!("Time error: {}", e)))?
            .as_secs();

        let expired_count = self.sessions.len();
        self.sessions.retain(|_, session| session.expires_at > now);

        Ok(expired_count - self.sessions.len())
    }

    /// アクティブなセッション数を取得
    pub fn active_sessions_count(&self) -> usize {
        self.sessions.len()
    }
}

/// パスワードユーティリティ
pub struct PasswordUtils;

impl PasswordUtils {
    /// パスワード強度をチェック
    pub fn check_password_strength(password: &str) -> Result<(bool, Vec<String>)> {
        let mut issues = Vec::new();
        let mut is_strong = true;

        if password.len() < 8 {
            issues.push("Password must be at least 8 characters long".to_string());
            is_strong = false;
        }

        if !password.chars().any(|c| c.is_uppercase()) {
            issues.push("Password must contain at least one uppercase letter".to_string());
            is_strong = false;
        }

        if !password.chars().any(|c| c.is_lowercase()) {
            issues.push("Password must contain at least one lowercase letter".to_string());
            is_strong = false;
        }

        if !password.chars().any(|c| c.is_numeric()) {
            issues.push("Password must contain at least one number".to_string());
            is_strong = false;
        }

        if !password.chars().any(|c| !c.is_alphanumeric()) {
            issues.push("Password must contain at least one special character".to_string());
            is_strong = false;
        }

        Ok((is_strong, issues))
    }

    /// 安全なパスワードを生成
    pub fn generate_secure_password(length: usize) -> String {
        use rand::Rng;

        let charset = "ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                      abcdefghijklmnopqrstuvwxyz\
                      0123456789\
                      !@#$%^&*()_+-=[]{}|;:,.<>?";

        let mut rng = rand::thread_rng();
        let password: String = (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..charset.len());
                charset.chars().nth(idx).unwrap()
            })
            .collect();

        password
    }
}

/// レートリミッター
pub struct RateLimiter {
    requests: HashMap<String, Vec<u64>>,
    max_requests: u32,
    window_seconds: u64,
}

impl RateLimiter {
    /// 新しいレートリミッターを作成
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            requests: HashMap::new(),
            max_requests,
            window_seconds,
        }
    }

    /// リクエストをチェック
    pub fn check_rate_limit(&mut self, identifier: &str) -> Result<bool> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| HandlerError::Jsonnet(format!("Time error: {}", e)))?
            .as_secs();

        let window_start = now.saturating_sub(self.window_seconds);

        // 古いリクエストをクリーンアップ
        if let Some(requests) = self.requests.get_mut(identifier) {
            requests.retain(|&timestamp| timestamp > window_start);
        }

        // リクエスト数をチェック
        let current_requests = self.requests
            .get(identifier)
            .map(|requests| requests.len())
            .unwrap_or(0);

        if current_requests >= self.max_requests as usize {
            return Ok(false); // レート制限超過
        }

        // 新しいリクエストを追加
        self.requests
            .entry(identifier.to_string())
            .or_insert_with(Vec::new)
            .push(now);

        Ok(true)
    }

    /// 残りのリクエスト数を取得
    pub fn remaining_requests(&self, identifier: &str) -> u32 {
        let current_requests = self.requests
            .get(identifier)
            .map(|requests| requests.len())
            .unwrap_or(0);

        self.max_requests.saturating_sub(current_requests as u32)
    }

    /// ウィンドウのリセット時間を取得
    pub fn reset_time(&self, identifier: &str) -> Option<u64> {
        self.requests.get(identifier)
            .and_then(|requests| requests.first())
            .map(|first_request| first_request + self.window_seconds)
    }
}

/// OAuth 2.0 ヘルパー
#[cfg(feature = "jsonwebtoken")]
pub struct OAuthHelper {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

#[cfg(feature = "jsonwebtoken")]
impl OAuthHelper {
    /// 新しいOAuthヘルパーを作成
    pub fn new(client_id: &str, client_secret: &str, redirect_uri: &str) -> Self {
        Self {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
        }
    }

    /// Google OAuth URLを生成
    pub fn generate_google_oauth_url(&self, state: &str) -> String {
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?\
             response_type=code&\
             client_id={}&\
             redirect_uri={}&\
             scope=openid%20email%20profile&\
             state={}",
            self.client_id,
            urlencoding::encode(&self.redirect_uri),
            urlencoding::encode(state)
        )
    }

    /// GitHub OAuth URLを生成
    pub fn generate_github_oauth_url(&self, state: &str) -> String {
        format!(
            "https://github.com/login/oauth/authorize?\
             client_id={}&\
             redirect_uri={}&\
             scope=user:email&\
             state={}",
            self.client_id,
            urlencoding::encode(&self.redirect_uri),
            urlencoding::encode(state)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_strength_check() {
        // 弱いパスワード
        let (is_strong, issues) = PasswordUtils::check_password_strength("weak").unwrap();
        assert!(!is_strong);
        assert!(issues.len() > 0);

        // 強いパスワード
        let (is_strong, issues) = PasswordUtils::check_password_strength("StrongPass123!").unwrap();
        assert!(is_strong);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_generate_secure_password() {
        let password = PasswordUtils::generate_secure_password(12);
        assert_eq!(password.len(), 12);

        // 生成されたパスワードが様々な文字を含むことを確認
        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_numeric());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        assert!(has_upper || has_lower || has_digit || has_special);
    }

    #[test]
    fn test_user_role_permissions() {
        let auth = AuthMiddleware::new(AuthConfig {
            jwt_secret: "test".to_string(),
            jwt_expiration_hours: 24,
            bcrypt_cost: 4,
            session_timeout_minutes: 60,
            allowed_origins: vec![],
            enable_cors: false,
        });

        // Admin permissions
        assert!(auth.check_permission(&UserRole::Admin, "delete_users"));
        assert!(auth.check_permission(&UserRole::Admin, "read_system_logs"));

        // User permissions
        assert!(auth.check_permission(&UserRole::User, "read_profile"));
        assert!(auth.check_permission(&UserRole::User, "create_posts"));
        assert!(!auth.check_permission(&UserRole::User, "delete_users"));

        // Guest permissions
        assert!(auth.check_permission(&UserRole::Guest, "read_posts"));
        assert!(auth.check_permission(&UserRole::Guest, "read_public"));
        assert!(!auth.check_permission(&UserRole::Guest, "create_posts"));

        // Custom role permissions
        assert!(auth.check_permission(&UserRole::Custom("editor".to_string()), "update_posts"));
        assert!(!auth.check_permission(&UserRole::Custom("editor".to_string()), "delete_users"));
    }

    #[test]
    fn test_session_manager() {
        let mut session_manager = SessionManager::new();

        // セッションを作成
        let session_id = session_manager.create_session("user123", 60).unwrap();
        assert!(!session_id.is_empty());

        // セッションを取得
        let session = session_manager.get_session(&session_id).unwrap();
        assert_eq!(session.user_id, "user123");

        // セッションを検証
        assert!(session_manager.validate_session(&session_id).unwrap());

        // セッションを削除
        session_manager.delete_session(&session_id).unwrap();
        assert!(!session_manager.validate_session(&session_id).unwrap());
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(3, 60); // 1分間に3リクエスト

        let identifier = "user123";

        // 最初の3リクエストは成功
        assert!(limiter.check_rate_limit(identifier).unwrap());
        assert!(limiter.check_rate_limit(identifier).unwrap());
        assert!(limiter.check_rate_limit(identifier).unwrap());

        // 4番目のリクエストは失敗
        assert!(!limiter.check_rate_limit(identifier).unwrap());

        // 残りリクエスト数をチェック
        assert_eq!(limiter.remaining_requests(identifier), 0);
    }

    #[test]
    fn test_auth_config_creation() {
        let config = AuthConfig {
            jwt_secret: "my-secret-key".to_string(),
            jwt_expiration_hours: 24,
            bcrypt_cost: 12,
            session_timeout_minutes: 60,
            allowed_origins: vec!["https://example.com".to_string()],
            enable_cors: true,
        };

        assert_eq!(config.jwt_expiration_hours, 24);
        assert_eq!(config.bcrypt_cost, 12);
        assert!(config.enable_cors);
    }
}
