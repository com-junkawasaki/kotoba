//! OAuth2 and OpenID Connect integration

use crate::error::{SecurityError, Result};
use crate::config::OAuth2Config;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenUrl, TokenResponse as OAuth2TokenResponse,
};
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreGenderClaim, CoreIdTokenClaims, CoreIdTokenVerifier,
    CoreProviderMetadata, CoreResponseType,
};
use openidconnect::{
    TokenResponse,AdditionalClaims, IdToken, IssuerUrl, Nonce, UserInfoClaims};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

/// OAuth2 provider types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OAuth2Provider {
    Google,
    GitHub,
    Microsoft,
    Facebook,
    Twitter,
    Custom(String),
}

impl OAuth2Provider {
    pub fn as_str(&self) -> &str {
        match self {
            OAuth2Provider::Google => "google",
            OAuth2Provider::GitHub => "github",
            OAuth2Provider::Microsoft => "microsoft",
            OAuth2Provider::Facebook => "facebook",
            OAuth2Provider::Twitter => "twitter",
            OAuth2Provider::Custom(name) => name,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "google" => OAuth2Provider::Google,
            "github" => OAuth2Provider::GitHub,
            "microsoft" => OAuth2Provider::Microsoft,
            "facebook" => OAuth2Provider::Facebook,
            "twitter" => OAuth2Provider::Twitter,
            _ => OAuth2Provider::Custom(s.to_string()),
        }
    }

    /// Get default configuration for provider
    pub fn default_config(&self) -> OAuth2ProviderConfig {
        match self {
            OAuth2Provider::Google => OAuth2ProviderConfig {
                client_id: String::new(),
                client_secret: String::new(),
                authorization_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                userinfo_url: Some("https://openidconnect.googleapis.com/v1/userinfo".to_string()),
                scope_separator: " ".to_string(),
                additional_params: HashMap::new(),
            },
            OAuth2Provider::GitHub => OAuth2ProviderConfig {
                client_id: String::new(),
                client_secret: String::new(),
                authorization_url: "https://github.com/login/oauth/authorize".to_string(),
                token_url: "https://github.com/login/oauth/access_token".to_string(),
                userinfo_url: Some("https://api.github.com/user".to_string()),
                scope_separator: " ".to_string(),
                additional_params: HashMap::new(),
            },
            OAuth2Provider::Microsoft => OAuth2ProviderConfig {
                client_id: String::new(),
                client_secret: String::new(),
                authorization_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string(),
                token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
                userinfo_url: Some("https://graph.microsoft.com/oidc/userinfo".to_string()),
                scope_separator: " ".to_string(),
                additional_params: HashMap::new(),
            },
            OAuth2Provider::Facebook => OAuth2ProviderConfig {
                client_id: String::new(),
                client_secret: String::new(),
                authorization_url: "https://www.facebook.com/v12.0/dialog/oauth".to_string(),
                token_url: "https://graph.facebook.com/v12.0/oauth/access_token".to_string(),
                userinfo_url: Some("https://graph.facebook.com/me".to_string()),
                scope_separator: ",".to_string(),
                additional_params: HashMap::new(),
            },
            OAuth2Provider::Twitter => OAuth2ProviderConfig {
                client_id: String::new(),
                client_secret: String::new(),
                authorization_url: "https://twitter.com/i/oauth2/authorize".to_string(),
                token_url: "https://api.twitter.com/2/oauth2/token".to_string(),
                userinfo_url: Some("https://api.twitter.com/2/users/me".to_string()),
                scope_separator: " ".to_string(),
                additional_params: HashMap::new(),
            },
            OAuth2Provider::Custom(_) => OAuth2ProviderConfig {
                client_id: String::new(),
                client_secret: String::new(),
                authorization_url: String::new(),
                token_url: String::new(),
                userinfo_url: None,
                scope_separator: " ".to_string(),
                additional_params: HashMap::new(),
            },
        }
    }
}

/// OAuth2 provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2ProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: String,
    pub token_url: String,
    pub userinfo_url: Option<String>,
    pub scope_separator: String,
    pub additional_params: HashMap<String, String>,
}

/// OAuth2 tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Tokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub scope: Option<String>,
    pub id_token: Option<String>,
}

/// OAuth2 authorization state
#[derive(Debug)]
struct OAuth2State {
    provider: OAuth2Provider,
    csrf_token: CsrfToken,
    pkce_verifier: PkceCodeVerifier,
    nonce: Option<Nonce>,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// OAuth2 service for managing OAuth2 flows
pub struct OAuth2Service {
    config: OAuth2Config,
    clients: HashMap<String, BasicClient>,
    oidc_clients: HashMap<String, CoreClient>,
    states: Arc<RwLock<HashMap<String, OAuth2State>>>,
    http_client: HttpClient,
}

impl OAuth2Service {
    /// Create dummy service for initialization
    fn new_dummy() -> Self {
        Self {
            config: OAuth2Config::default(),
            clients: HashMap::new(),
            oidc_clients: HashMap::new(),
            states: Arc::new(RwLock::new(HashMap::new())),
            http_client: HttpClient::new(),
        }
    }

