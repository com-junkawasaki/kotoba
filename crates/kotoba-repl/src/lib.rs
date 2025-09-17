//! Kotoba REPL - Interactive Shell

use std::path::PathBuf;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use colored::Colorize;

// モジュール宣言（後で実装）
pub mod config;
pub mod commands;
pub mod completer;
pub mod formatter;

/// REPLセッションの状態
#[derive(Debug, Clone)]
pub enum ReplState {
    Normal,
    Multiline,
    Help,
    Exiting,
}

/// REPLコマンドの種類
#[derive(Debug, Clone)]
pub enum ReplCommand {
    /// Kotobaコードの実行
    Execute(String),
    /// ヘルプ表示
    Help,
    /// 履歴表示
    History,
    /// 変数一覧表示
    Variables,
    /// クリア
    Clear,
    /// 終了
    Exit,
    /// ファイル読み込み
    Load(String),
    /// ファイル保存
    Save(String),
    /// 評価して終了
    Eval(String),
}

/// 実行結果
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub id: String,
    pub code: String,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration: std::time::Duration,
    pub timestamp: DateTime<Utc>,
}

impl ExecutionResult {
    pub fn new(code: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            code,
            output: None,
            error: None,
            duration: std::time::Duration::default(),
            timestamp: Utc::now(),
        }
    }

    pub fn success(mut self, output: String, duration: std::time::Duration) -> Self {
        self.output = Some(output);
        self.duration = duration;
        self
    }

    pub fn error(mut self, error: String, duration: std::time::Duration) -> Self {
        self.error = Some(error);
        self.duration = duration;
        self
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}

/// REPL設定
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReplConfig {
    pub prompt: String,
    pub multiline_prompt: String,
    pub history_file: Option<PathBuf>,
    pub max_history: usize,
    pub timeout: u64,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            prompt: "kotoba> ".to_string(),
            multiline_prompt: ".....> ".to_string(),
            history_file: dirs::home_dir().map(|h| h.join(".kotoba_repl_history")),
            max_history: 1000,
            timeout: 30,
        }
    }
}

/// REPLセッション
#[derive(Debug)]
pub struct ReplSession {
    id: String,
    state: ReplState,
    history: Vec<ExecutionResult>,
    variables: std::collections::HashMap<String, String>,
    config: ReplConfig,
    start_time: DateTime<Utc>,
}

