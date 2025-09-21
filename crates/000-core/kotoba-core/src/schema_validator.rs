//! JSON Schemaバリデーション機能
//! プロジェクトの公式JSON Schemaによるデータ検証

use crate::schema::*;
use crate::types::Result;
use kotoba_errors::KotobaError;
use jsonschema::JSONSchema;
use std::fs;
use std::path::Path;

/// JSON Schemaバリデーター
#[derive(Debug)]
pub struct SchemaValidator {
    schema: JSONSchema,
    schema_text: String,
}

impl SchemaValidator {
    /// 公式JSON Schemaからバリデーターを作成
    pub fn new() -> Result<Self> {
        let schema_path = Path::new("schemas/process-network-schema.json");
        let schema_text = fs::read_to_string(schema_path)
            .map_err(|e| KotobaError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Failed to read schema file: {}", e)
            )))?;

        let schema_value: serde_json::Value = serde_json::from_str(&schema_text)
            .map_err(|e| KotobaError::Parse(format!("Invalid schema JSON: {}", e)))?;

        let schema = JSONSchema::compile(&schema_value)
            .map_err(|e| KotobaError::Validation(format!("Schema compilation failed: {:?}", e)))?;

        Ok(Self {
            schema,
            schema_text,
        })
    }

    /// データをJSON Schemaで検証
    pub fn validate<T: serde::Serialize>(&self, data: &T) -> Result<()> {
        let json_value = serde_json::to_value(data)
            .map_err(|e| KotobaError::Parse(format!("Data serialization failed: {}", e)))?;

        let validation_result = self.schema.validate(&json_value);

        if let Err(errors) = validation_result {
            let error_messages: Vec<String> = errors
                .map(|error| format!("{}", error))
                .collect();

            return Err(KotobaError::Validation(format!(
                "Schema validation failed:\n{}",
                error_messages.join("\n")
            )));
        }

        Ok(())
    }

    /// ProcessNetworkデータを検証
    pub fn validate_process_network(&self, network: &ProcessNetwork) -> Result<()> {
        self.validate(network)
    }

    /// GraphInstanceデータを検証
    pub fn validate_graph_instance(&self, graph: &GraphInstance) -> Result<()> {
        self.validate(graph)
    }

    /// RuleDPOデータを検証
    pub fn validate_rule_dpo(&self, rule: &RuleDPO) -> Result<()> {
        self.validate(rule)
    }

    /// バリデーションエラーを詳細にレポート
    pub fn validate_with_detailed_report<T: serde::Serialize>(&self, data: &T) -> ValidationReport {
        let json_value = match serde_json::to_value(data) {
            Ok(v) => v,
            Err(e) => return ValidationReport {
                is_valid: false,
                errors: vec![format!("Data serialization failed: {}", e)],
                warnings: vec![],
            },
        };

        let validation_result = self.schema.validate(&json_value);

        match validation_result {
            Ok(_) => ValidationReport {
                is_valid: true,
                errors: vec![],
                warnings: vec![],
            },
            Err(errors) => {
                let error_messages: Vec<String> = errors
                    .map(|error| format!("{}", error))
                    .collect();

                ValidationReport {
                    is_valid: false,
                    errors: error_messages,
                    warnings: vec![],
                }
            }
        }
    }

    /// スキーマテキストを取得
    pub fn schema_text(&self) -> &str {
        &self.schema_text
    }

    /// スキーマのバージョンを取得
    pub fn get_schema_version(&self) -> Result<String> {
        let schema_value: serde_json::Value = serde_json::from_str(&self.schema_text)
            .map_err(|e| KotobaError::Parse(format!("Schema parse error: {}", e)))?;

        if let Some(version) = schema_value.get("version") {
            if let Some(version_str) = version.as_str() {
                Ok(version_str.to_string())
            } else {
                Ok("unknown".to_string())
            }
        } else {
            Ok("0.1.0".to_string())
        }
    }
}

/// バリデーション結果レポート
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    /// エラーレポートを文字列として取得
    pub fn error_report(&self) -> String {
        if self.errors.is_empty() {
            "No errors found".to_string()
        } else {
            format!("Validation Errors:\n{}", self.errors.join("\n"))
        }
    }

    /// 警告レポートを文字列として取得
    pub fn warning_report(&self) -> String {
        if self.warnings.is_empty() {
            "No warnings found".to_string()
        } else {
            format!("Validation Warnings:\n{}", self.warnings.join("\n"))
        }
    }

    /// 完全なレポートを文字列として取得
    pub fn full_report(&self) -> String {
        let mut report = String::new();

        if self.is_valid {
            report.push_str("✅ Validation passed\n");
        } else {
            report.push_str("❌ Validation failed\n");
        }

        if !self.errors.is_empty() {
            report.push_str(&format!("\nErrors:\n{}\n", self.errors.join("\n")));
        }

        if !self.warnings.is_empty() {
            report.push_str(&format!("\nWarnings:\n{}\n", self.warnings.join("\n")));
        }

        report
    }
}

