//! ログ管理
//!
//! Merkle DAG: cli_interface -> LogFormatter component

use tracing::{Level, Subscriber};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// ログシステムを初期化
pub fn init_logging(level: &str) -> Result<(), Box<dyn std::error::Error>> {
    let level = parse_log_level(level)?;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::from_default_env()
                .add_directive(format!("kotoba={}", level).parse().unwrap())
        });

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .compact()
        );

    subscriber.init();

    Ok(())
}

/// ログレベルをパース
fn parse_log_level(level: &str) -> Result<Level, Box<dyn std::error::Error>> {
    match level.to_lowercase().as_str() {
        "error" => Ok(Level::ERROR),
        "warn" => Ok(Level::WARN),
        "info" => Ok(Level::INFO),
        "debug" => Ok(Level::DEBUG),
        "trace" => Ok(Level::TRACE),
        _ => Err(format!("Invalid log level: {}", level).into()),
    }
}

/// ログレベルを文字列に変換
pub fn level_to_string(level: Level) -> &'static str {
    match level {
        Level::ERROR => "error",
        Level::WARN => "warn",
        Level::INFO => "info",
        Level::DEBUG => "debug",
        Level::TRACE => "trace",
    }
}

/// ログメッセージのフォーマッタ
/// Merkle DAG: cli_interface -> LogFormatter component
pub struct LogFormatter;

impl LogFormatter {
    /// 成功メッセージをフォーマット
    pub fn success(message: &str) -> String {
        format!("✅ {}", message)
    }

    /// エラーメッセージをフォーマット
    pub fn error(message: &str) -> String {
        format!("❌ {}", message)
    }

    /// 警告メッセージをフォーマット
    pub fn warning(message: &str) -> String {
        format!("⚠️  {}", message)
    }

    /// 情報メッセージをフォーマット
    pub fn info(message: &str) -> String {
        format!("ℹ️  {}", message)
    }

    /// デバッグメッセージをフォーマット
    pub fn debug(message: &str) -> String {
        format!("🔍 {}", message)
    }

    /// 処理中メッセージをフォーマット
    pub fn processing(message: &str) -> String {
        format!("⏳ {}", message)
    }

    /// 完了メッセージをフォーマット
    pub fn completed(message: &str) -> String {
        format!("✅ {}", message)
    }
}


/// ログレベルの設定を変更
pub fn set_log_level(level: &str) -> Result<(), Box<dyn std::error::Error>> {
    let level = parse_log_level(level)?;

    // 既存のフィルタを更新
    let filter = EnvFilter::from_default_env()
        .add_directive(format!("kotoba={}", level).parse().unwrap());

    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
    )?;

    Ok(())
}

/// ログをファイルに出力する設定
pub fn setup_file_logging(log_file: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::OpenOptions;
    use tracing_subscriber::fmt::format::Writer;

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(move || file.try_clone().unwrap())
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true);

    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(EnvFilter::from_default_env())
            .with(file_layer)
    )?;

    Ok(())
}
