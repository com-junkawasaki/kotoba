//! Session management for stateless authentication

use crate::error::{SecurityError, Result};
use crate::config::{SessionConfig, SessionStoreType, SameSitePolicy};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Session data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub attributes: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed_at: chrono::DateTime<chrono::Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl SessionData {
    /// Create new session data
    pub fn new(
        session_id: String,
        user_id: String,
        roles: Vec<String>,
        permissions: Vec<String>,
        max_age_seconds: Option<u64>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        let expires_at = max_age_seconds
            .map(|secs| now + chrono::Duration::seconds(secs as i64))
            .unwrap_or_else(|| now + chrono::Duration::hours(24)); // Default 24 hours

        Self {
            session_id,
            user_id,
            roles,
            permissions,
            attributes: HashMap::new(),
            created_at: now,
            expires_at,
            last_accessed_at: now,
            ip_address,
            user_agent,
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }

    /// Update last accessed time
    pub fn touch(&mut self) {
        self.last_accessed_at = chrono::Utc::now();
    }

    /// Extend session expiration
    pub fn extend(&mut self, additional_seconds: i64) {
        self.expires_at = chrono::Utc::now() + chrono::Duration::seconds(additional_seconds);
    }

    /// Get remaining time until expiration in seconds
    pub fn time_until_expiry(&self) -> i64 {
        let now = chrono::Utc::now();
        (self.expires_at - now).num_seconds()
    }

    /// Check if user has specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    /// Check if user has specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    /// Add custom attribute
    pub fn set_attribute(&mut self, key: String, value: serde_json::Value) {
        self.attributes.insert(key, value);
    }

    /// Get custom attribute
    pub fn get_attribute(&self, key: &str) -> Option<&serde_json::Value> {
        self.attributes.get(key)
    }

    /// Remove custom attribute
    pub fn remove_attribute(&mut self, key: &str) -> Option<serde_json::Value> {
        self.attributes.remove(key)
    }
}

/// Session store trait for different storage backends
#[async_trait::async_trait]
pub trait SessionStore: Send + Sync {
    /// Store session data
    async fn store(&self, session: SessionData) -> Result<()>;

    /// Retrieve session data by ID
    async fn get(&self, session_id: &str) -> Result<Option<SessionData>>;

    /// Update session data
    async fn update(&self, session: SessionData) -> Result<()>;

    /// Delete session by ID
    async fn delete(&self, session_id: &str) -> Result<()>;

    /// Delete all sessions for a user
    async fn delete_user_sessions(&self, user_id: &str) -> Result<usize>;

    /// Clean up expired sessions
    async fn cleanup_expired(&self) -> Result<usize>;

    /// Get session count
    async fn count(&self) -> Result<usize>;
}

/// In-memory session store
pub struct MemorySessionStore {
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,
}

impl MemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl SessionStore for MemorySessionStore {
    async fn store(&self, session: SessionData) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.session_id.clone(), session);
        Ok(())
    }

    async fn get(&self, session_id: &str) -> Result<Option<SessionData>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(session_id).cloned())
    }

    async fn update(&self, session: SessionData) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.session_id.clone(), session);
        Ok(())
    }

    async fn delete(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        Ok(())
    }

    async fn delete_user_sessions(&self, user_id: &str) -> Result<usize> {
        let mut sessions = self.sessions.write().await;
        let keys_to_remove: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.user_id == user_id)
            .map(|(key, _)| key.clone())
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            sessions.remove(&key);
        }

        Ok(count)
    }

    async fn cleanup_expired(&self) -> Result<usize> {
        let mut sessions = self.sessions.write().await;
        let expired_keys: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.is_expired())
            .map(|(key, _)| key.clone())
            .collect();

        let count = expired_keys.len();
        for key in expired_keys {
            sessions.remove(&key);
        }

        Ok(count)
    }

    async fn count(&self) -> Result<usize> {
        let sessions = self.sessions.read().await;
        Ok(sessions.len())
    }
}

