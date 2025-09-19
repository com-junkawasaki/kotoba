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
}