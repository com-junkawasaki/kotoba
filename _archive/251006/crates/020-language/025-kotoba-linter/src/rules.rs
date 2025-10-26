//! リンターール定義モジュール

use super::{Diagnostic, DiagnosticLevel};
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;

/// リンターールのトレイト
pub trait LintRule {
    /// ルールの名前
    fn name(&self) -> &str;

    /// ルールの説明
    fn description(&self) -> &str;

    /// 診断レベル
    fn level(&self) -> DiagnosticLevel;

    /// コンテンツをチェック
    fn check(&self, content: &str, file_path: &PathBuf) -> Result<Vec<Diagnostic>, Box<dyn std::error::Error>>;
}

/// 未使用変数検出ルール
pub struct NoUnusedVarsRule {
    config: Option<serde_json::Value>,
}

impl NoUnusedVarsRule {
    pub fn new(config: Option<serde_json::Value>) -> Self {
        Self { config }
    }
}

impl LintRule for NoUnusedVarsRule {
    fn name(&self) -> &str {
        "no-unused-vars"
    }

    fn description(&self) -> &str {
        "未使用の変数を検出します"
    }

    fn level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warning
    }

    fn check(&self, content: &str, file_path: &PathBuf) -> Result<Vec<Diagnostic>, Box<dyn std::error::Error>> {
        let mut diagnostics = Vec::new();

        // 変数宣言のパターン: let var_name = value
        let var_pattern = Regex::new(r"let\s+(\w+)\s*=\s*[^;]+")?;
        let mut declared_vars = HashMap::new();

        for (line_num, line) in content.lines().enumerate() {
            for cap in var_pattern.captures_iter(line) {
                if let Some(var_name) = cap.get(1) {
                    declared_vars.insert(var_name.as_str().to_string(), (line_num + 1, var_name.start()));
                }
            }
        }

        // 変数使用のパターン
        let usage_pattern = Regex::new(r"\b(\w+)\b")?;

        for (line_num, line) in content.lines().enumerate() {
            for cap in usage_pattern.captures_iter(line) {
                if let Some(var_name) = cap.get(1) {
                    declared_vars.remove(var_name.as_str());
                }
            }
        }

        // 残った変数は未使用
        for (var_name, (line, column)) in declared_vars {
            diagnostics.push(Diagnostic::new(
                DiagnosticLevel::Warning,
                "no-unused-vars".to_string(),
                format!("変数 '{}' は使用されていません", var_name),
                file_path.clone(),
                line,
                column + 1,
                var_name.len(),
            ).with_suggestion(format!("変数 '{}' を削除するか、使用してください", var_name)));
        }

        Ok(diagnostics)
    }
}

/// シャドウイング検出ルール
pub struct NoShadowingRule {
    config: Option<serde_json::Value>,
}

impl NoShadowingRule {
    pub fn new(config: Option<serde_json::Value>) -> Self {
        Self { config }
    }
}

impl LintRule for NoShadowingRule {
    fn name(&self) -> &str {
        "no-shadowing"
    }

    fn description(&self) -> &str {
        "変数のシャドウイングを検出します"
    }

    fn level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warning
    }

    fn check(&self, content: &str, file_path: &PathBuf) -> Result<Vec<Diagnostic>, Box<dyn std::error::Error>> {
        let mut diagnostics = Vec::new();
        let mut var_scope = HashMap::new();

        let var_pattern = Regex::new(r"let\s+(\w+)\s*=")?;

        for (line_num, line) in content.lines().enumerate() {
            for cap in var_pattern.captures_iter(line) {
                if let Some(var_name) = cap.get(1) {
                    let name = var_name.as_str();

                    if var_scope.contains_key(name) {
                        diagnostics.push(Diagnostic::new(
                            DiagnosticLevel::Warning,
                            "no-shadowing".to_string(),
                            format!("変数 '{}' は既に宣言されています", name),
                            file_path.clone(),
                            line_num + 1,
                            var_name.start() + 1,
                            name.len(),
                        ).with_help("異なる変数名を使用するか、スコープを変更してください".to_string()));
                    } else {
                        var_scope.insert(name.to_string(), line_num + 1);
                    }
                }
            }
        }

        Ok(diagnostics)
    }
}

