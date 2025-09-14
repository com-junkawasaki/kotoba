//! Main binary for kotoba2tsx CLI

use clap::Parser;
use kotoba2tsx::cli::Cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    cli.run().await
}