impl ReplSession {
    pub fn new(config: ReplConfig) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            state: ReplState::Normal,
            history: Vec::new(),
            variables: std::collections::HashMap::new(),
            config,
            start_time: Utc::now(),
        }
    }

    pub fn default() -> Self {
        Self::new(ReplConfig::default())
    }

    pub async fn execute(&mut self, code: &str) -> Result<ExecutionResult, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        let result = ExecutionResult::new(code.to_string());

        // 簡易的なコマンド処理
        if code.trim().is_empty() {
            return Ok(result.success(String::new(), start_time.elapsed()));
        }

        if code.starts_with(".help") {
            let help = self.get_help_text();
            return Ok(result.success(help, start_time.elapsed()));
        }

        if code.starts_with(".exit") {
            self.state = ReplState::Exiting;
            return Ok(result.success("Goodbye!".to_string(), start_time.elapsed()));
        }

        // Kotobaコードの実行
        match self.execute_kotoba_code(code).await {
            Ok(output) => Ok(result.success(output, start_time.elapsed())),
            Err(e) => Ok(result.error(e.to_string(), start_time.elapsed())),
        }
    }

    async fn execute_kotoba_code(&mut self, code: &str) -> Result<String, Box<dyn std::error::Error>> {
        // 変数宣言の処理
        if code.contains("let ") {
            self.handle_variable_declaration(code)?;
            Ok(format!("Variable declared: {}", code))
        }
        // 式の評価
        else if code.contains("=") || code.contains("+") || code.contains("-") {
            let result = self.evaluate_expression(code)?;
            Ok(result)
        }
        else {
            Ok(format!("Executed: {}", code))
        }
    }

    pub fn handle_variable_declaration(&mut self, code: &str) -> Result<(), Box<dyn std::error::Error>> {
        let var_pattern = regex::Regex::new(r"let\s+(\w+)\s*=\s*(.+)")?;

        if let Some(cap) = var_pattern.captures(code) {
            if let (Some(var_name), Some(var_value)) = (cap.get(1), cap.get(2)) {
                let name = var_name.as_str().to_string();
                let value = var_value.as_str().trim().to_string();
                self.variables.insert(name, value);
            }
        }

        Ok(())
    }

    pub fn evaluate_expression(&self, code: &str) -> Result<String, Box<dyn std::error::Error>> {
        // 簡易的な式評価
        if code.contains("+") {
            let parts: Vec<&str> = code.split('+').collect();
            if parts.len() == 2 {
                let a: i32 = parts[0].trim().parse().unwrap_or(0);
                let b: i32 = parts[1].trim().parse().unwrap_or(0);
                return Ok(format!("{}", a + b));
            }
        }

        Ok(format!("Evaluated: {}", code))
    }

    pub fn get_help_text(&self) -> String {
        r#"
Kotoba REPL Commands:
  .help, .h           Show this help
  .exit, .quit        Exit REPL

Kotoba Language:
  let x = 42          Variable declaration
  x + 5              Expression evaluation

Examples:
  let name = "Alice"
  let age = 25
  name + " is " + age
"#.to_string()
    }

    pub fn get_history_text(&self) -> String {
        let mut text = "Execution History:\n".to_string();

        for (i, result) in self.history.iter().enumerate() {
            let status = if result.is_success() { "✓" } else { "✗" };
            text.push_str(&format!("{}. {} {}\n", i + 1, status, result.code));
        }

        text
    }

    pub fn get_variables_text(&self) -> String {
        let mut text = "Defined Variables:\n".to_string();

        for (name, value) in &self.variables {
            text.push_str(&format!("  {} = {}\n", name, value));
        }

        if self.variables.is_empty() {
            text.push_str("  (no variables defined)\n");
        }

        text
    }

    pub fn clear_session(&mut self) {
        self.history.clear();
        self.variables.clear();
    }

    pub async fn load_file(&self, filename: &str) -> Result<String, Box<dyn std::error::Error>> {
        let content = tokio::fs::read_to_string(filename).await?;
        Ok(content)
    }

    pub async fn save_history(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut content = String::new();

        for result in &self.history {
            content.push_str(&format!("// {}\n", result.timestamp));
            content.push_str(&format!("{}\n\n", result.code));
        }

        tokio::fs::write(filename, content).await?;
        Ok(())
    }

}

/// REPLマネージャー
#[derive(Debug)]
pub struct ReplManager {
    session: std::sync::Arc<Mutex<ReplSession>>,
    config: ReplConfig,
}

impl ReplManager {
    pub fn new(config: ReplConfig) -> Self {
        let session = std::sync::Arc::new(Mutex::new(ReplSession::new(config.clone())));
        Self { session, config }
    }

    pub fn default() -> Self {
        Self::new(ReplConfig::default())
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Welcome to Kotoba REPL!");
        println!("Type '.help' for help, '.exit' to quit.");
        println!();

        use rustyline::DefaultEditor;

        let mut rl = DefaultEditor::new()?;

        loop {
            let prompt = self.config.prompt.clone();
            let readline = rl.readline(&prompt);

            match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str())?;

                    if line.trim().is_empty() {
                        continue;
                    }

                    let result = self.session.lock().await.execute(&line).await?;

                    if result.is_success() {
                        if let Some(output) = &result.output {
                            if !output.is_empty() {
                                println!("{}", output.green());
                            }
                        }
                    } else {
                        if let Some(error) = &result.error {
                            println!("{}", error.red());
                        }
                    }

                    if matches!(self.session.lock().await.state, ReplState::Exiting) {
                        break;
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    println!("Interrupted");
                    break;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }

        println!("Goodbye!");
        Ok(())
    }
}

// 各モジュールの再エクスポート（後で実装）
// pub use config::*;
// pub use commands::*;
// pub use completer::*;
// pub use formatter::*;