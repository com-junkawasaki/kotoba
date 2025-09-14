//! # Kotoba2TSX
//!
//! Convert Kotoba configuration files (.kotoba) to React TSX components.
//!
//! This crate provides functionality to parse Jsonnet-based .kotoba files
//! and generate corresponding React TSX component code.

pub mod types;
pub mod parser;
pub mod generator;
pub mod error;

#[cfg(feature = "cli")]
pub mod cli;

pub use types::*;
pub use parser::*;
pub use generator::*;
pub use error::*;

/// Convert a .kotoba file to TSX code
///
/// # Arguments
/// * `input_path` - Path to the .kotoba file
/// * `output_path` - Path where the generated .tsx file will be written
///
/// # Returns
/// Result<(), Kotoba2TSError> indicating success or failure
pub async fn convert_file(input_path: &str, output_path: &str) -> crate::error::Result<()> {
    let parser = KotobaParser::new();
    let config = parser.parse_file(input_path).await?;
    let generator = TsxGenerator::new();
    generator.generate_file(&config, output_path).await?;
    Ok(())
}

/// Convert .kotoba content string to TSX code string
///
/// # Arguments
/// * `content` - The .kotoba file content as a string
///
/// # Returns
/// Result<String, Kotoba2TSError> containing the generated TSX code
pub fn convert_content(content: &str) -> crate::error::Result<String> {
    let parser = KotobaParser::new();
    let config = parser.parse_content(content)?;
    let generator = TsxGenerator::new();
    generator.generate_tsx(&config)
}
