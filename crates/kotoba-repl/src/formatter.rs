//! 出力フォーマッティングモジュール

use super::ExecutionResult;
use colored::*;

/// 出力フォーマッター
pub struct OutputFormatter;

impl OutputFormatter {
    /// 実行結果をフォーマットして表示
    pub fn format_and_print(result: &ExecutionResult) {
        if result.is_success() {
            Self::print_success(result);
        } else {
            Self::print_error(result);
        }

        // 実行時間を表示（時間がかかった場合）
        if result.duration.as_millis() > 100 {
            println!("{}", format!("({:.2}ms)", result.duration.as_secs_f64() * 1000.0).dimmed());
        }
    }

    /// 成功結果を表示
    fn print_success(result: &ExecutionResult) {
        if let Some(output) = &result.output {
            if !output.is_empty() {
                // 複数行の場合はインデントを付けて表示
                if output.contains('\n') {
                    println!("{}", "Output:".green().bold());
                    for line in output.lines() {
                        println!("  {}", line.green());
                    }
                } else {
                    println!("{}", output.green());
                }
            }
        }
    }

    /// エラー結果を表示
    fn print_error(result: &ExecutionResult) {
        if let Some(error) = &result.error {
            println!("{}", "Error:".red().bold());
            println!("  {}", error.red());

            // エラーの詳細を表示
            if error.contains("undefined") {
                println!("  {}", "Hint: Check variable names and function definitions".yellow().dimmed());
            } else if error.contains("syntax") {
                println!("  {}", "Hint: Check syntax and parentheses".yellow().dimmed());
            }
        }
    }

    /// ウェルカムメッセージを表示
    pub fn print_welcome() {
        println!("{}", "=".repeat(50).cyan());
        println!("{}", "Welcome to Kotoba REPL!".bold().cyan());
        println!("{}", "=".repeat(50).cyan());
        println!();
        println!("Type {} for help, {} to quit", ".help".green(), ".exit".red());
        println!();
    }

    /// グッドバイメッセージを表示
    pub fn print_goodbye() {
        println!();
        println!("{}", "Goodbye! 👋".green().bold());
    }

    /// プロンプトを表示
    pub fn format_prompt(multiline: bool) -> String {
        if multiline {
            ".....> ".cyan().to_string()
        } else {
            "kotoba> ".green().to_string()
        }
    }

    /// ヘルプを表示
    pub fn print_help() {
        println!("{}", "Kotoba REPL Help".bold().cyan());
        println!("{}", "================".cyan());

        println!();
        println!("{}", "Commands:".bold());
        println!("  {}  - Show this help", ".help".green());
        println!("  {} - Show execution history", ".history".green());
        println!("  {}   - Show defined variables", ".vars".green());
        println!("  {}   - Clear session", ".clear".green());
        println!("  {}   - Load and execute file", r#".load <file>.green()"#);
        println!("  {}   - Save history to file", r#".save <file>.green()"#);
        println!("  {} - Evaluate code", r#".eval <code>.green()"#);
        println!("  {}   - Exit REPL", ".exit".green());

        println!();
        println!("{}", "Kotoba Language:".bold());
        println!("  {}          - Variable declaration", "let x = 42".yellow());
        println!("  {}            - Expression evaluation", "x + 5".yellow());
        println!("  {} - Graph definition", r#"graph mygraph { ... }"#.yellow());
        println!("  {}  - Query definition", r#"query myquery { ... }"#.yellow());

        println!();
        println!("{}", "Examples:".bold());
        println!("  {}", r#"let name = "Alice""#.dimmed());
        println!("  {}", "let age = 25".dimmed());
        println!("  {}", r#"name + " is " + age"#.dimmed());
        println!("  {}", r#"graph users { node user { name: name, age: age } }"#.dimmed());
    }

    /// 履歴を表示
    pub fn print_history(history: &[ExecutionResult]) {
        if history.is_empty() {
            println!("No history available.");
            return;
        }

        println!("{}", "Execution History".bold().cyan());
        println!("{}", "================".cyan());

        for (i, result) in history.iter().enumerate() {
            let status = if result.is_success() {
                "✓".green()
            } else {
                "✗".red()
            };

            let time_str = format!("{:.2}ms", result.duration.as_secs_f64() * 1000.0);
            println!("{}. {} {} {}", i + 1, status, result.code.dimmed(), time_str.dimmed());
        }
    }

    /// 変数を表示
    pub fn print_variables(variables: &std::collections::HashMap<String, String>) {
        if variables.is_empty() {
            println!("No variables defined.");
            return;
        }

        println!("{}", "Defined Variables".bold().cyan());
        println!("{}", "=================".cyan());

        for (name, value) in variables {
            println!("  {} = {}", name.green(), value.yellow());
        }
    }

    /// 統計情報を表示
    pub fn print_stats(session_id: &str, command_count: usize, variable_count: usize, duration: std::time::Duration) {
        println!();
        println!("{}", "Session Statistics".bold().cyan());
        println!("{}", "==================".cyan());
        println!("Session ID: {}", session_id.dimmed());
        println!("Commands executed: {}", command_count);
        println!("Variables defined: {}", variable_count);
        println!("Session duration: {:.2}s", duration.as_secs_f64());
    }
}

/// テーマ設定
pub struct Theme {
    pub success_color: Color,
    pub error_color: Color,
    pub info_color: Color,
    pub prompt_color: Color,
    pub code_color: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            success_color: Color::Green,
            error_color: Color::Red,
            info_color: Color::Blue,
            prompt_color: Color::Cyan,
            code_color: Color::Yellow,
        }
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self::default()
    }

    pub fn light() -> Self {
        Self {
            success_color: Color::Green,
            error_color: Color::Red,
            info_color: Color::Blue,
            prompt_color: Color::Black,
            code_color: Color::Magenta,
        }
    }
}

/// 色付き文字列の拡張
pub trait ColoredString {
    fn with_color(self, color: Color) -> colored::ColoredString;
}

impl ColoredString for &str {
    fn with_color(self, color: Color) -> colored::ColoredString {
        match color {
            Color::Red => self.red(),
            Color::Green => self.green(),
            Color::Blue => self.blue(),
            Color::Cyan => self.cyan(),
            Color::Yellow => self.yellow(),
            Color::Magenta => self.magenta(),
            Color::Black => self.black(),
            Color::White => self.white(),
        }
    }
}

impl ColoredString for String {
    fn with_color(self, color: Color) -> colored::ColoredString {
        self.as_str().with_color(color)
    }
}

/// 色定義
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Red,
    Green,
    Blue,
    Cyan,
    Yellow,
    Magenta,
    Black,
    White,
}