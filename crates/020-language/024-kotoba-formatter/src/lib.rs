//! Kotoba Code Formatter
//!
//! Denoの `deno fmt` に似た使い勝手で、.kotoba ファイルを
//! 統一されたスタイルでフォーマットします。
//!
//! ## 使用方法
//!
//! ```bash
//! # ファイルのフォーマット
//! kotoba fmt file.kotoba
//!
//! # チェックのみ（変更しない）
//! kotoba fmt --check file.kotoba
//!
//! # ディレクトリ内の全ファイルをフォーマット
//! kotoba fmt .
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod config;
pub mod formatter;
pub mod parser;
pub mod rules;
pub mod writer;

/// Formatterの結果
#[derive(Debug, Clone)]
pub struct FormatResult {
    /// 元のファイルパス
    pub file_path: PathBuf,
    /// フォーマット前の内容
    pub original_content: String,
    /// フォーマット後の内容
    pub formatted_content: String,
    /// 変更があったかどうか
    pub has_changes: bool,
    /// エラー（あれば）
    pub error: Option<String>,
}

impl FormatResult {
    /// 新しい結果を作成
    pub fn new(file_path: PathBuf, original_content: String) -> Self {
        Self {
            file_path,
            original_content: original_content.clone(),
            formatted_content: original_content,
            has_changes: false,
            error: None,
        }
    }

    /// フォーマット後の内容を設定
    pub fn set_formatted_content(&mut self, content: String) {
        self.has_changes = content != self.original_content;
        self.formatted_content = content;
    }

    /// エラーを設定
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }
}

/// Formatterの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatterConfig {
    /// インデントに使用する文字
    pub indent_style: IndentStyle,
    /// インデント幅
    pub indent_width: usize,
    /// 行の最大長
    pub line_width: usize,
    /// 改行スタイル
    pub line_ending: LineEnding,
    /// 波括弧のスタイル
    pub brace_style: BraceStyle,
    /// コンマの後ろにスペースを入れる
    pub trailing_comma: bool,
    /// 演算子の周りにスペースを入れる
    pub space_around_operators: bool,
    /// 空行の最大数
    pub max_empty_lines: usize,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            indent_style: IndentStyle::Space,
            indent_width: 4,
            line_width: 100,
            line_ending: LineEnding::Lf,
            brace_style: BraceStyle::SameLine,
            trailing_comma: true,
            space_around_operators: true,
            max_empty_lines: 2,
        }
    }
}

/// インデントスタイル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndentStyle {
    /// スペース
    Space,
    /// タブ
    Tab,
}

/// 改行スタイル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineEnding {
    /// LF (\n)
    Lf,
    /// CRLF (\r\n)
    Crlf,
    /// 自動検出
    Auto,
}

/// 波括弧のスタイル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BraceStyle {
    /// 同じ行に置く
    SameLine,
    /// 次の行に置く
    NextLine,
}

/// Formatterのメイン構造体
#[derive(Debug)]
pub struct Formatter {
    config: FormatterConfig,
    rules: rules::FormatRules,
}

impl Formatter {
    /// 新しいFormatterを作成
    pub fn new(config: FormatterConfig) -> Self {
        let rules = rules::FormatRules::new(&config);
        Self { config, rules }
    }

    /// デフォルト設定でFormatterを作成
    pub fn default() -> Self {
        Self::new(FormatterConfig::default())
    }

    /// 設定ファイルからFormatterを作成
    pub async fn from_config_file() -> Result<Self, Box<dyn std::error::Error>> {
        let config = config::load_config().await?;
        Ok(Self::new(config))
    }

    /// 単一ファイルをフォーマット
    pub async fn format_file(&self, file_path: &PathBuf) -> Result<FormatResult, Box<dyn std::error::Error>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let mut result = FormatResult::new(file_path.clone(), content);

