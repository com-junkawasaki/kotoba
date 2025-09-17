//! 診断結果処理モジュール

use super::{Diagnostic, DiagnosticLevel, LintResult};
use std::collections::HashMap;
use std::io::Write;

/// 診断コレクター
#[derive(Debug)]
pub struct DiagnosticCollector {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticCollector {
    /// 新しいコレクターを作成
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    /// 診断を追加
    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// 複数の診断を追加
    pub fn extend(&mut self, diagnostics: Vec<Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    /// 診断を取得
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// 診断をクリア
    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }

    /// エラー数をカウント
    pub fn error_count(&self) -> usize {
        self.diagnostics.iter()
            .filter(|d| matches!(d.level, DiagnosticLevel::Error))
            .count()
    }

    /// 警告数をカウント
    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter()
            .filter(|d| matches!(d.level, DiagnosticLevel::Warning))
            .count()
    }

    /// 情報数をカウント
    pub fn info_count(&self) -> usize {
        self.diagnostics.iter()
            .filter(|d| matches!(d.level, DiagnosticLevel::Info))
            .count()
    }

    /// ヒント数をカウント
    pub fn hint_count(&self) -> usize {
        self.diagnostics.iter()
            .filter(|d| matches!(d.level, DiagnosticLevel::Hint))
            .count()
    }

    /// 診断があるかどうか
    pub fn has_diagnostics(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    /// エラーがあるかどうか
    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }

