//! デプロイ設定ファイルパーサー
//!
//! このモジュールはJsonnet形式の.kotoba-deployファイルをパースします。
//! Jsonnetの高度な機能を活用して、動的な設定生成をサポートします。

use kotoba_core::types::{Result, Value, ContentHash};
use crate::deploy::config::{DeployConfig, DeployConfigBuilder};
// use serde_json::{Value as JsonValue, Map}; // 簡易実装では使用しない
use std::path::Path;
use std::fs;
use std::collections::HashMap;

/// デプロイ設定パーサー
pub struct DeployConfigParser {
    jsonnet_evaluator: JsonnetEvaluator,
}

/// Jsonnet評価器 (簡易実装)
struct JsonnetEvaluator;

impl JsonnetEvaluator {
    fn new() -> Self {
        Self
    }

    /// Jsonnetファイルを評価してJSONに変換
    fn evaluate_file<P: AsRef<Path>>(&self, path: P) -> Result<JsonValue> {
        let content = fs::read_to_string(path)?;
        self.evaluate_string(&content)
    }

    /// Jsonnet文字列を評価してJSONに変換
    fn evaluate_string(&self, content: &str) -> Result<JsonValue> {
        // 簡易的なJsonnetパーサー (実際の実装ではjsonnet crateを使用)
        self.parse_jsonnet_like(content)
    }

    /// Jsonnetライクな構文をパース (簡易実装)
    fn parse_jsonnet_like(&self, content: &str) -> Result<JsonValue> {
        // コメントを除去
        let content = self.remove_comments(content);

        // 基本的なJsonnet機能をサポート
        if content.trim().starts_with('{') {
            self.parse_object(&content)
        } else {
            Err(crate::types::KotobaError::InvalidArgument(
                "Unsupported Jsonnet syntax".to_string()
            ))
        }
    }

