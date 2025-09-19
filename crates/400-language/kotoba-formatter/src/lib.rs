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