/// インデント整合性ルール
pub struct ConsistentIndentationRule {
    config: Option<serde_json::Value>,
}

impl ConsistentIndentationRule {
    pub fn new(config: Option<serde_json::Value>) -> Self {
        Self { config }
    }
}

impl LintRule for ConsistentIndentationRule {
    fn name(&self) -> &str {
        "consistent-indentation"
    }

    fn description(&self) -> &str {
        "インデントの一貫性をチェックします"
    }

    fn level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warning
    }

    fn check(&self, content: &str, file_path: &PathBuf) -> Result<Vec<Diagnostic>, Box<dyn std::error::Error>> {
        let mut diagnostics = Vec::new();
        let mut indent_stack = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();

            // 空行はスキップ
            if trimmed.is_empty() {
                continue;
            }

            // ブロック開始
            if trimmed.starts_with("graph") ||
               trimmed.starts_with("node") ||
               trimmed.starts_with("edge") ||
               trimmed.starts_with("query") ||
               trimmed.starts_with("fn") ||
               trimmed.starts_with("if") ||
               trimmed.starts_with("for") ||
               trimmed.starts_with("while") {
                indent_stack.push(indent);
            }
            // ブロック終了
            else if trimmed.starts_with("}") {
                if let Some(expected_indent) = indent_stack.pop() {
                    if indent != expected_indent {
                        diagnostics.push(Diagnostic::new(
                            DiagnosticLevel::Warning,
                            "consistent-indentation".to_string(),
                            format!("インデントが一致しません。期待されるインデント: {}", expected_indent),
                            file_path.clone(),
                            line_num + 1,
                            1,
                            indent,
                        ));
                    }
                }
            }
            // ブロック内
            else if let Some(&expected_indent) = indent_stack.last() {
                let expected_with_extra = expected_indent + 4; // 4スペース追加
                if indent != expected_with_extra && indent != expected_indent {
                    diagnostics.push(Diagnostic::new(
                        DiagnosticLevel::Warning,
                        "consistent-indentation".to_string(),
                        format!("ブロック内のインデントが一致しません。期待されるインデント: {}", expected_with_extra),
                        file_path.clone(),
                        line_num + 1,
                        1,
                        indent,
                    ));
                }
            }
        }

        Ok(diagnostics)
    }
}

/// 末尾空白検出ルール
pub struct TrailingWhitespaceRule {
    config: Option<serde_json::Value>,
}

impl TrailingWhitespaceRule {
    pub fn new(config: Option<serde_json::Value>) -> Self {
        Self { config }
    }
}

impl LintRule for TrailingWhitespaceRule {
    fn name(&self) -> &str {
        "trailing-whitespace"
    }

    fn description(&self) -> &str {
        "行末の空白を検出します"
    }

    fn level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Info
    }

    fn check(&self, content: &str, file_path: &PathBuf) -> Result<Vec<Diagnostic>, Box<dyn std::error::Error>> {
        let mut diagnostics = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            if line.ends_with(' ') || line.ends_with('\t') {
                diagnostics.push(Diagnostic::new(
                    DiagnosticLevel::Info,
                    "trailing-whitespace".to_string(),
                    "行末に空白があります".to_string(),
                    file_path.clone(),
                    line_num + 1,
                    line.len(),
                    1,
                ).with_suggestion("行末の空白を削除してください".to_string()));
            }
        }

        Ok(diagnostics)
    }
}

/// セミコロン欠落検出ルール
pub struct MissingSemicolonsRule {
    config: Option<serde_json::Value>,
}

impl MissingSemicolonsRule {
    pub fn new(config: Option<serde_json::Value>) -> Self {
        Self { config }
    }
}

impl LintRule for MissingSemicolonsRule {
    fn name(&self) -> &str {
        "missing-semicolons"
    }

    fn description(&self) -> &str {
        "欠落したセミコロンを検出します"
    }

