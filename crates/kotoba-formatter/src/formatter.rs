//! フォーマッター実装モジュール

use super::{FormatterConfig, FormatResult, FormatRules};
use std::path::PathBuf;
use tokio::fs;

/// メインのフォーマッター実装
#[derive(Debug)]
pub struct CodeFormatter {
    config: FormatterConfig,
    rules: FormatRules,
}

impl CodeFormatter {
    /// 新しいフォーマッターを作成
    pub fn new(config: FormatterConfig) -> Self {
        let rules = FormatRules::new(&config);
        Self { config, rules }
    }

    /// 設定を取得
    pub fn config(&self) -> &FormatterConfig {
        &self.config
    }

    /// 設定を更新
    pub fn set_config(&mut self, config: FormatterConfig) {
        self.config = config.clone();
        self.rules = FormatRules::new(&config);
    }

    /// 単一のファイルをフォーマット
    pub async fn format_file(&self, file_path: &PathBuf) -> Result<FormatResult, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path).await?;
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
        // ルールを適用
        let formatted = self.rules.apply(content)?;

        // 最終的なクリーンアップ
        let cleaned = self.final_cleanup(&formatted);

        Ok(cleaned)
    }

    /// 最終的なクリーンアップ処理
    fn final_cleanup(&self, content: &str) -> String {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        // 末尾の空行を削除
        while let Some(line) = lines.last() {
            if line.trim().is_empty() {
                lines.pop();
            } else {
                break;
            }
        }

        // 各行の末尾の空白を削除
        for line in &mut lines {
            *line = line.trim_end().to_string();
        }

        lines.join(&self.get_line_ending())
    }

    /// 改行文字を取得
    fn get_line_ending(&self) -> String {
        match self.config.line_ending {
            super::LineEnding::Lf => "\n".to_string(),
            super::LineEnding::Crlf => "\r\n".to_string(),
            super::LineEnding::Auto => "\n".to_string(),
        }
    }
}

/// ユーティリティ関数
pub async fn format_file_with_config(
    file_path: &PathBuf,
    config: &FormatterConfig,
) -> Result<FormatResult, Box<dyn std::error::Error>> {
    let formatter = CodeFormatter::new(config.clone());
    formatter.format_file(file_path).await
}

pub async fn format_content_with_config(
    content: &str,
    config: &FormatterConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    let formatter = CodeFormatter::new(config.clone());
    formatter.format_content(content).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatter_creation() {
        let config = FormatterConfig::default();
        let formatter = CodeFormatter::new(config);
        assert_eq!(formatter.config().indent_width, 4);
    }

    #[tokio::test]
    async fn test_format_simple_content() {
        let config = FormatterConfig::default();
        let formatter = CodeFormatter::new(config);

        let input = "graph test{node a}";
        let result = formatter.format_content(input).await.unwrap();

        // フォーマット後の結果を検証
        assert!(!result.is_empty());
    }
}
