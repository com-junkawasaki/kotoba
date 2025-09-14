//! HTTP設定ファイルパーサー
//!
//! .kotoba.jsonと.kotobaファイル（Jsonnet形式）のパースを担当します。

use crate::types::{Value, ContentHash, Result, KotobaError};
use crate::http::ir::*;
use kotoba_security::{SecurityService, SecurityConfig, JwtService, OAuth2Service, OAuth2Provider};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use sha2::{Sha256, Digest};

/// .kotoba.jsonファイルのフォーマット
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KotobaJsonConfig {
    pub server: ServerConfig,
    pub routes: Vec<KotobaRoute>,
    pub middlewares: Option<Vec<KotobaMiddleware>>,
    pub static_files: Option<StaticConfig>,
}

/// .kotobaファイルのフォーマット（Jsonnet形式）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KotobaJsonnetConfig {
    pub config: Option<KotobaServerConfig>,
    pub routes: Option<Vec<KotobaRoute>>,
    pub middlewares: Option<Vec<KotobaMiddleware>>,
    pub server: Option<ServerConfig>,
    pub security: Option<KotobaSecurityConfig>,
    pub functions: Option<Vec<KotobaFunction>>,
}

/// セキュリティ設定（.kotobaファイル用）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KotobaSecurityConfig {
    pub jwt: Option<serde_json::Value>,
    pub oauth2: Option<serde_json::Value>,
    pub mfa: Option<serde_json::Value>,
    pub password: Option<serde_json::Value>,
    pub session: Option<serde_json::Value>,
}

/// 関数定義（.kotobaファイル用）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KotobaFunction {
    pub name: String,
    pub function_type: FunctionType,
    pub code: String,
    pub metadata: Option<serde_json::Value>,
}

/// 関数タイプ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionType {
    Jwt,
    OAuth2,
    Mfa,
    Password,
    Session,
    Security,
    Custom,
}

/// ルート設定（設定ファイル用）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KotobaRoute {
    pub method: String,
    pub pattern: String,
    pub handler: String, // ハンドラー名または関数定義
    pub metadata: Option<serde_json::Value>,
}

/// ミドルウェア設定（設定ファイル用）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KotobaMiddleware {
    pub name: String,
    pub order: i32,
    #[serde(alias = "function")]
    pub handler: String, // 関数名または関数定義
    pub metadata: Option<serde_json::Value>,
}

/// サーバー設定（設定ファイル用）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KotobaServerConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub max_connections: Option<usize>,
    pub timeout_ms: Option<u64>,
    pub tls: Option<TlsConfig>,
}

/// HTTP設定パーサー
pub struct HttpConfigParser {
    security_service: Option<Arc<SecurityService>>,
}

impl HttpConfigParser {
    /// 新しいパーサーを作成
    pub fn new() -> Self {
        Self {
            security_service: None,
        }
    }

    /// セキュリティサービスを設定
    pub fn with_security_service(mut self, security_service: Arc<SecurityService>) -> Self {
        self.security_service = Some(security_service);
        self
    }

    /// 関数実行エンジンを取得
    pub fn function_engine(&self) -> Option<FunctionEngine> {
        self.security_service.as_ref().map(|service| FunctionEngine::new(service.clone()))
    }

    /// .kotoba.jsonファイルをパース
    pub fn parse_kotoba_json<P: AsRef<Path>>(path: P) -> Result<HttpConfig> {
        let content = fs::read_to_string(path)
            .map_err(|e| KotobaError::IoError(format!("Failed to read config file: {}", e)))?;

        let config: KotobaJsonConfig = serde_json::from_str(&content)
            .map_err(|e| KotobaError::InvalidArgument(format!("Invalid JSON format: {}", e)))?;

        Self::convert_to_http_config(config)
    }

    /// .kotobaファイルをパース（Jsonnet形式）
    pub fn parse_kotoba_file<P: AsRef<Path>>(path: P) -> Result<HttpConfig> {
        let path = path.as_ref();

        // Jsonnetファイルを評価してJSONに変換
        let json_output = Self::evaluate_jsonnet(path)?;

        // JSONをパース
        let kotoba_config: KotobaJsonnetConfig = serde_json::from_str(&json_output)
            .map_err(|e| KotobaError::InvalidArgument(format!("Invalid Jsonnet output: {}", e)))?;

        Self::convert_jsonnet_to_http_config(kotoba_config)
    }

