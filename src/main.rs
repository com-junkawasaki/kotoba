//! EAF-IPG Runtime CLI
//!
//! Execute Jsonnet DSL programs using the unified IR runtime.

use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use eaf_ipg_runtime::{validator::validate, Error, engidb::EngiDB, Graph};

#[derive(Parser)]
#[command(name = "eaf-ipg")]
#[command(about = "ENGI EAF-IPG Schema & Runtime")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a Jsonnet DSL program
    Run {
        /// Path to the Jsonnet DSL file
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
    /// Validate a JSON IR file
    Validate {
        /// Path to the JSON IR file
        #[arg(short, long)]
        file: PathBuf,
    },
    /// Test simple JSON evaluation
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
            // Load and evaluate Jsonnet DSL
            let jsonnet_source = fs::read_to_string(&file)?;
            let json_output = rs_jsonnet::evaluate_to_json(&jsonnet_source)
                .map_err(|e| Error::JsonnetEval(e.to_string()))?;

            if export {
                println!("{}", json_output);
                return Ok(());
            }

            // Parse JSON into IR
            let graph: Graph = serde_json::from_str(&json_output)?;

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