    fn level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Error
    }

    fn check(&self, content: &str, file_path: &PathBuf) -> Result<Vec<Diagnostic>, Box<dyn std::error::Error>> {
        let mut diagnostics = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // 空行、コメント、ブロック開始/終了はスキップ
            if trimmed.is_empty() ||
               trimmed.starts_with("//") ||
               trimmed.starts_with("/*") ||
               trimmed.starts_with("graph") ||
               trimmed.starts_with("node") ||
               trimmed.starts_with("edge") ||
               trimmed.starts_with("query") ||
               trimmed.starts_with("fn") ||
               trimmed.starts_with("if") ||
               trimmed.starts_with("for") ||
               trimmed.starts_with("while") ||
               trimmed.starts_with("{") ||
               trimmed.starts_with("}") {
                continue;
            }

            // 代入文や式でセミコロンが欠落している可能性
            if !trimmed.ends_with(';') &&
               !trimmed.ends_with('{') &&
               !trimmed.ends_with('}') &&
               !trimmed.ends_with(',') &&
               (trimmed.contains('=') || trimmed.contains("return")) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticLevel::Error,
                    "missing-semicolons".to_string(),
                    "セミコロンが欠落しています".to_string(),
                    file_path.clone(),
                    line_num + 1,
                    line.len(),
                    1,
                ).with_suggestion("文末にセミコロンを追加してください".to_string()));
            }
        }

        Ok(diagnostics)
    }
}

/// 命名規則チェックルール
pub struct NamingConventionRule {
    config: Option<serde_json::Value>,
}

impl NamingConventionRule {
    pub fn new(config: Option<serde_json::Value>) -> Self {
        Self { config }
    }
}

impl LintRule for NamingConventionRule {
    fn name(&self) -> &str {
        "naming-convention"
    }

    fn description(&self) -> &str {
        "命名規則をチェックします"
    }

    fn level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warning
    }

    fn check(&self, content: &str, file_path: &PathBuf) -> Result<Vec<Diagnostic>, Box<dyn std::error::Error>> {
        let mut diagnostics = Vec::new();

        // 変数宣言のチェック
        let var_pattern = Regex::new(r"let\s+(\w+)\s*=")?;

        for (line_num, line) in content.lines().enumerate() {
            for cap in var_pattern.captures_iter(line) {
                if let Some(var_name) = cap.get(1) {
                    let name = var_name.as_str();

                    // camelCase を推奨
                    if name.contains('_') {
                        diagnostics.push(Diagnostic::new(
                            DiagnosticLevel::Warning,
                            "naming-convention".to_string(),
                            format!("変数 '{}' は snake_case です。camelCase を推奨します", name),
                            file_path.clone(),
                            line_num + 1,
                            var_name.start() + 1,
                            name.len(),
                        ).with_help("変数名は camelCase を使用してください".to_string()));
                    }
                }
            }
        }

        Ok(diagnostics)
    }
}

/// 複雑さチェックルール
pub struct ComplexityRule {
    config: Option<serde_json::Value>,
}

impl ComplexityRule {
    pub fn new(config: Option<serde_json::Value>) -> Self {
        Self { config }
    }
}

impl LintRule for ComplexityRule {
    fn name(&self) -> &str {
        "complexity"
    }

    fn description(&self) -> &str {
        "関数の複雑さをチェックします"
    }

    fn level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Info
    }

    fn check(&self, content: &str, file_path: &PathBuf) -> Result<Vec<Diagnostic>, Box<dyn std::error::Error>> {
        let mut diagnostics = Vec::new();

        // 関数定義の検出
        let func_pattern = Regex::new(r"fn\s+(\w+)\s*\([^)]*\)\s*\{")?;
        let mut func_start = None;

        for (line_num, line) in content.lines().enumerate() {
            if func_pattern.is_match(line) {
                func_start = Some(line_num);
            }

            // 関数内の行数をカウント
            if let Some(start) = func_start {
                let line_count = line_num - start;

                // 30行を超える関数は複雑すぎる可能性
                if line_count > 30 && line.trim() == "}" {
                    diagnostics.push(Diagnostic::new(
                        DiagnosticLevel::Info,
                        "complexity".to_string(),
                        format!("関数が {} 行と長すぎます。関数を分割することを検討してください", line_count),
                        file_path.clone(),
                        start + 1,
                        1,
                        10,
                    ).with_help("大きな関数は小さな関数に分割することを検討してください".to_string()));
                    func_start = None;
                }
            }

            // 関数終了
            if line.trim() == "}" && func_start.is_some() {
                func_start = None;
            }
        }

        Ok(diagnostics)
    }
}
