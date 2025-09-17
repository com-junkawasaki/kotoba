//! 診断レポート出力モジュール

use super::{Diagnostic, DiagnosticLevel, LintResult, OutputFormat};
use std::io::{self, Write};
use colored::*;

/// レポートライター
#[derive(Debug)]
pub struct Reporter {
    format: OutputFormat,
    writer: Box<dyn Write>,
}

impl Reporter {
    /// 新しいレポーターを作成
    pub fn new(format: OutputFormat) -> Self {
        Self {
            format,
            writer: Box::new(io::stdout()),
        }
    }

    /// ファイルライターを作成
    pub fn with_file(format: OutputFormat, file_path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::create(file_path)?;
        Ok(Self {
            format,
            writer: Box::new(file),
        })
    }

    /// 結果をレポート
    pub fn report_results(&mut self, results: &[LintResult]) -> Result<(), Box<dyn std::error::Error>> {
        match self.format {
            OutputFormat::Pretty => self.report_pretty(results),
            OutputFormat::Json => self.report_json(results),
            OutputFormat::Compact => self.report_compact(results),
        }
    }

    /// 単一の結果をレポート
    pub fn report_result(&mut self, result: &LintResult) -> Result<(), Box<dyn std::error::Error>> {
        self.report_results(&[result.clone()])
    }

    /// Pretty形式でレポート
    fn report_pretty(&mut self, results: &[LintResult]) -> Result<(), Box<dyn std::error::Error>> {
        let total_errors = results.iter().map(|r| r.error_count).sum::<usize>();
        let total_warnings = results.iter().map(|r| r.warning_count).sum::<usize>();
        let total_files = results.len();

        // ヘッダー
        writeln!(self.writer, "{}", "Kotoba Linter Report".bold())?;
        writeln!(self.writer, "{}", "===================".bold())?;
        writeln!(self.writer)?;

        // サマリー
        if total_errors > 0 {
            writeln!(self.writer, "{}: {}", "Errors".red().bold(), total_errors)?;
        }
        if total_warnings > 0 {
            writeln!(self.writer, "{}: {}", "Warnings".yellow().bold(), total_warnings)?;
        }
        writeln!(self.writer, "{}: {}", "Files checked".blue(), total_files)?;
        writeln!(self.writer)?;

        // 各ファイルの結果
        for result in results {
            if !result.diagnostics.is_empty() {
                writeln!(self.writer, "{}", result.file_path.display().to_string().cyan().bold())?;

                for diagnostic in &result.diagnostics {
                    self.report_diagnostic_pretty(diagnostic)?;
                }
                writeln!(self.writer)?;
            }
        }

        // フッター
        if total_errors == 0 && total_warnings == 0 {
            writeln!(self.writer, "{}", "✅ All checks passed!".green().bold())?;
        } else {
            writeln!(self.writer, "{}", "❌ Some issues found.".red().bold())?;
        }

        Ok(())
    }

    /// 診断をPretty形式で出力
    fn report_diagnostic_pretty(&mut self, diagnostic: &Diagnostic) -> Result<(), Box<dyn std::error::Error>> {
        let level_str = match diagnostic.level {
            DiagnosticLevel::Error => "error".red().bold(),
            DiagnosticLevel::Warning => "warning".yellow().bold(),
            DiagnosticLevel::Info => "info".blue().bold(),
            DiagnosticLevel::Hint => "hint".cyan(),
        };

        let location = format!("{}:{}:{}", diagnostic.file_path.display(), diagnostic.line, diagnostic.column);

        writeln!(self.writer, "  {} {} {}", level_str, diagnostic.code.dimmed(), location.dimmed())?;
        writeln!(self.writer, "    {}", diagnostic.message)?;

        if let Some(suggestion) = &diagnostic.suggestion {
            writeln!(self.writer, "    {} {}", "💡".cyan(), suggestion.bright_black())?;
        }

        if let Some(help) = &diagnostic.help {
            writeln!(self.writer, "    {} {}", "ℹ️".blue(), help.bright_black())?;
        }

        Ok(())
    }

    /// JSON形式でレポート
    fn report_json(&mut self, results: &[LintResult]) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(results)?;
        writeln!(self.writer, "{}", json)?;
        Ok(())
    }

    /// Compact形式でレポート
    fn report_compact(&mut self, results: &[LintResult]) -> Result<(), Box<dyn std::error::Error>> {
        for result in results {
            for diagnostic in &result.diagnostics {
                let level = match diagnostic.level {
                    DiagnosticLevel::Error => "E",
                    DiagnosticLevel::Warning => "W",
                    DiagnosticLevel::Info => "I",
                    DiagnosticLevel::Hint => "H",
                };

                writeln!(
                    self.writer,
                    "{}:{}:{}:{}: {}",
                    diagnostic.file_path.display(),
                    diagnostic.line,
                    diagnostic.column,
                    level,
                    diagnostic.message
                )?;
            }
        }
        Ok(())
    }
}

