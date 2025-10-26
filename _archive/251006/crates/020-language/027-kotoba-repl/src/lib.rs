//! Kotoba REPL - Interactive shell for Kotoba programming language

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// REPL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplConfig {
    /// Command timeout in seconds
    pub timeout: u64,
    /// Maximum command history size
    pub max_history: usize,
    /// Enable syntax highlighting
    pub syntax_highlighting: bool,
    /// Enable auto-completion
    pub auto_completion: bool,
    /// Show line numbers
    pub show_line_numbers: bool,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            timeout: 30,
            max_history: 1000,
            syntax_highlighting: true,
            auto_completion: true,
            show_line_numbers: false,
        }
    }
}

/// REPL session information
#[derive(Debug, Clone)]
pub struct ReplSessionInfo {
    /// Number of commands executed
    pub command_count: usize,
    /// Number of variables defined
    pub variable_count: usize,
    /// Session start time
    pub start_time: std::time::Instant,
}

/// Command execution result
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Whether the command executed successfully
    pub success: bool,
    /// Command output
    pub output: Option<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl CommandResult {
    /// Create a successful result
    pub fn success(output: String, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            output: Some(output),
            execution_time_ms,
        }
    }

    /// Create a failed result
    pub fn failure(output: String, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            output: Some(output),
            execution_time_ms,
        }
    }

    /// Check if the command was successful
    pub fn is_success(&self) -> bool {
        self.success
    }
}

/// REPL session
pub struct ReplSession {
    #[allow(dead_code)]
    config: ReplConfig,
    variables: HashMap<String, String>,
    command_count: usize,
    start_time: std::time::Instant,
}

impl ReplSession {
    /// Create a new REPL session
    pub fn new(config: ReplConfig) -> Self {
        Self {
            config,
            variables: HashMap::new(),
            command_count: 0,
            start_time: std::time::Instant::now(),
        }
    }

    /// Execute a command
    pub async fn execute(&mut self, command: &str) -> Result<CommandResult, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        self.command_count += 1;

        let result = match command.trim() {
            ".help" => {
                let help_text = r#"Kotoba REPL Commands:
.help          Show this help message
.vars          List all defined variables
.clear         Clear all variables
.exit          Exit the REPL
.quit          Exit the REPL

Examples:
let x = 42
let name = "Hello"
1 + 2
x * 2
"#;
                CommandResult::success(help_text.to_string(), start_time.elapsed().as_millis() as u64)
            }
            ".vars" => {
                let mut output = String::from("Defined variables:\n");
                if self.variables.is_empty() {
                    output.push_str("  (none)\n");
                } else {
                    for (name, value) in &self.variables {
                        output.push_str(&format!("  {} = {}\n", name, value));
                    }
                }
                CommandResult::success(output, start_time.elapsed().as_millis() as u64)
            }
            ".clear" => {
                self.variables.clear();
                CommandResult::success("All variables cleared".to_string(), start_time.elapsed().as_millis() as u64)
            }
            cmd if cmd.starts_with("let ") => {
                self.handle_let_command(cmd)
                    .map(|output| CommandResult::success(output, start_time.elapsed().as_millis() as u64))
                    .unwrap_or_else(|err| CommandResult::failure(err, start_time.elapsed().as_millis() as u64))
            }
            _ => {
                // For now, just echo the command as if it were evaluated
                let output = format!("Executed: {}\nResult: <evaluation not implemented yet>", command);
                CommandResult::success(output, start_time.elapsed().as_millis() as u64)
            }
        };

