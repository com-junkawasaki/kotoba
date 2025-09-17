//! Kotoba CLI のメインエントリーポイント

use clap::Parser;
use kotoba_cli::{Cli, Commands};
use std::process;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLIをパース
    let cli = Cli::parse();

    // ログレベルの設定
    setup_logging(&cli.log_level)?;

    // 作業ディレクトリの設定
    if let Some(cwd) = &cli.cwd {
        std::env::set_current_dir(cwd)?;
    }

    // コマンドの実行
    let result = match cli.command {
        Commands::Run { file, args, watch, allow_all, env_vars } => {
            run_command::execute_run(file, args, watch, allow_all, env_vars).await
        }
        Commands::Serve { port, host, config, dev } => {
            run_command::execute_serve(port, host, config, dev).await
        }
        Commands::Compile { file, output, optimize } => {
            run_command::execute_compile(file, output, optimize).await
        }
        Commands::Init { name, template, force } => {
            run_command::execute_init(name, template, force).await
        }
        Commands::Cache { subcommand } => {
            run_command::execute_cache(subcommand).await
        }
        Commands::Info { verbose } => {
            run_command::execute_info(verbose).await
        }
        Commands::Test { path, filter, verbose } => {
            run_command::execute_test(path, filter, verbose).await
        }
        Commands::Doc { file, output, open } => {
            run_command::execute_doc(file, output, open).await
        }
        Commands::Fmt { files, check, write } => {
            run_command::execute_fmt(files, check, write).await
        }
        Commands::Lint { files, config, fix } => {
            run_command::execute_lint(files, config, fix).await
        }
        Commands::Test { files, filter, verbose, coverage, format } => {
            run_command::execute_test(files, filter, verbose, coverage, format).await
        }
        Commands::Repl => {
            run_command::execute_repl().await
        }
        Commands::Task { name, args } => {
            run_command::execute_task(name, args).await
        }
        Commands::Upgrade { version, force } => {
            run_command::execute_upgrade(version, force).await
        }
    };

    // エラーハンドリング
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    Ok(())
}

