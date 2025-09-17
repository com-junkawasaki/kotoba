//! Kotoba Test Runner

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

// モジュール宣言（後で実装）
pub mod config;
pub mod runner;
pub mod assertions;
pub mod reporter;
pub mod coverage;

/// テスト結果の状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestResult {
    Passed,
    Failed,
    Skipped,
    Pending,
    Timeout,
}

impl std::fmt::Display for TestResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestResult::Passed => write!(f, "PASS"),
            TestResult::Failed => write!(f, "FAIL"),
            TestResult::Skipped => write!(f, "SKIP"),
            TestResult::Pending => write!(f, "PEND"),
            TestResult::Timeout => write!(f, "TIMEOUT"),
        }
    }
}

/// テストケース
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: PathBuf,
    pub line: usize,
    pub result: TestResult,
    pub duration: Duration,
    pub error_message: Option<String>,
}

impl TestCase {
    pub fn new(name: String, file_path: PathBuf, line: usize) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            file_path,
            line,
            result: TestResult::Pending,
            duration: Duration::default(),
            error_message: None,
        }
    }

    pub fn pass(&mut self, duration: Duration) {
        self.result = TestResult::Passed;
        self.duration = duration;
    }

    pub fn fail(&mut self, error_message: String, duration: Duration) {
        self.result = TestResult::Failed;
        self.error_message = Some(error_message);
        self.duration = duration;
    }
}

/// テストスイート
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: PathBuf,
    pub test_cases: Vec<TestCase>,
    pub duration: Duration,
}

impl TestSuite {
    pub fn new(name: String, file_path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            file_path,
            test_cases: Vec::new(),
            duration: Duration::default(),
        }
    }

    pub fn add_test_case(&mut self, test_case: TestCase) {
        self.test_cases.push(test_case);
    }

    pub fn passed_count(&self) -> usize {
        self.test_cases.iter().filter(|tc| tc.result == TestResult::Passed).count()
    }

    pub fn failed_count(&self) -> usize {
        self.test_cases.iter().filter(|tc| tc.result == TestResult::Failed).count()
    }

    pub fn total_count(&self) -> usize {
        self.test_cases.len()
    }
}

/// テスト実行結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResult {
    pub id: String,
    pub test_suites: Vec<TestSuite>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub total_duration: Duration,
}

impl TestRunResult {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            test_suites: Vec::new(),
            start_time: chrono::Utc::now(),
            end_time: None,
            total_duration: Duration::default(),
        }
    }

    pub fn add_test_suite(&mut self, test_suite: TestSuite) {
        self.test_suites.push(test_suite);
    }

    pub fn complete(&mut self) {
        self.end_time = Some(chrono::Utc::now());
        if let Some(end_time) = self.end_time {
            self.total_duration = end_time.signed_duration_since(self.start_time).to_std().unwrap_or_default();
        }
    }

    pub fn total_tests(&self) -> usize {
        self.test_suites.iter().map(|suite| suite.total_count()).sum()
    }

    pub fn passed_tests(&self) -> usize {
        self.test_suites.iter().map(|suite| suite.passed_count()).sum()
    }

    pub fn failed_tests(&self) -> usize {
        self.test_suites.iter().map(|suite| suite.failed_count()).sum()
    }
}

/// テスト設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub timeout: u64,
    pub concurrency: usize,
    pub filter: Option<String>,
    pub verbose: bool,
    pub coverage: bool,
    pub exclude_patterns: Vec<String>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout: 30,
            concurrency: num_cpus::get(),
            filter: None,
            verbose: false,
            coverage: false,
            exclude_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "build".to_string(),
            ],
        }
    }
}

/// メインのテストランナー
#[derive(Debug)]
pub struct TestRunner {
    config: TestConfig,
}

impl TestRunner {
    pub fn new(config: TestConfig) -> Self {
        Self { config }
    }

    pub fn default() -> Self {
        Self::new(TestConfig::default())
    }

    pub async fn run_tests(&self, patterns: Vec<String>) -> Result<TestRunResult, Box<dyn std::error::Error>> {
        let mut result = TestRunResult::new();
        let start_time = std::time::Instant::now();

        // テストファイルの発見
        let test_files = self.discover_test_files(&patterns).await?;

        if test_files.is_empty() {
            println!("No test files found.");
            result.complete();
            return Ok(result);
        }

        println!("Found {} test files", test_files.len());

        // テストファイルの実行
        for test_file in test_files {
            let suite_result = self.run_test_file(&test_file).await?;
            result.add_test_suite(suite_result);
        }

        result.total_duration = start_time.elapsed();
        result.complete();

        Ok(result)
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

    async fn run_test_file(&self, test_file: &PathBuf) -> Result<TestSuite, Box<dyn std::error::Error>> {
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

        Ok(suite)
    }

    async fn extract_and_run_tests(&self, content: &str, file_path: &PathBuf) -> Result<Vec<TestCase>, Box<dyn std::error::Error>> {
        let mut test_cases = Vec::new();

        // シンプルなテスト関数検出
        let test_pattern = regex::Regex::new(r"(?m)^(test_\w+|describe|it)\s*\(\s*[\"']([^\"']+)[\"']")?;

        for (line_num, line) in content.lines().enumerate() {
            for cap in test_pattern.captures_iter(line) {
                if let Some(test_name) = cap.get(2) {
                    let mut test_case = TestCase::new(
                        test_name.as_str().to_string(),
                        file_path.clone(),
                        line_num + 1
                    );

                    // テストの実行
                    let start_time = std::time::Instant::now();
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

            let start_time = std::time::Instant::now();
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

/// 便利関数
pub async fn run_tests(patterns: Vec<String>) -> Result<TestRunResult, Box<dyn std::error::Error>> {
    let runner = TestRunner::default();
    runner.run_tests(patterns).await
}

// 各モジュールの再エクスポート（後で実装）
// pub use config::*;
// pub use runner::*;
// pub use assertions::*;
// pub use reporter::*;
// pub use coverage::*;
