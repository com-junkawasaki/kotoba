//! EAF-IPG Runtime CLI
//!
//! Execute JSON-based graph programs using the unified IR runtime.

use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use eaf_ipg_runtime::{validator::validate, Error, engidb::EngiDB, Graph};

#[derive(Parser)]
#[command(name = "eaf-ipg")]
#[command(about = "Kotoba - Language Graph Database")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a JSON graph program
    Run {
        /// Path to the JSON graph file
        #[arg(short, long)]
        file: PathBuf,

        /// Path to the EngiDB database file
        #[arg(long)]
        db: PathBuf,

        /// Branch to commit to
        #[arg(long, default_value = "main")]
        branch: String,

        /// Commit author
        #[arg(long, default_value = "kotoba-cli")]
        author: String,

        /// Commit message
        #[arg(short, long)]
        message: String,

        /// Export mode: export JSON without execution
        #[arg(long)]
        export: bool,
    },
    /// Validate a JSON graph file
    Validate {
        /// Path to the JSON graph file
        #[arg(short, long)]
        file: PathBuf,
    },
    /// Test JSON parsing
    TestJson {
        /// Path to the JSON file
        #[arg(short, long)]
        file: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, export, db, branch, author, message } => {
            // Load JSON file
            let json_content = fs::read_to_string(&file)?;

            if export {
                println!("{}", json_content);
                return Ok(());
            }

            // Parse JSON into Graph
            let graph: Graph = serde_json::from_str(&json_content)?;

            // Open the database
            let engidb = EngiDB::open(&db)?;

            // Import the graph
            println!("Importing graph into database...");
            engidb.import_graph(&graph)?;
            println!("Import complete.");

            // Commit the changes
            println!("Committing to branch '{}'...", branch);
            let commit_cid = engidb.commit(&branch, author, message)?;
            println!("Successfully committed with CID: {}", commit_cid);
            
            // // Validate
            // validate(&graph)?;

            // // Lower to execution DAG
            // let exec_dag = lower_to_exec_dag(&graph)?;

            // // Execute
            // let mut runtime = eaf_ipg_runtime::Runtime::new();
            // schedule_and_run(&mut runtime, &exec_dag).await?;

            // println!("Execution completed successfully");
        }

        Commands::Validate { file } => {
            let json_content = fs::read_to_string(&file)?;
            let graph: Graph = serde_json::from_str(&json_content)?;

            match validate(&graph) {
                Ok(_) => println!("✓ Validation passed"),
                Err(Error::Validation(e)) => {
                    eprintln!("✗ Validation failed: {}", e);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("✗ Unexpected error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::TestJson { file } => {
            let json_content = fs::read_to_string(&file)?;
            let value: serde_json::Value = serde_json::from_str(&json_content)?;
            println!("✓ JSON parsed successfully: {}", value);
        }
    }

    Ok(())
}
