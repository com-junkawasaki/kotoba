//! Kotoba CLI - Core functionality only
//!
//! This binary provides the core CLI for Kotoba, focusing on stable published crates.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kotoba")]
#[command(about = "Kotoba - Graph processing system core")]
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
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute command
    let result = match cli.command {
        Commands::Info { verbose } => {
            execute_info(verbose).await
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
async fn execute_info(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}
