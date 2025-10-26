//! Kotoba Code Linter
//!
//! Denoの `deno lint` に似た使い勝手で、.kotoba ファイルの
//! 静的解析と品質チェックを行います。
//!
//! ## Pure Kernel & Effects Shell Architecture
//!
//! This crate follows the Pure Kernel/Effects Shell pattern:
//!
//! - **Pure Kernel**: `PureLinter` - performs deterministic code linting without side effects
//! - **Effects Shell**: `Linter` - wraps the pure linter and handles file I/O
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
pub mod pure_linter;

/// 診断結果のレベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize)]
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
pub async fn find_kotoba_files(dir: PathBuf, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
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
pub use pure_linter::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::collections::HashMap;

    #[test]
    fn test_diagnostic_level_display() {
        assert_eq!(format!("{}", DiagnosticLevel::Error), "error");
        assert_eq!(format!("{}", DiagnosticLevel::Warning), "warning");
        assert_eq!(format!("{}", DiagnosticLevel::Info), "info");
        assert_eq!(format!("{}", DiagnosticLevel::Hint), "hint");
    }

    #[test]
    fn test_diagnostic_level_debug() {
        let error = DiagnosticLevel::Error;
        let warning = DiagnosticLevel::Warning;
        let info = DiagnosticLevel::Info;
        let hint = DiagnosticLevel::Hint;

        let error_debug = format!("{:?}", error);
        let warning_debug = format!("{:?}", warning);
        let info_debug = format!("{:?}", info);
        let hint_debug = format!("{:?}", hint);

        assert!(error_debug.contains("Error"));
        assert!(warning_debug.contains("Warning"));
        assert!(info_debug.contains("Info"));
        assert!(hint_debug.contains("Hint"));
    }

    #[test]
    fn test_diagnostic_level_partial_eq() {
        assert_eq!(DiagnosticLevel::Error, DiagnosticLevel::Error);
        assert_ne!(DiagnosticLevel::Error, DiagnosticLevel::Warning);
        assert_ne!(DiagnosticLevel::Warning, DiagnosticLevel::Info);
        assert_ne!(DiagnosticLevel::Info, DiagnosticLevel::Hint);
    }

    #[test]
    fn test_diagnostic_level_hash() {
        use std::collections::HashSet;

        let mut levels = HashSet::new();
        levels.insert(DiagnosticLevel::Error);
        levels.insert(DiagnosticLevel::Warning);
        levels.insert(DiagnosticLevel::Info);
        levels.insert(DiagnosticLevel::Hint);

        assert_eq!(levels.len(), 4);

        // Inserting the same level should not increase the count
        levels.insert(DiagnosticLevel::Error);
        assert_eq!(levels.len(), 4);
    }

    #[test]
    fn test_diagnostic_creation() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let diagnostic = Diagnostic::new(
            DiagnosticLevel::Error,
            "E001".to_string(),
            "Test error message".to_string(),
            file_path.clone(),
            10,
            5,
            8,
        );

        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert_eq!(diagnostic.code, "E001");
        assert_eq!(diagnostic.message, "Test error message");
        assert_eq!(diagnostic.file_path, file_path);
        assert_eq!(diagnostic.line, 10);
        assert_eq!(diagnostic.column, 5);
        assert_eq!(diagnostic.length, 8);
        assert!(diagnostic.suggestion.is_none());
        assert!(diagnostic.help.is_none());
    }

    #[test]
    fn test_diagnostic_with_suggestion() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let diagnostic = Diagnostic::new(
            DiagnosticLevel::Warning,
            "W001".to_string(),
            "Test warning".to_string(),
            file_path,
            1,
            1,
            10,
        ).with_suggestion("Add semicolon".to_string());

        assert_eq!(diagnostic.level, DiagnosticLevel::Warning);
        assert_eq!(diagnostic.suggestion, Some("Add semicolon".to_string()));
    }

    #[test]
    fn test_diagnostic_with_help() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let diagnostic = Diagnostic::new(
            DiagnosticLevel::Info,
            "I001".to_string(),
            "Test info".to_string(),
            file_path,
            5,
            3,
            6,
        ).with_help("This is additional help information".to_string());

        assert_eq!(diagnostic.level, DiagnosticLevel::Info);
        assert_eq!(diagnostic.help, Some("This is additional help information".to_string()));
    }

    #[test]
    fn test_diagnostic_chaining() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let diagnostic = Diagnostic::new(
            DiagnosticLevel::Hint,
            "H001".to_string(),
            "Test hint".to_string(),
            file_path,
            2,
            8,
            4,
        ).with_suggestion("Consider using const".to_string())
         .with_help("This improves performance".to_string());

        assert_eq!(diagnostic.level, DiagnosticLevel::Hint);
        assert_eq!(diagnostic.suggestion, Some("Consider using const".to_string()));
        assert_eq!(diagnostic.help, Some("This improves performance".to_string()));
    }

    #[test]
    fn test_diagnostic_debug() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let diagnostic = Diagnostic::new(
            DiagnosticLevel::Error,
            "E001".to_string(),
            "Test error".to_string(),
            file_path,
            1,
            1,
            5,
        );

        let debug_str = format!("{:?}", diagnostic);
        assert!(debug_str.contains("Diagnostic"));
        assert!(debug_str.contains("E001"));
        assert!(debug_str.contains("Test error"));
    }

    #[test]
    fn test_diagnostic_clone() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let original = Diagnostic::new(
            DiagnosticLevel::Warning,
            "W001".to_string(),
            "Original warning".to_string(),
            file_path,
            3,
            2,
            7,
        ).with_suggestion("Fix this".to_string());

        let cloned = original.clone();

        assert_eq!(original.level, cloned.level);
        assert_eq!(original.code, cloned.code);
        assert_eq!(original.message, cloned.message);
        assert_eq!(original.file_path, cloned.file_path);
        assert_eq!(original.line, cloned.line);
        assert_eq!(original.column, cloned.column);
        assert_eq!(original.length, cloned.length);
        assert_eq!(original.suggestion, cloned.suggestion);
        assert_eq!(original.help, cloned.help);
    }

    #[test]
    fn test_diagnostic_serialization() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let diagnostic = Diagnostic::new(
            DiagnosticLevel::Error,
            "E001".to_string(),
            "Serialization test".to_string(),
            file_path,
            1,
            1,
            10,
        ).with_suggestion("Fix it".to_string())
         .with_help("Help text".to_string());

        // Test JSON serialization
        let json_result = serde_json::to_string(&diagnostic);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("E001"));
        assert!(json_str.contains("Serialization test"));
        assert!(json_str.contains("Fix it"));
        assert!(json_str.contains("Help text"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<Diagnostic> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.level, DiagnosticLevel::Error);
        assert_eq!(deserialized.code, "E001");
        assert_eq!(deserialized.message, "Serialization test");
        assert_eq!(deserialized.line, 1);
        assert_eq!(deserialized.column, 1);
        assert_eq!(deserialized.length, 10);
        assert_eq!(deserialized.suggestion, Some("Fix it".to_string()));
        assert_eq!(deserialized.help, Some("Help text".to_string()));
    }

    #[test]
    fn test_linter_config_creation() {
        let mut rule_config = HashMap::new();
        rule_config.insert("max-complexity".to_string(), serde_json::json!(10));

        let config = LinterConfig {
            enabled_rules: vec!["no-unused-vars".to_string(), "no-shadowing".to_string()],
            disabled_rules: vec!["trailing-whitespace".to_string()],
            rule_config,
            exclude_patterns: vec!["*.tmp".to_string(), "*.bak".to_string()],
            output_format: OutputFormat::Json,
        };

        assert_eq!(config.enabled_rules.len(), 2);
        assert_eq!(config.disabled_rules.len(), 1);
        assert_eq!(config.exclude_patterns.len(), 2);
        assert!(matches!(config.output_format, OutputFormat::Json));
        assert_eq!(config.rule_config.get("max-complexity"), Some(&serde_json::json!(10)));
    }

    #[test]
    fn test_linter_config_default() {
        let config = LinterConfig::default();

        assert!(!config.enabled_rules.is_empty());
        assert!(config.disabled_rules.is_empty());
        assert!(config.rule_config.is_empty());
        assert!(!config.exclude_patterns.is_empty());
        assert!(matches!(config.output_format, OutputFormat::Pretty));

        // Check default enabled rules
        assert!(config.enabled_rules.contains(&"no-unused-vars".to_string()));
        assert!(config.enabled_rules.contains(&"no-shadowing".to_string()));
        assert!(config.enabled_rules.contains(&"consistent-indentation".to_string()));

        // Check default exclude patterns
        assert!(config.exclude_patterns.contains(&"node_modules".to_string()));
        assert!(config.exclude_patterns.contains(&".git".to_string()));
        assert!(config.exclude_patterns.contains(&"target".to_string()));
    }

    #[test]
    fn test_linter_config_clone() {
        let original = LinterConfig::default();
        let cloned = original.clone();

        assert_eq!(original.enabled_rules, cloned.enabled_rules);
        assert_eq!(original.disabled_rules, cloned.disabled_rules);
        assert_eq!(original.exclude_patterns, cloned.exclude_patterns);
        assert_eq!(original.output_format, cloned.output_format);
    }

    #[test]
    fn test_linter_config_debug() {
        let config = LinterConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("LinterConfig"));
        assert!(debug_str.contains("Pretty"));
    }

    #[test]
    fn test_linter_config_serialization() {
        let mut rule_config = HashMap::new();
        rule_config.insert("complexity-threshold".to_string(), serde_json::json!(15));

        let config = LinterConfig {
            enabled_rules: vec!["test-rule".to_string()],
            disabled_rules: vec!["disabled-rule".to_string()],
            rule_config,
            exclude_patterns: vec!["*.log".to_string()],
            output_format: OutputFormat::Compact,
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&config);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("test-rule"));
        assert!(json_str.contains("disabled-rule"));
        assert!(json_str.contains("Compact"));
        assert!(json_str.contains("15"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<LinterConfig> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.enabled_rules, vec!["test-rule".to_string()]);
        assert_eq!(deserialized.disabled_rules, vec!["disabled-rule".to_string()]);
        assert_eq!(deserialized.exclude_patterns, vec!["*.log".to_string()]);
        assert!(matches!(deserialized.output_format, OutputFormat::Compact));
    }

    #[test]
    fn test_output_format_enum() {
        let pretty = OutputFormat::Pretty;
        let json = OutputFormat::Json;
        let compact = OutputFormat::Compact;

        assert!(matches!(pretty, OutputFormat::Pretty));
        assert!(matches!(json, OutputFormat::Json));
        assert!(matches!(compact, OutputFormat::Compact));

        let pretty_debug = format!("{:?}", pretty);
        let json_debug = format!("{:?}", json);
        let compact_debug = format!("{:?}", compact);

        assert!(pretty_debug.contains("Pretty"));
        assert!(json_debug.contains("Json"));
        assert!(compact_debug.contains("Compact"));
    }

    #[test]
    fn test_output_format_partial_eq() {
        assert_eq!(OutputFormat::Pretty, OutputFormat::Pretty);
        assert_ne!(OutputFormat::Pretty, OutputFormat::Json);
        assert_ne!(OutputFormat::Json, OutputFormat::Compact);
    }

    #[test]
    fn test_output_format_serialization() {
        // Test Pretty
        let pretty_json = serde_json::to_string(&OutputFormat::Pretty).unwrap();
        assert_eq!(pretty_json, "\"Pretty\"");

        let pretty: OutputFormat = serde_json::from_str("\"Pretty\"").unwrap();
        assert!(matches!(pretty, OutputFormat::Pretty));

        // Test Json
        let json_json = serde_json::to_string(&OutputFormat::Json).unwrap();
        assert_eq!(json_json, "\"Json\"");

        let json_format: OutputFormat = serde_json::from_str("\"Json\"").unwrap();
        assert!(matches!(json_format, OutputFormat::Json));

        // Test Compact
        let compact_json = serde_json::to_string(&OutputFormat::Compact).unwrap();
        assert_eq!(compact_json, "\"Compact\"");

        let compact: OutputFormat = serde_json::from_str("\"Compact\"").unwrap();
        assert!(matches!(compact, OutputFormat::Compact));
    }

    #[test]
    fn test_lint_result_creation() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let result = LintResult::new(file_path.clone());

        assert_eq!(result.file_path, file_path);
        assert!(result.diagnostics.is_empty());
        assert_eq!(result.error_count, 0);
        assert_eq!(result.warning_count, 0);
        assert_eq!(result.duration_ms, 0);
    }

    #[test]
    fn test_lint_result_add_diagnostic() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let mut result = LintResult::new(file_path.clone());

        // Add error diagnostic
        let error_diag = Diagnostic::new(
            DiagnosticLevel::Error,
            "E001".to_string(),
            "Error message".to_string(),
            file_path.clone(),
            1,
            1,
            5,
        );
        result.add_diagnostic(error_diag);

        // Add warning diagnostic
        let warning_diag = Diagnostic::new(
            DiagnosticLevel::Warning,
            "W001".to_string(),
            "Warning message".to_string(),
            file_path.clone(),
            2,
            1,
            8,
        );
        result.add_diagnostic(warning_diag);

        // Add info diagnostic (should not affect counts)
        let info_diag = Diagnostic::new(
            DiagnosticLevel::Info,
            "I001".to_string(),
            "Info message".to_string(),
            file_path,
            3,
            1,
            6,
        );
        result.add_diagnostic(info_diag);

        assert_eq!(result.diagnostics.len(), 3);
        assert_eq!(result.error_count, 1);
        assert_eq!(result.warning_count, 1);
        assert!(result.has_diagnostics());
        assert!(result.has_errors());
    }

    #[test]
    fn test_lint_result_has_diagnostics() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let mut result = LintResult::new(file_path.clone());

        assert!(!result.has_diagnostics());

        let diagnostic = Diagnostic::new(
            DiagnosticLevel::Info,
            "I001".to_string(),
            "Test".to_string(),
            file_path,
            1,
            1,
            4,
        );
        result.add_diagnostic(diagnostic);

        assert!(result.has_diagnostics());
    }

    #[test]
    fn test_lint_result_has_errors() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let mut result = LintResult::new(file_path.clone());

        // Add warning (not error)
        let warning_diag = Diagnostic::new(
            DiagnosticLevel::Warning,
            "W001".to_string(),
            "Warning".to_string(),
            file_path.clone(),
            1,
            1,
            5,
        );
        result.add_diagnostic(warning_diag);

        assert!(!result.has_errors());

        // Add error
        let error_diag = Diagnostic::new(
            DiagnosticLevel::Error,
            "E001".to_string(),
            "Error".to_string(),
            file_path,
            2,
            1,
            5,
        );
        result.add_diagnostic(error_diag);

        assert!(result.has_errors());
    }

    #[test]
    fn test_lint_result_debug() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let result = LintResult::new(file_path);

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("LintResult"));
        assert!(debug_str.contains("0"));
    }

    #[test]
    fn test_lint_result_clone() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let mut original = LintResult::new(file_path.clone());
        original.duration_ms = 100;

        let cloned = original.clone();

        assert_eq!(original.file_path, cloned.file_path);
        assert_eq!(original.diagnostics, cloned.diagnostics);
        assert_eq!(original.error_count, cloned.error_count);
        assert_eq!(original.warning_count, cloned.warning_count);
        assert_eq!(original.duration_ms, cloned.duration_ms);
    }

    #[test]
    fn test_lint_result_serialization() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let mut result = LintResult::new(file_path.clone());
        result.duration_ms = 500;

        let diagnostic = Diagnostic::new(
            DiagnosticLevel::Error,
            "E001".to_string(),
            "Test error".to_string(),
            file_path,
            1,
            1,
            10,
        );
        result.add_diagnostic(diagnostic);

        // Test JSON serialization
        let json_result = serde_json::to_string(&result);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("500"));
        assert!(json_str.contains("1"));
        assert!(json_str.contains("1"));
        assert!(json_str.contains("E001"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<LintResult> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.duration_ms, 500);
        assert_eq!(deserialized.error_count, 1);
        assert_eq!(deserialized.warning_count, 0);
        assert_eq!(deserialized.diagnostics.len(), 1);
    }

    #[test]
    fn test_linter_creation() {
        let config = LinterConfig::default();
        let linter = Linter::new(config.clone());

        assert_eq!(linter.config.enabled_rules, config.enabled_rules);
        // Should have created rules for enabled rule names
        assert!(!linter.rules.is_empty());
    }

    #[test]
    fn test_linter_default() {
        let linter = Linter::default();

        assert!(!linter.rules.is_empty());
        // Should have default configuration
        assert!(!linter.config.enabled_rules.is_empty());
    }

    #[test]
    fn test_linter_debug() {
        let linter = Linter::default();
        let debug_str = format!("{:?}", linter);
        assert!(debug_str.contains("Linter"));
    }

    #[test]
    fn test_create_rule_known_rules() {
        let config = LinterConfig::default();

        // Test creating known rules
        let no_unused_vars = Linter::create_rule("no-unused-vars", &config);
        assert!(no_unused_vars.is_some());

        let no_shadowing = Linter::create_rule("no-shadowing", &config);
        assert!(no_shadowing.is_some());

        let consistent_indentation = Linter::create_rule("consistent-indentation", &config);
        assert!(consistent_indentation.is_some());

        let trailing_whitespace = Linter::create_rule("trailing-whitespace", &config);
        assert!(trailing_whitespace.is_some());

        let missing_semicolons = Linter::create_rule("missing-semicolons", &config);
        assert!(missing_semicolons.is_some());

        let naming_convention = Linter::create_rule("naming-convention", &config);
        assert!(naming_convention.is_some());

        let complexity = Linter::create_rule("complexity", &config);
        assert!(complexity.is_some());
    }

    #[test]
    fn test_create_rule_unknown_rule() {
        let config = LinterConfig::default();

        // Test creating unknown rule
        let unknown = Linter::create_rule("unknown-rule", &config);
        assert!(unknown.is_none());
    }

    #[test]
    fn test_create_rule_with_config() {
        let mut config = LinterConfig::default();
        let mut rule_config = HashMap::new();
        rule_config.insert("threshold".to_string(), serde_json::json!(20));
        config.rule_config.insert("complexity".to_string(), serde_json::json!(rule_config));

        let rule = Linter::create_rule("complexity", &config);
        assert!(rule.is_some());
    }

    #[tokio::test]
    async fn test_lint_files_empty_list() {
        let result = lint_files(vec![]).await;
        assert!(result.is_ok());

        let results = result.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_find_kotoba_files_empty_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir_path = temp_dir.path().to_path_buf();
        let mut files = Vec::new();

        let result = find_kotoba_files(dir_path, &mut files).await;
        assert!(result.is_ok());
        assert!(files.is_empty());
    }

    #[test]
    fn test_print_stats_empty_results() {
        let results: Vec<LintResult> = vec![];
        // This should not panic
        print_stats(&results);
    }

    #[test]
    fn test_print_stats_with_results() {
        let file_path1 = PathBuf::from("/tmp/file1.kotoba");
        let file_path2 = PathBuf::from("/tmp/file2.kotoba");

        let mut result1 = LintResult::new(file_path1);
        let mut result2 = LintResult::new(file_path2);

        // Add diagnostics to result1
        let error_diag = Diagnostic::new(
            DiagnosticLevel::Error,
            "E001".to_string(),
            "Error".to_string(),
            result1.file_path.clone(),
            1,
            1,
            5,
        );
        result1.add_diagnostic(error_diag);

        let warning_diag = Diagnostic::new(
            DiagnosticLevel::Warning,
            "W001".to_string(),
            "Warning".to_string(),
            result1.file_path.clone(),
            2,
            1,
            5,
        );
        result1.add_diagnostic(warning_diag);

        // result2 has no diagnostics

        let results = vec![result1, result2];

        // This should not panic
        print_stats(&results);
    }

    #[test]
    fn test_diagnostic_level_copy() {
        let error = DiagnosticLevel::Error;
        let copied = error;

        assert_eq!(error, copied);
        assert!(matches!(copied, DiagnosticLevel::Error));
    }

    #[test]
    fn test_output_format_copy() {
        let pretty = OutputFormat::Pretty;
        let copied = pretty;

        assert_eq!(pretty, copied);
        assert!(matches!(copied, OutputFormat::Pretty));
    }

    #[test]
    fn test_linter_config_empty_rules() {
        let config = LinterConfig {
            enabled_rules: vec![],
            disabled_rules: vec![],
            rule_config: HashMap::new(),
            exclude_patterns: vec![],
            output_format: OutputFormat::Pretty,
        };

        let linter = Linter::new(config);
        assert!(linter.rules.is_empty());
    }

    #[test]
    fn test_lint_result_with_duration() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let mut result = LintResult::new(file_path);
        result.duration_ms = 1500;

        assert_eq!(result.duration_ms, 1500);
    }

    #[test]
    fn test_diagnostic_edge_cases() {
        let file_path = PathBuf::from("/tmp/test.kotoba");

        // Test with line 0 (should be allowed)
        let diagnostic = Diagnostic::new(
            DiagnosticLevel::Info,
            "I001".to_string(),
            "Line 0".to_string(),
            file_path.clone(),
            0,
            0,
            0,
        );

        assert_eq!(diagnostic.line, 0);
        assert_eq!(diagnostic.column, 0);
        assert_eq!(diagnostic.length, 0);

        // Test with very long message
        let long_message = "a".repeat(1000);
        let diagnostic2 = Diagnostic::new(
            DiagnosticLevel::Hint,
            "H001".to_string(),
            long_message.clone(),
            file_path,
            1,
            1,
            10,
        );

        assert_eq!(diagnostic2.message, long_message);
    }

    #[test]
    fn test_linter_config_edge_cases() {
        // Test with many rules
        let enabled_rules = (0..100).map(|i| format!("rule-{}", i)).collect::<Vec<_>>();
        let config = LinterConfig {
            enabled_rules,
            disabled_rules: vec![],
            rule_config: HashMap::new(),
            exclude_patterns: vec![],
            output_format: OutputFormat::Json,
        };

        let linter = Linter::new(config);
        assert_eq!(linter.rules.len(), 100); // Should have 100 rules (though most will be None)
    }

    #[test]
    fn test_lint_result_max_values() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let mut result = LintResult::new(file_path);

        // Test with maximum duration
        result.duration_ms = u64::MAX;
        assert_eq!(result.duration_ms, u64::MAX);

        // Test with many diagnostics
        for i in 0..1000 {
            let diagnostic = Diagnostic::new(
                if i % 2 == 0 { DiagnosticLevel::Error } else { DiagnosticLevel::Warning },
                format!("CODE{}", i),
                format!("Message {}", i),
                result.file_path.clone(),
                i + 1,
                1,
                10,
            );
            result.add_diagnostic(diagnostic);
        }

        assert_eq!(result.diagnostics.len(), 1000);
        assert_eq!(result.error_count, 500); // Half are errors
        assert_eq!(result.warning_count, 500); // Half are warnings
    }
}