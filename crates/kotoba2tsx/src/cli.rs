//! Command line interface for kotoba2tsx

use clap::{Parser, Subcommand};
use std::path::Path;

/// Kotoba to TSX converter CLI
#[derive(Parser)]
#[command(name = "kotoba2tsx")]
#[command(about = "Convert Kotoba configuration files to React TSX components")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Convert a .kotoba file to .tsx
    Convert {
        /// Input .kotoba file path
        #[arg(short, long)]
        input: String,

        /// Output .tsx file path
        #[arg(short, long)]
        output: String,

        /// Include TypeScript types
        #[arg(long, default_value = "true")]
        types: bool,

        /// Use functional components
        #[arg(long, default_value = "true")]
        functional: bool,

        /// Include prop types
        #[arg(long, default_value = "true")]
        prop_types: bool,

        /// Format output code
        #[arg(long, default_value = "true")]
        format: bool,
    },
    /// Convert .kotoba content from stdin to stdout
    Pipe {
        /// Include TypeScript types
        #[arg(long, default_value = "true")]
        types: bool,

        /// Use functional components
        #[arg(long, default_value = "true")]
        functional: bool,

        /// Include prop types
        #[arg(long, default_value = "true")]
        prop_types: bool,

        /// Format output code
        #[arg(long, default_value = "true")]
        format: bool,
    },
}

impl Cli {
    /// Run the CLI application
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.command {
            Commands::Convert {
                input,
                output,
                types,
                functional,
                prop_types,
                format,
            } => {
                self.convert_file(input, output, *types, *functional, *prop_types, *format).await?;
            }
            Commands::Pipe {
                types,
                functional,
                prop_types,
                format,
            } => {
                self.convert_stdin(*types, *functional, *prop_types, *format).await?;
            }
        }
        Ok(())
    }

    async fn convert_file(
        &self,
        input: &str,
        output: &str,
        include_types: bool,
        use_functional: bool,
        include_prop_types: bool,
        format_output: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Store output path to avoid partial move
        let output_path = output.to_string();
        use crate::{KotobaParser, TsxGenerator, TsxGenerationOptions};

        // Check if input file exists
        if !Path::new(input).exists() {
            eprintln!("Error: Input file '{}' does not exist", input);
            std::process::exit(1);
        }

        // Check if input file has .kotoba extension
        if !input.ends_with(".kotoba") {
            eprintln!("Warning: Input file '{}' does not have .kotoba extension", input);
        }

        // Check if output file has .tsx extension
        if !output.ends_with(".tsx") {
            eprintln!("Warning: Output file '{}' does not have .tsx extension", output);
        }

        println!("Converting {} to {}...", input, output);

        // Parse the .kotoba file
        let parser = KotobaParser::new();
        let config = match parser.parse_file(input).await {
            Ok(config) => {
                println!("✓ Successfully parsed {}", input);
                config
            }
            Err(e) => {
                eprintln!("Error parsing {}: {}", input, e);
                std::process::exit(1);
            }
        };

        // Generate TSX code
        let options = TsxGenerationOptions {
            include_types,
            include_imports: true,
            use_functional,
            include_prop_types,
            include_default_props: true,
            format_output,
        };

        let generator = TsxGenerator::with_options(options);
        match generator.generate_file(&config, &output_path).await {
            Ok(_) => {
                println!("✓ Successfully generated {}", output_path);
            }
            Err(e) => {
                eprintln!("Error generating TSX: {}", e);
                std::process::exit(1);
            }
        }

        Ok(())
    }

    async fn convert_stdin(
        &self,
        include_types: bool,
        use_functional: bool,
        include_prop_types: bool,
        format_output: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::{KotobaParser, TsxGenerator, TsxGenerationOptions};
        use std::io::{self, Read};

        // Read from stdin
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;

        if content.trim().is_empty() {
            eprintln!("Error: No input received from stdin");
            std::process::exit(1);
        }

        // Parse the content
        let parser = KotobaParser::new();
        let config = match parser.parse_content(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Error parsing input: {}", e);
                std::process::exit(1);
            }
        };

        // Generate TSX code
        let options = TsxGenerationOptions {
            include_types,
            include_imports: true,
            use_functional,
            include_prop_types,
            include_default_props: true,
            format_output,
        };

        let generator = TsxGenerator::with_options(options);
        let tsx_code = match generator.generate_tsx(&config) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error generating TSX: {}", e);
                std::process::exit(1);
            }
        };

        // Output to stdout
        println!("{}", tsx_code);

        Ok(())
    }
}