        match self.format_content(&result.original_content).await {
            Ok(formatted) => {
                result.set_formatted_content(formatted);
                Ok(result)
            }
            Err(e) => {
                result.set_error(e.to_string());
                Ok(result)
            }
        }
    }

    /// コンテンツをフォーマット
    pub async fn format_content(&self, content: &str) -> Result<String, Box<dyn std::error::Error>> {
        // 簡易的なフォーマット処理
        // TODO: より洗練されたASTベースのフォーマットを実装

        let mut lines = content.lines().collect::<Vec<_>>();
        let mut formatted_lines = Vec::new();

        for line in lines {
            let formatted = self.format_line(line);
            formatted_lines.push(formatted);
        }

        // 空行の整理
        self.cleanup_empty_lines(&mut formatted_lines);

        Ok(formatted_lines.join(&self.get_line_ending()))
    }

    /// 行をフォーマット
    fn format_line(&self, line: &str) -> String {
        let line = line.trim();

        if line.is_empty() {
            return String::new();
        }

        // 基本的な整形
        let line = self.format_braces(line);
        let line = self.format_operators(&line);
        let line = self.format_commas(&line);

        line
    }

    /// 波括弧をフォーマット
    fn format_braces(&self, line: &str) -> String {
        // 簡易実装
        line.to_string()
    }

    /// 演算子をフォーマット
    fn format_operators(&self, line: &str) -> String {
        if !self.config.space_around_operators {
            return line.to_string();
        }

        // 演算子の周りにスペースを入れる
        line.replace("=", " = ")
            .replace("==", " == ")
            .replace("!=", " != ")
            .replace("<", " < ")
            .replace(">", " > ")
            .replace("<=", " <= ")
            .replace(">=", " >= ")
            .replace("+", " + ")
            .replace("-", " - ")
            .replace("*", " * ")
            .replace("/", " / ")
            .replace("&&", " && ")
            .replace("||", " || ")
            .replace("->", " -> ")
            .replace(":", " : ")
            // 重複スペースを除去
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// コンマをフォーマット
    fn format_commas(&self, line: &str) -> String {
        if self.config.trailing_comma {
            // コンマの後にスペースを入れる
            line.replace(",", ", ")
        } else {
            line.to_string()
        }
    }

    /// 空行を整理
    fn cleanup_empty_lines(&self, lines: &mut Vec<String>) {
        let mut result = Vec::new();
        let mut empty_count = 0;

        for line in &mut *lines {
            if line.trim().is_empty() {
                empty_count += 1;
                if empty_count <= self.config.max_empty_lines {
                    result.push(line.clone());
                }
            } else {
                empty_count = 0;
                result.push(line.clone());
            }
        }

        *lines = result;
    }

    /// 改行文字を取得
    fn get_line_ending(&self) -> String {
        match self.config.line_ending {
            LineEnding::Lf => "\n".to_string(),
            LineEnding::Crlf => "\r\n".to_string(),
            LineEnding::Auto => "\n".to_string(),
        }
    }
}

// 便利関数
pub async fn format_files(files: Vec<PathBuf>, check_only: bool) -> Result<Vec<FormatResult>, Box<dyn std::error::Error>> {
    let formatter = Formatter::default();
    let mut results = Vec::new();

    for file in files {
        let result = formatter.format_file(&file).await?;
        results.push(result);
    }

    Ok(results)
}

pub async fn format_directory(dir: PathBuf, check_only: bool) -> Result<Vec<FormatResult>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();

    // .kotoba ファイルを再帰的に検索
    find_kotoba_files(dir, &mut files).await?;

    format_files(files, check_only).await
}

/// .kotoba ファイルを再帰的に検索
async fn find_kotoba_files(dir: PathBuf, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let mut entries = tokio::fs::read_dir(&dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_dir() {
            // node_modules や .git はスキップ
            if !path.ends_with("node_modules") && !path.ends_with(".git") {
                Box::pin(find_kotoba_files(path, files)).await?;
            }
        } else if path.extension().map_or(false, |ext| ext == "kotoba") {
            files.push(path);
        }
    }

    Ok(())
}

