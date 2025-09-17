//! テストレポート出力モジュール

use super::{TestSuite, TestResult};
use std::io::{self, Write};
use colored::*;

/// レポート形式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// 人間に読みやすい形式
    Pretty,
    /// JSON形式
    Json,
    /// JUnit XML形式
    Junit,
    /// TAP形式
    Tap,
}

/// レポートライター
#[derive(Debug)]
pub struct Reporter {
    format: ReportFormat,
    writer: Box<dyn Write>,
}

impl Reporter {
    /// 新しいレポーターを作成
    pub fn new(format: ReportFormat) -> Self {
        Self {
            format,
            writer: Box::new(io::stdout()),
        }
    }

    /// ファイルライターを作成
    pub fn with_file(format: ReportFormat, file_path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::create(file_path)?;
        Ok(Self {
            format,
            writer: Box::new(file),
        })
    }

    /// テストスイートをレポート
    pub fn report_suites(&mut self, suites: &[TestSuite]) -> Result<(), Box<dyn std::error::Error>> {
        match self.format {
            ReportFormat::Pretty => self.report_pretty(suites),
            ReportFormat::Json => self.report_json(suites),
            ReportFormat::Junit => self.report_junit(suites),
            ReportFormat::Tap => self.report_tap(suites),
        }
    }

    /// Pretty形式でレポート
    fn report_pretty(&mut self, suites: &[TestSuite]) -> Result<(), Box<dyn std::error::Error>> {
        let total_tests = suites.iter().map(|s| s.total_count()).sum::<usize>();
        let passed_tests = suites.iter().map(|s| s.passed_count()).sum::<usize>();
        let failed_tests = suites.iter().map(|s| s.failed_count()).sum::<usize>();
        let skipped_tests = suites.iter().map(|s| s.skipped_count()).sum::<usize>();

        // ヘッダー
        writeln!(self.writer, "{}", "Kotoba Test Runner".bold())?;
        writeln!(self.writer, "{}", "==================".bold())?;
        writeln!(self.writer)?;

        // 各スイートの結果
        for suite in suites {
            self.report_suite_pretty(suite)?;
        }

        // サマリー
        writeln!(self.writer)?;
        writeln!(self.writer, "{}", "Summary".bold())?;
        writeln!(self.writer, "{}", "=======".bold())?;
        writeln!(self.writer, "Suites: {}", suites.len())?;
        writeln!(self.writer, "Tests: {}", total_tests)?;
        writeln!(self.writer, "Passed: {}", passed_tests.to_string().green())?;
        writeln!(self.writer, "Failed: {}", failed_tests.to_string().red())?;
        writeln!(self.writer, "Skipped: {}", skipped_tests.to_string().yellow())?;

        let success_rate = if total_tests > 0 {
            (passed_tests as f64 / total_tests as f64) * 100.0
        } else {
            0.0
        };
        writeln!(self.writer, "Success rate: {:.1}%", success_rate)?;

        if failed_tests > 0 {
            writeln!(self.writer, "{}", "\n❌ Some tests failed".red().bold())?;
        } else {
            writeln!(self.writer, "{}", "\n✅ All tests passed!".green().bold())?;
        }

        Ok(())
    }

    /// スイートをPretty形式でレポート
    fn report_suite_pretty(&mut self, suite: &TestSuite) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(self.writer, "{}", suite.name.bold())?;

        for test_case in &suite.test_cases {
            let status = match test_case.result {
                TestResult::Passed => "✓".green(),
                TestResult::Failed => "✗".red(),
                TestResult::Skipped => "○".yellow(),
                TestResult::Pending => "○".cyan(),
                TestResult::Timeout => "⏰".red(),
            };

            write!(self.writer, "  {} {}", status, test_case.name)?;

            if let Some(error) = &test_case.error_message {
                write!(self.writer, " - {}", error.red())?;
            }

            if test_case.duration.as_millis() > 0 {
                write!(self.writer, " ({:.2}ms)", test_case.duration.as_secs_f64() * 1000.0)?;
            }

            writeln!(self.writer)?;
        }

