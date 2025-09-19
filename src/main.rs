//! Kotoba CLI - Core Graph Processing System
//!
//! This binary provides the complete CLI for Kotoba's core graph processing system
//! featuring GP2-based graph rewriting, Event Sourcing, and ISO GQL queries.
//! Supports multiple storage backends (RocksDB, Redis, In-Memory) through
//! Port/Adapter architecture with clean separation of business logic and infrastructure.

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "kotoba")]
#[command(about = "Kotoba - Core Graph Processing System (GP2 + Event Sourcing + ISO GQL)")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(long_about = "
Kotoba is a core graph processing system featuring GP2-based graph rewriting,
complete Event Sourcing, and ISO GQL-compliant queries, built with
Port/Adapter (Hexagonal) Architecture.

Key Features:
• GP2-based graph rewriting with theoretical foundations
• Complete Event Sourcing with projections and materialized views
• ISO GQL-compliant graph queries with pattern matching
• Port/Adapter pattern for pluggable storage backends
• Clean Architecture with dependency inversion
• Multiple storage options: RocksDB, Redis, In-Memory

Examples:
  kotoba --storage rocksdb event create my_stream
  kotoba --storage memory query --file my_query.gql
  kotoba --storage redis rewrite apply rule.jsonnet
")]
struct Cli {
    /// Storage backend to use
    #[arg(long, default_value = "memory")]
    #[arg(value_enum)]
    storage: StorageBackend,

    /// Storage connection URL (for Redis)
    #[arg(long)]
    url: Option<String>,

    /// Storage path (for RocksDB)
    #[arg(long)]
    path: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, clap::ValueEnum)]
enum StorageBackend {
    /// In-memory storage (development/testing)
    Memory,
    /// RocksDB persistent storage
    Rocksdb,
    /// Redis in-memory with persistence
    Redis,
}

#[derive(Clone, clap::ValueEnum)]
enum OutputFormat {
    /// JSON output
    Json,
    /// Pretty-printed JSON
    Pretty,
    /// Text output
    Text,
    /// Table format
    Table,
}

