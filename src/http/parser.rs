//! HTTP設定ファイルパーサー
//!
//! .kotoba.jsonと.kotobaファイル（Jsonnet形式）のパースを担当します。

use crate::types::{Value, ContentHash, Result, KotobaError};
use crate::http::ir::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
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