/// Session manager for handling session lifecycle
pub struct SessionManager {
    config: SessionConfig,
    store: Box<dyn SessionStore>,
}

impl SessionManager {
    /// Create new session manager
    pub fn new(config: SessionConfig) -> Self {
        let store: Box<dyn SessionStore> = match config.store_type {
            SessionStoreType::Memory => Box::new(MemorySessionStore::new()),
            SessionStoreType::Redis => {
                // TODO: Implement Redis session store
                Box::new(MemorySessionStore::new())
            }
            SessionStoreType::Database => {
                // TODO: Implement database session store
                Box::new(MemorySessionStore::new())
            }
        };

        Self { config, store }
    }

    /// Create new session
    pub async fn create_session(
        &self,
        user_id: &str,
        roles: Vec<String>,
        permissions: Vec<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<SessionData> {
        let session_id = self.generate_session_id();
        let session = SessionData::new(
            session_id,
            user_id.to_string(),
            roles,
            permissions,
            self.config.max_age_seconds,
            ip_address,
            user_agent,
        );

        self.store.store(session.clone()).await?;
        Ok(session)
    }

    /// Get session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionData>> {
        let mut session = self.store.get(session_id).await?;

        if let Some(ref mut session) = session {
            // Check if session is expired
            if session.is_expired() {
                // Clean up expired session
                self.store.delete(session_id).await?;
                return Ok(None);
            }

            // Update last accessed time
            session.touch();
            self.store.update(session.clone()).await?;
        }

        Ok(session)
    }

    /// Update session
    pub async fn update_session(&self, session: SessionData) -> Result<()> {
        if session.is_expired() {
            return Err(SecurityError::SessionExpired);
        }

        self.store.update(session).await
    }

    /// Delete session
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        self.store.delete(session_id).await
    }

    /// Delete all sessions for a user
    pub async fn delete_user_sessions(&self, user_id: &str) -> Result<usize> {
        self.store.delete_user_sessions(user_id).await
    }

    /// Extend session expiration
    pub async fn extend_session(&self, session_id: &str, additional_seconds: i64) -> Result<()> {
        let mut session = self.store.get(session_id).await?
            .ok_or_else(|| SecurityError::SessionInvalid)?;

        if session.is_expired() {
            return Err(SecurityError::SessionExpired);
        }

        session.extend(additional_seconds);
        self.store.update(session).await
    }

    /// Validate session and return user information
    pub async fn validate_session(&self, session_id: &str) -> Result<Option<SessionData>> {
        self.get_session(session_id).await
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<usize> {
        self.store.cleanup_expired().await
    }

    /// Get session statistics
    pub async fn get_stats(&self) -> Result<SessionStats> {
        let count = self.store.count().await?;
        Ok(SessionStats { total_sessions: count })
    }

    /// Generate unique session ID
    fn generate_session_id(&self) -> String {
        use uuid::Uuid;
        Uuid::new_v4().to_string()
    }
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_sessions: usize,
}

/// Cookie configuration for session management
#[derive(Debug, Clone)]
pub struct CookieConfig {
    pub name: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSitePolicy,
    pub domain: Option<String>,
    pub path: Option<String>,
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            name: "session_id".to_string(),
            secure: true,
            http_only: true,
            same_site: SameSitePolicy::Lax,
            domain: None,
            path: Some("/".to_string()),
        }
    }
}

impl From<&SessionConfig> for CookieConfig {
    fn from(config: &SessionConfig) -> Self {
        Self {
            name: config.cookie_name.clone(),
            secure: config.cookie_secure,
            http_only: config.cookie_http_only,
            same_site: config.cookie_same_site.clone(),
            domain: None,
            path: Some("/".to_string()),
        }
    }
}

/// Session cookie utilities
pub struct SessionCookie;

