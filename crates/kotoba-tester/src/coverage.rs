//! カバレッジ収集モジュール

use super::{TestSuite, TestCase};
use std::collections::HashMap;
use std::path::PathBuf;

/// カバレッジレポート
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoverageReport {
    /// ファイルごとのカバレッジ
    pub file_coverage: HashMap<String, FileCoverage>,
    /// 全体の行カバレッジ率
    pub line_coverage: f64,
    /// 全体のブランチカバレッジ率
    pub branch_coverage: f64,
    /// 全体の関数カバレッジ率
    pub function_coverage: f64,
    /// 総行数
    pub total_lines: usize,
    /// カバーされた行数
    pub covered_lines: usize,
    /// 総関数数
    pub total_functions: usize,
    /// カバーされた関数数
    pub covered_functions: usize,
}

/// ファイルごとのカバレッジ
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileCoverage {
    /// ファイルパス
    pub file_path: String,
    /// 実行された行
    pub executed_lines: Vec<usize>,
    /// 総行数
    pub total_lines: usize,
    /// 実行された関数
    pub executed_functions: Vec<String>,
    /// 総関数数
    pub total_functions: usize,
    /// 行カバレッジ率
    pub line_coverage: f64,
    /// 関数カバレッジ率
    pub function_coverage: f64,
}

/// カバレッジコレクター
#[derive(Debug)]
pub struct CoverageCollector {
    file_coverage: HashMap<String, FileCoverage>,
}

impl CoverageCollector {
    /// 新しいコレクターを作成
    pub fn new() -> Self {
        Self {
            file_coverage: HashMap::new(),
        }
    }

    /// テストスイートからカバレッジを収集
    pub fn collect_from_suites(&mut self, suites: &[TestSuite]) -> Result<(), Box<dyn std::error::Error>> {
        for suite in suites {
            self.collect_from_suite(suite)?;
        }
        Ok(())
    }

    /// テストスイートからカバレッジを収集
    fn collect_from_suite(&mut self, suite: &TestSuite) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = suite.file_path.to_string_lossy().to_string();

        // ファイルの内容を読み込んで解析
        let content = std::fs::read_to_string(&suite.file_path)?;

        // 行数をカウント
        let total_lines = content.lines().count();

        // 関数を検出
        let functions = self.extract_functions(&content);
        let total_functions = functions.len();

        // テストケースから実行された関数を推定
        let mut executed_lines = Vec::new();
        let mut executed_functions: Vec<String> = Vec::new();

        for test_case in &suite.test_cases {
            if test_case.result == super::TestResult::Passed {
                // 成功したテストは対応する行を実行したと仮定
                executed_lines.push(test_case.line);

                // 関数名から実行された関数を推定
                if let Some(func_name) = self.find_function_at_line(&functions, test_case.line) {
                    if !executed_functions.iter().any(|f| f == func_name) {
                        executed_functions.push(func_name.to_string());
                    }
                }
            }
        }

        let line_coverage = if total_lines > 0 {
            (executed_lines.len() as f64 / total_lines as f64) * 100.0
        } else {
            0.0
        };

        let function_coverage = if total_functions > 0 {
            (executed_functions.len() as f64 / total_functions as f64) * 100.0
        } else {
            0.0
        };

        let file_coverage = FileCoverage {
            file_path: file_path.clone(),
            executed_lines,
            total_lines,
            executed_functions,
            total_functions,
            line_coverage,
            function_coverage,
        };

        self.file_coverage.insert(file_path, file_coverage);