    /// Jsonnetファイルを評価してJSON文字列を返す
    fn evaluate_jsonnet<P: AsRef<Path>>(path: P) -> Result<String> {
        let output = Command::new("jsonnet")
            .arg("eval")
            .arg("--output")
            .arg("json")
            .arg(path.as_ref())
            .output()
            .map_err(|e| KotobaError::IoError(format!("Failed to execute jsonnet: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KotobaError::InvalidArgument(format!("Jsonnet evaluation failed: {}", stderr)));
        }

        let json_str = String::from_utf8(output.stdout)
            .map_err(|e| KotobaError::IoError(format!("Invalid UTF-8 output from jsonnet: {}", e)))?;

        Ok(json_str)
    }

    /// KotobaJsonnetConfigをHttpConfigに変換
    fn convert_jsonnet_to_http_config(config: KotobaJsonnetConfig) -> Result<HttpConfig> {
        let server_config = if let Some(server) = config.server {
            server
        } else if let Some(kotoba_config) = config.config {
            Self::convert_server_config(kotoba_config)
        } else {
            ServerConfig::default()
        };

        let mut http_config = HttpConfig::new(server_config);

        // ルートを変換
        if let Some(routes) = config.routes {
            for route in routes {
                let route_ir = Self::convert_route(route)?;
                http_config.routes.push(route_ir);
            }
        }

        // ミドルウェアを変換
        if let Some(middlewares) = config.middlewares {
            for mw in middlewares {
                let mw_ir = Self::convert_middleware(mw)?;
                http_config.middlewares.push(mw_ir);
            }
        }

        Ok(http_config)
    }

    /// ファイル拡張子に基づいて適切なパーサーを選択
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<HttpConfig> {
        let path = path.as_ref();

        if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("json") => Self::parse_kotoba_json(path),
                Some("kotoba") => Self::parse_kotoba_file(path),
                _ => Err(KotobaError::InvalidArgument(
                    "Unsupported file extension. Use .json or .kotoba".to_string()
                )),
            }
        } else {
            Err(KotobaError::InvalidArgument(
                "File must have .json or .kotoba extension".to_string()
            ))
        }
    }

    /// KotobaJsonConfigをHttpConfigに変換
    fn convert_to_http_config(config: KotobaJsonConfig) -> Result<HttpConfig> {
        let mut http_config = HttpConfig::new(config.server);

        // ルートを変換
        for route in config.routes {
            let route_ir = Self::convert_route(route)?;
            http_config.routes.push(route_ir);
        }

        // ミドルウェアを変換
        if let Some(middlewares) = config.middlewares {
            for mw in middlewares {
                let mw_ir = Self::convert_middleware(mw)?;
                http_config.middlewares.push(mw_ir);
            }
        }

        http_config.static_files = config.static_files;
        Ok(http_config)
    }

    /// KotobaRouteをHttpRouteに変換
    fn convert_route(route: KotobaRoute) -> Result<HttpRoute> {
        let method = HttpMethod::from_str(&route.method)?;
        let handler_hash = Self::hash_function(&route.handler);

        let mut http_route = HttpRoute::new(
            format!("{}_{}", route.method, route.pattern.replace('/', "_")),
            method,
            route.pattern,
            handler_hash,
        );

        // メタデータを設定
        if let Some(metadata) = route.metadata {
            http_route.metadata.insert("handler_source".to_string(), Value::String(route.handler));
            if let Ok(json_str) = serde_json::to_string(&metadata) {
                http_route.metadata.insert("metadata".to_string(), Value::String(json_str));
            }
        } else {
            http_route.metadata.insert("handler_source".to_string(), Value::String(route.handler));
        }

        Ok(http_route)
    }

    /// KotobaMiddlewareをHttpMiddlewareに変換
    fn convert_middleware(mw: KotobaMiddleware) -> Result<HttpMiddleware> {
        let function_hash = Self::hash_function(&mw.handler);

        let mut http_mw = HttpMiddleware::new(
            format!("mw_{}_{}", mw.name, mw.order),
            mw.name,
            mw.order,
            function_hash,
        );

        // メタデータを設定
        if let Some(metadata) = mw.metadata {
            http_mw.metadata.insert("function_source".to_string(), Value::String(mw.handler));
            if let Ok(json_str) = serde_json::to_string(&metadata) {
                http_mw.metadata.insert("metadata".to_string(), Value::String(json_str));
            }
        } else {
            http_mw.metadata.insert("function_source".to_string(), Value::String(mw.handler));
        }

        Ok(http_mw)
    }