impl SessionCookie {
    /// Generate session cookie header value
    pub fn generate_cookie_header(session_id: &str, config: &CookieConfig, max_age: Option<u64>) -> String {
        let mut cookie = format!("{}={}", config.name, session_id);

        if config.http_only {
            cookie.push_str("; HttpOnly");
        }

        if config.secure {
            cookie.push_str("; Secure");
        }

        match config.same_site {
            SameSitePolicy::Strict => cookie.push_str("; SameSite=Strict"),
            SameSitePolicy::Lax => cookie.push_str("; SameSite=Lax"),
            SameSitePolicy::None => cookie.push_str("; SameSite=None"),
        }

        if let Some(domain) = &config.domain {
            cookie.push_str(&format!("; Domain={}", domain));
        }

        if let Some(path) = &config.path {
            cookie.push_str(&format!("; Path={}", path));
        }

        if let Some(max_age) = max_age {
            cookie.push_str(&format!("; Max-Age={}", max_age));
        }

        cookie
    }

    /// Parse session ID from cookie header
    pub fn parse_session_id(cookie_header: &str, cookie_name: &str) -> Option<String> {
        for cookie in cookie_header.split(';') {
            let cookie = cookie.trim();
            if let Some(value) = cookie.strip_prefix(&format!("{}=", cookie_name)) {
                return Some(value.to_string());
            }
        }
        None
    }

