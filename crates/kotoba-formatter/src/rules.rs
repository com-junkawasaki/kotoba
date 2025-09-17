//! フォーマットルール定義モジュール

use super::{FormatterConfig, IndentStyle};
use regex::Regex;
use std::collections::HashMap;

/// フォーマットルールの集合
#[derive(Debug)]
pub struct FormatRules {
    config: FormatterConfig,
    rules: HashMap<String, Box<dyn Rule>>,
}

impl FormatRules {
    /// 新しいルールセットを作成
    pub fn new(config: &FormatterConfig) -> Self {
        let mut rules = HashMap::new();

        // 基本的なルールを追加
        rules.insert("indent".to_string(), Box::new(IndentRule::new(config)) as Box<dyn Rule>);
        rules.insert("spacing".to_string(), Box::new(SpacingRule::new(config)) as Box<dyn Rule>);
        rules.insert("line_breaks".to_string(), Box::new(LineBreakRule::new(config)) as Box<dyn Rule>);
        rules.insert("braces".to_string(), Box::new(BraceRule::new(config)) as Box<dyn Rule>);
        rules.insert("alignment".to_string(), Box::new(AlignmentRule::new(config)) as Box<dyn Rule>);

        Self {
            config: config.clone(),
            rules,
        }
    }

    /// ルールを適用
    pub fn apply(&self, content: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut result = content.to_string();

        for rule in self.rules.values() {
            result = rule.apply(&result)?;
        }

        Ok(result)
    }

    /// 特定のルールを取得
    pub fn get_rule(&self, name: &str) -> Option<&Box<dyn Rule>> {
        self.rules.get(name)
    }

    /// ルールを追加
    pub fn add_rule(&mut self, name: String, rule: Box<dyn Rule>) {
        self.rules.insert(name, rule);
    }

    /// ルールを削除
    pub fn remove_rule(&mut self, name: &str) {
        self.rules.remove(name);
    }
}

/// ルールのトレイト
pub trait Rule: std::fmt::Debug {
    /// ルールを適用
    fn apply(&self, content: &str) -> Result<String, Box<dyn std::error::Error>>;
    /// ルールの名前
    fn name(&self) -> &str;
}

/// インデントルール
#[derive(Debug)]
pub struct IndentRule {
    config: FormatterConfig,
}

impl IndentRule {
    pub fn new(config: &FormatterConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
}

impl Rule for IndentRule {
    fn apply(&self, content: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut result = String::new();
        let indent_str = self.get_indent_string();

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();

            if !trimmed.is_empty() {
                // 行番号に基づいてインデントを計算
                let indent_level = self.calculate_indent_level(line, i);
                let indent = indent_str.repeat(indent_level);
                result.push_str(&indent);
            }

            result.push_str(line.trim_start());
            result.push('\n');
        }

        Ok(result)
    }

    fn name(&self) -> &str {
        "indent"
    }
}

impl IndentRule {
    fn get_indent_string(&self) -> String {
        match self.config.indent_style {
            IndentStyle::Space => " ".repeat(self.config.indent_width),
            IndentStyle::Tab => "\t".to_string(),
        }
    }

    fn calculate_indent_level(&self, line: &str, line_number: usize) -> usize {
        // 簡易的なインデントレベル計算
        // TODO: ASTベースの正確な計算を実装

        let mut level = 0;
        let trimmed = line.trim_start();

        // キーワードに基づいてインデントレベルを調整
        if trimmed.starts_with("graph") ||
           trimmed.starts_with("node") ||
           trimmed.starts_with("edge") ||
           trimmed.starts_with("query") ||
           trimmed.starts_with("fn") ||
           trimmed.starts_with("if") ||
           trimmed.starts_with("for") ||
           trimmed.starts_with("while") {
            level = 0;
        } else if trimmed.starts_with("}") {
            level = 0; // 閉じ括弧の場合はインデントなし
        } else if line.starts_with("    ") || line.starts_with("\t") {
            level = 1; // 既にインデントされている場合は維持
        }

        level
    }
}

/// スペーシングルール
#[derive(Debug)]
pub struct SpacingRule {
    config: FormatterConfig,
}

impl SpacingRule {
    pub fn new(config: &FormatterConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
}

impl Rule for SpacingRule {
    fn apply(&self, content: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut result = content.to_string();

        if self.config.space_around_operators {
            // 演算子の周りにスペースを入れる
            let operators = ["=", "==", "!=", "<", ">", "<=", ">=", "+", "-", "*", "/", "&&", "||", "->", ":"];

            for op in &operators {
                let pattern = format!(r"(\w)({})(?=\w)", regex::escape(op));
                let replacement = format!("$1 {} ", op);
                result = Regex::new(&pattern)?.replace_all(&result, replacement).to_string();
            }
        }

        // コンマの後のスペース
        if self.config.trailing_comma {
            result = result.replace(",", ", ");
        }

        // 重複スペースの除去
        result = Regex::new(r" +")?.replace_all(&result, " ").to_string();

        Ok(result)
    }

    fn name(&self) -> &str {
        "spacing"
    }
}

/// 改行ルール
#[derive(Debug)]
pub struct LineBreakRule {
    config: FormatterConfig,
}

impl LineBreakRule {
    pub fn new(config: &FormatterConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
}

impl Rule for LineBreakRule {
    fn apply(&self, content: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut result = Vec::new();
        let mut empty_count = 0;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                empty_count += 1;
                if empty_count <= self.config.max_empty_lines {
                    result.push(line);
                }
            } else {
                empty_count = 0;
                result.push(line);
            }
        }

        Ok(result.join("\n"))
    }

    fn name(&self) -> &str {
        "line_breaks"
    }
}

/// 波括弧ルール
#[derive(Debug)]
pub struct BraceRule {
    config: FormatterConfig,
}

impl BraceRule {
    pub fn new(config: &FormatterConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
}

impl Rule for BraceRule {
    fn apply(&self, content: &str) -> Result<String, Box<dyn std::error::Error>> {
        // TODO: より洗練された波括弧フォーマットを実装
        Ok(content.to_string())
    }

    fn name(&self) -> &str {
        "braces"
    }
}

/// アライメントルール
#[derive(Debug)]
pub struct AlignmentRule {
    config: FormatterConfig,
}

impl AlignmentRule {
    pub fn new(config: &FormatterConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
}

impl Rule for AlignmentRule {
    fn apply(&self, content: &str) -> Result<String, Box<dyn std::error::Error>> {
        // TODO: 変数宣言やプロパティのアライメントを実装
        Ok(content.to_string())
    }

    fn name(&self) -> &str {
        "alignment"
    }
}