        Ok(result)
    }

    /// Handle let command for variable assignment
    fn handle_let_command(&mut self, command: &str) -> Result<String, String> {
        let parts: Vec<&str> = command.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err("Invalid variable assignment syntax. Use: let name = value".to_string());
        }

        let var_name = parts[0].trim().strip_prefix("let ").unwrap_or(parts[0].trim()).trim();
        let var_value = parts[1].trim();

        if var_name.is_empty() {
            return Err("Variable name cannot be empty".to_string());
        }

        // Remove quotes if present
        let clean_value = if var_value.starts_with('"') && var_value.ends_with('"') {
            var_value[1..var_value.len()-1].to_string()
        } else {
            var_value.to_string()
        };

        self.variables.insert(var_name.to_string(), clean_value.clone());
        Ok(format!("Variable '{}' set to '{}'", var_name, clean_value))
    }

    /// Get session information
    pub fn get_info(&self) -> ReplSessionInfo {
        ReplSessionInfo {
            command_count: self.command_count,
            variable_count: self.variables.len(),
            start_time: self.start_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_repl_basic_commands() {
        let config = ReplConfig::default();
        let mut session = ReplSession::new(config);

        // 変数宣言のテスト
        let result = session.execute("let x = 42").await.unwrap();
        assert!(result.is_success());
        assert!(result.output.is_some());

        // ヘルプコマンドのテスト
        let help_result = session.execute(".help").await.unwrap();
        assert!(help_result.is_success());
        assert!(help_result.output.is_some());
        assert!(help_result.output.as_ref().unwrap().contains("Kotoba REPL Commands"));

        // セッション情報のテスト
        let info = session.get_info();
        assert_eq!(info.command_count, 2);
    }

    #[tokio::test]
    async fn test_variable_operations() {
        let config = ReplConfig::default();
        let mut session = ReplSession::new(config);

        // 変数宣言
        let result1 = session.execute("let name = \"Alice\"").await.unwrap();
        assert!(result1.is_success());

        // 変数一覧表示
        let result2 = session.execute(".vars").await.unwrap();
        assert!(result2.is_success());
        assert!(result2.output.as_ref().unwrap().contains("name"));
        assert!(result2.output.as_ref().unwrap().contains("Alice"));
    }

    #[tokio::test]
    async fn test_expression_evaluation() {
        let config = ReplConfig::default();
        let mut session = ReplSession::new(config);

        // 簡単な式の評価
        let result = session.execute("1 + 2").await.unwrap();
        assert!(result.is_success());
        // 簡易的な評価なので、結果は実行されたことを示すメッセージになる
    }

    #[test]
    fn test_repl_config_default() {
        let config = ReplConfig::default();
        assert_eq!(config.timeout, 30);
        assert_eq!(config.max_history, 1000);
        assert!(config.syntax_highlighting);
        assert!(config.auto_completion);
        assert!(!config.show_line_numbers);
    }

    #[test]
    fn test_command_result() {
        let success_result = CommandResult::success("output".to_string(), 100);
        assert!(success_result.is_success());
        assert_eq!(success_result.output, Some("output".to_string()));
        assert_eq!(success_result.execution_time_ms, 100);

        let failure_result = CommandResult::failure("error".to_string(), 50);
        assert!(!failure_result.is_success());
        assert_eq!(failure_result.output, Some("error".to_string()));
        assert_eq!(failure_result.execution_time_ms, 50);
    }

    #[test]
    fn test_repl_config_creation() {
        let config = ReplConfig {
            timeout: 60,
            max_history: 500,
            syntax_highlighting: false,
            auto_completion: false,
            show_line_numbers: true,
        };

        assert_eq!(config.timeout, 60);
        assert_eq!(config.max_history, 500);
        assert!(!config.syntax_highlighting);
        assert!(!config.auto_completion);
        assert!(config.show_line_numbers);
    }

    #[test]
    fn test_repl_config_clone() {
        let original = ReplConfig::default();
        let cloned = original.clone();

        assert_eq!(original.timeout, cloned.timeout);
        assert_eq!(original.max_history, cloned.max_history);
        assert_eq!(original.syntax_highlighting, cloned.syntax_highlighting);
        assert_eq!(original.auto_completion, cloned.auto_completion);
        assert_eq!(original.show_line_numbers, cloned.show_line_numbers);
    }

    #[test]
    fn test_repl_config_debug() {
        let config = ReplConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("ReplConfig"));
        assert!(debug_str.contains("30"));
        assert!(debug_str.contains("1000"));
    }

    #[test]
    fn test_repl_config_serialization() {
        let config = ReplConfig {
            timeout: 45,
            max_history: 2000,
            syntax_highlighting: true,
            auto_completion: false,
            show_line_numbers: true,
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&config);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("45"));
        assert!(json_str.contains("2000"));
        assert!(json_str.contains("true"));
        assert!(json_str.contains("false"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<ReplConfig> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.timeout, 45);
        assert_eq!(deserialized.max_history, 2000);
        assert!(deserialized.syntax_highlighting);
        assert!(!deserialized.auto_completion);
        assert!(deserialized.show_line_numbers);
    }

    #[test]
    fn test_repl_session_info_creation() {
        let start_time = std::time::Instant::now();
        let info = ReplSessionInfo {
            command_count: 10,
            variable_count: 5,
            start_time,
        };

        assert_eq!(info.command_count, 10);
        assert_eq!(info.variable_count, 5);
    }

    #[test]
    fn test_repl_session_info_debug() {
        let info = ReplSessionInfo {
            command_count: 25,
            variable_count: 8,
            start_time: std::time::Instant::now(),
        };

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("ReplSessionInfo"));
        assert!(debug_str.contains("25"));
        assert!(debug_str.contains("8"));
    }

    #[test]
    fn test_command_result_creation() {
        let result = CommandResult {
            success: true,
            output: Some("test output".to_string()),
            execution_time_ms: 200,
        };

        assert!(result.is_success());
        assert_eq!(result.output, Some("test output".to_string()));
        assert_eq!(result.execution_time_ms, 200);
    }

    #[test]
    fn test_command_result_debug() {
        let result = CommandResult::success("debug test".to_string(), 100);
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("CommandResult"));
        assert!(debug_str.contains("debug test"));
        assert!(debug_str.contains("100"));
    }

    #[test]
    fn test_command_result_clone() {
        let original = CommandResult::failure("clone test".to_string(), 50);
        let cloned = original.clone();

        assert_eq!(original.success, cloned.success);
        assert_eq!(original.output, cloned.output);
        assert_eq!(original.execution_time_ms, cloned.execution_time_ms);
    }

    #[test]
    fn test_repl_session_creation() {
        let config = ReplConfig::default();
        let session = ReplSession::new(config.clone());

        assert_eq!(session.config.timeout, config.timeout);
        assert_eq!(session.command_count, 0);
        assert!(session.variables.is_empty());
    }

    #[tokio::test]
    async fn test_repl_session_get_info() {
        let config = ReplConfig::default();
        let session = ReplSession::new(config);

        let info = session.get_info();
        assert_eq!(info.command_count, 0);
        assert_eq!(info.variable_count, 0);
    }

    #[tokio::test]
    async fn test_repl_session_clear_command() {
        let config = ReplConfig::default();
        let mut session = ReplSession::new(config);

        // Add some variables
        session.variables.insert("test".to_string(), "value".to_string());
        assert_eq!(session.variables.len(), 1);

        let result = session.execute(".clear").await.unwrap();
        assert!(result.is_success());
        assert!(result.output.is_some());
        assert!(result.output.unwrap().contains("All variables cleared"));
        assert_eq!(session.variables.len(), 0);
    }

    #[tokio::test]
    async fn test_repl_session_let_command_valid() {
        let config = ReplConfig::default();
        let mut session = ReplSession::new(config);

        let result = session.execute("let x = 42").await.unwrap();
        assert!(result.is_success());
        assert!(result.output.is_some());
        assert!(result.output.unwrap().contains("Variable 'x' set to '42'"));
        assert_eq!(session.variables.get("x"), Some(&"42".to_string()));
    }

    #[tokio::test]
    async fn test_repl_session_let_command_invalid_syntax() {
        let config = ReplConfig::default();
        let mut session = ReplSession::new(config);

        let result = session.execute("let invalid").await.unwrap();
        assert!(!result.is_success());
        assert!(result.output.is_some());
        assert!(result.output.unwrap().contains("Invalid variable assignment syntax"));
    }

    #[tokio::test]
    async fn test_repl_session_unknown_command() {
        let config = ReplConfig::default();
        let mut session = ReplSession::new(config);

        let result = session.execute("unknown_command").await.unwrap();
        assert!(result.is_success());
        assert!(result.output.is_some());
        assert!(result.output.unwrap().contains("Executed: unknown_command"));
        assert!(result.output.unwrap().contains("evaluation not implemented yet"));
    }

    #[tokio::test]
    async fn test_repl_session_command_count_increment() {
        let config = ReplConfig::default();
        let mut session = ReplSession::new(config);

        assert_eq!(session.get_info().command_count, 0);

        session.execute("let x = 1").await.unwrap();
        assert_eq!(session.get_info().command_count, 1);

        session.execute("let y = 2").await.unwrap();
        assert_eq!(session.get_info().command_count, 2);

        session.execute(".help").await.unwrap();
        assert_eq!(session.get_info().command_count, 3);
    }

    #[tokio::test]
    async fn test_repl_session_variable_overwrite() {
        let config = ReplConfig::default();
        let mut session = ReplSession::new(config);

        // Set initial value
        session.execute("let x = 1").await.unwrap();
        assert_eq!(session.variables.get("x"), Some(&"1".to_string()));

        // Overwrite value
        session.execute("let x = 2").await.unwrap();
        assert_eq!(session.variables.get("x"), Some(&"2".to_string()));
    }

    #[test]
    fn test_repl_session_debug() {
        let config = ReplConfig::default();
        let session = ReplSession::new(config);

        let debug_str = format!("{:?}", session);
        assert!(debug_str.contains("ReplSession"));
    }

    #[test]
    fn test_repl_config_edge_cases() {
        // Test with zero values
        let config1 = ReplConfig {
            timeout: 0,
            max_history: 0,
            syntax_highlighting: false,
            auto_completion: false,
            show_line_numbers: false,
        };

        assert_eq!(config1.timeout, 0);
        assert_eq!(config1.max_history, 0);

        // Test with very large values
        let config2 = ReplConfig {
            timeout: u64::MAX,
            max_history: usize::MAX,
            syntax_highlighting: true,
            auto_completion: true,
            show_line_numbers: true,
        };

        assert_eq!(config2.timeout, u64::MAX);
        assert_eq!(config2.max_history, usize::MAX);
    }

    #[test]
    fn test_command_result_edge_cases() {
        // Test with empty output
        let result1 = CommandResult::success("".to_string(), 0);
        assert!(result1.is_success());
        assert_eq!(result1.output, Some("".to_string()));
        assert_eq!(result1.execution_time_ms, 0);

        // Test with very large execution time
        let result2 = CommandResult::success("done".to_string(), u64::MAX);
        assert!(result2.is_success());
        assert_eq!(result2.execution_time_ms, u64::MAX);
    }
}