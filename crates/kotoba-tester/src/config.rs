//! テストランナー設定管理モジュール

use super::TestConfig;
use std::path::PathBuf;
use tokio::fs;

/// 設定ファイルの名前
const CONFIG_FILE_NAMES: &[&str] = &[
    "kotoba-test.toml",
    ".kotoba-test.toml",
    "test.toml",
];

/// 設定を読み込む
pub async fn load_config() -> Result<TestConfig, Box<dyn std::error::Error>> {
    // カレントディレクトリから設定ファイルを検索
    for file_name in CONFIG_FILE_NAMES {
        let path = PathBuf::from(file_name);
        if path.exists() {
            return load_config_from_file(&path).await;
        }
    }

    // ホームディレクトリをチェック
    if let Some(home_dir) = dirs::home_dir() {
        let config_path = home_dir.join(".config").join("kotoba").join("test.toml");
        if config_path.exists() {
            return load_config_from_file(&config_path).await;
        }
    }

    // デフォルト設定を使用
    Ok(TestConfig::default())
}

/// ファイルから設定を読み込む
pub async fn load_config_from_file(path: &PathBuf) -> Result<TestConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path).await?;
    let config: TestConfig = toml::from_str(&content)?;
    Ok(config)
}

/// 設定をファイルに保存
pub async fn save_config(config: &TestConfig, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // ディレクトリが存在することを確認
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let content = toml::to_string_pretty(config)?;
    fs::write(path, content).await?;
    Ok(())
}

/// プロジェクト固有の設定を取得
pub async fn load_project_config(project_root: &PathBuf) -> Result<TestConfig, Box<dyn std::error::Error>> {
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
pub fn create_default_config() -> TestConfig {
    TestConfig::default()
}

/// 設定の例を表示
pub fn print_config_example() {
    let config = TestConfig::default();
    let toml_content = toml::to_string_pretty(&config).unwrap_or_else(|_| {
        r#"# Kotoba Test Runner Configuration

[test]
# タイムアウト時間（秒）
timeout = 30

# 並列実行数
concurrency = 4

# フィルター
filter = "user"

# 詳細出力
verbose = false

# カバレッジ収集
coverage = false

# 除外ファイルパターン
exclude_patterns = [
    "node_modules",
    ".git",
    "target",
    "build"
]
"#.to_string()
    });

    println!("Example kotoba-test.toml configuration:");
    println!("{}", toml_content);
}

/// 設定の検証
pub fn validate_config(config: &TestConfig) -> Result<(), Box<dyn std::error::Error>> {
    if config.timeout == 0 {
        return Err("timeout must be greater than 0".into());
    }

    if config.concurrency == 0 {
        return Err("concurrency must be greater than 0".into());
    }

    Ok(())
}

/// 設定をマージ（プロジェクト設定 + ユーザー設定）
pub fn merge_configs(project_config: TestConfig, user_config: TestConfig) -> TestConfig {
    TestConfig {
        timeout: if project_config.timeout != 30 { project_config.timeout } else { user_config.timeout },
        concurrency: if project_config.concurrency != num_cpus::get() { project_config.concurrency } else { user_config.concurrency },
        filter: project_config.filter.or(user_config.filter),
        verbose: project_config.verbose || user_config.verbose,
        coverage: project_config.coverage || user_config.coverage,
        exclude_patterns: {
            let mut patterns = project_config.exclude_patterns;
            patterns.extend(user_config.exclude_patterns);
            patterns
        },
    }
}
