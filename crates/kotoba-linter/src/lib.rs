//! Kotoba Code Linter
//!
//! Denoの `deno lint` に似た使い勝手で、.kotoba ファイルの
//! 静的解析と品質チェックを行います。
//!
//! ## 使用方法
//!
//! ```bash
//! # ファイルのリンター実行
//! kotoba lint file.kotoba
//!
//! # ディレクトリ内の全ファイルをチェック
//! kotoba lint .
//!
//! # JSON形式で出力
//! kotoba lint --format json file.kotoba
//!
//! # 特定のルールを無効化
//! kotoba lint --rules "no-unused-vars,no-shadowing" file.kotoba
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::Mutex;

pub mod config;
pub mod rules;
pub mod diagnostics;
pub mod analyzer;
pub mod reporter;

/// 診断結果のレベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticLevel {
    /// エラー（プログラムの実行を妨げる）
    Error,
    /// 警告（潜在的な問題）
    Warning,
    /// 情報（改善提案）
    Info,
    /// ヒント（スタイルの提案）
    Hint,
}

impl std::fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticLevel::Error => write!(f, "error"),
            DiagnosticLevel::Warning => write!(f, "warning"),
            DiagnosticLevel::Info => write!(f, "info"),
            DiagnosticLevel::Hint => write!(f, "hint"),
        }
    }
}

/// 診断情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// 診断レベル
    pub level: DiagnosticLevel,
    /// 診断コード
    pub code: String,
    /// メッセージ
    pub message: String,
    /// ファイルパス
    pub file_path: PathBuf,
    /// 行番号（1-based）
    pub line: usize,
    /// 列番号（1-based）
    pub column: usize,
    /// 行の長さ
    pub length: usize,
    /// 修正提案（オプション）
    pub suggestion: Option<String>,
    /// 追加のヘルプ情報
    pub help: Option<String>,
}

impl Diagnostic {
    /// 新しい診断を作成
    pub fn new(
        level: DiagnosticLevel,
        code: String,
        message: String,
        file_path: PathBuf,
        line: usize,
        column: usize,
        length: usize,
    ) -> Self {
        Self {
            level,
            code,
            message,
            file_path,
            line,
            column,
            length,
            suggestion: None,
            help: None,
        }
    }

    /// 修正提案を追加
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    /// ヘルプ情報を追加
    pub fn with_help(mut self, help: String) -> Self {
        self.help = Some(help);
        self
    }
}

/// リンター設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinterConfig {
    /// 有効なルール
    pub enabled_rules: Vec<String>,
    /// 無効化されたルール
    pub disabled_rules: Vec<String>,
    /// ルールごとの設定
    pub rule_config: HashMap<String, serde_json::Value>,
    /// 除外ファイルパターン
    pub exclude_patterns: Vec<String>,
    /// 出力フォーマット
    pub output_format: OutputFormat,
}

impl Default for LinterConfig {
    fn default() -> Self {
        Self {
            enabled_rules: vec![
                "no-unused-vars".to_string(),
                "no-shadowing".to_string(),
                "consistent-indentation".to_string(),
                "trailing-whitespace".to_string(),
                "missing-semicolons".to_string(),
                "naming-convention".to_string(),
                "complexity".to_string(),
            ],
            disabled_rules: vec![],
            rule_config: HashMap::new(),
            exclude_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
            ],
            output_format: OutputFormat::Pretty,
        }
    }
}

/// 出力フォーマット
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    /// 人間に読みやすい形式
    Pretty,
    /// JSON形式
    Json,
    /// コンパクト形式
    Compact,
}

/// リンター実行結果
#[derive(Debug, Clone)]
pub struct LintResult {
    /// ファイルパス
    pub file_path: PathBuf,
    /// 検出された診断
    pub diagnostics: Vec<Diagnostic>,
    /// エラー数
    pub error_count: usize,
    /// 警告数
    pub warning_count: usize,
    /// 処理時間（ミリ秒）
    pub duration_ms: u64,
}

impl LintResult {
    /// 新しい結果を作成
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            diagnostics: Vec::new(),
            error_count: 0,
            warning_count: 0,
            duration_ms: 0,
        }
    }

    /// 診断を追加
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        match diagnostic.level {
            DiagnosticLevel::Error => self.error_count += 1,
            DiagnosticLevel::Warning => self.warning_count += 1,
            _ => {}
        }
        self.diagnostics.push(diagnostic);
    }

    /// 診断があるかどうか
    pub fn has_diagnostics(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    /// エラーがあるかどうか
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }
}

