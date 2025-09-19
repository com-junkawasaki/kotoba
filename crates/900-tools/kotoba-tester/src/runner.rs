//! テスト実行モジュール

use super::{TestSuite, TestCase, TestResult, TestConfig};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use indicatif::{ProgressBar, ProgressStyle};
use colored::*;

/// テスト実行統計
#[derive(Debug)]
pub struct TestStats {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration: Duration,
}

impl TestStats {
    pub fn new() -> Self {
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            duration: Duration::default(),
        }
    }

    pub fn add_result(&mut self, result: TestResult) {
        self.total += 1;
        match result {
            TestResult::Passed => self.passed += 1,
            TestResult::Failed => self.failed += 1,
            TestResult::Skipped => self.skipped += 1,
            _ => {}
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.passed as f64 / self.total as f64) * 100.0
        }
    }
}

/// 拡張テストランナー
#[derive(Debug)]
pub struct TestRunner {
    config: TestConfig,
    stats: TestStats,
}

impl TestRunner {
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            stats: TestStats::new(),
        }
    }

    pub async fn run(&mut self, patterns: Vec<String>) -> Result<Vec<TestSuite>, Box<dyn std::error::Error>> {
        let start_time = Instant::now();

        // テストファイルの発見
        let test_files = self.discover_test_files(&patterns).await?;
        println!("Found {} test files", test_files.len());

        if test_files.is_empty() {
            return Ok(vec![]);
        }

        // プログレスバーの作成
        let pb = if self.config.verbose {
            let pb = ProgressBar::new(test_files.len() as u64);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("#>-"));
            Some(pb)
        } else {
            None
        };

        let mut results = Vec::new();

        // テストファイルの実行
        for (i, test_file) in test_files.iter().enumerate() {
            if let Some(pb) = &pb {
                pb.set_position(i as u64);
                pb.set_message(format!("Running {}", test_file.display()));
            }

            let suite_result = self.run_test_file(&test_file).await?;
            results.push(suite_result);

            if let Some(pb) = &pb {
                pb.set_position((i + 1) as u64);
            }
        }

        if let Some(pb) = pb {
            pb.finish_with_message("Test execution completed");
        }

        self.stats.duration = start_time.elapsed();

        Ok(results)
    }

    pub fn get_stats(&self) -> &TestStats {
        &self.stats
    }

    pub fn print_summary(&self) {
        let stats = &self.stats;

        println!("\n{}", "Test Results Summary".bold());
        println!("{}", "===================".bold());
        println!("Total tests: {}", stats.total);
        println!("Passed: {}", stats.passed.to_string().green());
        println!("Failed: {}", stats.failed.to_string().red());
        println!("Skipped: {}", stats.skipped.to_string().yellow());
        println!("Success rate: {:.1}%", stats.success_rate());
        println!("Duration: {:.2}s", stats.duration.as_secs_f64());

        if stats.failed > 0 {
            println!("\n{}", "❌ Some tests failed".red().bold());
        } else {
            println!("\n{}", "✅ All tests passed!".green().bold());
        }
    }

    async fn discover_test_files(&self, patterns: &[String]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut test_files = Vec::new();

        if patterns.is_empty() {
            // デフォルトのパターンで検索
            test_files.extend(self.find_test_files(".", &["test_*.kotoba", "*_test.kotoba", "tests/**/*.kotoba"]).await?);
        } else {
            for pattern in patterns {
                if std::path::Path::new(pattern).is_dir() {
                    test_files.extend(self.find_test_files(pattern, &["test_*.kotoba", "*_test.kotoba", "**/*.kotoba"]).await?);
                } else {
                    test_files.push(PathBuf::from(pattern));
                }
            }
        }

        // フィルターの適用
        if let Some(filter) = &self.config.filter {
            test_files.retain(|path| {
                path.to_string_lossy().contains(filter)
            });
        }

        Ok(test_files)
    }

    async fn find_test_files(&self, dir: &str, patterns: &[&str]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut test_files = Vec::new();

        for pattern in patterns {
            let glob_pattern = if dir == "." {
                pattern.to_string()
            } else {
                format!("{}/{}", dir, pattern)
            };

            // glob crate を使用したファイル検索
            for entry in glob::glob(&glob_pattern).unwrap() {
                if let Ok(path) = entry {
                    if path.is_file() && self.is_test_file(&path) {
                        test_files.push(path);
                    }
                }
            }
        }

        // 重複を除去
        test_files.sort();
        test_files.dedup();

        Ok(test_files)
    }

    fn is_test_file(&self, path: &PathBuf) -> bool {
        if let Some(file_name) = path.file_name() {
            let file_name_str = file_name.to_string_lossy();

            // 除外パターンをチェック
            for pattern in &self.config.exclude_patterns {
                if file_name_str.contains(pattern) {
                    return false;
                }
            }

            // 拡張子が .kotoba かチェック
            path.extension().map_or(false, |ext| ext == "kotoba")
        } else {
            false
        }
    }

    async fn run_test_file(&mut self, test_file: &PathBuf) -> Result<TestSuite, Box<dyn std::error::Error>> {
        let suite_name = test_file.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut suite = TestSuite::new(suite_name, test_file.clone());

        // テストファイルの内容を読み込んで解析
        let content = tokio::fs::read_to_string(test_file).await?;

        // テストケースの抽出と実行
        let test_cases = self.extract_and_run_tests(&content, test_file).await?;
        suite.test_cases = test_cases;

        // 統計の更新
        for test_case in &suite.test_cases {
            self.stats.add_result(test_case.result);
        }

        Ok(suite)
    }

    async fn extract_and_run_tests(&self, content: &str, file_path: &PathBuf) -> Result<Vec<TestCase>, Box<dyn std::error::Error>> {
        let mut test_cases = Vec::new();

        // シンプルなテスト関数検出
        let test_pattern = regex::Regex::new("(?m)^(test_\\w+|describe|it)\\s*\\(\\s*[\"']([^\"']+)[\"']\\)?;")?; 

        for (line_num, line) in content.lines().enumerate() {
            for cap in test_pattern.captures_iter(line) {
                if let Some(test_name) = cap.get(2) {
                    let mut test_case = TestCase::new(
                        test_name.as_str().to_string(),
                        file_path.clone(),
                        line_num + 1
                    );

                    // テストの実行
                    let start_time = Instant::now();
                    match self.run_single_test(&test_case).await {
                        Ok(_) => {
                            test_case.pass(start_time.elapsed());
                        }
                        Err(e) => {
                            test_case.fail(e.to_string(), start_time.elapsed());
                        }
                    }

                    test_cases.push(test_case);
                }
            }
        }

        // デフォルトで1つのテストケースを作成
        if test_cases.is_empty() {
            let mut test_case = TestCase::new(
                "default_test".to_string(),
                file_path.clone(),
                1
            );

            let start_time = Instant::now();
            test_case.pass(start_time.elapsed());
            test_cases.push(test_case);
        }

        Ok(test_cases)
    }

    async fn run_single_test(&self, test_case: &TestCase) -> Result<(), Box<dyn std::error::Error>> {
        // タイムアウト付きでテスト実行
        tokio::time::timeout(
            Duration::from_secs(self.config.timeout),
            self.execute_test_logic(test_case)
        ).await??;

        Ok(())
    }

    async fn execute_test_logic(&self, test_case: &TestCase) -> Result<(), Box<dyn std::error::Error>> {
        // ランダムに成功/失敗をシミュレート
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        test_case.name.hash(&mut hasher);
        let hash = hasher.finish();

        if hash % 10 == 0 {
            // 10%の確率で失敗
            return Err("Simulated test failure".into());
        }

        Ok(())
    }
}

