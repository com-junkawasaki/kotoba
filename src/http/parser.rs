//! HTTP設定ファイルパーサー
//!
//! kotoba-kotobanet を使用して .kotoba.json と .kotoba ファイルのパースを担当します。

use crate::types::{Value, ContentHash, Result, KotobaError};
use crate::http::ir::*;
use kotoba_kotobanet::HttpParser as KotobaNetHttpParser;
use std::fs;
use std::path::Path;
use sha2::{Sha256, Digest};
use tempfile::NamedTempFile;

/// HTTP設定パーサー
///
/// kotoba-kotobanet::HttpParser を使用して HTTP 設定をパースします。
pub struct HttpConfigParser;

impl HttpConfigParser {
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

    /// .kotoba.jsonファイルをパース
    pub fn parse_kotoba_json<P: AsRef<Path>>(path: P) -> Result<HttpConfig> {
        let content = fs::read_to_string(path)
            .map_err(|e| KotobaError::IoError(format!("Failed to read config file: {}", e)))?;

        // kotoba-kotobanet の HttpParser を使用
        let http_config = KotobaNetHttpParser::parse(&content)
            .map_err(|e| KotobaError::InvalidArgument(format!("HTTP config parsing failed: {}", e)))?;

        // kotoba-kotobanet の HttpConfig を Kotoba の HttpConfig に変換
        Self::convert_from_kotobanet_config(http_config)
    }

    /// .kotobaファイルをパース（Jsonnet形式）
    pub fn parse_kotoba_file<P: AsRef<Path>>(path: P) -> Result<HttpConfig> {
        // kotoba-kotobanet の HttpParser を使用
        let http_config = KotobaNetHttpParser::parse_file(path)
            .map_err(|e| KotobaError::InvalidArgument(format!("HTTP config parsing failed: {}", e)))?;

        // kotoba-kotobanet の HttpConfig を Kotoba の HttpConfig に変換
        Self::convert_from_kotobanet_config(http_config)
    }

    /// kotoba-kotobanet::HttpConfig を Kotoba の HttpConfig に変換
    fn convert_from_kotobanet_config(kotobanet_config: kotoba_kotobanet::HttpConfig) -> Result<HttpConfig> {
        // ServerConfig を変換
        let server_config = Self::convert_server_config(&kotobanet_config)?;

        let mut http_config = HttpConfig::new(server_config);

        // Routes を変換
        for route in kotobanet_config.routes {
            let route_ir = Self::convert_route(route)?;
            http_config.routes.push(route_ir);
        }

        // Middlewares を変換 (必要に応じて)
        // TODO: kotobanet_config.middleware を処理

        // Static files を変換
        if let Some(static_files) = kotobanet_config.static_files {
            http_config.static_files = Some(StaticConfig {
                root: static_files.root,
                index_file: static_files.index_file,
                cache_control: static_files.cache_control,
            });
        }

        Ok(http_config)
    }

    /// kotoba-kotobanet::ServerConfig を変換
    fn convert_server_config(kotobanet_config: &kotoba_kotobanet::HttpConfig) -> Result<ServerConfig> {
        // TODO: 実際のサーバー設定を抽出するロジックを実装
        // 現時点ではデフォルト設定を使用
        Ok(ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: None,
            timeout_ms: None,
            tls: None,
        })
    }

    /// kotoba-kotobanet::HttpRouteConfig を HttpRoute に変換
    fn convert_route(route: kotoba_kotobanet::HttpRouteConfig) -> Result<HttpRoute> {
        let method = match route.method {
            kotoba_kotobanet::HttpMethod::GET => HttpMethod::GET,
            kotoba_kotobanet::HttpMethod::POST => HttpMethod::POST,
            kotoba_kotobanet::HttpMethod::PUT => HttpMethod::PUT,
            kotoba_kotobanet::HttpMethod::DELETE => HttpMethod::DELETE,
            kotoba_kotobanet::HttpMethod::PATCH => HttpMethod::PATCH,
            kotoba_kotobanet::HttpMethod::OPTIONS => HttpMethod::OPTIONS,
            kotoba_kotobanet::HttpMethod::HEAD => HttpMethod::HEAD,
        };

        let handler_hash = Self::hash_function(&route.handler);

        let mut http_route = HttpRoute::new(
            format!("{}_{}", serde_json::to_string(&method).unwrap_or_default().trim_matches('"'), route.path.replace('/', "_")),
            method,
            route.path,
            handler_hash,
        );

        // メタデータを設定
        http_route.metadata.insert("handler_source".to_string(), Value::String(route.handler));
        http_route.metadata.insert("auth_required".to_string(), Value::Boolean(route.auth_required));
        http_route.metadata.insert("cors_enabled".to_string(), Value::Boolean(route.cors_enabled));

        if let Some(rate_limit) = route.rate_limit {
            http_route.metadata.insert("rate_limit_requests".to_string(), Value::Number(rate_limit.requests_per_minute as f64));
            http_route.metadata.insert("rate_limit_burst".to_string(), Value::Number(rate_limit.burst_limit as f64));
        }

        Ok(http_route)
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

    #[test]
    fn test_parse_kotoba_json() {
        let config_str = r#"
        {
            "routes": [
                {
                    "method": "GET",
                    "path": "/ping",
                    "handler": "ping_handler",
                    "authRequired": false,
                    "corsEnabled": true
                }
            ]
        }
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_str.as_bytes()).unwrap();

        let config = HttpConfigParser::parse_kotoba_json(temp_file.path()).unwrap();

        assert_eq!(config.routes.len(), 1);
        assert_eq!(config.routes[0].method, HttpMethod::GET);
        assert_eq!(config.routes[0].pattern, "/ping");
    }

    #[test]
    fn test_parse_kotoba_file() {
        let config_str = r#"
        {
          routes: [
            {
              method: "GET",
              path: "/health",
              handler: "health_check",
              authRequired: false,
              corsEnabled: true
            }
          ]
        }
        "#;

        let mut temp_file = NamedTempFile::with_suffix(".kotoba").unwrap();
        temp_file.write_all(config_str.as_bytes()).unwrap();

        let config = HttpConfigParser::parse_kotoba_file(temp_file.path()).unwrap();

        assert_eq!(config.routes.len(), 1);
        assert_eq!(config.routes[0].method, HttpMethod::GET);
        assert_eq!(config.routes[0].pattern, "/health");
    }
}