/// メインのリンター構造体
#[derive(Debug)]
pub struct Linter {
    config: LinterConfig,
    rules: Vec<Box<dyn rules::LintRule>>,
}

impl Linter {
    /// 新しいリンターを作成
    pub fn new(config: LinterConfig) -> Self {
        let mut rules = Vec::new();

        // 有効なルールを追加
        for rule_name in &config.enabled_rules {
            if let Some(rule) = Self::create_rule(rule_name, &config) {
                rules.push(rule);
            }
        }

        Self { config, rules }
    }

    /// デフォルト設定でリンターを作成
    pub fn default() -> Self {
        Self::new(LinterConfig::default())
    }

    /// 設定ファイルからリンターを作成
    pub async fn from_config_file() -> Result<Self, Box<dyn std::error::Error>> {
        let config = config::load_config().await?;
        Ok(Self::new(config))
    }

    /// ルールを作成
    fn create_rule(rule_name: &str, config: &LinterConfig) -> Option<Box<dyn rules::LintRule>> {
        let rule_config = config.rule_config.get(rule_name).cloned();

        match rule_name {
            "no-unused-vars" => Some(Box::new(rules::NoUnusedVarsRule::new(rule_config))),
            "no-shadowing" => Some(Box::new(rules::NoShadowingRule::new(rule_config))),
            "consistent-indentation" => Some(Box::new(rules::ConsistentIndentationRule::new(rule_config))),
            "trailing-whitespace" => Some(Box::new(rules::TrailingWhitespaceRule::new(rule_config))),
            "missing-semicolons" => Some(Box::new(rules::MissingSemicolonsRule::new(rule_config))),
            "naming-convention" => Some(Box::new(rules::NamingConventionRule::new(rule_config))),
            "complexity" => Some(Box::new(rules::ComplexityRule::new(rule_config))),
            _ => None,
        }
    }

    /// 単一ファイルをチェック
    pub async fn lint_file(&self, file_path: &PathBuf) -> Result<LintResult, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();

        let content = tokio::fs::read_to_string(file_path).await?;
        let mut result = LintResult::new(file_path.clone());

        // 各ルールでチェック
        for rule in &self.rules {
            let diagnostics = rule.check(&content, file_path)?;
            for diagnostic in diagnostics {
                result.add_diagnostic(diagnostic);
            }
        }

        result.duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(result)
    }

    /// 複数のファイルをチェック
    pub async fn lint_files(&self, files: Vec<PathBuf>) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for file in files {
            let result = self.lint_file(&file).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// ディレクトリ内のファイルをチェック
    pub async fn lint_directory(&self, dir: PathBuf) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let mut files = Vec::new();
        find_kotoba_files(dir, &mut files).await?;
        self.lint_files(files).await
    }
}

/// .kotoba ファイルを再帰的に検索
async fn find_kotoba_files(dir: PathBuf, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let mut entries = tokio::fs::read_dir(&dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_dir() {
            // 除外パターンをチェック
            if !path.ends_with("node_modules") && !path.ends_with(".git") && !path.ends_with("target") {
                Box::pin(find_kotoba_files(path, files)).await?;
            }
        } else if path.extension().map_or(false, |ext| ext == "kotoba") {
            files.push(path);
        }
    }

    Ok(())
}

/// 便利関数
pub async fn lint_files(files: Vec<PathBuf>) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
    let linter = Linter::default();
    linter.lint_files(files).await
}

pub async fn lint_directory(dir: PathBuf) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
    let linter = Linter::default();
    linter.lint_directory(dir).await
}

/// 統計情報
pub fn print_stats(results: &[LintResult]) {
    let total_files = results.len();
    let total_errors = results.iter().map(|r| r.error_count).sum::<usize>();
    let total_warnings = results.iter().map(|r| r.warning_count).sum::<usize>();
    let total_issues = total_errors + total_warnings;

    println!("Linting complete:");
    println!("  Files checked: {}", total_files);
    println!("  Total issues: {}", total_issues);
    println!("  Errors: {}", total_errors);
    println!("  Warnings: {}", total_warnings);

    if total_issues > 0 {
        println!("\nFiles with issues:");
        for result in results.iter().filter(|r| r.has_diagnostics()) {
            println!("  {}: {} errors, {} warnings",
                result.file_path.display(),
                result.error_count,
                result.warning_count
            );
        }
    }
}

// 各モジュールの再エクスポート
pub use config::*;
pub use rules::*;
pub use diagnostics::*;
pub use analyzer::*;
pub use reporter::*;