        let passed = suite.passed_count();
        let failed = suite.failed_count();
        let total = suite.total_count();

        writeln!(self.writer, "  {} passed, {} failed, {} total", passed, failed, total)?;
        writeln!(self.writer)?;

        Ok(())
    }

    /// JSON形式でレポート
    fn report_json(&mut self, suites: &[TestSuite]) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(suites)?;
        writeln!(self.writer, "{}", json)?;
        Ok(())
    }

    /// JUnit XML形式でレポート
    fn report_junit(&mut self, suites: &[TestSuite]) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(self.writer, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
        writeln!(self.writer, r#"<testsuites>"#)?;

        for suite in suites {
            let tests = suite.total_count();
            let failures = suite.failed_count();
            let skipped = suite.skipped_count();

            writeln!(
                self.writer,
                r#"  <testsuite name="{}" tests="{}" failures="{}" skipped="{}">"#,
                suite.name, tests, failures, skipped
            )?;

            for test_case in &suite.test_cases {
                write!(self.writer, r#"    <testcase name="{}""#, test_case.name)?;

                if test_case.duration.as_millis() > 0 {
                    write!(self.writer, r#" time="{}""#, test_case.duration.as_secs_f64())?;
                }

                match test_case.result {
                    TestResult::Failed => {
                        writeln!(self.writer, r#">"#)?;
                        if let Some(error) = &test_case.error_message {
                            writeln!(self.writer, r#"      <failure message="{}">{}</failure>"#, error, error)?;
                        }
                        writeln!(self.writer, r#"    </testcase>"#)?;
                    }
                    TestResult::Skipped => {
                        writeln!(self.writer, r#">"#)?;
                        writeln!(self.writer, r#"      <skipped/>"#)?;
                        writeln!(self.writer, r#"    </testcase>"#)?;
                    }
                    _ => {
                        writeln!(self.writer, r#"/>"#)?;
                    }
                }
            }

            writeln!(self.writer, r#"  </testsuite>"#)?;
        }

        writeln!(self.writer, r#"</testsuites>"#)?;
        Ok(())
    }

    /// TAP形式でレポート
    fn report_tap(&mut self, suites: &[TestSuite]) -> Result<(), Box<dyn std::error::Error>> {
        let total_tests = suites.iter().map(|s| s.total_count()).sum::<usize>();

        writeln!(self.writer, "1..{}", total_tests)?;
        let mut test_number = 1;

        for suite in suites {
            for test_case in &suite.test_cases {
                let status = match test_case.result {
                    TestResult::Passed => "ok",
                    TestResult::Failed => "not ok",
                    TestResult::Skipped => "ok", // TAPではskipはok扱い
                    _ => "ok",
                };

                write!(self.writer, "{} {} - {}", status, test_number, test_case.name)?;

                if let Some(error) = &test_case.error_message {
                    write!(self.writer, " # {}", error)?;
                }

                if test_case.result == TestResult::Skipped {
                    write!(self.writer, " # SKIP")?;
                }

                writeln!(self.writer)?;
                test_number += 1;
            }
        }

        Ok(())
    }
}

impl Default for Reporter {
    fn default() -> Self {
        Self::new(ReportFormat::Pretty)
    }
}

/// コンソールレポーター
pub struct ConsoleReporter;

impl ConsoleReporter {
    /// リアルタイムでテスト結果を表示
    pub fn print_test_result(test_case: &super::TestCase) {
        let status = match test_case.result {
            TestResult::Passed => "✓".green(),
            TestResult::Failed => "✗".red(),
            TestResult::Skipped => "○".yellow(),
            TestResult::Pending => "○".cyan(),
            TestResult::Timeout => "⏰".red(),
        };

        print!("{} {} ", status, test_case.name);

        if let Some(error) = &test_case.error_message {
            print!("- {}", error.red());
        }

        if test_case.duration.as_millis() > 0 {
            print!(" ({:.2}ms)", test_case.duration.as_secs_f64() * 1000.0);
        }

        println!();
    }

    /// スイートの開始を表示
    pub fn print_suite_start(suite: &TestSuite) {
        println!("{}", suite.name.bold());
    }

    /// スイートの終了を表示
    pub fn print_suite_end(suite: &TestSuite) {
        let passed = suite.passed_count();
        let failed = suite.failed_count();
        let total = suite.total_count();

        println!("  {} passed, {} failed, {} total", passed, failed, total);
    }
}

/// 統計レポーター
pub struct StatsReporter;

impl StatsReporter {
    /// 詳細な統計を表示
    pub fn print_detailed_stats(suites: &[TestSuite]) {
        use std::collections::HashMap;

        let mut file_stats = HashMap::new();
        let mut result_stats = HashMap::new();

        for suite in suites {
            file_stats.insert(suite.file_path.clone(), suite.total_count());

            for test_case in &suite.test_cases {
                *result_stats.entry(test_case.result).or_insert(0) += 1;
            }
        }

        println!("\n{}", "Detailed Statistics".bold());
        println!("{}", "==================".bold());

        println!("\n{}", "By Result:".bold());
        for (result, count) in result_stats.iter() {
            let result_str = match result {
                TestResult::Passed => "Passed",
                TestResult::Failed => "Failed",
                TestResult::Skipped => "Skipped",
                TestResult::Pending => "Pending",
                TestResult::Timeout => "Timeout",
            };
            println!("  {}: {}", result_str, count);
        }

        println!("\n{}", "By File:".bold());
        for (file_path, count) in file_stats.iter() {
            println!("  {}: {} tests", file_path.display(), count);
        }
    }

    /// パフォーマンス統計を表示
    pub fn print_performance_stats(suites: &[TestSuite]) {
        let mut total_duration = std::time::Duration::default();
        let mut test_durations = Vec::new();

        for suite in suites {
            for test_case in &suite.test_cases {
                total_duration += test_case.duration;
                test_durations.push(test_case.duration);
            }
        }

        if test_durations.is_empty() {
            return;
        }

        // ソートしてパーセンタイルを計算
        test_durations.sort();

        let p50 = test_durations[test_durations.len() / 2];
        let p90 = test_durations[(test_durations.len() as f64 * 0.9) as usize];
        let p95 = test_durations[(test_durations.len() as f64 * 0.95) as usize];
        let max = *test_durations.last().unwrap();

        println!("\n{}", "Performance Statistics".bold());
        println!("{}", "======================".bold());
        println!("Total duration: {:.2}s", total_duration.as_secs_f64());
        println!("Average: {:.2}ms", total_duration.as_millis() as f64 / test_durations.len() as f64);
        println!("Median (P50): {:.2}ms", p50.as_secs_f64() * 1000.0);
        println!("P90: {:.2}ms", p90.as_secs_f64() * 1000.0);
        println!("P95: {:.2}ms", p95.as_secs_f64() * 1000.0);
        println!("Max: {:.2}ms", max.as_secs_f64() * 1000.0);
    }
}

/// 便利関数
pub fn print_summary(suites: &[TestSuite]) {
    let total_tests = suites.iter().map(|s| s.total_count()).sum::<usize>();
    let passed_tests = suites.iter().map(|s| s.passed_count()).sum::<usize>();
    let failed_tests = suites.iter().map(|s| s.failed_count()).sum::<usize>();

    println!("\n{}", "Test Summary".bold());
    println!("Tests: {}", total_tests);
    println!("Passed: {}", passed_tests.to_string().green());
    println!("Failed: {}", failed_tests.to_string().red());

    if failed_tests > 0 {
        println!("{}", "❌ Some tests failed".red().bold());
        std::process::exit(1);
    } else {
        println!("{}", "✅ All tests passed!".green().bold());
    }
}