impl Default for Reporter {
    fn default() -> Self {
        Self::new(OutputFormat::Pretty)
    }
}

/// 診断サマリー
#[derive(Debug)]
pub struct DiagnosticSummary {
    pub total_files: usize,
    pub total_diagnostics: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub hints: usize,
    pub duration_ms: u64,
}

impl DiagnosticSummary {
    /// 結果からサマリーを作成
    pub fn from_results(results: &[LintResult]) -> Self {
        let mut summary = Self {
            total_files: results.len(),
            total_diagnostics: 0,
            errors: 0,
            warnings: 0,
            infos: 0,
            hints: 0,
            duration_ms: 0,
        };

        for result in results {
            summary.total_diagnostics += result.diagnostics.len();
            summary.errors += result.error_count;
            summary.warnings += result.warning_count;
            summary.duration_ms += result.duration_ms;

            for diagnostic in &result.diagnostics {
                match diagnostic.level {
                    DiagnosticLevel::Info => summary.infos += 1,
                    DiagnosticLevel::Hint => summary.hints += 1,
                    _ => {} // Error and Warning are already counted
                }
            }
        }

        summary
    }

    /// サマリーを表示
    pub fn print(&self) {
        println!("{}", "Lint Summary".bold());
        println!("{}", "=============".bold());
        println!("Files checked: {}", self.total_files);
        println!("Total issues: {}", self.total_diagnostics);

        if self.errors > 0 {
            println!("Errors: {}", self.errors.to_string().red());
        }
        if self.warnings > 0 {
            println!("Warnings: {}", self.warnings.to_string().yellow());
        }
        if self.infos > 0 {
            println!("Infos: {}", self.infos.to_string().blue());
        }
        if self.hints > 0 {
            println!("Hints: {}", self.hints.to_string().cyan());
        }

        println!("Duration: {:.2}ms", self.duration_ms as f64);
    }

    /// CIで使用する終了コードを取得
    pub fn exit_code(&self) -> i32 {
        if self.errors > 0 {
            1 // エラーがあれば失敗
        } else {
            0 // 成功
        }
    }
}

/// GitHub Actions対応レポーター
pub struct GitHubReporter;

impl GitHubReporter {
    /// GitHub Actions形式で診断を出力
    pub fn report_diagnostics(diagnostics: &[&Diagnostic]) {
        for diagnostic in diagnostics {
            let level = match diagnostic.level {
                DiagnosticLevel::Error => "error",
                DiagnosticLevel::Warning => "warning",
                DiagnosticLevel::Info => "notice",
                DiagnosticLevel::Hint => "notice",
            };

            println!(
                "::{} file={},line={},col={},title={}::{}",
                level,
                diagnostic.file_path.display(),
                diagnostic.line,
                diagnostic.column,
                diagnostic.code,
                diagnostic.message
            );
        }
    }
}

/// 統計レポーター
pub struct StatsReporter;

impl StatsReporter {
    /// 詳細な統計を表示
    pub fn print_detailed_stats(results: &[LintResult]) {
        use std::collections::HashMap;

        let mut rule_counts = HashMap::new();
        let mut level_counts = HashMap::new();

        for result in results {
            for diagnostic in &result.diagnostics {
                *rule_counts.entry(diagnostic.code.clone()).or_insert(0) += 1;
                *level_counts.entry(diagnostic.level).or_insert(0) += 1;
            }
        }

        println!("{}", "Detailed Statistics".bold());
        println!("{}", "==================".bold());

        println!("\n{}", "By Rule:".bold());
        for (rule, count) in rule_counts.iter() {
            println!("  {}: {}", rule, count);
        }

        println!("\n{}", "By Level:".bold());
        for (level, count) in level_counts.iter() {
            let level_str = match level {
                DiagnosticLevel::Error => "Errors",
                DiagnosticLevel::Warning => "Warnings",
                DiagnosticLevel::Info => "Infos",
                DiagnosticLevel::Hint => "Hints",
            };
            println!("  {}: {}", level_str, count);
        }
    }
}

/// 進捗レポーター
pub struct ProgressReporter {
    total_files: usize,
    processed_files: usize,
}

impl ProgressReporter {
    pub fn new(total_files: usize) -> Self {
        Self {
            total_files,
            processed_files: 0,
        }
    }

    pub fn update(&mut self, file_path: &std::path::Path) {
        self.processed_files += 1;
        let percentage = (self.processed_files as f64 / self.total_files as f64 * 100.0) as usize;
        println!("[{}/{}] {}% - {}", self.processed_files, self.total_files, percentage, file_path.display());
    }

    pub fn finish(&self) {
        println!("✅ Linting completed!");
    }
}
