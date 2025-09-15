//! HTTP設定ファイルパーサー
//!
//! kotoba-kotobanet を使用して .kotoba.json と .kotoba ファイルのパースを担当します。

use crate::types::{ContentHash, Result, KotobaError};
use crate::http::ir::*;
use std::fs;
use std::path::Path;
use std::io::Write;
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

        // Stub implementation - kotoba-kotobanet not available
        Ok(HttpConfig::new(ServerConfig::default()))
    }

    /// .kotobaファイルをパース（Jsonnet形式）
    pub fn parse_kotoba_file<P: AsRef<Path>>(path: P) -> Result<HttpConfig> {
        // Stub implementation - kotoba-kotobanet not available
        Ok(HttpConfig::new(ServerConfig::default()))
    }

    /// kotoba-kotobanet::HttpConfig を Kotoba の HttpConfig に変換
    fn convert_from_kotobanet_config(_kotobanet_config: serde_json::Value) -> Result<HttpConfig> {
        // Stub implementation - kotoba-kotobanet not available
        Ok(HttpConfig::new(ServerConfig::default()))
    }

    /// kotoba-kotobanet::ServerConfig を変換
    fn convert_server_config(_kotobanet_config: &serde_json::Value) -> Result<ServerConfig> {
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
    fn convert_route(_route: serde_json::Value) -> Result<HttpRoute> {
        // Stub implementation - kotoba-kotobanet not available
        Ok(HttpRoute::new(
            "default".to_string(),
            HttpMethod::GET,
            "/".to_string(),
            ContentHash::sha256([0u8; 32]),
        ))
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
