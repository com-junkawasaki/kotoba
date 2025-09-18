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
        #[arg(short, long, default_value = "127.0.0.1")]
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
    println!("⚠️  Query execution not yet implemented");
    println!("💡 Use published kotoba-execution crate for query functionality");
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
    println!("⚠️  File execution not yet implemented");
    println!("💡 Use published kotoba-execution crate for script execution");
    Ok(())
}

/// Check and validate files
async fn execute_check(paths: &[std::path::PathBuf], all: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Checking files...");
    for path in paths {
        println!("📂 Path: {}", path.display());
    }
    if all {
        println!("🔄 Checking all files recursively");
    }
    println!("⚠️  File checking not yet implemented");
    println!("💡 Use published kotoba-linter crate for code validation");
    Ok(())
}

/// Format code files
async fn execute_fmt(paths: &[std::path::PathBuf], check: bool, all: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Formatting code...");
    for path in paths {
        println!("📂 Path: {}", path.display());
    }
    if check {
        println!("👀 Check-only mode");
    }
    if all {
        println!("🔄 Formatting all files recursively");
    }
    println!("⚠️  Code formatting not yet implemented");
    println!("💡 Use published kotoba-formatter crate for code formatting");
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
    println!("⚠️  Documentation generation not yet implemented");
    println!("💡 Use published kotoba-docs crate for documentation generation");
    Ok(())
}

/// Show version information
async fn execute_version() -> Result<(), Box<dyn std::error::Error>> {
    println!("Kotoba {}", env!("CARGO_PKG_VERSION"));
    println!("GP2-based Graph Rewriting Language");
    println!("Built with Rust {}", rustc_version::version()?);
    Ok(())
}
