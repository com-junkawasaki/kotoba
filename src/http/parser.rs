//! HTTP設定ファイルパーサー
//!
//! .kotoba.jsonと.kotobaファイルのパースを担当します。

use crate::types::{Value, ContentHash, Result, KotobaError};
use crate::http::ir::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use sha2::{Sha256, Digest};

/// .kotoba.jsonファイルのフォーマット
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KotobaJsonConfig {
    pub server: ServerConfig,
    pub routes: Vec<KotobaRoute>,
    pub middlewares: Option<Vec<KotobaMiddleware>>,
    pub static_files: Option<StaticConfig>,
}

/// .kotobaファイルのフォーマット（JSON Lines形式）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum KotobaEntry {
    #[serde(rename = "route")]
    Route(KotobaRoute),
    #[serde(rename = "middleware")]
    Middleware(KotobaMiddleware),
    #[serde(rename = "config")]
    Config(KotobaServerConfig),
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
    pub function: String, // 関数名または関数定義
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
pub struct HttpConfigParser;

impl HttpConfigParser {
    /// .kotoba.jsonファイルをパース
    pub fn parse_kotoba_json<P: AsRef<Path>>(path: P) -> Result<HttpConfig> {
        let content = fs::read_to_string(path)
            .map_err(|e| KotobaError::IoError(format!("Failed to read config file: {}", e)))?;

        let config: KotobaJsonConfig = serde_json::from_str(&content)
            .map_err(|e| KotobaError::InvalidArgument(format!("Invalid JSON format: {}", e)))?;

        Self::convert_to_http_config(config)
    }

    /// .kotobaファイルをパース（JSON Lines形式）
    pub fn parse_kotoba_file<P: AsRef<Path>>(path: P) -> Result<HttpConfig> {
        let content = fs::read_to_string(path)
            .map_err(|e| KotobaError::IoError(format!("Failed to read config file: {}", e)))?;

        let mut config = HttpConfig::new(ServerConfig::default());
        let mut server_config: Option<KotobaServerConfig> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue; // 空行とコメントをスキップ
            }

            let entry: KotobaEntry = serde_json::from_str(line)
                .map_err(|e| KotobaError::InvalidArgument(format!("Invalid JSON line: {} - {}", line, e)))?;

            match entry {
                KotobaEntry::Route(route) => {
                    let route_ir = Self::convert_route(route)?;
                    config.routes.push(route_ir);
                }
                KotobaEntry::Middleware(mw) => {
                    let mw_ir = Self::convert_middleware(mw)?;
                    config.middlewares.push(mw_ir);
                }
                KotobaEntry::Config(srv_config) => {
                    server_config = Some(srv_config);
                }
            }
        }

        // サーバー設定を適用
        if let Some(srv_config) = server_config {
            config.server = Self::convert_server_config(srv_config);
        }

        Ok(config)
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
        let function_hash = Self::hash_function(&mw.function);

        let mut http_mw = HttpMiddleware::new(
            format!("mw_{}_{}", mw.name, mw.order),
            mw.name,
            mw.order,
            function_hash,
        );

        // メタデータを設定
        if let Some(metadata) = mw.metadata {
            http_mw.metadata.insert("function_source".to_string(), Value::String(mw.function));
            if let Ok(json_str) = serde_json::to_string(&metadata) {
                http_mw.metadata.insert("metadata".to_string(), Value::String(json_str));
            }
        } else {
            http_mw.metadata.insert("function_source".to_string(), Value::String(mw.function));
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
        {"type": "config", "host": "127.0.0.1", "port": 4000}
        {"type": "route", "method": "GET", "pattern": "/health", "handler": "health_check"}
        {"type": "middleware", "name": "logger", "order": 100, "function": "log_middleware"}
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
