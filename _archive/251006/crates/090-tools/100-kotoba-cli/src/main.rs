//! Kotoba CLI - Pure Kernel & Effects Shell Architecture

/// Pure representation of CLI commands
#[derive(Debug, Clone, PartialEq)]
pub enum CliCommand {
    Build { release: bool },
    Test { filter: Option<String> },
    Help,
    Version,
}

/// Pure CLI parser - no side effects
pub struct PureCliParser;

impl PureCliParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse command line arguments (pure function)
    pub fn parse_args(&self, args: &[String]) -> Result<CliCommand, String> {
        match args.get(1).map(|s| s.as_str()) {
            Some("build") => Ok(CliCommand::Build {
                release: args.contains(&"--release".to_string())
            }),
            Some("test") => Ok(CliCommand::Test {
                filter: args.get(2).cloned()
            }),
            Some("help") | Some("--help") => Ok(CliCommand::Help),
            Some("version") | Some("--version") => Ok(CliCommand::Version),
            _ => Err("Unknown command".to_string()),
        }
    }

    /// Generate help text (pure function)
    pub fn generate_help(&self) -> String {
        "Kotoba CLI - Pure Kernel & Effects Shell\n\n\
         COMMANDS:\n\
         \x20\x20build    Build project\n\
         \x20\x20test     Run tests\n\
         \x20\x20help     Show help\n".to_string()
    }
}

/// Effects Shell CLI executor
pub struct CliExecutor;

impl CliExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Execute command (effects: I/O, external commands)
    pub async fn execute(&self, command: CliCommand) -> Result<(), String> {
        match command {
            CliCommand::Build { release } => {
                println!("Building project (release: {})...", release);
                // In real implementation: run cargo build
                Ok(())
            }
            CliCommand::Test { filter } => {
                println!("Running tests (filter: {:?})...", filter);
                // In real implementation: run cargo test
                Ok(())
            }
            CliCommand::Help => {
                println!("{}", PureCliParser::new().generate_help());
                Ok(())
            }
            CliCommand::Version => {
                println!("Kotoba CLI v{}", env!("CARGO_PKG_VERSION"));
                Ok(())
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let parser = PureCliParser::new();
    let executor = CliExecutor::new();

    match parser.parse_args(&args) {
        Ok(command) => {
            if let Err(e) = executor.execute(command).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            println!("{}", parser.generate_help());
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_parsing() {
        let parser = PureCliParser::new();

        let args = vec!["kotoba".to_string(), "build".to_string(), "--release".to_string()];
        let command = parser.parse_args(&args).unwrap();
        assert!(matches!(command, CliCommand::Build { release: true }));

        let args = vec!["kotoba".to_string(), "test".to_string(), "auth".to_string()];
        let command = parser.parse_args(&args).unwrap();
        assert!(matches!(command, CliCommand::Test { filter: Some(ref f) } if f == "auth"));
    }
}
