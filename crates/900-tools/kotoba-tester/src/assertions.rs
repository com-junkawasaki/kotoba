//! アサーションライブラリモジュール

use super::{TestResult, TestCase};
use std::fmt::Debug;

/// アサーション結果
#[derive(Debug, Clone)]
pub struct AssertionResult {
    pub description: String,
    pub passed: bool,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub error_message: Option<String>,
    pub line: usize,
}

impl AssertionResult {
    pub fn pass(description: String, line: usize) -> Self {
        Self {
            description,
            passed: true,
            expected: None,
            actual: None,
            error_message: None,
            line,
        }
    }

    pub fn fail(
        description: String,
        expected: Option<String>,
        actual: Option<String>,
        error_message: Option<String>,
        line: usize,
    ) -> Self {
        Self {
            description,
            passed: false,
            expected,
            actual,
            error_message,
            line,
        }
    }
}

/// アサーションビルダー
#[derive(Debug)]
pub struct AssertionBuilder {
    test_case: TestCase,
    assertions: Vec<AssertionResult>,
}

impl AssertionBuilder {
    pub fn new(test_case: TestCase) -> Self {
        Self {
            test_case,
            assertions: Vec::new(),
        }
    }

    /// 等価性をチェック
    pub fn assert_equal<T: Debug + PartialEq>(mut self, expected: T, actual: T, description: &str, line: usize) -> Self {
        let result = if expected == actual {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some(format!("{:?}", expected)),
                Some(format!("{:?}", actual)),
                Some("Values are not equal".to_string()),
                line,
            )
        };
        self.assertions.push(result);
        self
    }

    /// trueであることをチェック
    pub fn assert_true(mut self, value: bool, description: &str, line: usize) -> Self {
        let result = if value {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some("true".to_string()),
                Some("false".to_string()),
                Some("Value is not true".to_string()),
                line,
            )
        };
        self.assertions.push(result);
        self
    }

    /// falseであることをチェック
    pub fn assert_false(mut self, value: bool, description: &str, line: usize) -> Self {
        let result = if !value {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some("false".to_string()),
                Some("true".to_string()),
                Some("Value is not false".to_string()),
                line,
            )
        };
        self.assertions.push(result);
        self
    }

    /// nullであることをチェック
    pub fn assert_null<T>(mut self, value: Option<T>, description: &str, line: usize) -> Self {
        let result = if value.is_none() {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some("null".to_string()),
                Some("not null".to_string()),
                Some("Value is not null".to_string()),
                line,
            )
        };
        self.assertions.push(result);
        self
    }

    /// nullでないことをチェック
    pub fn assert_not_null<T>(mut self, value: Option<T>, description: &str, line: usize) -> Self {
        let result = if value.is_some() {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some("not null".to_string()),
                Some("null".to_string()),
                Some("Value is null".to_string()),
                line,
            )
        };
        self.assertions.push(result);
        self
    }

    /// 文字列が含まれることをチェック
    pub fn assert_contains(mut self, haystack: &str, needle: &str, description: &str, line: usize) -> Self {
        let result = if haystack.contains(needle) {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some(format!("contains '{}'", needle)),
                Some(format!("'{}'", haystack)),
                Some("String does not contain expected substring".to_string()),
                line,
            )
        };
        self.assertions.push(result);
        self
    }

    /// 長さをチェック
    pub fn assert_length<T: Into<usize>>(mut self, collection: &[T], expected_length: usize, description: &str, line: usize) -> Self {
        let actual_length = collection.len();
        let result = if actual_length == expected_length {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some(expected_length.to_string()),
                Some(actual_length.to_string()),
                Some("Collection length does not match".to_string()),
                line,
            )
        };
        self.assertions.push(result);
        self
    }

    /// 数値の範囲をチェック
    pub fn assert_in_range<T: PartialOrd + Debug>(mut self, value: T, min: T, max: T, description: &str, line: usize) -> Self {
        let result = if value >= min && value <= max {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some(format!("between {:?} and {:?}", min, max)),
                Some(format!("{:?}", value)),
                Some("Value is not in expected range".to_string()),
                line,
            )
        };
        self.assertions.push(result);
        self
    }

    /// エラーが発生することをチェック
    pub fn assert_throws<F, E>(mut self, f: F, description: &str, line: usize) -> Self
    where
        F: FnOnce() -> Result<(), E>,
        E: Debug,
    {
        let result = match f() {
            Ok(_) => AssertionResult::fail(
                description.to_string(),
                Some("should throw".to_string()),
                Some("did not throw".to_string()),
                Some("Expected function to throw an error".to_string()),
                line,
            ),
            Err(_) => AssertionResult::pass(description.to_string(), line),
        };
        self.assertions.push(result);
        self
    }

    /// テストケースを完了
    pub fn finish(mut self) -> TestCase {
        // アサーション結果に基づいてテスト結果を決定
        let has_failures = self.assertions.iter().any(|a| !a.passed);

        if has_failures {
            let error_messages: Vec<String> = self.assertions
                .iter()
                .filter(|a| !a.passed)
                .filter_map(|a| a.error_message.clone())
                .collect();

            let combined_error = if error_messages.is_empty() {
                "Assertion failed".to_string()
            } else {
                error_messages.join("; ")
            };

            self.test_case.fail(combined_error, std::time::Duration::default());
        } else {
            self.test_case.pass(std::time::Duration::default());
        }

        self.test_case
    }

    /// 生のアサーション結果を取得
    pub fn get_assertions(&self) -> &[AssertionResult] {
        &self.assertions
    }
}