    /// コメントを除去
    fn remove_comments(&self, content: &str) -> String {
        let mut result = String::new();
        let mut in_multiline_comment = false;
        let mut in_singleline_comment = false;

        let chars: Vec<char> = content.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if !in_multiline_comment && !in_singleline_comment {
                // 複数行コメントの開始
                if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
                    in_multiline_comment = true;
                    i += 2;
                    continue;
                }
                // 単一行コメントの開始
                if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                    in_singleline_comment = true;
                    i += 2;
                    continue;
                }
                result.push(chars[i]);
            } else if in_multiline_comment {
                // 複数行コメントの終了
                if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '/' {
                    in_multiline_comment = false;
                    i += 2;
                    continue;
                }
            } else if in_singleline_comment {
                // 単一行コメントの終了
                if chars[i] == '\n' {
                    in_singleline_comment = false;
                    result.push(chars[i]);
                }
            }
            i += 1;
        }

        result
    }

    /// オブジェクトをパース
    fn parse_object(&self, content: &str) -> Result<JsonValue> {
        let content = content.trim();

        if !content.starts_with('{') || !content.ends_with('}') {
            return Err(crate::types::KotobaError::InvalidArgument(
                "Invalid object syntax".to_string()
            ));
        }

        let inner = &content[1..content.len() - 1];
        let mut map = Map::new();

        // 簡易的なキーバリューパース
        let pairs = self.split_object_pairs(inner)?;

        for pair in pairs {
            let parts: Vec<&str> = pair.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].trim().trim_matches('"').to_string();
            let value_str = parts[1].trim();

            let value = if value_str.starts_with('"') && value_str.ends_with('"') {
                JsonValue::String(value_str[1..value_str.len() - 1].to_string())
            } else if value_str == "true" {
                JsonValue::Bool(true)
            } else if value_str == "false" {
                JsonValue::Bool(false)
            } else if value_str == "null" {
                JsonValue::Null
            } else if let Ok(num) = value_str.parse::<i64>() {
                JsonValue::Number(num.into())
            } else if let Ok(num) = value_str.parse::<f64>() {
                JsonValue::Number(serde_json::Number::from_f64(num).unwrap())
            } else if value_str.starts_with('{') {
                self.parse_object(value_str)?
            } else if value_str.starts_with('[') {
                self.parse_array(value_str)?
            } else {
                JsonValue::String(value_str.to_string())
            };

            map.insert(key, value);
        }

        Ok(JsonValue::Object(map))
    }

    /// 配列をパース
    fn parse_array(&self, content: &str) -> Result<JsonValue> {
        let content = content.trim();

        if !content.starts_with('[') || !content.ends_with(']') {
            return Err(crate::types::KotobaError::InvalidArgument(
                "Invalid array syntax".to_string()
            ));
        }

        let inner = &content[1..content.len() - 1];
        let mut array = Vec::new();

        if !inner.trim().is_empty() {
            let elements = self.split_array_elements(inner)?;

            for element in elements {
                let value = if element.starts_with('"') && element.ends_with('"') {
                    JsonValue::String(element[1..element.len() - 1].to_string())
                } else if element == "true" {
                    JsonValue::Bool(true)
                } else if element == "false" {
                    JsonValue::Bool(false)
                } else if element == "null" {
                    JsonValue::Null
                } else if let Ok(num) = element.parse::<i64>() {
                    JsonValue::Number(num.into())
                } else if let Ok(num) = element.parse::<f64>() {
                    JsonValue::Number(serde_json::Number::from_f64(num).unwrap())
                } else {
                    JsonValue::String(element.to_string())
                };

                array.push(value);
            }
        }

        Ok(JsonValue::Array(array))
    }

    /// オブジェクトのキーバリューペアを分割
    fn split_object_pairs(&self, content: &str) -> Result<Vec<String>> {
        let mut pairs = Vec::new();
        let mut current = String::new();
        let mut brace_depth = 0;
        let mut bracket_depth = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for ch in content.chars() {
            match ch {
                '"' if !escape_next => {
                    in_string = !in_string;
                    current.push(ch);
                }
                '\\' if in_string => {
                    escape_next = true;
                    current.push(ch);
                }
                '{' if !in_string => {
                    brace_depth += 1;
                    current.push(ch);
                }
                '}' if !in_string => {
                    brace_depth -= 1;
                    current.push(ch);
                }
                '[' if !in_string => {
                    bracket_depth += 1;
                    current.push(ch);
                }
                ']' if !in_string => {
                    bracket_depth -= 1;
                    current.push(ch);
                }
                ',' if !in_string && brace_depth == 0 && bracket_depth == 0 => {
                    if !current.trim().is_empty() {
                        pairs.push(current.trim().to_string());
                        current.clear();
                    }
                }
                _ => {
                    if escape_next {
                        escape_next = false;
                    }
                    current.push(ch);
                }
            }
        }

        if !current.trim().is_empty() {
            pairs.push(current.trim().to_string());
        }

        Ok(pairs)
    }

    /// 配列の要素を分割
    fn split_array_elements(&self, content: &str) -> Result<Vec<String>> {
        let mut elements = Vec::new();
        let mut current = String::new();
        let mut brace_depth = 0;
        let mut bracket_depth = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for ch in content.chars() {
            match ch {
                '"' if !escape_next => {
                    in_string = !in_string;
                    current.push(ch);
                }
                '\\' if in_string => {
                    escape_next = true;
                    current.push(ch);
                }
                '{' if !in_string => {
                    brace_depth += 1;
                    current.push(ch);
                }
                '}' if !in_string => {
                    brace_depth -= 1;
                    current.push(ch);
                }
                '[' if !in_string => {
                    bracket_depth += 1;
                    current.push(ch);
                }
                ']' if !in_string => {
                    bracket_depth -= 1;
                    current.push(ch);
                }
                ',' if !in_string && brace_depth == 0 && bracket_depth == 0 => {
                    if !current.trim().is_empty() {
                        elements.push(current.trim().to_string());
                        current.clear();
                    }
                }
                _ => {
                    if escape_next {
                        escape_next = false;
                    }
                    current.push(ch);
                }
            }
        }

        if !current.trim().is_empty() {
            elements.push(current.trim().to_string());
        }

        Ok(elements)
    }
}

