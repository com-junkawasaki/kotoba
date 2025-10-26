//! Kotoba Code Formatter
//!
//! Denoの `deno fmt` に似た使い勝手で、.kotoba ファイルを
//! 統一されたスタイルでフォーマットします。
//!
//! ## Pure Kernel & Effects Shell Architecture
//!
//! This crate follows the Pure Kernel/Effects Shell pattern:
//!
//! - **Pure Kernel**: `PureFormatter` - performs deterministic code formatting without side effects
//! - **Effects Shell**: `CodeFormatter` - wraps the pure formatter and handles file I/O
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
use std::path::PathBuf;

pub mod config;
pub mod formatter;
pub mod pure_formatter;

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

// 便利関数
pub async fn format_files(files: Vec<PathBuf>, _check_only: bool) -> Result<Vec<FormatResult>, Box<dyn std::error::Error>> {
    let formatter = formatter::CodeFormatter::new(FormatterConfig::default());
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
pub use pure_formatter::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
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

    #[tokio::test]
    async fn test_format_content_basic() {
        let formatter = formatter::CodeFormatter::new(FormatterConfig::default());
        let content = "{ \"a\": 1, \"b\": 2 }";

        let result = formatter.format_content(content).await;
        assert!(result.is_ok());

        let formatted = result.unwrap();
        // A simple check to see if it's formatted with newlines and indentation
        assert!(formatted.contains('\n'));
        assert!(formatted.contains("    "));
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
}