#[derive(Subcommand)]
enum EventCommands {
    /// Create a new event stream
    Create {
        /// Stream name
        name: String,
        /// Stream configuration
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Add event to stream
    Add {
        /// Stream name
        stream: String,
        /// Event type
        event_type: String,
        /// Event data (JSON)
        data: String,
    },
    /// List events in stream
    List {
        /// Stream name
        stream: String,
        /// Maximum number of events to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum RewriteCommands {
    /// Apply graph rewriting rule
    Apply {
        /// Rule file (Jsonnet format)
        rule: PathBuf,
        /// Input graph file
        #[arg(short, long)]
        input: Option<PathBuf>,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Validate rewriting rule
    Validate {
        /// Rule file to validate
        rule: PathBuf,
    },
    /// List available rules
    List,
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

    /// Execute GQL query (ISO GQL-compliant)
    Query {
        /// GQL query string or file path
        query: String,
        /// Output format
        #[arg(short, long, default_value = "json")]
        #[arg(value_enum)]
        format: OutputFormat,
        /// Read query from file
        #[arg(short, long)]
        file: bool,
    },

    /// Event Sourcing commands
    #[command(subcommand)]
    Event(EventCommands),

    /// Graph rewriting commands
    #[command(subcommand)]
    Rewrite(RewriteCommands),

    /// Execute .kotoba file (KotobaScript)
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

    /// Build the project
    #[command(subcommand)]
    Build(BuildCommands),

    /// Lint files
    Lint {
        /// Files or directories to lint
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
    },

    /// Run tests
    Test {
        /// Test filter
        filter: Option<String>,
    },

    /// Deploy the project
    Deploy {
        /// Deployment target
        target: String,
    },

    /// Backup data
    Backup {
        /// Backup destination
        destination: PathBuf,
    },

    /// Restore data
    Restore {
        /// Backup source
        source: PathBuf,
    },

    /// Profile code execution
    Profile {
        /// File to profile
        file: PathBuf,
    },

    /// Manage workflows
    #[command(subcommand)]
    Workflow(WorkflowCommands),

    /// Manage packages
    #[command(subcommand)]
    Package(PackageCommands),

    /// Convert .kotoba to .tsx
    K2tsx {
        /// Input .kotoba file
        input: PathBuf,
        /// Output .tsx file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum BuildCommands {
    /// Run the default build
    Default,
    /// Run a specific task
    Task {
        /// Task name to run
        name: String,
    },
    /// List available tasks
    Tasks,
}

#[derive(Subcommand)]
enum WorkflowCommands {
    /// Run a workflow
    Run {
        /// Workflow file
        file: PathBuf,
    },
    /// List workflows
    List,
}

#[derive(Subcommand)]
enum PackageCommands {
    /// Install a package
    Install {
        /// Package name
        name: String,
    },
    /// Publish a package
    Publish {
        /// Path to package
        path: Option<PathBuf>,
    },
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
        Commands::Build(build_command) => {
            execute_build(build_command).await
        }
        Commands::Lint { paths } => {
            execute_lint(&paths).await
        }
        Commands::Test { filter } => {
            execute_test(filter.as_deref()).await
        }
        Commands::Deploy { target } => {
            execute_deploy(&target).await
        }
        Commands::Backup { destination } => {
            execute_backup(&destination).await
        }
        Commands::Restore { source } => {
            execute_restore(&source).await
        }
        Commands::Profile { file } => {
            execute_profile(&file).await
        }
        Commands::Workflow(workflow_command) => {
            execute_workflow(workflow_command).await
        }
        Commands::Package(package_command) => {
            execute_package(package_command).await
        }
        Commands::K2tsx { input, output } => {
            execute_k2tsx(&input, output.as_deref()).await
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
    }

    Ok(())
}

/// Execute GQL query
async fn execute_query(query: &str, format: &str, _db: Option<&std::path::Path>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Executing GQL query: {}", query);
    println!("📄 Output format: {}", format);

    use kotoba_execution::execution::gql_parser::GqlParser;

    // GQLパーサーを作成
    let parser = GqlParser::new();

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

    Ok(())
}

/// Check and validate files
async fn execute_check(paths: &[std::path::PathBuf], all: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Checking files...");

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

    use kotoba_formatter::{format_files, format_directory};

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
    Ok(())
}

/// Start HTTP server
async fn execute_server(port: u16, host: &str, config: Option<&std::path::Path>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 Starting Kotoba server...");
    println!("📡 Address: {}:{}", host, port);
    if let Some(config_path) = config {
        println!("⚙️  Config: {}", config_path.display());
    }

    kotoba_server::start_server(host, port).await?;

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

    kotoba_package_manager::init_project(name.map(|s| s.to_string())).await?;

    Ok(())
}

/// Generate documentation
async fn execute_doc(output: &std::path::Path, format: &str, source: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("📚 Generating documentation...");
    println!("📂 Source: {}", source.display());
    println!("📁 Output: {}", output.display());
    println!("📄 Format: {}", format);

    // kotoba-kotobas crateを使ってドキュメント生成
    // 簡易実装として、ソースファイルの解析とHTML生成を行う
    println!("⚠️  Full documentation generation not yet implemented");
    println!("💡 Documentation will be generated using kotoba-kotobas parsing capabilities");

    // TODO: 実際のドキュメント生成を実装
    // - ソースファイルの解析
    // - マークダウン/HTML生成
    // - インデックス作成

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

    println!("⚠️  Full REPL implementation not yet complete");
    println!("💡 REPL will be available with kotoba-repl crate integration");

    // TODO: 実際のREPL実装
    // - コマンドライン入力の読み取り
    // - kotoba-repl crateの使用
    // - 履歴管理
    // - スクリプト実行

    Ok(())
}

/// Show version information
async fn execute_version() -> Result<(), Box<dyn std::error::Error>> {
    println!("Kotoba {}", env!("CARGO_PKG_VERSION"));
    println!("GP2-based Graph Rewriting Language");
    println!("Built with Rust {}", rustc_version::version()?);
    Ok(())
}

async fn execute_build(command: BuildCommands) -> Result<(), Box<dyn std::error::Error>> {
    println!("🏗️  Running build command...");

    match command {
        BuildCommands::Default => {
            // Execute kotoba-build binary with default build
            let status = std::process::Command::new("cargo")
                .args(&["run", "-p", "kotoba-build", "--bin", "kotoba-build", "--"])
                .status()?;

            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        BuildCommands::Task { name } => {
            // Execute kotoba-build binary with specific task
            let status = std::process::Command::new("cargo")
                .args(&["run", "-p", "kotoba-build", "--bin", "kotoba-build", "--", &name])
                .status()?;

            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        BuildCommands::Tasks => {
            // Execute kotoba-build binary with --list flag
            let status = std::process::Command::new("cargo")
                .args(&["run", "-p", "kotoba-build", "--bin", "kotoba-build", "--", "--list"])
                .status()?;

            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
    }

    Ok(())
}

async fn execute_lint(paths: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Running linter on paths: {:?}", paths);

    // Prepare command arguments
    let mut args = vec![];
    for path in paths {
        args.push(path.to_str().unwrap_or("."));
    }

    // Execute kotoba-lint binary from kotoba-linter package
    let status = std::process::Command::new("cargo")
        .args(&["run", "-p", "kotoba-linter", "--bin", "kotoba-lint", "--"])
        .args(&args)
        .status()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

async fn execute_test(filter: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Running tests... Filter: {}", filter.unwrap_or("none"));

    // Prepare command arguments
    let mut args = vec![];
    if let Some(f) = filter {
        args.push("--filter");
        args.push(f);
    }

    // Execute kotoba-test binary from kotoba-tester package
    let status = std::process::Command::new("cargo")
        .args(&["run", "-p", "kotoba-tester", "--bin", "kotoba-test", "--"])
        .args(&args)
        .status()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

async fn execute_deploy(target: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Deploying to target: {}", target);

    // Execute kotoba-deploy-cli binary with deploy subcommand
    let status = std::process::Command::new("cargo")
        .args(&["run", "-p", "kotoba-deploy-cli", "--bin", "kotoba-deploy", "--", "deploy", "--name", target])
        .status()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

async fn execute_backup(destination: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("💾 Creating backup to: {}", destination.display());

    // Execute kotoba-backup binary with backup subcommand
    let status = std::process::Command::new("cargo")
        .args(&[
            "run",
            "-p",
            "kotoba-backup",
            "--bin",
            "kotoba-backup",
            "--",
            "backup",
            &destination.to_string_lossy(),
        ])
        .status()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

async fn execute_restore(source: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Restoring from: {}", source.display());

    // Execute kotoba-backup binary with restore subcommand
    let status = std::process::Command::new("cargo")
        .args(&[
            "run",
            "-p",
            "kotoba-backup",
            "--bin",
            "kotoba-backup",
            "--",
            "restore",
            &source.to_string_lossy(),
        ])
        .status()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

async fn execute_profile(file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Profiling file: {}", file.display());

    // Execute kotoba-profiler binary with profile subcommand
    let status = std::process::Command::new("cargo")
        .args(&[
            "run",
            "-p",
            "kotoba-profiler",
            "--bin",
            "kotoba-profiler",
            "--",
            "profile",
            "--db-path",
            &file.to_string_lossy(),
        ])
        .status()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

async fn execute_workflow(command: WorkflowCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        WorkflowCommands::Run { file } => {
            println!("Running workflow: {}", file.display());
            // TODO: Implement using kotoba-workflow
        }
        WorkflowCommands::List => {
            println!("Listing workflows...");
            // TODO: Implement using kotoba-workflow
        }
    }
    Ok(())
}

async fn execute_package(command: PackageCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        PackageCommands::Install { name } => {
            println!("Installing package: {}", name);
            // TODO: Implement using kotoba-package-manager
        }
        PackageCommands::Publish { path } => {
            println!("Publishing package at: {:?}", path);
            // TODO: Implement using kotoba-package-manager
        }
    }
    Ok(())
}

async fn execute_k2tsx(input: &PathBuf, _output: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Converting {} to tsx...", input.display());
    // TODO: Implement using kotoba2tsx
    Ok(())
}
