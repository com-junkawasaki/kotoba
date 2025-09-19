//! Kotoba Linter CLI
//!
//! Command line interface for the Kotoba code linter.

use clap::{Arg, ArgMatches, Command};
use std::path::PathBuf;
use kotoba_linter::{Linter, OutputFormat, Reporter, find_kotoba_files};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("kotoba-lint")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Kotoba Team")
        .about("Kotoba Code Linter - Lint .kotoba files for code quality")
        .arg(
            Arg::new("files")
                .help("Files or directories to lint")
                .value_name("FILES")
                .num_args(1..)
                .default_value(".")
        )
        .arg(
            Arg::new("format")
                .long("format")
                .short('f')
                .help("Output format")
                .value_name("FORMAT")
                .value_parser(["pretty", "json", "compact"])
                .default_value("pretty")
        )
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .help("Path to config file")
                .value_name("CONFIG")
        )
        .get_matches();

    let files: Vec<PathBuf> = matches
        .get_many::<String>("files")
        .unwrap_or_default()
        .map(|s| PathBuf::from(s))
        .collect();

    let output_format = match matches.get_one::<String>("format").map(|s| s.as_str()) {
        Some("json") => OutputFormat::Json,
        Some("compact") => OutputFormat::Compact,
        _ => OutputFormat::Pretty,
    };

    // Create linter with custom config if specified
    let linter = if let Some(config_path) = matches.get_one::<String>("config") {
        // Load config from file (not implemented yet)
        println!("⚠️ Custom config loading not yet implemented, using defaults");
        Linter::default()
    } else {
        Linter::default()
    };

    // Collect all files to lint
    let mut all_files = Vec::new();
    for path in files {
        if path.is_dir() {
            let mut dir_files = Vec::new();
            find_kotoba_files(path, &mut dir_files).await?;
            all_files.extend(dir_files);
        } else if path.extension().map_or(false, |ext| ext == "kotoba") {
            all_files.push(path);
        }
    }

    if all_files.is_empty() {
        println!("No .kotoba files found to lint");
        return Ok(());
    }

    println!("🔍 Linting {} file(s)...", all_files.len());

    // Lint all files
    let results = linter.lint_files(all_files).await?;
    let mut reporter = Reporter::new(output_format);
    reporter.report_results(&results)?;

    // Check for errors
    let total_errors = results.iter().map(|r| r.error_count).sum::<usize>();
    if total_errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}