    /// Create new OAuth2 service
    pub async fn new(config: OAuth2Config) -> Result<Self> {
        let mut clients = HashMap::new();
        let mut oidc_clients = HashMap::new();

        for (name, provider_config) in &config.providers {
            let provider = OAuth2Provider::from_str(name);

            // Create OAuth2 client
            let client = Self::create_oauth2_client(&provider_config, &config.redirect_uri)?;
            clients.insert(name.clone(), client);

            // Try to create OpenID Connect client
            if let Ok(oidc_client) = Self::create_oidc_client(&OAuth2Service::new_dummy(), &provider_config, &config.redirect_uri).await {
                oidc_clients.insert(name.clone(), oidc_client);
            }
        }

        Ok(Self {
            config,
            clients,
            oidc_clients,
            states: Arc::new(RwLock::new(HashMap::new())),
            http_client: HttpClient::new(),
        })
    }

    /// Get authorization URL for OAuth2 flow
    pub async fn get_authorization_url(&self, provider: OAuth2Provider) -> Result<String> {
        let provider_name = provider.as_str();
        let client = self.clients.get(provider_name)
            .ok_or_else(|| SecurityError::Configuration(format!("OAuth2 provider '{}' not configured", provider_name)))?;

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let mut auth_request = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge);

        // Add scopes
        for scope in &self.config.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        // Add additional parameters if configured
        if let Some(provider_config) = self.config.providers.get(provider_name) {
            for (key, value) in &provider_config.additional_params {
                auth_request = auth_request.add_extra_param(key, value);
            }
        }

        let (auth_url, csrf_token) = auth_request.url();

        // Store state
        let state = OAuth2State {
            provider: provider.clone(),
            csrf_token: csrf_token.clone(),
            pkce_verifier,
            nonce: None, // For OAuth2 only
            created_at: chrono::Utc::now(),
        };

        let state_key = csrf_token.secret().clone();
        self.states.write().await.insert(state_key, state);

        Ok(auth_url.to_string())
    }

    /// Get authorization URL for OpenID Connect flow
    pub fn get_oidc_authorization_url(&self, provider: OAuth2Provider) -> Result<String> {
        let provider_name = provider.as_str();
        let client = self.oidc_clients.get(provider_name)
            .ok_or_else(|| SecurityError::Configuration(format!("OpenID Connect provider '{}' not configured", provider_name)))?;

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let nonce = Nonce::new_random();

        let mut auth_request = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .set_pkce_challenge(pkce_challenge);

        // Add scopes
        for scope in &self.config.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        let (auth_url, csrf_token, nonce_returned) = auth_request.url();

        // Store state
        let state = OAuth2State {
            provider: provider.clone(),
            csrf_token: csrf_token.clone(),
            pkce_verifier,
            nonce: Some(nonce_returned),
            created_at: chrono::Utc::now(),
        };

        let state_key = csrf_token.secret().clone();
        {
            let mut states = self.states.write().await;
            states.insert(state_key, state);
            drop(states);
        }

        Ok(auth_url.to_string())
    }

    /// Exchange authorization code for tokens (OAuth2)
    pub async fn exchange_code(&self, provider: OAuth2Provider, code: &str, state: &str) -> Result<OAuth2Tokens> {
        let provider_name = provider.as_str();

        // Retrieve and validate state
        let state_data = {
            let mut states = self.states.write().await;
            let state_data = states.remove(state)
                .ok_or_else(|| SecurityError::StateMismatch)?;

            // Check if state is expired
            let elapsed = chrono::Utc::now().signed_duration_since(state_data.created_at);
            if elapsed > chrono::Duration::seconds(self.config.state_timeout_seconds as i64) {
                return Err(SecurityError::StateMismatch);
            }

            state_data
        };

        let client = self.clients.get(provider_name)
            .ok_or_else(|| SecurityError::Configuration(format!("OAuth2 provider '{}' not configured", provider_name)))?;

        let token_result = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(state_data.pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| SecurityError::OAuth2(format!("Token exchange failed: {}", e)))?;

        let tokens = OAuth2Tokens {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
            token_type: token_result.token_type().as_ref().to_string(),
            expires_in: token_result.expires_in().map(|d| d.as_secs()),
            scope: token_result.scopes().map(|scopes| {
                scopes.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ")
            }),
            id_token: None, // OAuth2 doesn't have ID tokens
        };

        Ok(tokens)
    }