impl DeployConfigParser {
    /// 新しいパーサーを作成
    pub fn new() -> Self {
        Self {
            jsonnet_evaluator: JsonnetEvaluator::new(),
        }
    }

    /// 設定ファイルをパース
    pub fn parse<P: AsRef<Path>>(&self, path: P) -> Result<DeployConfig> {
        let json_value = self.jsonnet_evaluator.evaluate_file(path)?;
        self.parse_json_value(json_value)
    }

    /// JSON文字列から設定をパース
    pub fn parse_string(&self, content: &str) -> Result<DeployConfig> {
        let json_value = self.jsonnet_evaluator.evaluate_string(content)?;
        self.parse_json_value(json_value)
    }

    /// JSON値からDeployConfigを構築
    fn parse_json_value(&self, value: JsonValue) -> Result<DeployConfig> {
        let obj = value.as_object().ok_or_else(|| {
            crate::types::KotobaError::InvalidArgument(
                "Root must be an object".to_string()
            )
        })?;

        // デフォルト設定から開始
        let mut builder = DeployConfigBuilder::new(
            "default-app".to_string(),
            "main.rs".to_string(),
        );

        // metadataセクション
        if let Some(metadata) = obj.get("metadata") {
            if let Some(metadata_obj) = metadata.as_object() {
                if let Some(name) = metadata_obj.get("name").and_then(|v| v.as_str()) {
                    builder = DeployConfigBuilder::new(
                        name.to_string(),
                        "main.rs".to_string(), // 一旦デフォルト
                    );
                }

                if let Some(version) = metadata_obj.get("version").and_then(|v| v.as_str()) {
                    builder = builder.version(version.to_string());
                }

                if let Some(description) = metadata_obj.get("description").and_then(|v| v.as_str()) {
                    builder = builder.description(description.to_string());
                }
            }
        }

        // applicationセクション
        if let Some(application) = obj.get("application") {
            if let Some(app_obj) = application.as_object() {
                if let Some(entry_point) = app_obj.get("entry_point").and_then(|v| v.as_str()) {
                    // エントリーポイントを更新
                    let mut config = builder.build();
                    config.application.entry_point = entry_point.to_string();
                    builder = DeployConfigBuilder::new(
                        config.metadata.name,
                        config.application.entry_point,
                    ).version(config.metadata.version);

                    if let Some(desc) = &config.metadata.description {
                        builder = builder.description(desc.clone());
                    }
                }

                if let Some(runtime_str) = app_obj.get("runtime").and_then(|v| v.as_str()) {
                    let runtime = match runtime_str {
                        "http_server" => crate::deploy::RuntimeType::HttpServer,
                        "frontend" => crate::deploy::RuntimeType::Frontend,
                        "graphql" => crate::deploy::RuntimeType::GraphQL,
                        "microservice" => crate::deploy::RuntimeType::Microservice,
                        custom => crate::deploy::RuntimeType::Custom(custom.to_string()),
                    };
                    builder = builder.runtime(runtime);
                }
            }
        }

        // scalingセクション
        if let Some(scaling) = obj.get("scaling") {
            if let Some(scale_obj) = scaling.as_object() {
                let min_instances = scale_obj
                    .get("min_instances")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as u32;
                let max_instances = scale_obj
                    .get("max_instances")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as u32;

                builder = builder.scaling(min_instances, max_instances);
            }
        }

        // networkセクション
        if let Some(network) = obj.get("network") {
            if let Some(net_obj) = network.as_object() {
                if let Some(domains) = net_obj.get("domains").and_then(|v| v.as_array()) {
                    for domain in domains {
                        if let Some(domain_str) = domain.as_str() {
                            builder = builder.add_domain(domain_str.to_string());
                        }
                    }
                }

                if let Some(regions) = net_obj.get("regions").and_then(|v| v.as_array()) {
                    for region in regions {
                        if let Some(region_str) = region.as_str() {
                            builder = builder.add_region(region_str.to_string());
                        }
                    }
                }
            }
        }

        // environmentセクション
        if let Some(environment) = obj.get("environment") {
            if let Some(env_obj) = environment.as_object() {
                for (key, value) in env_obj {
                    if let Some(value_str) = value.as_str() {
                        builder = builder.env(key.clone(), value_str.to_string());
                    }
                }
            }
        }

        let mut config = builder.build();

        // 設定の検証
        config.validate()?;

        // ハッシュの計算
        config.metadata.config_hash = Some(config.calculate_hash()?);

        Ok(config)
    }
}