/// ログレベルの設定
fn setup_logging(log_level: &str) -> Result<(), Box<dyn std::error::Error>> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let level = match log_level {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        "debug" => tracing::Level::DEBUG,
        "trace" => tracing::Level::TRACE,
        _ => tracing::Level::INFO,
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("kotoba={}", level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

/// コマンド実行モジュール
mod run_command {
    use super::*;

    pub async fn execute_run(
        _file: std::path::PathBuf,
        _args: Vec<String>,
        _watch: bool,
        _allow_all: bool,
        _env_vars: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Running file... (not implemented yet)");
        Ok(())
    }

    pub async fn execute_serve(
        _port: u16,
        _host: String,
        _config: Option<std::path::PathBuf>,
        _dev: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting server... (not implemented yet)");
        Ok(())
    }

    pub async fn execute_compile(
        _file: std::path::PathBuf,
        _output: Option<std::path::PathBuf>,
        _optimize: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Compiling file... (not implemented yet)");
        Ok(())
    }

    pub async fn execute_init(
        _name: Option<String>,
        _template: String,
        _force: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Initializing project... (not implemented yet)");
        Ok(())
    }

    pub async fn execute_cache(
        _subcommand: crate::CacheCommands,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Managing cache... (not implemented yet)");
        Ok(())
    }

    pub async fn execute_info(_verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
        println!("Kotoba v{}", env!("CARGO_PKG_VERSION"));
        println!("Graph processing system inspired by Deno");
        Ok(())
    }

    pub async fn execute_test(
        _path: std::path::PathBuf,
        _filter: Option<String>,
        _verbose: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Running tests... (not implemented yet)");
        Ok(())
    }

    pub async fn execute_doc(
        _file: Option<std::path::PathBuf>,
        _output: std::path::PathBuf,
        _open: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Generating documentation... (not implemented yet)");
        Ok(())
    }

    pub async fn execute_fmt(
        files: Vec<std::path::PathBuf>,
        check: bool,
        write: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use kotoba_formatter::{format_files as fmt_files, format_directory, Writer, WriterConfig};

        if files.is_empty() {
            // ディレクトリ内の全ファイルをフォーマット
            let current_dir = std::path::PathBuf::from(".");
            let results = format_directory(current_dir, check).await?;
            handle_format_results(results, check, write).await
        } else {
            let results = fmt_files(files, check).await?;
            handle_format_results(results, check, write).await
        }
    }

    async fn handle_format_results(
        results: Vec<kotoba_formatter::FormatResult>,
        check: bool,
        write: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use kotoba_formatter::{Writer, WriterConfig};

        if check {
            // チェックモード
            match Writer::check_results(&results) {
                Ok(_) => Ok(()),
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        } else if write {
            // 書き込みモード
            let writer_config = WriterConfig {
                overwrite: true,
                create_backup: false,
                output_dir: None,
            };
            let writer = Writer::new(writer_config);
            writer.write_results(&results).await?;
            Ok(())
        } else {
            // 結果を表示
            for result in &results {
                Writer::print_result(result);
            }
            Writer::print_stats(&results);
            Ok(())
        }
    }

    pub async fn execute_lint(
        files: Vec<std::path::PathBuf>,
        config: Option<std::path::PathBuf>,
        fix: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use kotoba_linter::{lint_files, lint_directory, Reporter, DiagnosticSummary, OutputFormat};

        if fix {
            println!("🔧 Auto-fix is not implemented yet. Use --fix flag when available.");
            return Ok(());
        }

        // リンターモジュールの初期化
        let mut linter = if let Some(config_path) = config {
            // カスタム設定ファイルを使用
            println!("Using config file: {}", config_path.display());
            kotoba_linter::Linter::from_config_file().await?
        } else {
            kotoba_linter::Linter::default()
        };

        let results = if files.is_empty() {
            // ディレクトリ全体をチェック
            let current_dir = std::path::PathBuf::from(".");
            println!("Linting directory: {}", current_dir.display());
            lint_directory(current_dir).await?
        } else {
            // 指定されたファイルをチェック
            println!("Linting {} files...", files.len());
            lint_files(files).await?
        };

        // 結果のレポート
        let mut reporter = Reporter::new(OutputFormat::Pretty);
        reporter.report_results(&results)?;

        // サマリーの表示
        let summary = DiagnosticSummary::from_results(&results);
        summary.print();

        // エラーがあれば終了コード1で終了
        if summary.errors > 0 {
            std::process::exit(1);
        }

        Ok(())
    }

    pub async fn execute_test(
        files: Vec<std::path::PathBuf>,
        filter: Option<String>,
        verbose: bool,
        coverage: bool,
        format: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use kotoba_tester::{TestRunner, TestConfig, Reporter, CoverageReporter, CoverageFormat};

        // 設定を作成
        let mut config = TestConfig::default();
        config.filter = filter;
        config.verbose = verbose;
        config.coverage = coverage;

        // テストランナーを作成
        let mut runner = TestRunner::new(config);

        // テストを実行
        let patterns: Vec<String> = if files.is_empty() {
            vec![".".to_string()]
        } else {
            files.iter().map(|p| p.to_string_lossy().to_string()).collect()
        };

        let results = runner.run(patterns).await?;

        // レポート形式を決定
        let report_format = match format.as_str() {
            "json" => kotoba_tester::ReportFormat::Json,
            "junit" => kotoba_tester::ReportFormat::Junit,
            "tap" => kotoba_tester::ReportFormat::Tap,
            _ => kotoba_tester::ReportFormat::Pretty,
        };

        // 結果をレポート
        let mut reporter = Reporter::new(report_format);
        reporter.report_suites(&results)?;

        // カバレッジレポート
        if coverage {
            use kotoba_tester::{CoverageCollector, CoverageReporter};

            let mut collector = CoverageCollector::new();
            collector.collect_from_suites(&results)?;
            let coverage_report = collector.generate_report();

            let coverage_reporter = CoverageReporter::new(CoverageFormat::Console);
            coverage_reporter.report(&coverage_report)?;
        }

        // サマリーを表示
        runner.print_summary();

        // 失敗したテストがあれば終了コード1で終了
        if runner.get_stats().failed > 0 {
            std::process::exit(1);
        }

        Ok(())
    }

    pub async fn execute_repl() -> Result<(), Box<dyn std::error::Error>> {
        use kotoba_repl::{ReplManager, ReplConfig};

        // REPL設定を作成
        let config = ReplConfig::default();

        // REPLマネージャーを作成
        let repl_manager = ReplManager::new(config);

        // REPLを開始
        repl_manager.start().await?;

        Ok(())
    }

    pub async fn execute_task(
        _name: String,
        _args: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Running task... (not implemented yet)");
        Ok(())
    }

    pub async fn execute_upgrade(
        _version: Option<String>,
        _force: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Upgrading Kotoba... (not implemented yet)");
        Ok(())
    }
}