    /// Generate delete cookie header
    pub fn generate_delete_cookie_header(config: &CookieConfig) -> String {
        let mut cookie = format!("{}=; Max-Age=0", config.name);

        if config.http_only {
            cookie.push_str("; HttpOnly");
        }

        if config.secure {
            cookie.push_str("; Secure");
        }

        match config.same_site {
            SameSitePolicy::Strict => cookie.push_str("; SameSite=Strict"),
            SameSitePolicy::Lax => cookie.push_str("; SameSite=Lax"),
            SameSitePolicy::None => cookie.push_str("; SameSite=None"),
        }

        if let Some(domain) = &config.domain {
            cookie.push_str(&format!("; Domain={}", domain));
        }

        if let Some(path) = &config.path {
            cookie.push_str(&format!("; Path={}", path));
        }

        cookie
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    async fn create_test_manager() -> SessionManager {
        let config = SessionConfig::default();
        SessionManager::new(config)
    }

    #[tokio::test]
    async fn test_session_creation() {
        let manager = create_test_manager().await;

        let session = manager.create_session(
            "user123",
            vec!["admin".to_string()],
            vec!["read".to_string(), "write".to_string()],
            Some("127.0.0.1".to_string()),
            Some("Test Browser".to_string()),
        ).await.unwrap();

        assert_eq!(session.user_id, "user123");
        assert!(session.has_role("admin"));
        assert!(session.has_permission("read"));
        assert!(!session.is_expired());
        assert!(session.time_until_expiry() > 0);
    }

    #[tokio::test]
    async fn test_session_retrieval() {
        let manager = create_test_manager().await;

        let session = manager.create_session(
            "user123",
            vec!["user".to_string()],
            vec![],
            None,
            None,
        ).await.unwrap();

        let retrieved = manager.get_session(&session.session_id).await.unwrap().unwrap();
        assert_eq!(retrieved.user_id, "user123");
        assert_eq!(retrieved.session_id, session.session_id);
    }

    #[tokio::test]
    async fn test_session_update() {
        let manager = create_test_manager().await;

        let mut session = manager.create_session(
            "user123",
            vec!["user".to_string()],
            vec![],
            None,
            None,
        ).await.unwrap();

        session.set_attribute("theme".to_string(), serde_json::Value::String("dark".to_string()));
        manager.update_session(session.clone()).await.unwrap();

        let updated = manager.get_session(&session.session_id).await.unwrap().unwrap();
        assert_eq!(updated.get_attribute("theme").unwrap().as_str().unwrap(), "dark");
    }

    #[tokio::test]
    async fn test_session_deletion() {
        let manager = create_test_manager().await;

        let session = manager.create_session(
            "user123",
            vec!["user".to_string()],
            vec![],
            None,
            None,
        ).await.unwrap();

        // Verify session exists
        let retrieved = manager.get_session(&session.session_id).await.unwrap();
        assert!(retrieved.is_some());

        // Delete session
        manager.delete_session(&session.session_id).await.unwrap();

        // Verify session is deleted
        let retrieved = manager.get_session(&session.session_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_user_session_deletion() {
        let manager = create_test_manager().await;

        // Create multiple sessions for the same user
        let session1 = manager.create_session(
            "user123",
            vec!["user".to_string()],
            vec![],
            None,
            None,
        ).await.unwrap();

        let session2 = manager.create_session(
            "user123",
            vec!["user".to_string()],
            vec![],
            None,
            None,
        ).await.unwrap();

        // Delete all sessions for user
        let deleted_count = manager.delete_user_sessions("user123").await.unwrap();
        assert_eq!(deleted_count, 2);

        // Verify sessions are deleted
        let retrieved1 = manager.get_session(&session1.session_id).await.unwrap();
        let retrieved2 = manager.get_session(&session2.session_id).await.unwrap();
        assert!(retrieved1.is_none());
        assert!(retrieved2.is_none());
    }

    #[tokio::test]
    async fn test_session_extension() {
        let manager = create_test_manager().await;

        let session = manager.create_session(
            "user123",
            vec!["user".to_string()],
            vec![],
            None,
            None,
        ).await.unwrap();

        let original_expiry = session.time_until_expiry();

        // Extend session by 1 hour
        manager.extend_session(&session.session_id, 3600).await.unwrap();

        let updated = manager.get_session(&session.session_id).await.unwrap().unwrap();
        let new_expiry = updated.time_until_expiry();

        // New expiry should be longer than original
        assert!(new_expiry > original_expiry);
    }

    #[tokio::test]
    fn test_cookie_header_generation() {
        let config = CookieConfig::default();
        let session_id = "session123";

        let cookie_header = SessionCookie::generate_cookie_header(session_id, &config, Some(3600));
        assert!(cookie_header.contains("session_id=session123"));
        assert!(cookie_header.contains("HttpOnly"));
        assert!(cookie_header.contains("Secure"));
        assert!(cookie_header.contains("SameSite=Lax"));
        assert!(cookie_header.contains("Max-Age=3600"));
    }

    #[tokio::test]
    fn test_cookie_parsing() {
        let cookie_header = "session_id=abc123; other=value; session_id=def456";
        let session_id = SessionCookie::parse_session_id(cookie_header, "session_id");

        assert_eq!(session_id, Some("abc123".to_string()));
    }

    #[tokio::test]
    fn test_delete_cookie_generation() {
        let config = CookieConfig::default();
        let delete_header = SessionCookie::generate_delete_cookie_header(&config);

        assert!(delete_header.contains("session_id="));
        assert!(delete_header.contains("Max-Age=0"));
        assert!(delete_header.contains("HttpOnly"));
        assert!(delete_header.contains("Secure"));
    }

    #[tokio::test]
    async fn test_session_attributes() {
        let session = SessionData::new(
            "session123".to_string(),
            "user123".to_string(),
            vec!["user".to_string()],
            vec!["read".to_string()],
            Some(3600),
            None,
            None,
        );

        // Test has_role and has_permission
        assert!(session.has_role("user"));
        assert!(!session.has_role("admin"));
        assert!(session.has_permission("read"));
        assert!(!session.has_permission("write"));

        // Test custom attributes
        let mut session = session;
        session.set_attribute("theme".to_string(), serde_json::Value::String("dark".to_string()));
        session.set_attribute("locale".to_string(), serde_json::Value::String("en".to_string()));

        assert_eq!(session.get_attribute("theme").unwrap().as_str().unwrap(), "dark");
        assert_eq!(session.get_attribute("locale").unwrap().as_str().unwrap(), "en");

        // Test attribute removal
        let removed = session.remove_attribute("theme");
        assert_eq!(removed.unwrap().as_str().unwrap(), "dark");
        assert!(session.get_attribute("theme").is_none());
    }
}
