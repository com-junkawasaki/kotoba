//! Kotoba CLI - Complete command line interface
//!
//! This binary provides the complete CLI for Kotoba with all core features.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "kotoba")]
#[command(about = "Kotoba - GP2-based Graph Rewriting Language")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show project information
    Info {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Execute GQL query
    Query {
        /// GQL query string
        query: String,
        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Database file path
        #[arg(short, long)]
        db: Option<PathBuf>,
    },

    /// Execute .kotoba file
    Run {
        /// File to execute
        file: PathBuf,
        /// Arguments to pass to the script
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Watch mode - restart on file changes
        #[arg(short, long)]
        watch: bool,
    },

    /// Check and validate files
    Check {
        /// Files or directories to check
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
        /// Check all files recursively
        #[arg(short, long)]
        all: bool,
    },

    /// Format code files
    Fmt {
        /// Files or directories to format
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
        /// Check only, don't modify files
        #[arg(long)]
        check: bool,
        /// Format all files recursively
        #[arg(short, long)]
        all: bool,
    },

    /// Start HTTP server
    Server {
        /// Server port
        #[arg(short, long, default_value = "3000")]
        port: u16,
        /// Server host
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Initialize new project
    Init {
        /// Project name
        name: Option<String>,
        /// Project template
        #[arg(short, long, default_value = "basic")]
        template: String,
        /// Force overwrite existing files
        #[arg(short, long)]
        force: bool,
    },

    /// Generate documentation
    Doc {
        /// Output directory
        #[arg(short, long, default_value = "./docs")]
        output: PathBuf,
        /// Documentation format
        #[arg(short, long, default_value = "html")]
        format: String,
        /// Source directory
        #[arg(long, default_value = "src")]
        source: PathBuf,
    },

    /// Start interactive REPL
    Repl {
        /// Script file to load on startup
        #[arg(short, long)]
        script: Option<PathBuf>,
        /// History file path
        #[arg(long)]
        history: Option<PathBuf>,
    },

    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute command
    let result = match cli.command {
        Commands::Info { verbose, json } => {
            execute_info(verbose, json).await
        }
        Commands::Query { query, format, db } => {
            execute_query(&query, &format, db.as_deref()).await
        }
        Commands::Run { file, args, watch } => {
            execute_run(&file, &args, watch).await
        }
        Commands::Check { paths, all } => {
            execute_check(&paths, all).await
        }
        Commands::Fmt { paths, check, all } => {
            execute_fmt(&paths, check, all).await
        }
        Commands::Server { port, host, config } => {
            execute_server(port, &host, config.as_deref()).await
        }
        Commands::Init { name, template, force } => {
            execute_init(name.as_deref(), &template, force).await
        }
        Commands::Doc { output, format, source } => {
            execute_doc(&output, &format, &source).await
        }
        Commands::Repl { script, history } => {
            execute_repl(script.as_deref(), history.as_deref()).await
        }
        Commands::Version => {
            execute_version().await
        }
    };

    // Handle result
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

/// Execute the info command
async fn execute_info(verbose: bool, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        let info = serde_json::json!({
            "name": "Kotoba",
            "version": env!("CARGO_PKG_VERSION"),
            "architecture": "Process Network Graph Model",
            "description": "GP2-based Graph Rewriting Language - ISO GQL-compliant queries, MVCC+Merkle persistence, and distributed execution",
            "core_libraries": {
                "kotoba-core": "0.1.21",
                "kotoba-errors": "0.1.2",
                "kotoba-graph": "0.1.21",
                "kotoba-storage": "0.1.21",
                "kotoba-execution": "0.1.21",
                "kotoba-rewrite": "0.1.21"
            },
            "features": ["graph-rewriting", "gql-queries", "mvcc-storage", "distributed-execution"]
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("🌟 Kotoba - Graph Processing System Core");
        println!("=======================================");
        println!("📦 Version: {}", env!("CARGO_PKG_VERSION"));
        println!("🏗️  Architecture: Process Network Graph Model");
        println!("📚 Core Libraries:");

        if verbose {
            println!("  ✅ kotoba-core v0.1.21 (Published)");
            println!("  ✅ kotoba-errors v0.1.2 (Published)");
            println!("  ✅ kotoba-graph v0.1.21 (Published)");
            println!("  ✅ kotoba-storage v0.1.21 (Published)");
            println!("  ✅ kotoba-execution v0.1.21 (Published)");
            println!("  ✅ kotoba-rewrite v0.1.21 (Published)");
        } else {
            println!("  ✅ Core crates published to crates.io");
        }

        println!("\n🚀 Workflow and server features are optional and disabled for core stability.");
        println!("🔄 Use published crates directly for advanced features.");
    }

    Ok(())
}

/// Execute GQL query
async fn execute_query(query: &str, format: &str, _db: Option<&std::path::Path>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Executing GQL query: {}", query);
    println!("📄 Output format: {}", format);

    #[cfg(feature = "execution")]
    {
        use kotoba_execution::execution::gql_parser::GqlParser;

        // GQLパーサーを作成
        let mut parser = GqlParser::new();

        // クエリを解析
        match parser.parse(query) {
            Ok(parsed_query) => {
                println!("✅ Query parsed successfully");
                println!("📊 Parsed query: {:?}", parsed_query);

                // クエリ実行（簡易実装）
                println!("⚠️  Full query execution not yet implemented");
                println!("💡 Query structure parsed, but execution requires storage backend");
            }
            Err(e) => {
                println!("❌ Failed to parse query: {}", e);
                return Err(format!("Failed to parse query: {}", e).into());
            }
        }
    }

    #[cfg(not(feature = "execution"))]
    {
        println!("⚠️  Query execution not available - build with --features execution");
        println!("💡 Use published kotoba-execution crate for query functionality");
    }

    Ok(())
}

/// Execute .kotoba file
async fn execute_run(file: &std::path::Path, args: &[String], watch: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Running file: {}", file.display());
    if !args.is_empty() {
        println!("📝 Arguments: {:?}", args);
    }
    if watch {
        println!("👀 Watch mode enabled");
    }

    // ファイルの存在チェック
    if !file.exists() {
        println!("❌ File not found: {}", file.display());
        return Err(format!("File not found: {}", file.display()).into());
    }

    #[cfg(feature = "kotobas")]
    {
        use kotoba_kotobas::evaluate_kotoba;

        // ファイルを読み込み
        let content = tokio::fs::read_to_string(file).await?;

        // Jsonnet/Jsonnet拡張として評価
        match evaluate_kotoba(&content) {
            Ok(result) => {
                println!("✅ File executed successfully");
                println!("📄 Result: {:?}", result);

                // TODO: より詳細な実行結果の処理
                println!("⚠️  Full execution pipeline not yet implemented");
            }
            Err(e) => {
                println!("❌ Failed to execute file: {}", e);
                return Err(format!("Failed to execute file: {}", e).into());
            }
        }
    }

    #[cfg(not(feature = "kotobas"))]
    {
        println!("⚠️  File execution not available - build with --features kotobas");
        println!("💡 Use published kotoba-kotobas crate for .kotoba file execution");
    }

    Ok(())
}

/// Check and validate files
async fn execute_check(paths: &[std::path::PathBuf], all: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Checking files...");

    #[cfg(feature = "formatter")]
    {
        use kotoba_formatter::format_files;

        let mut files_to_check = Vec::new();

        for path in paths {
            if path.is_file() {
                // 単一ファイルの場合
                if path.extension().map_or(false, |ext| ext == "kotoba") {
                    files_to_check.push(path.clone());
                } else {
                    println!("⚠️  Skipping non-.kotoba file: {}", path.display());
                }
            } else if path.is_dir() {
                // ディレクトリの場合
                if all {
                    println!("  🔄 Checking all .kotoba files in: {}", path.display());
                    use kotoba_formatter::format_directory;
                    let results = format_directory(path.clone(), true).await?;
                    for result in results {
                        if result.has_changes {
                            println!("❌ File needs formatting: {}", result.file_path.display());
                        } else if result.error.is_some() {
                            println!("❌ File has syntax errors: {} ({})",
                                   result.file_path.display(),
                                   result.error.as_ref().unwrap());
                        } else {
                            println!("✅ File is valid: {}", result.file_path.display());
                        }
                    }
                    return Ok(());
                } else {
                    println!("⚠️  Directory checking requires --all flag: {}", path.display());
                }
            }
        }

        if !files_to_check.is_empty() {
            println!("  📋 Checking {} file(s)...", files_to_check.len());

            let results = format_files(files_to_check, true).await?;
            let mut has_errors = false;

            for result in results {
                if result.error.is_some() {
                    println!("❌ Syntax error in {}: {}",
                           result.file_path.display(),
                           result.error.as_ref().unwrap());
                    has_errors = true;
                } else if result.has_changes {
                    println!("❌ File needs formatting: {}", result.file_path.display());
                    has_errors = true;
                } else {
                    println!("✅ File is valid: {}", result.file_path.display());
                }
            }

            if has_errors {
                println!("💡 Run 'kotoba fmt' to fix formatting issues");
                return Err("Files have validation errors".into());
            }
        }
    }

    #[cfg(not(feature = "formatter"))]
    {
        println!("⚠️  File checking not available - build with --features formatter");
        println!("💡 Use published kotoba-formatter crate for file validation");
    }

    Ok(())
}

/// Format code files
async fn execute_fmt(paths: &[std::path::PathBuf], check: bool, all: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Formatting code...");
    if check {
        println!("🔍 Check-only mode (no changes will be made)");
    }
    if all {
        println!("🔄 Formatting all files recursively");
    }

    #[cfg(feature = "formatter")]
    {
        use kotoba_formatter::{format_files, format_directory};
        use tokio::fs;

        let mut total_files = 0;
        let mut formatted_files = 0;
        let mut error_files = 0;

        for path in paths {
            if path.is_file() {
                // 単一ファイルの場合
                if path.extension().map_or(false, |ext| ext == "kotoba") {
                    let results = format_files(vec![path.clone()], check).await?;
                    total_files += 1;

                    for result in results {
                        if result.error.is_some() {
                            println!("❌ Failed to format {}: {}",
                                   result.file_path.display(),
                                   result.error.as_ref().unwrap());
                            error_files += 1;
                        } else if result.has_changes && !check {
                            println!("✅ Formatted: {}", result.file_path.display());
                            formatted_files += 1;
                        } else if result.has_changes && check {
                            println!("⚠️  Needs formatting: {}", result.file_path.display());
                        } else {
                            println!("📋 Already formatted: {}", result.file_path.display());
                        }
                    }
                } else {
                    println!("⚠️  Skipping non-.kotoba file: {}", path.display());
                }
            } else if path.is_dir() {
                // ディレクトリの場合
                if all {
                    println!("📁 Formatting directory: {}", path.display());
                    let results = format_directory(path.clone(), check).await?;
                    total_files += results.len();

                    for result in results {
                        if result.error.is_some() {
                            println!("❌ Failed to format {}: {}",
                                   result.file_path.display(),
                                   result.error.as_ref().unwrap());
                            error_files += 1;
                        } else if result.has_changes && !check {
                            println!("✅ Formatted: {}", result.file_path.display());
                            formatted_files += 1;
                        } else if result.has_changes && check {
                            println!("⚠️  Needs formatting: {}", result.file_path.display());
                        } else if !check {
                            println!("📋 Already formatted: {}", result.file_path.display());
                        }
                    }
                } else {
                    println!("⚠️  Directory formatting requires --all flag: {}", path.display());
                }
            }
        }

        // サマリー出力
        println!("\n📊 Formatting Summary:");
        println!("   Total files: {}", total_files);
        if !check {
            println!("   Formatted files: {}", formatted_files);
        } else {
            println!("   Files needing formatting: {}", formatted_files);
        }
        if error_files > 0 {
            println!("   Files with errors: {}", error_files);
        }

        if check && formatted_files > 0 {
            println!("💡 Run 'kotoba fmt' without --check to apply formatting");
            return Err("Some files need formatting".into());
        }

        if error_files > 0 {
            return Err("Some files had formatting errors".into());
        }
    }

    #[cfg(not(feature = "formatter"))]
    {
        println!("⚠️  Code formatting not available - build with --features formatter");
        println!("💡 Use published kotoba-formatter crate for code formatting");
    }

    Ok(())
}

/// Start HTTP server
async fn execute_server(port: u16, host: &str, config: Option<&std::path::Path>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 Starting Kotoba server...");
    println!("📡 Address: {}:{}", host, port);
    if let Some(config_path) = config {
        println!("⚙️  Config: {}", config_path.display());
    }

    #[cfg(feature = "server")]
    {
        kotoba_server::start_server(host, port).await?;
    }

    #[cfg(not(feature = "server"))]
    {
        println!("⚠️  Server feature not enabled");
        println!("💡 Build with --features server or use published kotoba-server crate");
    }

    Ok(())
}

/// Initialize new project
async fn execute_init(name: Option<&str>, template: &str, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("📦 Initializing new Kotoba project...");
    if let Some(project_name) = name {
        println!("📝 Project name: {}", project_name);
    }
    println!("🎨 Template: {}", template);
    if force {
        println!("💪 Force mode enabled");
    }

    #[cfg(feature = "package-manager")]
    {
        kotoba_package_manager::init_project(name.map(|s| s.to_string())).await?;
    }

    #[cfg(not(feature = "package-manager"))]
    {
        println!("⚠️  Package manager feature not enabled");
        println!("💡 Build with --features package-manager or use published kotoba-package-manager crate");
    }

    Ok(())
}

/// Generate documentation
async fn execute_doc(output: &std::path::Path, format: &str, source: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("📚 Generating documentation...");
    println!("📂 Source: {}", source.display());
    println!("📁 Output: {}", output.display());
    println!("📄 Format: {}", format);

    #[cfg(feature = "kotobas")]
    {
        // kotoba-kotobas crateを使ってドキュメント生成
        // 簡易実装として、ソースファイルの解析とHTML生成を行う
        println!("⚠️  Full documentation generation not yet implemented");
        println!("💡 Documentation will be generated using kotoba-kotobas parsing capabilities");

        // TODO: 実際のドキュメント生成を実装
        // - ソースファイルの解析
        // - マークダウン/HTML生成
        // - インデックス作成
    }

    #[cfg(not(feature = "kotobas"))]
    {
        println!("⚠️  Documentation generation not available - build with --features kotobas");
        println!("💡 Use published documentation tools for full documentation generation");
    }

    Ok(())
}

/// Start interactive REPL
async fn execute_repl(script: Option<&std::path::Path>, history: Option<&std::path::Path>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🖥️  Starting Kotoba REPL...");
    if let Some(script_path) = script {
        println!("📄 Loading script: {}", script_path.display());
    }
    if let Some(history_path) = history {
        println!("📝 History file: {}", history_path.display());
    }

    #[cfg(feature = "repl")]
    {
        println!("⚠️  Full REPL implementation not yet complete");
        println!("💡 REPL will be available with kotoba-repl crate integration");

        // TODO: 実際のREPL実装
        // - コマンドライン入力の読み取り
        // - kotoba-repl crateの使用
        // - 履歴管理
        // - スクリプト実行
    }

    #[cfg(not(feature = "repl"))]
    {
        println!("⚠️  REPL not available - build with --features repl");
        println!("💡 Use published kotoba-repl crate for interactive development");
    }

    Ok(())
}

/// Show version information
async fn execute_version() -> Result<(), Box<dyn std::error::Error>> {
    println!("Kotoba {}", env!("CARGO_PKG_VERSION"));
    println!("GP2-based Graph Rewriting Language");
    println!("Built with Rust {}", rustc_version::version()?);
    Ok(())
}