/// アサーション関数群
pub mod assert {
    use super::*;

    /// 等価性をチェック
    pub fn equal<T: Debug + PartialEq>(expected: T, actual: T, description: &str, line: usize) -> AssertionResult {
        if expected == actual {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some(format!("{:?}", expected)),
                Some(format!("{:?}", actual)),
                Some("Values are not equal".to_string()),
                line,
            )
        }
    }

    /// trueであることをチェック
    pub fn is_true(value: bool, description: &str, line: usize) -> AssertionResult {
        if value {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some("true".to_string()),
                Some("false".to_string()),
                Some("Value is not true".to_string()),
                line,
            )
        }
    }

    /// falseであることをチェック
    pub fn is_false(value: bool, description: &str, line: usize) -> AssertionResult {
        if !value {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some("false".to_string()),
                Some("true".to_string()),
                Some("Value is not false".to_string()),
                line,
            )
        }
    }

    /// nullであることをチェック
    pub fn is_null<T>(value: Option<T>, description: &str, line: usize) -> AssertionResult {
        if value.is_none() {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some("null".to_string()),
                Some("not null".to_string()),
                Some("Value is not null".to_string()),
                line,
            )
        }
    }

    /// 文字列が含まれることをチェック
    pub fn contains(haystack: &str, needle: &str, description: &str, line: usize) -> AssertionResult {
        if haystack.contains(needle) {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some(format!("contains '{}'", needle)),
                Some(format!("'{}'", haystack)),
                Some("String does not contain expected substring".to_string()),
                line,
            )
        }
    }

    /// 近似等価性をチェック（浮動小数点用）
    pub fn approx_equal(expected: f64, actual: f64, epsilon: f64, description: &str, line: usize) -> AssertionResult {
        if (expected - actual).abs() <= epsilon {
            AssertionResult::pass(description.to_string(), line)
        } else {
            AssertionResult::fail(
                description.to_string(),
                Some(format!("≈{}", expected)),
                Some(format!("{}", actual)),
                Some(format!("Values are not approximately equal (epsilon: {})", epsilon)),
                line,
            )
        }
    }
}

/// マクロ定義
#[macro_export]
macro_rules! assert_eq {
    ($expected:expr, $actual:expr) => {
        assert::equal($expected, $actual, &format!("{} == {}", stringify!($expected), stringify!($actual)), line!())
    };
}

#[macro_export]
macro_rules! assert_true {
    ($value:expr) => {
        assert::is_true($value, &format!("{} is true", stringify!($value)), line!())
    };
}

#[macro_export]
macro_rules! assert_false {
    ($value:expr) => {
        assert::is_false($value, &format!("{} is false", stringify!($value)), line!())
    };
}

#[macro_export]
macro_rules! assert_null {
    ($value:expr) => {
        assert::is_null($value, &format!("{} is null", stringify!($value)), line!())
    };
}

#[macro_export]
macro_rules! assert_contains {
    ($haystack:expr, $needle:expr) => {
        assert::contains($haystack, $needle, &format!("{} contains {}", stringify!($haystack), stringify!($needle)), line!())
    };
}

/// BDDスタイルのテスト記述
#[macro_export]
macro_rules! describe {
    ($name:expr, $body:block) => {
        println!("Running test suite: {}", $name);
        $body
    };
}

#[macro_export]
macro_rules! it {
    ($description:expr, $body:block) => {
        println!("  Running test: {}", $description);
        $body
    };
}

#[macro_export]
macro_rules! expect {
    ($value:expr) => {
        AssertionBuilder::new(TestCase::new($value.to_string(), std::path::PathBuf::from(file!()), line!()))
    };
}