    /// Exchange authorization code for tokens (OpenID Connect)
    pub async fn exchange_oidc_code(&self, provider: OAuth2Provider, code: &str, state: &str) -> Result<(OAuth2Tokens, Option<UserInfo>)> {
        let provider_name = provider.as_str();

        // Retrieve and validate state
        let state_data = {
            let mut states = self.states.write().await;
            let state_data = states.remove(state)
                .ok_or_else(|| SecurityError::StateMismatch)?;

            // Check if state is expired
            let elapsed = chrono::Utc::now().signed_duration_since(state_data.created_at);
            if elapsed > chrono::Duration::seconds(self.config.state_timeout_seconds as i64) {
                return Err(SecurityError::StateMismatch);
            }

            state_data
        };

        let client = self.oidc_clients.get(provider_name)
            .ok_or_else(|| SecurityError::Configuration(format!("OpenID Connect provider '{}' not configured", provider_name)))?;

        let nonce = state_data.nonce.ok_or_else(|| SecurityError::OAuth2("Missing nonce".to_string()))?;

        let token_response = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(state_data.pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| SecurityError::OAuth2(format!("Token exchange failed: {}", e)))?;

        let id_token_verifier = client.id_token_verifier();
        let id_token = token_response.id_token()
            .ok_or_else(|| SecurityError::OAuth2("Missing ID token".to_string()))?;

        let claims: CoreIdTokenClaims = id_token
            .claims(&id_token_verifier, &nonce)
            .map_err(|e| SecurityError::OAuth2(format!("ID token validation failed: {}", e)))?;

        let tokens = OAuth2Tokens {
            access_token: token_response.access_token().secret().clone(),
            refresh_token: token_response.refresh_token().map(|t| t.secret().clone()),
            token_type: token_response.token_type().as_ref().to_string(),
            expires_in: token_response.expires_in().map(|d| d.as_secs()),
            scope: token_response.scopes().map(|scopes| {
                scopes.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ")
            }),
            id_token: Some(id_token.to_string()),
        };

        let user_info = UserInfo::from_id_token_claims(&claims);

        Ok((tokens, Some(user_info)))
    }

    /// Get user info from OAuth2 provider
    pub async fn get_user_info(&self, provider: OAuth2Provider, access_token: &str) -> Result<UserInfo> {
        let provider_name = provider.as_str();
        let provider_config = self.config.providers.get(provider_name)
            .ok_or_else(|| SecurityError::Configuration(format!("Provider '{}' not configured", provider_name)))?;

        let userinfo_url = provider_config.userinfo_url.as_ref()
            .ok_or_else(|| SecurityError::Configuration(format!("Userinfo URL not configured for provider '{}'", provider_name)))?;

        let response = self.http_client
            .get(userinfo_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| SecurityError::Http(e))?;

        if !response.status().is_success() {
            return Err(SecurityError::OAuth2(format!("Userinfo request failed: {}", response.status())));
        }

        let userinfo_data: serde_json::Value = response.json().await
            .map_err(|e| SecurityError::OAuth2(format!("JSON parsing failed: {}", e)))?;

        Self::parse_user_info(provider, &userinfo_data)
    }

    /// Check if provider supports OpenID Connect
    pub fn supports_oidc(&self, provider: &OAuth2Provider) -> bool {
        self.oidc_clients.contains_key(provider.as_str())
    }

    /// Create OAuth2 client
    fn create_oauth2_client(provider_config: &OAuth2ProviderConfig, redirect_uri: &str) -> Result<BasicClient> {
        let client = BasicClient::new(
            ClientId::new(provider_config.client_id.clone()),
            Some(ClientSecret::new(provider_config.client_secret.clone())),
            AuthUrl::new(provider_config.authorization_url.clone())
                .map_err(|e| SecurityError::Configuration(format!("Invalid auth URL: {}", e)))?,
            Some(TokenUrl::new(provider_config.token_url.clone())
                .map_err(|e| SecurityError::Configuration(format!("Invalid token URL: {}", e)))?)
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string())
            .map_err(|e| SecurityError::Configuration(format!("Invalid redirect URL: {}", e)))?);

