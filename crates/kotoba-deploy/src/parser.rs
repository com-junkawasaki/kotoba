//! デプロイ設定ファイルパーサー
//!
//! kotoba-kotobas を使用して .kotoba-deploy ファイルをパースします。

use kotoba_core::types::{Result, Value, ContentHash};
use kotoba_errors::KotobaError;
use crate::config::{DeployConfig, DeployConfigBuilder};
// use kotoba_kotobanet::DeployParser as KotobaNetDeployParser; // Commented out due to stability issues
use std::path::Path;
use std::fs;

/// デプロイ設定パーサー
///
/// kotoba-kotobas::DeployParser を使用してデプロイ設定をパースします。
pub struct DeployConfigParser;

impl DeployConfigParser {
    /// 新しいパーサーを作成
    pub fn new() -> Self {
        Self
    }

    /// 設定ファイルをパース
    pub fn parse<P: AsRef<Path>>(&self, path: P) -> Result<DeployConfig> {
        // kotoba-kotobas の DeployParser を使用 (コメントアウト due to stability issues)
        // let deploy_config = KotobaNetDeployParser::parse_file(path)
        //     .map_err(|e| KotobaError::InvalidArgument(
        //         format!("Deploy config parsing failed: {}", e)
        //     ))?;

        // kotoba-kotobas::DeployConfig を Kotoba の DeployConfig に変換
        // Self::convert_from_kotobanet_config(deploy_config)
        todo!("Implement deploy config parsing without kotoba-kotobas")
    }

    /// JSON文字列から設定をパース
    pub fn parse_string(&self, content: &str) -> Result<DeployConfig> {
        // kotoba-kotobas の DeployParser を使用 (コメントアウト due to stability issues)
        // let deploy_config = KotobaNetDeployParser::parse(content)
        //     .map_err(|e| KotobaError::InvalidArgument(
        //         format!("Deploy config parsing failed: {}", e)
        //     ))?;

        // kotoba-kotobas::DeployConfig を Kotoba の DeployConfig に変換
        // Self::convert_from_kotobanet_config(deploy_config)
        todo!("Implement deploy config parsing without kotoba-kotobas")
    }

    // Commented out due to kotoba-kotobas stability issues
    // fn convert_from_kotobanet_config(kotobanet_config: kotoba_kotobanet::DeployConfig) -> Result<DeployConfig> {
    //     // 基本的な変換を行う（必要に応じて拡張）
    //     let mut builder = DeployConfigBuilder::new(
    //         kotobanet_config.name,
    //         "main.rs".to_string(), // デフォルトのエントリーポイント
    //     ).version(kotobanet_config.version);
    //
    //     // スケーリング設定
    //     builder = builder.scaling(
    //         kotobanet_config.scaling.min_instances,
    //         kotobanet_config.scaling.max_instances,
    //     );
    //
    //     // リージョン設定
    //     for region in kotobanet_config.regions {
    //         builder = builder.add_region(region.name);
    //     }
    //
    //     // 環境変数（必要に応じて拡張）
    //     // TODO: kotobanet_config から環境変数を抽出
    //
    //     let mut config = builder.build();
    //
    //     // 設定の検証
    //     config.validate()?;
    //
    //     // ハッシュの計算
    //     config.metadata.config_hash = Some(config.calculate_hash()?);
    //
    //     Ok(config)
    // }
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
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_simple_deploy_config() {
        let content = r#"
        {
            name: "test-app",
            version: "1.0.0",
            environment: "production",
            scaling: {
                minInstances: 1,
                maxInstances: 5,
            },
            regions: [
                {
                    name: "us-east-1",
                    provider: "AWS",
                    instanceType: "t3.medium",
                }
            ]
        }
        "#;

        let parser = DeployConfigParser::new();
        let config = parser.parse_string(content).unwrap();

        assert_eq!(config.metadata.name, "test-app");
        assert_eq!(config.metadata.version, "1.0.0");
        assert_eq!(config.scaling.min_instances, 1);
        assert_eq!(config.scaling.max_instances, 5);
        assert_eq!(config.network.regions.len(), 1);
    }

    #[test]
    fn test_find_deploy_config() {
        // テスト用の設定ファイルを作成
        let mut temp_file = NamedTempFile::with_suffix(".kotoba-deploy").unwrap();
        let content = r#"
        {
            name: "test-app",
            version: "1.0.0",
            scaling: {
                minInstances: 1,
                maxInstances: 3,
            }
        }
        "#;
        std::io::Write::write_all(&mut temp_file, content.as_bytes()).unwrap();

        let temp_dir = temp_file.path().parent().unwrap();
        let config = find_and_parse_deploy_config(temp_dir).unwrap().unwrap();

        assert_eq!(config.metadata.name, "test-app");
        assert_eq!(config.scaling.min_instances, 1);
    }
}
