//! REPLコマンド処理モジュール

use super::{ReplCommand, ReplSession, ExecutionResult};

/// コマンドプロセッサー
pub struct CommandProcessor;

impl CommandProcessor {
    /// コマンドを解析して実行
    pub async fn process_command(session: &mut ReplSession, input: &str) -> Result<ExecutionResult, Box<dyn std::error::Error>> {
        let command = Self::parse_command(input)?;

        match command {
            ReplCommand::Execute(code) => {
                Self::execute_kotoba_code(session, &code).await
            }
            ReplCommand::Help => {
                Ok(ExecutionResult::new(input.to_string()).success(session.get_help_text(), std::time::Duration::default()))
            }
            ReplCommand::History => {
                Ok(ExecutionResult::new(input.to_string()).success(session.get_history_text(), std::time::Duration::default()))
            }
            ReplCommand::Variables => {
                Ok(ExecutionResult::new(input.to_string()).success(session.get_variables_text(), std::time::Duration::default()))
            }
            ReplCommand::Clear => {
                session.clear_session();
                Ok(ExecutionResult::new(input.to_string()).success("Session cleared".to_string(), std::time::Duration::default()))
            }
            ReplCommand::Exit => {
                session.state = super::ReplState::Exiting;
                Ok(ExecutionResult::new(input.to_string()).success("Goodbye!".to_string(), std::time::Duration::default()))
            }
            ReplCommand::Load(filename) => {
                match session.load_file(&filename).await {
                    Ok(_) => Ok(ExecutionResult::new(input.to_string()).success(format!("Loaded file: {}", filename), std::time::Duration::default())),
                    Err(e) => Ok(ExecutionResult::new(input.to_string()).error(format!("Failed to load file: {}", e), std::time::Duration::default())),
                }
            }
            ReplCommand::Save(filename) => {
                match session.save_history(&filename).await {
                    Ok(_) => Ok(ExecutionResult::new(input.to_string()).success(format!("Saved history to: {}", filename), std::time::Duration::default())),
                    Err(e) => Ok(ExecutionResult::new(input.to_string()).error(format!("Failed to save history: {}", e), std::time::Duration::default())),
                }
            }
            ReplCommand::Eval(code) => {
                Self::execute_kotoba_code(session, &code).await
            }
        }
    }

    /// 入力を解析してコマンドを特定
    fn parse_command(input: &str) -> Result<ReplCommand, Box<dyn std::error::Error>> {
        let input = input.trim();

        if input.starts_with(".help") {
            Ok(ReplCommand::Help)
        } else if input.starts_with(".history") {
            Ok(ReplCommand::History)
        } else if input.starts_with(".vars") || input.starts_with(".variables") {
            Ok(ReplCommand::Variables)
        } else if input.starts_with(".clear") {
            Ok(ReplCommand::Clear)
        } else if input.starts_with(".exit") || input.starts_with(".quit") {
            Ok(ReplCommand::Exit)
        } else if input.starts_with(".load ") {
            let filename = input[6..].trim();
            Ok(ReplCommand::Load(filename.to_string()))
        } else if input.starts_with(".save ") {
            let filename = input[6..].trim();
            Ok(ReplCommand::Save(filename.to_string()))
        } else if input.starts_with(".eval ") {
            let code = input[6..].trim();
            Ok(ReplCommand::Eval(code.to_string()))
        } else {
            Ok(ReplCommand::Execute(input.to_string()))
        }
    }

    /// Kotobaコードを実行
    async fn execute_kotoba_code(session: &mut ReplSession, code: &str) -> Result<ExecutionResult, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        let result = ExecutionResult::new(code.to_string());

        // 簡易的なKotobaコード実行
        if code.trim().is_empty() {
            return Ok(result.success(String::new(), start_time.elapsed()));
        }

        // 変数宣言の処理
        if code.contains("let ") {
            session.handle_variable_declaration(code)?;
            return Ok(result.success(format!("Variable declared: {}", code), start_time.elapsed()));
        }

        // 式の評価
        if code.contains("=") || code.contains("+") || code.contains("-") {
            let output = session.evaluate_expression(code)?;
            return Ok(result.success(output, start_time.elapsed()));
        }

        // デフォルトの実行
        Ok(result.success(format!("Executed: {}", code), start_time.elapsed()))
    }
}