impl Default for DeployConfigParser {
    fn default() -> Self {
        Self::new()
    }
}

/// デプロイ設定ファイルの検出と読み込み
pub fn find_and_parse_deploy_config<P: AsRef<Path>>(
    project_root: P,
) -> Result<Option<DeployConfig>> {
    let project_root = project_root.as_ref();

    // .kotoba-deployファイルの候補
    let candidates = [
        ".kotoba-deploy.jsonnet",
        ".kotoba-deploy.json",
        ".kotoba-deploy",
        "kotoba-deploy.jsonnet",
        "kotoba-deploy.json",
        "deploy.jsonnet",
        "deploy.json",
    ];

    for candidate in &candidates {
        let path = project_root.join(candidate);
        if path.exists() {
            let parser = DeployConfigParser::new();
            let config = parser.parse(&path)?;
            return Ok(Some(config));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_simple_config() {
        let content = r#"
        {
            metadata: {
                name: "test-app",
                version: "1.0.0",
                description: "Test application",
            },
            application: {
                entry_point: "src/main.rs",
                runtime: "http_server",
            },
            scaling: {
                min_instances: 1,
                max_instances: 5,
            },
            network: {
                domains: ["example.com"],
                regions: ["us-east-1"],
            },
            environment: {
                NODE_ENV: "production",
            },
        }
        "#;

        let parser = DeployConfigParser::new();
        let config = parser.parse_string(content).unwrap();

        assert_eq!(config.metadata.name, "test-app");
        assert_eq!(config.metadata.version, "1.0.0");
        assert_eq!(config.application.entry_point, "src/main.rs");
        assert_eq!(config.scaling.min_instances, 1);
        assert_eq!(config.scaling.max_instances, 5);
        assert_eq!(config.network.domains.len(), 1);
        assert_eq!(config.network.regions.len(), 1);
        assert_eq!(config.environment.get("NODE_ENV"), Some(&"production".to_string()));
    }

    #[test]
    fn test_parse_with_comments() {
        let content = r#"
// This is a comment
{
    metadata: {
        name: "test-app", // inline comment
        version: "1.0.0",
    },
    /* multi-line
       comment */
    application: {
        entry_point: "main.rs",
    },
}
        "#;

        let parser = DeployConfigParser::new();
        let config = parser.parse_string(content).unwrap();

        assert_eq!(config.metadata.name, "test-app");
        assert_eq!(config.application.entry_point, "main.rs");
    }

    #[test]
    fn test_invalid_syntax() {
        let content = "invalid syntax";

        let parser = DeployConfigParser::new();
        assert!(parser.parse_string(content).is_err());
    }

    #[test]
    fn test_find_deploy_config() {
        // テスト用の設定ファイルを作成
        let mut temp_file = NamedTempFile::with_suffix(".kotoba-deploy").unwrap();
        let content = r#"
        {
            metadata: { name: "test-app" },
            application: { entry_point: "main.rs" },
        }
        "#;
        temp_file.write_all(content.as_bytes()).unwrap();

        let temp_dir = temp_file.path().parent().unwrap();
        let config = find_and_parse_deploy_config(temp_dir).unwrap().unwrap();

        assert_eq!(config.metadata.name, "test-app");
    }
}