        Ok(())
    }

    /// 関数を抽出
    fn extract_functions(&self, content: &str) -> Vec<(String, usize)> {
        let mut functions = Vec::new();

        // 関数定義のパターン
        let patterns = vec![
            regex::Regex::new(r"fn\s+(\w+)\s*\(").unwrap(),
            regex::Regex::new(r"test_\w+\s*\(").unwrap(),
        ];

        for (line_num, line) in content.lines().enumerate() {
            for pattern in &patterns {
                for cap in pattern.captures_iter(line) {
                    if let Some(func_name) = cap.get(1) {
                        functions.push((func_name.as_str().to_string(), line_num + 1));
                    }
                }
            }
        }

        functions
    }

    /// 指定行の関数を見つける
    fn find_function_at_line<'a>(&self, functions: &'a [(String, usize)], line: usize) -> Option<&'a str> {
        // 指定行に最も近い関数を見つける
        let mut closest_func = None;
        let mut min_distance = usize::MAX;

        for (func_name, func_line) in functions {
            let distance = if line >= *func_line {
                line - func_line
            } else {
                func_line - line
            };

            if distance < min_distance {
                min_distance = distance;
                closest_func = Some(func_name.as_str());
            }
        }

        closest_func
    }

    /// カバレッジレポートを生成
    pub fn generate_report(&self) -> CoverageReport {
        let mut total_lines = 0;
        let mut covered_lines = 0;
        let mut total_functions = 0;
        let mut covered_functions = 0;

        for file_coverage in self.file_coverage.values() {
            total_lines += file_coverage.total_lines;
            covered_lines += file_coverage.executed_lines.len();
            total_functions += file_coverage.total_functions;
            covered_functions += file_coverage.executed_functions.len();
        }

        let line_coverage = if total_lines > 0 {
            (covered_lines as f64 / total_lines as f64) * 100.0
        } else {
            0.0
        };

        let function_coverage = if total_functions > 0 {
            (covered_functions as f64 / total_functions as f64) * 100.0
        } else {
            0.0
        };

        CoverageReport {
            file_coverage: self.file_coverage.clone(),
            line_coverage,
            branch_coverage: 0.0, // 簡易版ではブランチカバレッジを計算しない
            function_coverage,
            total_lines,
            covered_lines,
            total_functions,
            covered_functions,
        }
    }
}

impl Default for CoverageCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// カバレッジレポーター
#[derive(Debug)]
pub struct CoverageReporter {
    format: CoverageFormat,
}

#[derive(Debug, Clone, Copy)]
pub enum CoverageFormat {
    /// コンソール出力
    Console,
    /// HTMLレポート
    Html,
    /// LCOV形式
    Lcov,
    /// JSON形式
    Json,
}

impl CoverageReporter {
    /// 新しいレポーターを作成
    pub fn new(format: CoverageFormat) -> Self {
        Self { format }
    }

    /// カバレッジレポートを出力
    pub fn report(&self, coverage_report: &CoverageReport) -> Result<(), Box<dyn std::error::Error>> {
        match self.format {
            CoverageFormat::Console => self.report_console(coverage_report),
            CoverageFormat::Html => self.report_html(coverage_report),
            CoverageFormat::Lcov => self.report_lcov(coverage_report),
            CoverageFormat::Json => self.report_json(coverage_report),
        }
    }

    /// コンソール形式でレポート
    fn report_console(&self, report: &CoverageReport) -> Result<(), Box<dyn std::error::Error>> {
        use colored::*;

        println!("\n{}", "Coverage Report".bold());
        println!("{}", "===============".bold());

        println!("Overall Coverage:");
        println!("  Lines: {:.1}% ({}/{})", report.line_coverage, report.covered_lines, report.total_lines);
        println!("  Functions: {:.1}% ({}/{})", report.function_coverage, report.covered_functions, report.total_functions);
        println!("  Branches: {:.1}%", report.branch_coverage);

        if !report.file_coverage.is_empty() {
            println!("\n{}", "File Coverage:".bold());
            for file_coverage in report.file_coverage.values() {
                let line_color = if file_coverage.line_coverage >= 80.0 {
                    file_coverage.line_coverage.to_string().green()
                } else if file_coverage.line_coverage >= 60.0 {
                    file_coverage.line_coverage.to_string().yellow()
                } else {
                    file_coverage.line_coverage.to_string().red()
                };

                println!("  {}: {}% lines, {}% functions",
                    file_coverage.file_path,
                    line_color,
                    file_coverage.function_coverage
                );
            }
        }

        Ok(())
    }