    /// ルールごとの診断数をカウント
    pub fn count_by_rule(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for diagnostic in &self.diagnostics {
            *counts.entry(diagnostic.code.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// レベルごとの診断数をカウント
    pub fn count_by_level(&self) -> HashMap<DiagnosticLevel, usize> {
        let mut counts = HashMap::new();
        for diagnostic in &self.diagnostics {
            *counts.entry(diagnostic.level).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for DiagnosticCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// 診断フィルタ
#[derive(Debug)]
pub struct DiagnosticFilter {
    pub include_rules: Vec<String>,
    pub exclude_rules: Vec<String>,
    pub min_level: Option<DiagnosticLevel>,
    pub max_level: Option<DiagnosticLevel>,
}

impl DiagnosticFilter {
    /// 新しいフィルタを作成
    pub fn new() -> Self {
        Self {
            include_rules: Vec::new(),
            exclude_rules: Vec::new(),
            min_level: None,
            max_level: None,
        }
    }

    /// ルールでフィルタ
    pub fn with_rules(mut self, rules: Vec<String>) -> Self {
        self.include_rules = rules;
        self
    }

    /// 除外ルールでフィルタ
    pub fn exclude_rules(mut self, rules: Vec<String>) -> Self {
        self.exclude_rules = rules;
        self
    }

    /// 最小レベルでフィルタ
    pub fn min_level(mut self, level: DiagnosticLevel) -> Self {
        self.min_level = Some(level);
        self
    }

    /// 最大レベルでフィルタ
    pub fn max_level(mut self, level: DiagnosticLevel) -> Self {
        self.max_level = Some(level);
        self
    }

    /// 診断をフィルタ
    pub fn filter(&self, diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
        diagnostics.iter().filter(|d| self.matches(d)).cloned().collect()
    }

    /// 診断がフィルタにマッチするか
    fn matches(&self, diagnostic: &Diagnostic) -> bool {
        // ルールフィルタ
        if !self.include_rules.is_empty() && !self.include_rules.contains(&diagnostic.code) {
            return false;
        }

        if self.exclude_rules.contains(&diagnostic.code) {
            return false;
        }

        // レベルフィルタ
        if let Some(min_level) = self.min_level {
            if !self.level_matches_or_above(diagnostic.level, min_level) {
                return false;
            }
        }

        if let Some(max_level) = self.max_level {
            if !self.level_matches_or_below(diagnostic.level, max_level) {
                return false;
            }
        }

        true
    }

    /// レベルが指定レベル以上か
    fn level_matches_or_above(&self, level: DiagnosticLevel, min_level: DiagnosticLevel) -> bool {
        match (level, min_level) {
            (DiagnosticLevel::Error, _) => true,
            (DiagnosticLevel::Warning, DiagnosticLevel::Warning | DiagnosticLevel::Info | DiagnosticLevel::Hint) => true,
            (DiagnosticLevel::Info, DiagnosticLevel::Info | DiagnosticLevel::Hint) => true,
            (DiagnosticLevel::Hint, DiagnosticLevel::Hint) => true,
            _ => false,
        }
    }

    /// レベルが指定レベル以下か
    fn level_matches_or_below(&self, level: DiagnosticLevel, max_level: DiagnosticLevel) -> bool {
        match (level, max_level) {
            (_, DiagnosticLevel::Error) => matches!(level, DiagnosticLevel::Error),
            (_, DiagnosticLevel::Warning) => matches!(level, DiagnosticLevel::Error | DiagnosticLevel::Warning),
            (_, DiagnosticLevel::Info) => matches!(level, DiagnosticLevel::Error | DiagnosticLevel::Warning | DiagnosticLevel::Info),
            (_, DiagnosticLevel::Hint) => true,
        }
    }
}

impl Default for DiagnosticFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// 診断サマライザー
#[derive(Debug)]
pub struct DiagnosticSummarizer {
    results: Vec<LintResult>,
}

impl DiagnosticSummarizer {
    /// 新しいサマライザーを作成
    pub fn new(results: Vec<LintResult>) -> Self {
        Self { results }
    }

    /// 全体の統計を取得
    pub fn overall_stats(&self) -> DiagnosticStats {
        let mut stats = DiagnosticStats::default();

        for result in &self.results {
            stats.files_checked += 1;
            stats.total_diagnostics += result.diagnostics.len();
            stats.error_count += result.error_count;
            stats.warning_count += result.warning_count;
            stats.total_duration_ms += result.duration_ms;
        }

        stats
    }

    /// ファイルごとの統計を取得
    pub fn file_stats(&self) -> Vec<FileDiagnosticStats> {
        self.results.iter().map(|result| {
            FileDiagnosticStats {
                file_path: result.file_path.clone(),
                diagnostic_count: result.diagnostics.len(),
                error_count: result.error_count,
                warning_count: result.warning_count,
                duration_ms: result.duration_ms,
            }
        }).collect()
    }

    /// ルールごとの統計を取得
    pub fn rule_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();

        for result in &self.results {
            for diagnostic in &result.diagnostics {
                *stats.entry(diagnostic.code.clone()).or_insert(0) += 1;
            }
        }

        stats
    }
}

/// 診断統計
#[derive(Debug, Clone, Default)]
pub struct DiagnosticStats {
    pub files_checked: usize,
    pub total_diagnostics: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub total_duration_ms: u64,
}

/// ファイルごとの診断統計
#[derive(Debug, Clone)]
pub struct FileDiagnosticStats {
    pub file_path: std::path::PathBuf,
    pub diagnostic_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub duration_ms: u64,
}

/// 診断エクスポーター
pub struct DiagnosticExporter;

impl DiagnosticExporter {
    /// JSON形式でエクスポート
    pub fn to_json(results: &[LintResult]) -> Result<String, Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(results)?;
        Ok(json)
    }

    /// SARIF形式でエクスポート
    pub fn to_sarif(results: &[LintResult]) -> Result<String, Box<dyn std::error::Error>> {
        let mut sarif = serde_json::json!({
            "version": "2.1.0",
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "kotoba-linter",
                        "version": env!("CARGO_PKG_VERSION"),
                        "informationUri": "https://github.com/jun784/kotoba"
                    }
                },
                "results": []
            }]
        });

        let sarif_results = sarif["runs"][0]["results"].as_array_mut().unwrap();

        for result in results {
            for diagnostic in &result.diagnostics {
                let sarif_result = serde_json::json!({
                    "ruleId": diagnostic.code,
                    "level": match diagnostic.level {
                        DiagnosticLevel::Error => "error",
                        DiagnosticLevel::Warning => "warning",
                        _ => "note"
                    },
                    "message": {
                        "text": diagnostic.message
                    },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": diagnostic.file_path.to_string_lossy()
                            },
                            "region": {
                                "startLine": diagnostic.line,
                                "startColumn": diagnostic.column,
                                "endColumn": diagnostic.column + diagnostic.length
                            }
                        }
                    }]
                });

                if let Some(suggestion) = &diagnostic.suggestion {
                    if let Some(fixes) = sarif_result["fixes"].as_array_mut() {
                        fixes.push(serde_json::json!({
                            "description": {
                                "text": suggestion
                            }
                        }));
                    }
                }

                sarif_results.push(sarif_result);
            }
        }

        Ok(serde_json::to_string_pretty(&sarif)?)
    }

    /// JUnit XML形式でエクスポート
    pub fn to_junit_xml(results: &[LintResult]) -> Result<String, Box<dyn std::error::Error>> {
        let mut xml = String::new();
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push_str("\n<testsuites>\n");

        for result in results {
            xml.push_str(&format!(
                r#"  <testsuite name="{}" tests="{}" failures="{}" time="{}">"#,
                result.file_path.display(),
                result.diagnostics.len(),
                result.error_count,
                result.duration_ms as f64 / 1000.0
            ));

            for diagnostic in &result.diagnostics {
                xml.push_str(&format!(
                    r#"
    <testcase name="{}" time="0">
      <failure message="{}">{}</failure>
    </testcase>"#,
                    diagnostic.code,
                    diagnostic.message,
                    diagnostic.message
                ));
            }

            xml.push_str("\n  </testsuite>\n");
        }

        xml.push_str("</testsuites>\n");
        Ok(xml)
    }
}
