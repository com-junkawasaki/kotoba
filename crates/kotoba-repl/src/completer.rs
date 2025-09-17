//! 自動補完機能モジュール

use rustyline::completion::{Completer, Pair};
use rustyline::Context;
use std::collections::HashMap;

/// REPL補完エンジン
pub struct ReplCompleter {
    keywords: Vec<String>,
    commands: Vec<String>,
    variables: HashMap<String, String>,
}

impl ReplCompleter {
    /// 新しい補完エンジンを作成
    pub fn new() -> Self {
        let keywords = vec![
            "let".to_string(),
            "graph".to_string(),
            "node".to_string(),
            "edge".to_string(),
            "query".to_string(),
            "fn".to_string(),
            "if".to_string(),
            "for".to_string(),
            "while".to_string(),
            "return".to_string(),
        ];

        let commands = vec![
            ".help".to_string(),
            ".history".to_string(),
            ".vars".to_string(),
            ".clear".to_string(),
            ".exit".to_string(),
            ".load".to_string(),
            ".save".to_string(),
            ".eval".to_string(),
        ];

        Self {
            keywords,
            commands,
            variables: HashMap::new(),
        }
    }

    /// 変数を更新
    pub fn update_variables(&mut self, variables: &HashMap<String, String>) {
        self.variables = variables.clone();
    }

    /// 補完候補を取得
    fn get_completion_candidates(&self, line: &str, pos: usize) -> Vec<String> {
        let mut candidates = Vec::new();

        // 行の現在の単語を取得
        let (start, word) = self.get_current_word(line, pos);

        // コマンドの補完
        if line.trim().starts_with('.') {
            for cmd in &self.commands {
                if cmd.starts_with(&word) {
                    candidates.push(cmd.clone());
                }
            }
        }
        // キーワードの補完
        else if word.is_empty() || start == 0 {
            for keyword in &self.keywords {
                if keyword.starts_with(&word) {
                    candidates.push(keyword.clone());
                }
            }
        }
        // 変数の補完
        else {
            for var_name in self.variables.keys() {
                if var_name.starts_with(&word) {
                    candidates.push(var_name.clone());
                }
            }
        }

        candidates
    }

    /// 現在の単語を取得
    fn get_current_word(&self, line: &str, pos: usize) -> (usize, String) {
        let line_before_cursor = &line[..pos];
        let words: Vec<&str> = line_before_cursor.split_whitespace().collect();

        if let Some(last_word) = words.last() {
            let start_pos = pos - last_word.len();
            (start_pos, last_word.to_string())
        } else {
            (pos, String::new())
        }
    }
}

impl Completer for ReplCompleter {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let candidates = self.get_completion_candidates(line, pos);
        let (start, _) = self.get_current_word(line, pos);

        let pairs: Vec<Pair> = candidates
            .into_iter()
            .map(|candidate| Pair {
                display: candidate.clone(),
                replacement: candidate,
            })
            .collect();

        Ok((start, pairs))
    }
}

/// ヒストリーベースの補完
pub struct HistoryCompleter {
    history: Vec<String>,
}

impl HistoryCompleter {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    pub fn add_to_history(&mut self, line: &str) {
        if !line.trim().is_empty() && !self.history.contains(&line.to_string()) {
            self.history.push(line.to_string());

            // 履歴を制限
            if self.history.len() > 1000 {
                self.history.remove(0);
            }
        }
    }

    pub fn get_history_completions(&self, prefix: &str) -> Vec<String> {
        self.history
            .iter()
            .filter(|item| item.starts_with(prefix))
            .take(10)
            .cloned()
            .collect()
    }
}

impl Completer for HistoryCompleter {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let prefix = &line[..pos];
        let candidates = self.get_history_completions(prefix);

        let pairs: Vec<Pair> = candidates
            .into_iter()
            .map(|candidate| Pair {
                display: candidate.clone(),
                replacement: candidate,
            })
            .collect();

        Ok((0, pairs))
    }
}

/// 統合補完エンジン
pub struct IntegratedCompleter {
    repl_completer: ReplCompleter,
    history_completer: HistoryCompleter,
}

impl IntegratedCompleter {
    pub fn new() -> Self {
        Self {
            repl_completer: ReplCompleter::new(),
            history_completer: HistoryCompleter::new(),
        }
    }

    pub fn update_variables(&mut self, variables: &HashMap<String, String>) {
        self.repl_completer.update_variables(variables);
    }

    pub fn add_to_history(&mut self, line: &str) {
        self.history_completer.add_to_history(line);
    }
}

impl Completer for IntegratedCompleter {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        // まずREPL補完を試す
        let (start, repl_candidates) = self.repl_completer.complete(line, pos, ctx)?;

        if !repl_candidates.is_empty() {
            return Ok((start, repl_candidates));
        }

        // REPL補完がない場合は履歴補完を試す
        self.history_completer.complete(line, pos, ctx)
    }
}
