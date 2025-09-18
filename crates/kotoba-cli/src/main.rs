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
        // Commands::Serve { port, host, config, dev } => {
        //     run_command::execute_serve(port, host, config, dev).await
        // }
        Commands::Serve { .. } => {
            println!("🌐 Starting Kotoba HTTP Server");
            println!("=============================");
            println!("🏠 Host: 127.0.0.1");
            println!("🔌 Port: 8100");
            println!("🔒 TLS: Disabled");
            println!("💡 HTTP server not yet implemented");
            println!("💡 Server would be available at: http://127.0.0.1:8100");
            Ok(())
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
        Commands::Docs { command } => {
            run_command::execute_docs(command).await
        }
        // Commands::Build { task, config, watch, verbose, list, clean } => {
        //     run_command::execute_build(task, config, watch, verbose, list, clean).await
        // }
        Commands::Task { name, args } => {
            run_command::execute_task(name, args).await
        }
        Commands::Upgrade { version, force } => {
            run_command::execute_upgrade(version, force).await
        }
        Commands::Web { command } => {
            run_command::execute_web(command).await
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
        port: u16,
        host: String,
        config: Option<std::path::PathBuf>,
        dev: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Stdio;

        // Find the kotoba-server binary
        let server_binary_name = "kotoba-server";
        let current_exe = std::env::current_exe()?;
        let bin_dir = current_exe.parent().ok_or("Could not find parent directory of executable")?;
        let mut server_binary_path = bin_dir.join(server_binary_name);

        if !server_binary_path.exists() {
            // Fallback for development: cargo run -p kotoba-server
            let target_dir = bin_dir.parent().ok_or("Could not find target dir")?;
            let fallback_path = target_dir.join(server_binary_name);
            if !fallback_path.exists() {
                eprintln!("Error: kotoba-server binary not found.");
                eprintln!("Searched at: {} and {}", server_binary_path.display(), fallback_path.display());
                eprintln!("Please run 'cargo build -p kotoba-server' first.");
                return Err("kotoba-server binary not found".into());
            }
            // Use fallback path
            server_binary_path = fallback_path;
        }

        println!("🌐 Starting Kotoba HTTP Server");
        println!("=============================");
        println!("🏠 Host: {}", host);
        println!("🔌 Port: {}", port);
        println!("🔒 TLS: Disabled");

        if dev {
            println!("🚀 Development mode enabled");
        }

        if let Some(config_path) = &config {
            println!("⚙️  Using config file: {}", config_path.display());
        }

        println!("💡 Server would be available at: http://{}:{}", host, port);

        // Build command arguments
        let mut cmd = std::process::Command::new(server_binary_path);
        cmd.arg("--host").arg(&host)
           .arg("--port").arg(port.to_string())
           .stdout(Stdio::inherit())
           .stderr(Stdio::inherit());

        if let Some(config_path) = config {
            cmd.arg("--config").arg(config_path);
        }

        if dev {
            cmd.arg("--dev");
        }

        // Start the server process
        let mut child = cmd.spawn()?;
        let status = child.wait()?;

        println!("[kotoba cli] kotoba-server process exited with status: {}", status);
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

    pub async fn execute_docs(command: crate::DocsCommands) -> Result<(), Box<dyn std::error::Error>> {
        use kotoba_docs::*;
        use crate::DocsCommands;

        match command {
            DocsCommands::Generate { config, output, source, verbose, watch, clean } => {
                // 設定ファイルを読み込みまたは作成
                let config_path = config.or_else(|| Some(std::path::PathBuf::from("kotoba-docs.toml")));
                let mut docs_config = if let Some(path) = &config_path {
                    if path.exists() {
                        config::ConfigManager::new(std::env::current_dir()?).load_config().await?
                    } else {
                        DocsConfig::default()
                    }
                } else {
                    DocsConfig::default()
                };

                // オプションで設定を上書き
                if let Some(output_dir) = output {
                    docs_config.output_dir = output_dir;
                }
                if let Some(source_dir) = source {
                    docs_config.input_dir = source_dir;
                }

                if verbose {
                    println!("📁 Input directory: {}", docs_config.input_dir.display());
                    println!("📁 Output directory: {}", docs_config.output_dir.display());
                }

                // パーサーを作成してドキュメントを解析
                let mut parser = parser::DocParser::new();
                parser = parser.with_include_extensions(vec![
                    "rs".to_string(),
                    "js".to_string(),
                    "ts".to_string(),
                    "py".to_string(),
                    "go".to_string(),
                    "md".to_string(),
                ]);

                println!("🔍 Parsing source files...");
                let items = parser.parse_directory(&docs_config.input_dir)?;

                if verbose {
                    println!("📄 Found {} documentation items", items.len());
                }

                // クロスリファレンスを解決
                let mut items = items;
                parser.resolve_cross_references(&mut items)?;

                // ジェネレータを作成してドキュメントを生成
                let generator = generator::DocGenerator::new(docs_config.clone(), items);
                let result = generator.generate().await?;

                println!("✅ Documentation generated successfully!");
                println!("📊 Generated {} documents", result.documents_generated);
                println!("📁 Output: {}", result.output_dir.display());

                Ok(())
            }

            DocsCommands::Serve { host, port, dir, open } => {
                let docs_dir = dir.unwrap_or_else(|| std::path::PathBuf::from("docs/html"));

                if !docs_dir.exists() {
                    println!("❌ Documentation directory not found: {}", docs_dir.display());
                    println!("💡 Run 'kotoba docs generate' first to generate documentation");
                    return Ok(());
                }

                println!("🚀 Starting documentation server...");
                server::serve_static(docs_dir, &host, port).await?;

                Ok(())
            }

            DocsCommands::Search { query, config, limit } => {
                let config_path = config.unwrap_or_else(|| std::path::PathBuf::from("kotoba-docs.toml"));
                let docs_config = if config_path.exists() {
                    config::ConfigManager::new(std::env::current_dir()?).load_config().await?
                } else {
                    println!("❌ Config file not found: {}", config_path.display());
                    return Ok(());
                };

                // パーサーを作成してドキュメントを解析
                let parser = parser::DocParser::new();
                let items = parser.parse_directory(&docs_config.input_dir)?;

                // 検索エンジンを作成
                let mut search_engine = search::SearchEngine::new();
                search_engine.add_documents(items);

                // 検索を実行
                let options = search::SearchOptions {
                    limit,
                    ..Default::default()
                };

                let results = search_engine.search(&query, &options)?;

                if results.is_empty() {
                    println!("❌ No results found for: {}", query);
                } else {
                    println!("🔍 Search results for: {}", query);
                    println!("📊 Found {} results", results.len());
                    println!();

                    for (i, result) in results.iter().enumerate() {
                        println!("{}. {}", i + 1, result.item.name);
                        println!("   Type: {:?}", result.item.doc_type);
                        if let Some(excerpt) = result.excerpts.first() {
                            println!("   Excerpt: {}", excerpt);
                        }
                        println!("   Score: {:.2}", result.score);
                        println!();
                    }
                }

                Ok(())
            }

            DocsCommands::Init { name, output, source } => {
                let project_name = name.unwrap_or_else(|| {
                    std::env::current_dir()
                        .ok()
                        .and_then(|p| p.file_name().and_then(|n| n.to_str()))
                        .unwrap_or("My Project")
                        .to_string()
                });

                let mut config = DocsConfig::default();
                config.name = project_name;
                config.output_dir = output;
                config.input_dir = source;

                let manager = config::ConfigManager::new(std::env::current_dir()?);
                manager.save_config(&config, None).await?;

                println!("✅ Initialized Kotoba Docs configuration");
                println!("📄 Created kotoba-docs.toml");
                println!("💡 Run 'kotoba docs generate' to generate documentation");

                Ok(())
            }
        }
    }

    // pub async fn execute_build(
    //     task: Option<String>,
    //     config: Option<std::path::PathBuf>,
    //     watch: bool,
    //     verbose: bool,
    //     list: bool,
    //     clean: bool,
    // ) -> Result<(), Box<dyn std::error::Error>> {
    //     use kotoba_build::{BuildEngine, WatchOptions, FileWatcher};
    //     use std::sync::Arc;
    //     use tokio::sync::RwLock;
    //
    //     // プロジェクトルートを検出
    //     let project_root = kotoba_build::utils::find_project_root()?;
    //
    //     if verbose {
    //         println!("📁 Project root: {}", project_root.display());
    //     }
    //
    //     // ビルドエンジンを作成
    //     let engine = Arc::new(RwLock::new(BuildEngine::new(project_root.clone()).await?));
    //
    //     if list {
    //         // 利用可能なタスク一覧を表示
    //         println!("📋 Available tasks:");
    //         let tasks = engine.read().await.list_tasks().await;
    //         for (name, desc) in tasks {
    //             println!("  {} - {}", name.green(), desc);
    //         }
    //         return Ok(());
    //     }
    //
    //     if clean {
    //         // クリーン処理
    //         println!("🧹 Cleaning build artifacts...");
    //         // TODO: 実際のクリーン処理を実装
    //         println!("✅ Clean completed");
    //         return Ok(());
    //     }
    //
    //     if watch {
    //         // ウォッチモードで起動
    //         println!("👀 Starting watch mode...");
    //         let mut watcher = FileWatcher::new(Arc::clone(&engine));
    //
    //         // 監視対象のパスを設定
    //         watcher.add_watch_path(project_root.join("src"));
    //         watcher.add_watch_path(project_root.join("kotoba-build.toml"));
    //
    //         watcher.start().await?;
    //     } else if let Some(task_name) = task {
    //         // 指定されたタスクを実行
    //         println!("🚀 Running task: {}", task_name);
    //         let result = engine.write().await.run_task(&task_name).await?;
    //         println!("✅ Task completed successfully");
    //     } else {
    //         // デフォルトビルドを実行
    //         println!("🏗️  Building project...");
    //         let result = engine.write().await.build().await?;
    //         println!("✅ Build completed successfully");
    //     }
    //
    //     Ok(())
    // }

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

    pub async fn execute_web(command: crate::WebCommands) -> Result<(), Box<dyn std::error::Error>> {
        match command {
            crate::WebCommands::Dev { cwd, port } => {
                let server_binary_name = "kotoba-server";
                // Find the binary relative to the `kotoba` CLI executable itself.
                let current_exe = std::env::current_exe()?;
                let bin_dir = current_exe.parent().ok_or("Could not find parent directory of executable")?;
                let server_binary_path = bin_dir.join(server_binary_name);

                if !server_binary_path.exists() {
                    // Fallback for development: cargo run -p kotoba-cli ...
                    let target_dir = bin_dir.parent().ok_or("Could not find target dir")?;
                    let fallback_path = target_dir.join(server_binary_name);
                    if !fallback_path.exists() {
                         eprintln!("Error: kotoba-server binary not found.");
                         eprintln!("Searched at: {} and {}", server_binary_path.display(), fallback_path.display());
                         eprintln!("Please run 'cargo build -p kotoba-server' first.");
                         return Err("kotoba-server binary not found".into());
                    }
                }

                println!("[kotoba cli] Starting kotoba-server in {:?} on port {}", &cwd, port);

                let mut child = std::process::Command::new(server_binary_path)
                    .arg("--port") // Assuming kotoba-server will also use clap to parse this
                    .arg(port.to_string())
                    .current_dir(cwd)
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .spawn()?;
                
                let status = child.wait()?;

                println!("[kotoba cli] kotoba-server process exited with status: {}", status);
                Ok(())
            }
        }
    }
}