    /// KotobaServerConfigをServerConfigに変換
    fn convert_server_config(config: KotobaServerConfig) -> ServerConfig {
        ServerConfig {
            host: config.host.unwrap_or_else(|| "127.0.0.1".to_string()),
            port: config.port.unwrap_or(8080),
            max_connections: config.max_connections,
            timeout_ms: config.timeout_ms,
            tls: config.tls,
        }
    }

    /// 関数定義のハッシュを計算
    fn hash_function(function_def: &str) -> ContentHash {
        let mut hasher = Sha256::new();
        hasher.update(function_def.as_bytes());
        let result = hasher.finalize();
        ContentHash(hex::encode(result))
    }
}

/// 関数実行エンジン
pub struct FunctionEngine {
    security_service: Arc<SecurityService>,
}

impl FunctionEngine {
    /// 新しい関数実行エンジンを作成
    pub fn new(security_service: Arc<SecurityService>) -> Self {
        Self { security_service }
    }

    /// 関数を実行
    pub async fn execute_function(&self, function: &KotobaFunction, params: serde_json::Value) -> Result<serde_json::Value> {
        match function.function_type {
            FunctionType::Jwt => self.execute_jwt_function(&function.code, params).await,
            FunctionType::OAuth2 => self.execute_oauth2_function(&function.code, params).await,
            FunctionType::Mfa => self.execute_mfa_function(&function.code, params).await,
            FunctionType::Password => self.execute_password_function(&function.code, params).await,
            FunctionType::Session => self.execute_session_function(&function.code, params).await,
            FunctionType::Security => self.execute_security_function(&function.code, params).await,
            FunctionType::Custom => self.execute_custom_function(&function.code, params).await,
        }
    }