// 各モジュールの再エクスポート
pub use config::*;
pub use formatter::*;
pub use parser::*;
pub use rules::*;
pub use writer::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio::fs;
    use tempfile::tempdir;

    #[test]
    fn test_format_result_creation() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let content = "let x = 1;".to_string();

        let result = FormatResult::new(file_path.clone(), content.clone());

        assert_eq!(result.file_path, file_path);
        assert_eq!(result.original_content, content);
        assert_eq!(result.formatted_content, content);
        assert!(!result.has_changes);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_format_result_set_formatted_content() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let original = "let x=1;".to_string();
        let mut result = FormatResult::new(file_path, original);

        let formatted = "let x = 1;".to_string();
        result.set_formatted_content(formatted.clone());

        assert_eq!(result.formatted_content, formatted);
        assert!(result.has_changes);
    }

    #[test]
    fn test_format_result_set_formatted_content_no_change() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let content = "let x = 1;".to_string();
        let mut result = FormatResult::new(file_path, content.clone());

        result.set_formatted_content(content.clone());

        assert_eq!(result.formatted_content, content);
        assert!(!result.has_changes);
    }

    #[test]
    fn test_format_result_set_error() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let content = "let x = 1;".to_string();
        let mut result = FormatResult::new(file_path, content);

        let error = "Parse error".to_string();
        result.set_error(error.clone());

        assert_eq!(result.error, Some(error));
    }

    #[test]
    fn test_format_result_debug() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let content = "let x = 1;".to_string();
        let result = FormatResult::new(file_path, content);

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("FormatResult"));
        assert!(debug_str.contains("/tmp/test.kotoba"));
    }

    #[test]
    fn test_format_result_clone() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let content = "let x = 1;".to_string();
        let mut original = FormatResult::new(file_path.clone(), content);
        original.set_error("test error".to_string());

        let cloned = original.clone();

        assert_eq!(original.file_path, cloned.file_path);
        assert_eq!(original.original_content, cloned.original_content);
        assert_eq!(original.formatted_content, cloned.formatted_content);
        assert_eq!(original.has_changes, cloned.has_changes);
        assert_eq!(original.error, cloned.error);
    }

    #[test]
    fn test_formatter_config_creation() {
        let config = FormatterConfig {
            indent_style: IndentStyle::Tab,
            indent_width: 2,
            line_width: 80,
            line_ending: LineEnding::Crlf,
            brace_style: BraceStyle::NextLine,
            trailing_comma: false,
            space_around_operators: false,
            max_empty_lines: 1,
        };

        assert!(matches!(config.indent_style, IndentStyle::Tab));
        assert_eq!(config.indent_width, 2);
        assert_eq!(config.line_width, 80);
        assert!(matches!(config.line_ending, LineEnding::Crlf));
        assert!(matches!(config.brace_style, BraceStyle::NextLine));
        assert!(!config.trailing_comma);
        assert!(!config.space_around_operators);
        assert_eq!(config.max_empty_lines, 1);
    }

    #[test]
    fn test_formatter_config_default() {
        let config = FormatterConfig::default();

        assert!(matches!(config.indent_style, IndentStyle::Space));
        assert_eq!(config.indent_width, 4);
        assert_eq!(config.line_width, 100);
        assert!(matches!(config.line_ending, LineEnding::Lf));
        assert!(matches!(config.brace_style, BraceStyle::SameLine));
        assert!(config.trailing_comma);
        assert!(config.space_around_operators);
        assert_eq!(config.max_empty_lines, 2);
    }

    #[test]
    fn test_formatter_config_clone() {
        let original = FormatterConfig::default();
        let cloned = original.clone();

        assert!(matches!(cloned.indent_style, IndentStyle::Space));
        assert_eq!(cloned.indent_width, 4);
        assert_eq!(cloned.line_width, 100);
        assert!(matches!(cloned.line_ending, LineEnding::Lf));
        assert!(matches!(cloned.brace_style, BraceStyle::SameLine));
        assert!(cloned.trailing_comma);
        assert!(cloned.space_around_operators);
        assert_eq!(cloned.max_empty_lines, 2);
    }

    #[test]
    fn test_formatter_config_debug() {
        let config = FormatterConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("FormatterConfig"));
        assert!(debug_str.contains("4"));
        assert!(debug_str.contains("100"));
    }

    #[test]
    fn test_formatter_config_serialization() {
        let config = FormatterConfig {
            indent_style: IndentStyle::Tab,
            indent_width: 2,
            line_width: 120,
            line_ending: LineEnding::Auto,
            brace_style: BraceStyle::NextLine,
            trailing_comma: false,
            space_around_operators: true,
            max_empty_lines: 3,
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&config);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("120"));
        assert!(json_str.contains("3"));
        assert!(json_str.contains("Tab"));
        assert!(json_str.contains("Auto"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<FormatterConfig> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert!(matches!(deserialized.indent_style, IndentStyle::Tab));
        assert_eq!(deserialized.indent_width, 2);
        assert_eq!(deserialized.line_width, 120);
        assert!(matches!(deserialized.line_ending, LineEnding::Auto));
        assert!(matches!(deserialized.brace_style, BraceStyle::NextLine));
        assert!(!deserialized.trailing_comma);
        assert!(deserialized.space_around_operators);
        assert_eq!(deserialized.max_empty_lines, 3);
    }

    #[test]
    fn test_indent_style_enum() {
        let space = IndentStyle::Space;
        let tab = IndentStyle::Tab;

        assert!(matches!(space, IndentStyle::Space));
        assert!(matches!(tab, IndentStyle::Tab));

        let debug_space = format!("{:?}", space);
        let debug_tab = format!("{:?}", tab);
        assert!(debug_space.contains("Space"));
        assert!(debug_tab.contains("Tab"));
    }

    #[test]
    fn test_line_ending_enum() {
        let lf = LineEnding::Lf;
        let crlf = LineEnding::Crlf;
        let auto = LineEnding::Auto;

        assert!(matches!(lf, LineEnding::Lf));
        assert!(matches!(crlf, LineEnding::Crlf));
        assert!(matches!(auto, LineEnding::Auto));

        let debug_lf = format!("{:?}", lf);
        let debug_crlf = format!("{:?}", crlf);
        let debug_auto = format!("{:?}", auto);
        assert!(debug_lf.contains("Lf"));
        assert!(debug_crlf.contains("Crlf"));
        assert!(debug_auto.contains("Auto"));
    }

    #[test]
    fn test_brace_style_enum() {
        let same_line = BraceStyle::SameLine;
        let next_line = BraceStyle::NextLine;

        assert!(matches!(same_line, BraceStyle::SameLine));
        assert!(matches!(next_line, BraceStyle::NextLine));

        let debug_same = format!("{:?}", same_line);
        let debug_next = format!("{:?}", next_line);
        assert!(debug_same.contains("SameLine"));
        assert!(debug_next.contains("NextLine"));
    }

    #[test]
    fn test_formatter_creation() {
        let config = FormatterConfig::default();
        let formatter = Formatter::new(config.clone());

        assert_eq!(formatter.config.indent_width, config.indent_width);
        assert_eq!(formatter.config.line_width, config.line_width);
    }

    #[test]
    fn test_formatter_default() {
        let formatter = Formatter::default();

        assert_eq!(formatter.config.indent_width, 4);
        assert_eq!(formatter.config.line_width, 100);
        assert!(matches!(formatter.config.indent_style, IndentStyle::Space));
    }

    #[test]
    fn test_formatter_debug() {
        let formatter = Formatter::default();
        let debug_str = format!("{:?}", formatter);
        assert!(debug_str.contains("Formatter"));
    }

    #[test]
    fn test_get_line_ending() {
        let mut config = FormatterConfig::default();

        // Test LF
        let formatter = Formatter::new(config.clone());
        assert_eq!(formatter.get_line_ending(), "\n");

        // Test CRLF
        config.line_ending = LineEnding::Crlf;
        let formatter = Formatter::new(config.clone());
        assert_eq!(formatter.get_line_ending(), "\r\n");

        // Test Auto (defaults to LF)
        config.line_ending = LineEnding::Auto;
        let formatter = Formatter::new(config);
        assert_eq!(formatter.get_line_ending(), "\n");
    }

    #[tokio::test]
    async fn test_format_content_basic() {
        let formatter = Formatter::default();
        let content = "let x=1;";

        let result = formatter.format_content(content).await;
        assert!(result.is_ok());

        let formatted = result.unwrap();
        // Should contain spaces around operators
        assert!(formatted.contains(" = "));
    }

    #[tokio::test]
    async fn test_format_content_empty() {
        let formatter = Formatter::default();
        let content = "";

        let result = formatter.format_content(content).await;
        assert!(result.is_ok());

        let formatted = result.unwrap();
        assert_eq!(formatted, "");
    }

    #[tokio::test]
    async fn test_format_content_multiple_lines() {
        let formatter = Formatter::default();
        let content = "let x=1;\nlet y=2;\nlet z=3;";

        let result = formatter.format_content(content).await;
        assert!(result.is_ok());

        let formatted = result.unwrap();
        assert!(formatted.contains(" = "));
        assert!(formatted.contains("\n"));
    }

    #[test]
    fn test_format_line_basic() {
        let formatter = Formatter::default();

        // Test trimming
        let result = formatter.format_line("  let x = 1;  ");
        assert_eq!(result, "let x = 1;");

        // Test empty line
        let result = formatter.format_line("");
        assert_eq!(result, "");

        // Test whitespace only
        let result = formatter.format_line("   ");
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_line_with_operators() {
        let formatter = Formatter::default();

        let result = formatter.format_line("let x=1+2*3");
        assert!(result.contains(" = "));
        assert!(result.contains(" + "));
        assert!(result.contains(" * "));
    }

    #[test]
    fn test_format_operators_enabled() {
        let formatter = Formatter::default(); // space_around_operators is true by default

        let result = formatter.format_operators("x=1+2");
        assert!(result.contains(" = "));
        assert!(result.contains(" + "));
    }

    #[test]
    fn test_format_operators_disabled() {
        let config = FormatterConfig {
            space_around_operators: false,
            ..FormatterConfig::default()
        };
        let formatter = Formatter::new(config);

        let result = formatter.format_operators("x=1+2");
        assert_eq!(result, "x=1+2");
    }

    #[test]
    fn test_format_operators_various() {
        let formatter = Formatter::default();

        let result = formatter.format_operators("x==y&&a!=b||c<d&&e>=f");
        assert!(result.contains(" == "));
        assert!(result.contains(" && "));
        assert!(result.contains(" != "));
        assert!(result.contains(" || "));
        assert!(result.contains(" < "));
        assert!(result.contains(" >= "));
    }

    #[test]
    fn test_format_commas_with_trailing() {
        let formatter = Formatter::default(); // trailing_comma is true by default

        let result = formatter.format_commas("func(a,b,c)");
        assert_eq!(result, "func(a, b, c)");
    }

    #[test]
    fn test_format_commas_without_trailing() {
        let config = FormatterConfig {
            trailing_comma: false,
            ..FormatterConfig::default()
        };
        let formatter = Formatter::new(config);

        let result = formatter.format_commas("func(a,b,c)");
        assert_eq!(result, "func(a,b,c)");
    }

    #[test]
    fn test_cleanup_empty_lines() {
        let formatter = Formatter::default(); // max_empty_lines = 2

        let mut lines = vec![
            "line1".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(), // This should be removed
            "line2".to_string(),
        ];

        formatter.cleanup_empty_lines(&mut lines);

        assert_eq!(lines.len(), 4); // line1 + 2 empty lines + line2
        assert_eq!(lines[0], "line1");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "");
        assert_eq!(lines[3], "line2");
    }

    #[test]
    fn test_cleanup_empty_lines_max_zero() {
        let config = FormatterConfig {
            max_empty_lines: 0,
            ..FormatterConfig::default()
        };
        let formatter = Formatter::new(config);

        let mut lines = vec![
            "line1".to_string(),
            "".to_string(),
            "line2".to_string(),
        ];

        formatter.cleanup_empty_lines(&mut lines);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "line1");
        assert_eq!(lines[1], "line2");
    }

    #[test]
    fn test_cleanup_empty_lines_max_one() {
        let config = FormatterConfig {
            max_empty_lines: 1,
            ..FormatterConfig::default()
        };
        let formatter = Formatter::new(config);

        let mut lines = vec![
            "line1".to_string(),
            "".to_string(),
            "".to_string(),
            "line2".to_string(),
        ];

        formatter.cleanup_empty_lines(&mut lines);

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line1");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "line2");
    }

    #[test]
    fn test_format_braces() {
        let formatter = Formatter::default();

        // Currently a no-op implementation
        let result = formatter.format_braces("func() { return 1; }");
        assert_eq!(result, "func() { return 1; }");
    }

    #[tokio::test]
    async fn test_format_files_empty_list() {
        let result = format_files(vec![], false).await;
        assert!(result.is_ok());

        let results = result.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_format_directory_nonexistent() {
        let dir = PathBuf::from("/nonexistent/directory");
        let result = format_directory(dir, false).await;

        // Should fail because directory doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_enum_serialization() {
        // Test IndentStyle serialization
        let space_json = serde_json::to_string(&IndentStyle::Space).unwrap();
        let tab_json = serde_json::to_string(&IndentStyle::Tab).unwrap();
        assert!(space_json.contains("Space"));
        assert!(tab_json.contains("Tab"));

        // Test LineEnding serialization
        let lf_json = serde_json::to_string(&LineEnding::Lf).unwrap();
        let crlf_json = serde_json::to_string(&LineEnding::Crlf).unwrap();
        let auto_json = serde_json::to_string(&LineEnding::Auto).unwrap();
        assert!(lf_json.contains("Lf"));
        assert!(crlf_json.contains("Crlf"));
        assert!(auto_json.contains("Auto"));

        // Test BraceStyle serialization
        let same_json = serde_json::to_string(&BraceStyle::SameLine).unwrap();
        let next_json = serde_json::to_string(&BraceStyle::NextLine).unwrap();
        assert!(same_json.contains("SameLine"));
        assert!(next_json.contains("NextLine"));
    }

    #[test]
    fn test_enum_deserialization() {
        // Test IndentStyle deserialization
        let space: IndentStyle = serde_json::from_str("\"Space\"").unwrap();
        let tab: IndentStyle = serde_json::from_str("\"Tab\"").unwrap();
        assert!(matches!(space, IndentStyle::Space));
        assert!(matches!(tab, IndentStyle::Tab));

        // Test LineEnding deserialization
        let lf: LineEnding = serde_json::from_str("\"Lf\"").unwrap();
        let crlf: LineEnding = serde_json::from_str("\"Crlf\"").unwrap();
        let auto: LineEnding = serde_json::from_str("\"Auto\"").unwrap();
        assert!(matches!(lf, LineEnding::Lf));
        assert!(matches!(crlf, LineEnding::Crlf));
        assert!(matches!(auto, LineEnding::Auto));

        // Test BraceStyle deserialization
        let same: BraceStyle = serde_json::from_str("\"SameLine\"").unwrap();
        let next: BraceStyle = serde_json::from_str("\"NextLine\"").unwrap();
        assert!(matches!(same, BraceStyle::SameLine));
        assert!(matches!(next, BraceStyle::NextLine));
    }

    #[test]
    fn test_enum_clone() {
        let space = IndentStyle::Space;
        let cloned = space.clone();
        assert!(matches!(cloned, IndentStyle::Space));

        let lf = LineEnding::Lf;
        let cloned = lf.clone();
        assert!(matches!(cloned, LineEnding::Lf));

        let same = BraceStyle::SameLine;
        let cloned = same.clone();
        assert!(matches!(cloned, BraceStyle::SameLine));
    }

    #[test]
    fn test_formatter_config_edge_cases() {
        // Test with zero values
        let config = FormatterConfig {
            indent_width: 0,
            line_width: 0,
            max_empty_lines: 0,
            ..FormatterConfig::default()
        };

        assert_eq!(config.indent_width, 0);
        assert_eq!(config.line_width, 0);
        assert_eq!(config.max_empty_lines, 0);

        let formatter = Formatter::new(config);
        assert_eq!(formatter.config.indent_width, 0);
    }

    #[test]
    fn test_format_result_with_empty_paths() {
        let file_path = PathBuf::new();
        let content = "test".to_string();
        let result = FormatResult::new(file_path, content);

        assert!(result.file_path.as_os_str().is_empty());
    }

    #[tokio::test]
    async fn test_format_content_with_special_characters() {
        let formatter = Formatter::default();
        let content = "let x=特殊文字+テスト;";

        let result = formatter.format_content(content).await;
        assert!(result.is_ok());

        let formatted = result.unwrap();
        // Should still format operators even with special characters
        assert!(formatted.contains(" = "));
        assert!(formatted.contains(" + "));
    }

    #[test]
    fn test_format_operators_edge_cases() {
        let formatter = Formatter::default();

        // Test with already spaced operators
        let result = formatter.format_operators("x = 1 + 2");
        // Should not add extra spaces
        assert!(!result.contains("  =  "));
        assert!(!result.contains("  +  "));

        // Test with multiple operators in sequence
        let result = formatter.format_operators("x>=y<=z");
        assert!(result.contains(" >= "));
        assert!(result.contains(" <= "));

        // Test with arrow functions
        let result = formatter.format_operators("func->result");
        assert!(result.contains(" -> "));
    }

    #[test]
    fn test_cleanup_empty_lines_edge_cases() {
        let formatter = Formatter::default();

        // Test with all empty lines
        let mut lines = vec!["".to_string(), "".to_string(), "".to_string()];
        formatter.cleanup_empty_lines(&mut lines);
        assert_eq!(lines.len(), 2); // Should keep max 2 empty lines

        // Test with no empty lines
        let mut lines = vec!["line1".to_string(), "line2".to_string()];
        formatter.cleanup_empty_lines(&mut lines);
        assert_eq!(lines.len(), 2);

        // Test with empty vector
        let mut lines: Vec<String> = vec![];
        formatter.cleanup_empty_lines(&mut lines);
        assert!(lines.is_empty());
    }

    #[tokio::test]
    async fn test_format_content_line_endings() {
        let mut config = FormatterConfig::default();
        config.line_ending = LineEnding::Crlf;
        let formatter = Formatter::new(config);

        let content = "line1\nline2\nline3";

        let result = formatter.format_content(content).await;
        assert!(result.is_ok());

        let formatted = result.unwrap();
        assert!(formatted.contains("\r\n"));
    }

    #[test]
    fn test_formatter_config_extreme_values() {
        // Test with very large values
        let config = FormatterConfig {
            indent_width: usize::MAX,
            line_width: usize::MAX,
            max_empty_lines: usize::MAX,
            ..FormatterConfig::default()
        };

        assert_eq!(config.indent_width, usize::MAX);
        assert_eq!(config.line_width, usize::MAX);
        assert_eq!(config.max_empty_lines, usize::MAX);
    }

    #[test]
    fn test_format_result_error_handling() {
        let file_path = PathBuf::from("/tmp/test.kotoba");
        let content = "invalid content".to_string();
        let mut result = FormatResult::new(file_path, content);

        // Test setting multiple errors (last one should win)
        result.set_error("first error".to_string());
        result.set_error("second error".to_string());

        assert_eq!(result.error, Some("second error".to_string()));
    }
}