    /// HTML形式でレポート
    fn report_html(&self, report: &CoverageReport) -> Result<(), Box<dyn std::error::Error>> {
        // HTMLレポート生成（簡易版）
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Kotoba Coverage Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .summary {{ background: #f5f5f5; padding: 20px; border-radius: 5px; }}
        .file {{ margin: 10px 0; }}
        .high {{ color: green; }}
        .medium {{ color: orange; }}
        .low {{ color: red; }}
    </style>
</head>
<body>
    <h1>Kotoba Coverage Report</h1>
    <div class="summary">
        <h2>Overall Coverage</h2>
        <p>Lines: {:.1}% ({}/{})</p>
        <p>Functions: {:.1}% ({}/{})</p>
        <p>Branches: {:.1}%</p>
    </div>
    <h2>File Coverage</h2>
    {}
</body>
</html>"#,
            report.line_coverage, report.covered_lines, report.total_lines,
            report.function_coverage, report.covered_functions, report.total_functions,
            report.branch_coverage,
            self.generate_file_html(report)
        );

        std::fs::write("coverage-report.html", html)?;
        println!("HTML coverage report generated: coverage-report.html");

        Ok(())
    }

    /// ファイルごとのHTMLを生成
    fn generate_file_html(&self, report: &CoverageReport) -> String {
        let mut html = String::new();

        for file_coverage in report.file_coverage.values() {
            let coverage_class = if file_coverage.line_coverage >= 80.0 {
                "high"
            } else if file_coverage.line_coverage >= 60.0 {
                "medium"
            } else {
                "low"
            };

            html.push_str(&format!(
                r#"<div class="file">
    <strong>{}</strong><br>
    <span class="{}">Lines: {:.1}% ({}/{})</span><br>
    <span>Functions: {:.1}% ({}/{})</span>
</div>"#,
                file_coverage.file_path,
                coverage_class,
                file_coverage.line_coverage,
                file_coverage.executed_lines.len(),
                file_coverage.total_lines,
                file_coverage.function_coverage,
                file_coverage.executed_functions.len(),
                file_coverage.total_functions
            ));
        }

        html
    }

    /// LCOV形式でレポート
    fn report_lcov(&self, report: &CoverageReport) -> Result<(), Box<dyn std::error::Error>> {
        let mut lcov = String::new();

        for file_coverage in report.file_coverage.values() {
            lcov.push_str(&format!("SF:{}\n", file_coverage.file_path));

            // 関数情報
            for func_name in &file_coverage.executed_functions {
                lcov.push_str(&format!("FN:{},0,{}\n", 0, func_name));
            }

            // 関数カバレッジ
            lcov.push_str(&format!("FNF:{}\n", file_coverage.total_functions));
            lcov.push_str(&format!("FNH:{}\n", file_coverage.executed_functions.len()));

            // 行カバレッジ
            for &line in &file_coverage.executed_lines {
                lcov.push_str(&format!("DA:{},1\n", line));
            }

            lcov.push_str(&format!("LF:{}\n", file_coverage.total_lines));
            lcov.push_str(&format!("LH:{}\n", file_coverage.executed_lines.len()));

            lcov.push_str("end_of_record\n");
        }

        std::fs::write("coverage.lcov", lcov)?;
        println!("LCOV coverage report generated: coverage.lcov");

        Ok(())
    }

    /// JSON形式でレポート
    fn report_json(&self, report: &CoverageReport) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(report)?;
        std::fs::write("coverage.json", json)?;
        println!("JSON coverage report generated: coverage.json");
        Ok(())
    }
}

impl Default for CoverageReporter {
    fn default() -> Self {
        Self::new(CoverageFormat::Console)
    }
}

/// カバレッジフィルタ
#[derive(Debug)]
pub struct CoverageFilter {
    pub min_line_coverage: Option<f64>,
    pub min_function_coverage: Option<f64>,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

impl CoverageFilter {
    pub fn new() -> Self {
        Self {
            min_line_coverage: None,
            min_function_coverage: None,
            include_patterns: Vec::new(),
            exclude_patterns: vec![
                "test_*.kotoba".to_string(),
                "*_test.kotoba".to_string(),
            ],
        }
    }

    pub fn should_include_file(&self, file_path: &str) -> bool {
        // 除外パターンをチェック
        for pattern in &self.exclude_patterns {
            if file_path.contains(pattern.trim_end_matches("*.kotoba").trim_end_matches("*_test")) {
                return false;
            }
        }

        // インクルードパターンをチェック
        if self.include_patterns.is_empty() {
            return true;
        }

        for pattern in &self.include_patterns {
            if file_path.contains(pattern) {
                return true;
            }
        }

        false
    }

    pub fn should_include_coverage(&self, file_coverage: &FileCoverage) -> bool {
        if let Some(min_line) = self.min_line_coverage {
            if file_coverage.line_coverage < min_line {
                return false;
            }
        }

        if let Some(min_func) = self.min_function_coverage {
            if file_coverage.function_coverage < min_func {
                return false;
            }
        }

        true
    }
}

impl Default for CoverageFilter {
    fn default() -> Self {
        Self::new()
    }
}