    /// JWT関数を実行
    async fn execute_jwt_function(&self, code: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        match code {
            "generate_access_token" => {
                let user_id = params.get("user_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("user_id required".to_string()))?;

                let roles = params.get("roles")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| KotobaError::Configuration("roles required".to_string()))?;

                let roles_str: Vec<String> = roles.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();

                let token = self.security_service.jwt().generate_access_token(user_id, roles_str)
                    .map_err(|e| KotobaError::Security(format!("JWT generation failed: {:?}", e)))?;

                Ok(serde_json::json!({ "token": token }))
            }
            "validate_token" => {
                let token = params.get("token")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("token required".to_string()))?;

                let claims = self.security_service.jwt().validate_token(token)
                    .map_err(|e| KotobaError::Security(format!("JWT validation failed: {:?}", e)))?;

                Ok(serde_json::json!({
                    "sub": claims.sub,
                    "roles": claims.roles,
                    "exp": claims.exp,
                    "iat": claims.iat
                }))
            }
            "generate_token_pair" => {
                let user_id = params.get("user_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("user_id required".to_string()))?;

                let roles = params.get("roles")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| KotobaError::Configuration("roles required".to_string()))?;

                let roles_str: Vec<String> = roles.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();

                let token_pair = self.security_service.jwt().generate_token_pair(user_id, roles_str)
                    .map_err(|e| KotobaError::Security(format!("Token pair generation failed: {:?}", e)))?;

                Ok(serde_json::json!({
                    "access_token": token_pair.access_token,
                    "refresh_token": token_pair.refresh_token,
                    "token_type": token_pair.token_type,
                    "expires_in": token_pair.expires_in
                }))
            }
            "refresh_token" => {
                let refresh_token = params.get("refresh_token")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("refresh_token required".to_string()))?;

                let token_pair = self.security_service.jwt().refresh_access_token(refresh_token)
                    .map_err(|e| KotobaError::Security(format!("Token refresh failed: {:?}", e)))?;

                Ok(serde_json::json!({
                    "access_token": token_pair.access_token,
                    "refresh_token": token_pair.refresh_token,
                    "token_type": token_pair.token_type,
                    "expires_in": token_pair.expires_in
                }))
            }
            _ => Err(KotobaError::Configuration(format!("Unknown JWT function: {}", code))),
        }
    }

    /// OAuth2関数を実行
    async fn execute_oauth2_function(&self, code: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        match code {
            "get_authorization_url" => {
                let provider = params.get("provider")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("provider required".to_string()))?;

                let oauth_provider = match provider {
                    "google" => OAuth2Provider::Google,
                    "github" => OAuth2Provider::GitHub,
                    "microsoft" => OAuth2Provider::Microsoft,
                    _ => return Err(KotobaError::Configuration(format!("Unknown provider: {}", provider))),
                };

                let auth_url = self.security_service.oauth2().as_ref()
                    .ok_or_else(|| KotobaError::Configuration("OAuth2 not configured".to_string()))?
                    .get_authorization_url(oauth_provider).await
                    .map_err(|e| KotobaError::Security(format!("OAuth2 URL generation failed: {:?}", e)))?;

                Ok(serde_json::json!({ "authorization_url": auth_url }))
            }
            "exchange_code" => {
                let provider = params.get("provider")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("provider required".to_string()))?;

                let code = params.get("code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("code required".to_string()))?;

                let state = params.get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let oauth_provider = match provider {
                    "google" => OAuth2Provider::Google,
                    "github" => OAuth2Provider::GitHub,
                    "microsoft" => OAuth2Provider::Microsoft,
                    _ => return Err(KotobaError::Configuration(format!("Unknown provider: {}", provider))),
                };

                let tokens = self.security_service.oauth2().as_ref()
                    .ok_or_else(|| KotobaError::Configuration("OAuth2 not configured".to_string()))?
                    .exchange_code(oauth_provider, code, state).await
                    .map_err(|e| KotobaError::Security(format!("OAuth2 code exchange failed: {:?}", e)))?;

                Ok(serde_json::json!({
                    "access_token": tokens.access_token,
                    "refresh_token": tokens.refresh_token,
                    "token_type": tokens.token_type,
                    "expires_in": tokens.expires_in,
                    "scope": tokens.scope
                }))
            }
            _ => Err(KotobaError::Configuration(format!("Unknown OAuth2 function: {}", code))),
        }
    }

    /// MFA関数を実行
    async fn execute_mfa_function(&self, code: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        match code {
            "generate_secret" => {
                let account_name = params.get("account_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("account_name required".to_string()))?;

                let (secret, qr_code) = self.security_service.mfa().generate_secret(account_name)
                    .map_err(|e| KotobaError::Security(format!("MFA secret generation failed: {:?}", e)))?;

                Ok(serde_json::json!({
                    "secret": secret,
                    "qr_code": qr_code
                }))
            }
            "verify_code" => {
                let secret = params.get("secret")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("secret required".to_string()))?;

                let code = params.get("code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("code required".to_string()))?;

                let is_valid = self.security_service.mfa().verify_code(secret, code)
                    .map_err(|e| KotobaError::Security(format!("MFA verification failed: {:?}", e)))?;

                Ok(serde_json::json!({ "valid": is_valid }))
            }
            _ => Err(KotobaError::Configuration(format!("Unknown MFA function: {}", code))),
        }
    }

    /// パスワード関数を実行
    async fn execute_password_function(&self, code: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        match code {
            "hash_password" => {
                let password = params.get("password")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("password required".to_string()))?;

                let hash = self.security_service.password().hash_password(password)
                    .map_err(|e| KotobaError::Security(format!("Password hashing failed: {:?}", e)))?;

                Ok(serde_json::json!({
                    "hash": hash.to_string()
                }))
            }
            "verify_password" => {
                let password = params.get("password")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("password required".to_string()))?;

                let hash_str = params.get("hash")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("hash required".to_string()))?;

                // パースして検証
                let hash = self.security_service.password().parse_password_hash(hash_str)
                    .map_err(|e| KotobaError::Security(format!("Hash parsing failed: {:?}", e)))?;

                let is_valid = self.security_service.password().verify_password(password, &hash)
                    .map_err(|e| KotobaError::Security(format!("Password verification failed: {:?}", e)))?;

                Ok(serde_json::json!({ "valid": is_valid }))
            }
            "validate_password_complexity" => {
                let password = params.get("password")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("password required".to_string()))?;

                let errors = self.security_service.password().validate_password_complexity(password)
                    .map_err(|e| KotobaError::Security(format!("Password validation failed: {:?}", e)))?;

                Ok(serde_json::json!({ "errors": errors }))
            }
            _ => Err(KotobaError::Configuration(format!("Unknown password function: {}", code))),
        }
    }

    /// セッション関数を実行
    async fn execute_session_function(&self, code: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        match code {
            "create_session" => {
                let user_id = params.get("user_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("user_id required".to_string()))?;

                let roles = params.get("roles")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| KotobaError::Configuration("roles required".to_string()))?;

                let roles_str: Vec<String> = roles.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();

                let permissions = vec![]; // デフォルトで空
                let ip_address = params.get("ip_address").and_then(|v| v.as_str()).map(|s| s.to_string());
                let user_agent = params.get("user_agent").and_then(|v| v.as_str()).map(|s| s.to_string());

                let session = self.security_service.session().create_session(
                    user_id,
                    roles_str,
                    permissions,
                    ip_address,
                    user_agent,
                ).await
                .map_err(|e| KotobaError::Security(format!("Session creation failed: {:?}", e)))?;

                Ok(serde_json::json!({
                    "session_id": session.session_id,
                    "user_id": session.user_id,
                    "expires_at": session.expires_at.timestamp()
                }))
            }
            "get_session" => {
                let session_id = params.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("session_id required".to_string()))?;

                let session = self.security_service.session().get_session(session_id).await
                    .map_err(|e| KotobaError::Security(format!("Session retrieval failed: {:?}", e)))?;

                match session {
                    Some(session) => Ok(serde_json::json!({
                        "session_id": session.session_id,
                        "user_id": session.user_id,
                        "roles": session.roles,
                        "expires_at": session.expires_at.timestamp()
                    })),
                    None => Ok(serde_json::json!({ "session": null })),
                }
            }
            "delete_session" => {
                let session_id = params.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("session_id required".to_string()))?;

                self.security_service.session().delete_session(session_id).await
                    .map_err(|e| KotobaError::Security(format!("Session deletion failed: {:?}", e)))?;

                Ok(serde_json::json!({ "deleted": true }))
            }
            _ => Err(KotobaError::Configuration(format!("Unknown session function: {}", code))),
        }
    }

    /// セキュリティ関数を実行
    async fn execute_security_function(&self, code: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        match code {
            "authenticate" => {
                let identifier = params.get("identifier")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("identifier required".to_string()))?;

                let password = params.get("password")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| KotobaError::Configuration("password required".to_string()))?;

                // 簡易的な認証（実際の実装ではユーザーデータベースから検証）
                if identifier == "admin" && password == "password" {
                    let roles = vec!["admin".to_string(), "user".to_string()];
                    let token_pair = self.security_service.jwt().generate_token_pair(identifier, roles)
                        .map_err(|e| KotobaError::Security(format!("Token generation failed: {:?}", e)))?;

                    Ok(serde_json::json!({
                        "authenticated": true,
                        "user_id": identifier,
                        "token_pair": {
                            "access_token": token_pair.access_token,
                            "refresh_token": token_pair.refresh_token
                        }
                    }))
                } else {
                    Ok(serde_json::json!({
                        "authenticated": false,
                        "error": "Invalid credentials"
                    }))
                }
            }
            _ => Err(KotobaError::Configuration(format!("Unknown security function: {}", code))),
        }
    }

    /// カスタム関数を実行
    async fn execute_custom_function(&self, code: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        // カスタム関数の実行ロジック（拡張可能）
        Err(KotobaError::Configuration(format!("Custom function '{}' not implemented", code)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_kotoba_json() {
        let config_str = r#"
        {
            "server": {
                "host": "127.0.0.1",
                "port": 3000
            },
            "routes": [
                {
                    "method": "GET",
                    "pattern": "/ping",
                    "handler": "ping_handler",
                    "metadata": {"description": "Health check endpoint"}
                }
            ]
        }
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_str.as_bytes()).unwrap();

        let config = HttpConfigParser::parse_kotoba_json(temp_file.path()).unwrap();

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.routes.len(), 1);
        assert_eq!(config.routes[0].method, HttpMethod::GET);
        assert_eq!(config.routes[0].pattern, "/ping");
    }

    #[test]
    fn test_parse_kotoba_file() {
        let config_str = r#"
        {
          config: {
            host: "127.0.0.1",
            port: 4000
          },
          routes: [
            {
              method: "GET",
              pattern: "/health",
              handler: "health_check"
            }
          ],
          middlewares: [
            {
              name: "logger",
              order: 100,
              handler: "log_middleware"
            }
          ]
        }
        "#;

        let mut temp_file = NamedTempFile::with_suffix(".kotoba").unwrap();
        temp_file.write_all(config_str.as_bytes()).unwrap();

        let config = HttpConfigParser::parse_kotoba_file(temp_file.path()).unwrap();

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 4000);
        assert_eq!(config.routes.len(), 1);
        assert_eq!(config.middlewares.len(), 1);
        assert_eq!(config.routes[0].method, HttpMethod::GET);
        assert_eq!(config.routes[0].pattern, "/health");
        assert_eq!(config.middlewares[0].name, "logger");
    }
}
