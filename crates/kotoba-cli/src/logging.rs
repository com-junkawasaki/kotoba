//! ログ管理

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

/// プログレスバー
pub struct ProgressBar {
    total: usize,
    current: usize,
    message: String,
}

impl ProgressBar {
    /// 新しいプログレスバーを作成
    pub fn new(total: usize, message: String) -> Self {
        Self {
            total,
            current: 0,
            message,
        }
    }

    /// 進捗を更新
    pub fn update(&mut self, current: usize) {
        self.current = current;
        self.display();
    }

    /// 進捗をインクリメント
    pub fn increment(&mut self) {
        self.current += 1;
        self.display();
    }

    /// プログレスバーを表示
    fn display(&self) {
        let percentage = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as usize
        } else {
            100
        };

        let width = 20;
        let filled = (percentage * width) / 100;
        let empty = width - filled;

        let bar = "█".repeat(filled) + &"░".repeat(empty);

        print!("\r{} [{}] {}/{} ({}%)", self.message, bar, self.current, self.total, percentage);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    }

    /// 完了
    pub fn finish(&self) {
        println!("\r{} [{}] {}/{} (100%) ✅", self.message, "█".repeat(20), self.total, self.total);
    }

    /// 完了（エラー）
    pub fn finish_with_error(&self, error: &str) {
        println!("\r{} ❌ {}", self.message, error);
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
        .with_thread_names(true)
        .json();

    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(EnvFilter::from_default_env())
            .with(file_layer)
    )?;

    Ok(())
}