        Ok(client)
    }

    /// Create OpenID Connect client
    async fn create_oidc_client(&self, provider_config: &OAuth2ProviderConfig, redirect_uri: &str) -> Result<CoreClient> {
        // For OpenID Connect, we need to discover the provider metadata
        // This is a simplified implementation - in production, you might want to cache this
        let issuer_url = match OAuth2Provider::from_str("google") {
            OAuth2Provider::Google => "https://accounts.google.com",
            OAuth2Provider::Microsoft => "https://login.microsoftonline.com/common/v2.0",
            _ => return Err(SecurityError::ProviderNotSupported),
        };

        // For now, skip OpenID Connect discovery and use manual configuration
        // TODO: Implement proper HTTP client function for OpenID Connect discovery
        Err(SecurityError::Configuration("OpenID Connect discovery not yet implemented".to_string()))
    }

    /// Parse user info from provider response
    fn parse_user_info(provider: OAuth2Provider, data: &serde_json::Value) -> Result<UserInfo> {
        match provider {
            OAuth2Provider::Google => Self::parse_google_userinfo(data),
            OAuth2Provider::GitHub => Self::parse_github_userinfo(data),
            OAuth2Provider::Microsoft => Self::parse_microsoft_userinfo(data),
            _ => Err(SecurityError::ProviderNotSupported),
        }
    }

    fn parse_google_userinfo(data: &serde_json::Value) -> Result<UserInfo> {
        let obj = data.as_object()
            .ok_or_else(|| SecurityError::OAuth2("Invalid userinfo response".to_string()))?;

        Ok(UserInfo {
            id: obj.get("sub").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            email: obj.get("email").and_then(|v| v.as_str()).map(|s| s.to_string()),
            email_verified: Some(obj.get("email_verified").and_then(|v| v.as_bool()).unwrap_or(false)),
            name: obj.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            given_name: obj.get("given_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            family_name: obj.get("family_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            picture: obj.get("picture").and_then(|v| v.as_str()).map(|s| s.to_string()),
            locale: obj.get("locale").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    fn parse_github_userinfo(data: &serde_json::Value) -> Result<UserInfo> {
        let obj = data.as_object()
            .ok_or_else(|| SecurityError::OAuth2("Invalid userinfo response".to_string()))?;

        Ok(UserInfo {
            id: obj.get("id").and_then(|v| v.as_u64()).unwrap_or(0).to_string(),
            email: obj.get("email").and_then(|v| v.as_str()).map(|s| s.to_string()),
            email_verified: Some(true), // GitHub emails are verified
            name: obj.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            given_name: None,
            family_name: None,
            picture: obj.get("avatar_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
            locale: None,
        })
    }

    fn parse_microsoft_userinfo(data: &serde_json::Value) -> Result<UserInfo> {
        let obj = data.as_object()
            .ok_or_else(|| SecurityError::OAuth2("Invalid userinfo response".to_string()))?;

        Ok(UserInfo {
            id: obj.get("sub").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            email: obj.get("email").and_then(|v| v.as_str()).map(|s| s.to_string()),
            email_verified: Some(obj.get("email_verified").and_then(|v| v.as_str())
                .map(|s| s == "true").unwrap_or(false)),
            name: obj.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            given_name: obj.get("given_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            family_name: obj.get("family_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            picture: None,
            locale: None,
        })
    }
}

/// User information from OAuth2/OIDC provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
    pub locale: Option<String>,
}

impl UserInfo {
    /// Create UserInfo from OpenID Connect ID token claims
    pub fn from_id_token_claims(claims: &CoreIdTokenClaims) -> Self {
        Self {
            id: claims.subject().to_string(),
            email: claims.email().map(|e| e.to_string()),
            email_verified: claims.email_verified(),
            name: claims.name().and_then(|n| n.get(None).map(|s| s.to_string())),
            given_name: claims.given_name().and_then(|n| n.get(None).map(|s| s.to_string())),
            family_name: claims.family_name().and_then(|n| n.get(None).map(|s| s.to_string())),
            picture: None, // Not in standard claims
            locale: claims.locale().map(|l| l.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_str() {
        assert_eq!(OAuth2Provider::from_str("google"), OAuth2Provider::Google);
        assert_eq!(OAuth2Provider::from_str("GitHub"), OAuth2Provider::GitHub);
        assert_eq!(OAuth2Provider::from_str("custom"), OAuth2Provider::Custom("custom".to_string()));
    }

    #[test]
    fn test_provider_as_str() {
        assert_eq!(OAuth2Provider::Google.as_str(), "google");
        assert_eq!(OAuth2Provider::GitHub.as_str(), "github");
        assert_eq!(OAuth2Provider::Custom("test".to_string()).as_str(), "test");
    }

    #[test]
    fn test_user_info_creation() {
        let user_info = UserInfo {
            id: "123".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: Some(true),
            name: Some("Test User".to_string()),
            given_name: Some("Test".to_string()),
            family_name: Some("User".to_string()),
            picture: Some("http://example.com/pic.jpg".to_string()),
            locale: Some("en".to_string()),
        };

        assert_eq!(user_info.id, "123");
        assert_eq!(user_info.email.as_ref().unwrap(), "test@example.com");
        assert_eq!(user_info.name.as_ref().unwrap(), "Test User");
    }
}
