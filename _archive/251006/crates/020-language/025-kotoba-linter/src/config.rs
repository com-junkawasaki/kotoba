//! リンター設定管理モジュール

use super::LinterConfig;
use std::path::PathBuf;
use tokio::fs;

/// 設定ファイルの名前
const CONFIG_FILE_NAMES: &[&str] = &[
    "kotoba-lint.toml",
    ".kotoba-lint.toml",
    "linter.toml",
];

/// 設定を読み込む
pub async fn load_config() -> Result<LinterConfig, Box<dyn std::error::Error>> {
    // カレントディレクトリから設定ファイルを検索
    for file_name in CONFIG_FILE_NAMES {
        let path = PathBuf::from(file_name);
        if path.exists() {
            return load_config_from_file(&path).await;
        }
    }

    // ホームディレクトリをチェック
    if let Some(home_dir) = dirs::home_dir() {
        let config_path = home_dir.join(".config").join("kotoba").join("linter.toml");
        if config_path.exists() {
            return load_config_from_file(&config_path).await;
        }
    }

    // デフォルト設定を使用
    Ok(LinterConfig::default())
}

/// ファイルから設定を読み込む
pub async fn load_config_from_file(path: &PathBuf) -> Result<LinterConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path).await?;
    let config: LinterConfig = toml::from_str(&content)?;
    Ok(config)
}

/// 設定をファイルに保存
pub async fn save_config(config: &LinterConfig, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // ディレクトリが存在することを確認
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let content = toml::to_string_pretty(config)?;
    fs::write(path, content).await?;
    Ok(())
}

/// プロジェクト固有の設定を取得
pub async fn load_project_config(project_root: &PathBuf) -> Result<LinterConfig, Box<dyn std::error::Error>> {
    for file_name in CONFIG_FILE_NAMES {
        let path = project_root.join(file_name);
        if path.exists() {
            return load_config_from_file(&path).await;
        }
    }

    // プロジェクト固有の設定がない場合はグローバル設定を使用
    load_config().await
}

/// 設定ファイルを検索
pub async fn find_config_file(project_root: &PathBuf) -> Option<PathBuf> {
    for file_name in CONFIG_FILE_NAMES {
        let path = project_root.join(file_name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// デフォルト設定を生成
pub fn create_default_config() -> LinterConfig {
    LinterConfig::default()
}

/// 設定の例を表示
pub fn print_config_example() {
    let config = LinterConfig::default();
    let toml_content = toml::to_string_pretty(&config).unwrap_or_else(|_| {
        r#"# Kotoba Linter Configuration

[linter]
# 有効なルール
enabled_rules = [
    "no-unused-vars",
    "no-shadowing",
    "consistent-indentation",
    "trailing-whitespace",
    "missing-semicolons",
    "naming-convention",
    "complexity"
]

# 無効化されたルール
disabled_rules = []

# 除外ファイルパターン
exclude_patterns = [
    "node_modules",
    ".git",
    "target"
]

# 出力フォーマット: "pretty", "json", "compact"
output_format = "pretty"

# ルールごとの設定
[rule_config."complexity"]
max_lines = 30

[rule_config."naming-convention"]
style = "camelCase"
"#.to_string()
    });

    println!("Example kotoba-lint.toml configuration:");
    println!("{}", toml_content);
}

/// 設定の検証
pub fn validate_config(config: &LinterConfig) -> Result<(), Box<dyn std::error::Error>> {
    // 有効なルール名をチェック
    let valid_rules = vec![
        "no-unused-vars",
        "no-shadowing",
        "consistent-indentation",
        "trailing-whitespace",
        "missing-semicolons",
        "naming-convention",
        "complexity"
    ];

    for rule in &config.enabled_rules {
        if !valid_rules.contains(&rule.as_str()) {
            return Err(format!("Unknown rule: {}", rule).into());
        }
    }

    for rule in &config.disabled_rules {
        if !valid_rules.contains(&rule.as_str()) {
            return Err(format!("Unknown rule: {}", rule).into());
        }
    }

    // 重複チェック
    let mut all_rules = config.enabled_rules.clone();
    all_rules.extend(config.disabled_rules.clone());

    let mut seen = std::collections::HashSet::new();
    for rule in &all_rules {
        if !seen.insert(rule.clone()) {
            return Err(format!("Duplicate rule: {}", rule).into());
        }
    }

    Ok(())
}

/// 設定をマージ（プロジェクト設定 + ユーザー設定）
pub fn merge_configs(project_config: LinterConfig, user_config: LinterConfig) -> LinterConfig {
    LinterConfig {
        enabled_rules: if project_config.enabled_rules.is_empty() {
            user_config.enabled_rules
        } else {
            project_config.enabled_rules
        },
        disabled_rules: {
            let mut disabled = project_config.disabled_rules;
            disabled.extend(user_config.disabled_rules);
            disabled
        },
        rule_config: {
            let mut config = user_config.rule_config;
            config.extend(project_config.rule_config);
            config
        },
        exclude_patterns: {
            let mut patterns = project_config.exclude_patterns;
            patterns.extend(user_config.exclude_patterns);
            patterns
        },
        output_format: project_config.output_format,
    }
}