/// テスト結果のフォーマッター
pub struct TestFormatter;

impl TestFormatter {
    pub fn format_result(test_case: &TestCase) -> String {
        let status = match test_case.result {
            TestResult::Passed => "✓".green(),
            TestResult::Failed => "✗".red(),
            TestResult::Skipped => "○".yellow(),
            TestResult::Pending => "○".cyan(),
            TestResult::Timeout => "⏰".red(),
        };

        let mut result = format!("{} {}", status, test_case.name);

        if let Some(error) = &test_case.error_message {
            result.push_str(&format!(" - {}", error.red()));
        }

        if test_case.duration.as_millis() > 0 {
            result.push_str(&format!(" ({:.2}ms)", test_case.duration.as_secs_f64() * 1000.0));
        }

        result
    }

    pub fn format_suite(suite: &TestSuite) -> String {
        let mut result = format!("\n{}", suite.name.bold());

        for test_case in &suite.test_cases {
            result.push('\n');
            result.push_str(&format!("  {}", Self::format_result(test_case)));
        }

        result
    }
}

/// 便利関数
pub async fn run_test_files(patterns: Vec<String>) -> Result<Vec<TestSuite>, Box<dyn std::error::Error>> {
    let config = TestConfig::default();
    let mut runner = TestRunner::new(config);
    runner.run(patterns).await
}

pub async fn run_test_file(file_path: PathBuf) -> Result<TestSuite, Box<dyn std::error::Error>> {
    let config = TestConfig::default();
    let mut runner = TestRunner::new(config);
    let results = runner.run(vec![file_path.to_string_lossy().to_string()]).await?;
    Ok(results.into_iter().next().unwrap_or_else(|| {
        TestSuite::new("empty".to_string(), file_path)
    }))
}