/// スキーマ検証ユーティリティ関数
pub mod utils {
    use super::*;

    /// ファイルからJSONを読み込んで検証
    pub fn validate_json_file<P: AsRef<Path>>(file_path: P, validator: &SchemaValidator) -> Result<ValidationReport> {
        let json_text = fs::read_to_string(file_path)
            .map_err(|e| KotobaError::Io(e))?;

        let json_value: serde_json::Value = serde_json::from_str(&json_text)
            .map_err(|e| KotobaError::Parse(format!("JSON parse error: {}", e)))?;

        // ProcessNetworkとして検証を試みる
        if let Ok(process_network) = serde_json::from_value::<ProcessNetwork>(json_value.clone()) {
            let report = validator.validate_with_detailed_report(&process_network);
            Ok(report)
        } else {
            // 個別の構造体として検証を試みる
            let report = ValidationReport {
                is_valid: false,
                errors: vec!["Data does not match ProcessNetwork schema".to_string()],
                warnings: vec!["Try validating individual components".to_string()],
            };
            Ok(report)
        }
    }

    /// ディレクトリ内のすべてのJSONファイルを検証
    pub fn validate_json_directory<P: AsRef<Path>>(dir_path: P, validator: &SchemaValidator) -> Result<Vec<(String, ValidationReport)>> {
        let dir_path = dir_path.as_ref();
        let mut results = Vec::new();

        if !dir_path.exists() || !dir_path.is_dir() {
            return Err(KotobaError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Directory not found"
            )));
        }

        for entry in fs::read_dir(dir_path)
            .map_err(|e| KotobaError::Io(e))?
        {
            let entry = entry.map_err(|e| KotobaError::Io(e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    match validate_json_file(&path, validator) {
                        Ok(report) => results.push((file_name.to_string(), report)),
                        Err(e) => results.push((file_name.to_string(), ValidationReport {
                            is_valid: false,
                            errors: vec![format!("File read error: {}", e)],
                            warnings: vec![],
                        })),
                    }
                }
            }
        }

        Ok(results)
    }

    /// スキーマの互換性をチェック
    pub fn check_schema_compatibility(validator: &SchemaValidator, data: &serde_json::Value) -> CompatibilityReport {
        let validation_report = ValidationReport {
            is_valid: validator.schema.validate(data).is_ok(),
            errors: vec![],
            warnings: vec![],
        };

        // スキーマバージョンチェック
        let schema_version = validator.get_schema_version().unwrap_or_else(|_| "unknown".to_string());

        CompatibilityReport {
            is_compatible: validation_report.is_valid,
            schema_version,
            validation_report,
        }
    }
}

/// スキーマ互換性レポート
#[derive(Debug)]
pub struct CompatibilityReport {
    pub is_compatible: bool,
    pub schema_version: String,
    pub validation_report: ValidationReport,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use crate::types::Cid;

    #[test]
    fn test_schema_validator_creation() {
        let validator = SchemaValidator::new();
        assert!(validator.is_ok());
    }

    #[test]
    fn test_process_network_validation() {
        let validator = SchemaValidator::new().unwrap();

        // 有効なProcessNetworkデータ
        let process_network = ProcessNetwork {
            meta: Some(MetaInfo {
                model: "GTS-DPO-OpenGraph-Merkle".to_string(),
                version: "0.2.0".to_string(),
                cid_algo: None,
            }),
            type_graph: GraphType {
                core: GraphCore {
                    nodes: vec![],
                    edges: vec![],
                    boundary: None,
                    attrs: None,
                },
                kind: GraphKind::Graph,
                cid: Cid::from_hex("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap(),
                typing: None,
            },
            graphs: vec![],
            components: vec![],
            rules: vec![],
            strategies: vec![],
            queries: vec![],
            pg_view: None,
        };

        let result = validator.validate_process_network(&process_network);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_report() {
        let report = ValidationReport {
            is_valid: false,
            errors: vec!["Test error".to_string()],
            warnings: vec!["Test warning".to_string()],
        };

        let full_report = report.full_report();
        assert!(full_report.contains("❌ Validation failed"));
        assert!(full_report.contains("Test error"));
        assert!(full_report.contains("Test warning"));
    }
}
