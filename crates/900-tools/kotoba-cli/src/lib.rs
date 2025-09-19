//! Kotoba CLI - Deno-inspired command line interface
//!
//! Merkle DAG: cli_interface (build_order: 10)
//! Provides: Cli, Commands, ConfigManager, ProgressBar, LogFormatter
//! Dependencies: types, distributed_engine, network_protocol, cid_system

use clap::{Parser, Subcommand};

// Re-export core types for CLI interface
pub use config::ConfigManager;
pub use logging::LogFormatter;
pub use utils::ProgressBar;

// Import modules
pub mod commands;
pub mod config;
pub mod logging;
pub mod utils;

/// Kotoba CLIのメイン構造体
/// Merkle DAG: cli_interface -> Cli component
#[derive(Parser)]
#[command(name = "kotoba")]
#[command(about = "Kotoba - GP2-based Graph Rewriting Language")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Kotoba CLIのサブコマンド
/// Merkle DAG: cli_interface -> Commands component
#[derive(Subcommand)]
pub enum Commands {
    /// プロジェクト情報を表示
    Info {
        /// 詳細表示
        #[arg(short, long)]
        verbose: bool,
    },

    /// 指定されたKotobaファイルを評価
    Eval {
        /// 評価するファイルのパス
        path: String,
    },

    /// ドキュメント生成・管理コマンド
    #[command(subcommand)]
    Docs(DocsCommand),
}

/// ドキュメント関連サブコマンド
/// Merkle DAG: docs_cli -> docs generate, docs serve, docs search, docs init
#[derive(Subcommand)]
pub enum DocsCommand {
    /// ドキュメントを生成
    Generate {
        /// ソースディレクトリ
        #[arg(short, long, default_value = "src")]
        source: String,

        /// 出力ディレクトリ
        #[arg(short, long, default_value = "docs")]
        output: String,

        /// 設定ファイル
        #[arg(short, long)]
        config: Option<String>,

        /// ウォッチモード
        #[arg(short, long)]
        watch: bool,
    },

    /// ドキュメントサーバーを起動
    Serve {
        /// ポート番号
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// ホストアドレス
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,

        /// ドキュメントディレクトリ
        #[arg(short, long, default_value = "docs")]
        dir: String,

        /// オープン後にブラウザで開く
        #[arg(short, long)]
        open: bool,
    },

    /// ドキュメントを検索
    Search {
        /// 検索クエリ
        query: String,

        /// 検索対象ディレクトリ
        #[arg(short, long, default_value = "docs")]
        dir: String,

        /// JSON形式で出力
        #[arg(short, long)]
        json: bool,
    },

    /// ドキュメント設定を初期化
    Init {
        /// 設定ファイル名
        #[arg(short, long, default_value = "kdoc.toml")]
        config: String,

        /// 強制的に上書き
        #[arg(short, long)]
        force: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_info_command_parsing() {
        // Test parsing info command without verbose flag
        let cli = Cli::parse_from(&["kotoba", "info"]);
        match cli.command {
            Commands::Info { verbose } => {
                assert!(!verbose);
            }
            _ => panic!("Expected Info command"),
        }

        // Test parsing info command with verbose flag
        let cli = Cli::parse_from(&["kotoba", "info", "--verbose"]);
        match cli.command {
            Commands::Info { verbose } => {
                assert!(verbose);
            }
            _ => panic!("Expected Info command"),
        }

        // Test parsing info command with short verbose flag
        let cli = Cli::parse_from(&["kotoba", "info", "-v"]);
        match cli.command {
            Commands::Info { verbose } => {
                assert!(verbose);
            }
            _ => panic!("Expected Info command"),
        }
    }

    #[test]
    fn test_cli_eval_command_parsing() {
        let cli = Cli::parse_from(&["kotoba", "eval", "test.kotoba"]);
        match cli.command {
            Commands::Eval { path } => {
                assert_eq!(path, "test.kotoba");
            }
            _ => panic!("Expected Eval command"),
        }
    }

    #[test]
    fn test_cli_docs_generate_command_parsing() {
        // Test with default values
        let cli = Cli::parse_from(&["kotoba", "docs", "generate"]);
        match cli.command {
            Commands::Docs(DocsCommand::Generate { source, output, config, watch }) => {
                assert_eq!(source, "src");
                assert_eq!(output, "docs");
                assert!(config.is_none());
                assert!(!watch);
            }
            _ => panic!("Expected Docs Generate command"),
        }

        // Test with custom values
        let cli = Cli::parse_from(&[
            "kotoba", "docs", "generate",
            "--source", "lib",
            "--output", "build/docs",
            "--config", "docs.toml",
            "--watch"
        ]);
        match cli.command {
            Commands::Docs(DocsCommand::Generate { source, output, config, watch }) => {
                assert_eq!(source, "lib");
                assert_eq!(output, "build/docs");
                assert_eq!(config, Some("docs.toml".to_string()));
                assert!(watch);
            }
            _ => panic!("Expected Docs Generate command"),
        }
    }

    #[test]
    fn test_cli_docs_serve_command_parsing() {
        // Test with default values
        let cli = Cli::parse_from(&["kotoba", "docs", "serve"]);
        match cli.command {
            Commands::Docs(DocsCommand::Serve { port, host, dir, open }) => {
                assert_eq!(port, 3000);
                assert_eq!(host, "127.0.0.1");
                assert_eq!(dir, "docs");
                assert!(!open);
            }
            _ => panic!("Expected Docs Serve command"),
        }

        // Test with custom values
        let cli = Cli::parse_from(&[
            "kotoba", "docs", "serve",
            "--port", "8080",
            "--host", "0.0.0.0",
            "--dir", "public/docs",
            "--open"
        ]);
        match cli.command {
            Commands::Docs(DocsCommand::Serve { port, host, dir, open }) => {
                assert_eq!(port, 8080);
                assert_eq!(host, "0.0.0.0");
                assert_eq!(dir, "public/docs");
                assert!(open);
            }
            _ => panic!("Expected Docs Serve command"),
        }
    }

    #[test]
    fn test_cli_docs_search_command_parsing() {
        // Test with default values
        let cli = Cli::parse_from(&["kotoba", "docs", "search", "query"]);
        match cli.command {
            Commands::Docs(DocsCommand::Search { query, dir, json }) => {
                assert_eq!(query, "query");
                assert_eq!(dir, "docs");
                assert!(!json);
            }
            _ => panic!("Expected Docs Search command"),
        }

        // Test with custom values
        let cli = Cli::parse_from(&[
            "kotoba", "docs", "search", "advanced query",
            "--dir", "custom/docs",
            "--json"
        ]);
        match cli.command {
            Commands::Docs(DocsCommand::Search { query, dir, json }) => {
                assert_eq!(query, "advanced query");
                assert_eq!(dir, "custom/docs");
                assert!(json);
            }
            _ => panic!("Expected Docs Search command"),
        }
    }

    #[test]
    fn test_cli_docs_init_command_parsing() {
        // Test with default values
        let cli = Cli::parse_from(&["kotoba", "docs", "init"]);
        match cli.command {
            Commands::Docs(DocsCommand::Init { config, force }) => {
                assert_eq!(config, "kdoc.toml");
                assert!(!force);
            }
            _ => panic!("Expected Docs Init command"),
        }

        // Test with custom values
        let cli = Cli::parse_from(&[
            "kotoba", "docs", "init",
            "--config", "docs-config.toml",
            "--force"
        ]);
        match cli.command {
            Commands::Docs(DocsCommand::Init { config, force }) => {
                assert_eq!(config, "docs-config.toml");
                assert!(force);
            }
            _ => panic!("Expected Docs Init command"),
        }
    }

    #[test]
    fn test_cli_version_and_name() {
        // Test that CLI has correct name and version
        let cli = Cli::command();
        assert_eq!(cli.get_name(), "kotoba");
        assert!(cli.get_version().is_some());
        assert!(cli.get_about().is_some());
    }

    #[test]
    fn test_cli_debug_formatting() {
        let cli = Cli::parse_from(&["kotoba", "info"]);
        let debug_str = format!("{:?}", cli);
        assert!(debug_str.contains("Cli"));
        assert!(debug_str.contains("Info"));
    }

    #[test]
    fn test_commands_debug_formatting() {
        let info_cmd = Commands::Info { verbose: true };
        let debug_str = format!("{:?}", info_cmd);
        assert!(debug_str.contains("Info"));
        assert!(debug_str.contains("true"));

        let eval_cmd = Commands::Eval { path: "test.kotoba".to_string() };
        let debug_str = format!("{:?}", eval_cmd);
        assert!(debug_str.contains("Eval"));
        assert!(debug_str.contains("test.kotoba"));

        let docs_cmd = Commands::Docs(DocsCommand::Generate {
            source: "src".to_string(),
            output: "docs".to_string(),
            config: None,
            watch: false,
        });
        let debug_str = format!("{:?}", docs_cmd);
        assert!(debug_str.contains("Docs"));
        assert!(debug_str.contains("Generate"));
    }

    #[test]
    fn test_docs_commands_debug_formatting() {
        let generate_cmd = DocsCommand::Generate {
            source: "src".to_string(),
            output: "docs".to_string(),
            config: Some("config.toml".to_string()),
            watch: true,
        };
        let debug_str = format!("{:?}", generate_cmd);
        assert!(debug_str.contains("Generate"));
        assert!(debug_str.contains("src"));
        assert!(debug_str.contains("docs"));
        assert!(debug_str.contains("config.toml"));
        assert!(debug_str.contains("true"));

        let serve_cmd = DocsCommand::Serve {
            port: 8080,
            host: "0.0.0.0".to_string(),
            dir: "public".to_string(),
            open: true,
        };
        let debug_str = format!("{:?}", serve_cmd);
        assert!(debug_str.contains("Serve"));
        assert!(debug_str.contains("8080"));
        assert!(debug_str.contains("0.0.0.0"));
        assert!(debug_str.contains("public"));
        assert!(debug_str.contains("true"));

        let search_cmd = DocsCommand::Search {
            query: "test query".to_string(),
            dir: "docs".to_string(),
            json: true,
        };
        let debug_str = format!("{:?}", search_cmd);
        assert!(debug_str.contains("Search"));
        assert!(debug_str.contains("test query"));
        assert!(debug_str.contains("docs"));
        assert!(debug_str.contains("true"));

        let init_cmd = DocsCommand::Init {
            config: "docs.toml".to_string(),
            force: true,
        };
        let debug_str = format!("{:?}", init_cmd);
        assert!(debug_str.contains("Init"));
        assert!(debug_str.contains("docs.toml"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn test_cli_parsing_edge_cases() {
        // Test empty args (should fail gracefully)
        let result = std::panic::catch_unwind(|| {
            Cli::parse_from::<&[&str], &[&str]>(&[]);
        });
        assert!(result.is_err()); // Should panic or fail

        // Test invalid subcommand
        let result = std::panic::catch_unwind(|| {
            Cli::parse_from(&["kotoba", "invalid"]);
        });
        assert!(result.is_err()); // Should panic or fail
    }

    #[test]
    fn test_cli_with_long_arguments() {
        // Test with very long file paths
        let long_path = "a".repeat(1000);
        let cli = Cli::parse_from(&["kotoba", "eval", &long_path]);
        match cli.command {
            Commands::Eval { path } => {
                assert_eq!(path, long_path);
            }
            _ => panic!("Expected Eval command"),
        }
    }

    #[test]
    fn test_docs_generate_with_all_flags() {
        let cli = Cli::parse_from(&[
            "kotoba", "docs", "generate",
            "-s", "source",
            "-o", "output",
            "-c", "config.toml",
            "-w"
        ]);
        match cli.command {
            Commands::Docs(DocsCommand::Generate { source, output, config, watch }) => {
                assert_eq!(source, "source");
                assert_eq!(output, "output");
                assert_eq!(config, Some("config.toml".to_string()));
                assert!(watch);
            }
            _ => panic!("Expected Docs Generate command"),
        }
    }

    #[test]
    fn test_docs_serve_with_all_flags() {
        let cli = Cli::parse_from(&[
            "kotoba", "docs", "serve",
            "-p", "9000",
            "-H", "localhost",
            "-d", "dist/docs",
            "-o"
        ]);
        match cli.command {
            Commands::Docs(DocsCommand::Serve { port, host, dir, open }) => {
                assert_eq!(port, 9000);
                assert_eq!(host, "localhost");
                assert_eq!(dir, "dist/docs");
                assert!(open);
            }
            _ => panic!("Expected Docs Serve command"),
        }
    }

    #[test]
    fn test_docs_search_with_all_flags() {
        let cli = Cli::parse_from(&[
            "kotoba", "docs", "search", "search term",
            "-d", "custom/dir",
            "-j"
        ]);
        match cli.command {
            Commands::Docs(DocsCommand::Search { query, dir, json }) => {
                assert_eq!(query, "search term");
                assert_eq!(dir, "custom/dir");
                assert!(json);
            }
            _ => panic!("Expected Docs Search command"),
        }
    }

    #[test]
    fn test_docs_init_with_all_flags() {
        let cli = Cli::parse_from(&[
            "kotoba", "docs", "init",
            "-c", "custom-config.toml",
            "-f"
        ]);
        match cli.command {
            Commands::Docs(DocsCommand::Init { config, force }) => {
                assert_eq!(config, "custom-config.toml");
                assert!(force);
            }
            _ => panic!("Expected Docs Init command"),
        }
    }

    #[test]
    fn test_cli_command_variants() {
        // Test all main command variants
        let commands = vec![
            vec!["kotoba", "info"],
            vec!["kotoba", "eval", "file.kotoba"],
            vec!["kotoba", "docs", "generate"],
            vec!["kotoba", "docs", "serve"],
            vec!["kotoba", "docs", "search", "query"],
            vec!["kotoba", "docs", "init"],
        ];

        for args in commands {
            let cli_result = std::panic::catch_unwind(|| {
                Cli::parse_from(&args)
            });
            assert!(cli_result.is_ok(), "Failed to parse command: {:?}", args);
        }
    }

    #[test]
    fn test_docs_command_variants() {
        // Test all docs subcommand variants
        let docs_commands = vec![
            vec!["kotoba", "docs", "generate"],
            vec!["kotoba", "docs", "serve"],
            vec!["kotoba", "docs", "search", "test"],
            vec!["kotoba", "docs", "init"],
        ];

        for args in docs_commands {
            let cli = Cli::parse_from(&args);
            match cli.command {
                Commands::Docs(_) => assert!(true),
                _ => panic!("Expected Docs command variant"),
            }
        }
    }

    #[test]
    fn test_cli_with_special_characters() {
        // Test with special characters in paths and queries
        let cli = Cli::parse_from(&["kotoba", "eval", "/path/to/file-with-dashes.kotoba"]);
        match cli.command {
            Commands::Eval { path } => {
                assert_eq!(path, "/path/to/file-with-dashes.kotoba");
            }
            _ => panic!("Expected Eval command"),
        }

        let cli = Cli::parse_from(&["kotoba", "docs", "search", "query with spaces"]);
        match cli.command {
            Commands::Docs(DocsCommand::Search { query, .. }) => {
                assert_eq!(query, "query with spaces");
            }
            _ => panic!("Expected Docs Search command"),
        }
    }

    #[test]
    fn test_cli_numeric_arguments() {
        // Test with numeric port
        let cli = Cli::parse_from(&["kotoba", "docs", "serve", "--port", "9999"]);
        match cli.command {
            Commands::Docs(DocsCommand::Serve { port, .. }) => {
                assert_eq!(port, 9999);
            }
            _ => panic!("Expected Docs Serve command"),
        }

        // Test with port 0
        let cli = Cli::parse_from(&["kotoba", "docs", "serve", "--port", "0"]);
        match cli.command {
            Commands::Docs(DocsCommand::Serve { port, .. }) => {
                assert_eq!(port, 0);
            }
            _ => panic!("Expected Docs Serve command"),
        }

        // Test with maximum port
        let cli = Cli::parse_from(&["kotoba", "docs", "serve", "--port", "65535"]);
        match cli.command {
            Commands::Docs(DocsCommand::Serve { port, .. }) => {
                assert_eq!(port, 65535);
            }
            _ => panic!("Expected Docs Serve command"),
        }
    }

    #[test]
    fn test_cli_boolean_flag_combinations() {
        // Test various combinations of boolean flags
        let combinations = vec![
            vec!["kotoba", "docs", "generate"],
            vec!["kotoba", "docs", "generate", "--watch"],
            vec!["kotoba", "docs", "serve"],
            vec!["kotoba", "docs", "serve", "--open"],
            vec!["kotoba", "docs", "search", "test"],
            vec!["kotoba", "docs", "search", "test", "--json"],
            vec!["kotoba", "docs", "init"],
            vec!["kotoba", "docs", "init", "--force"],
        ];

        for args in combinations {
            let cli_result = std::panic::catch_unwind(|| {
                Cli::parse_from(&args)
            });
            assert!(cli_result.is_ok(), "Failed to parse combination: {:?}", args);
        }
    }

    #[test]
    fn test_cli_help_and_version() {
        // Test that help can be generated
        let help = Cli::command().render_help();
        let help_str = help.to_string();
        assert!(help_str.contains("kotoba"));
        assert!(help_str.contains("Kotoba"));
        assert!(help_str.contains("info"));
        assert!(help_str.contains("eval"));
        assert!(help_str.contains("docs"));

        // Test that version is available
        let version = Cli::command().get_version().unwrap();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_cli_subcommand_help() {
        // Test help for specific subcommands
        let help_info = Cli::command().find_subcommand("info").unwrap().render_help();
        assert!(help_info.to_string().contains("verbose"));

        let help_docs = Cli::command().find_subcommand("docs").unwrap().render_help();
        assert!(help_docs.to_string().contains("generate"));
        assert!(help_docs.to_string().contains("serve"));
        assert!(help_docs.to_string().contains("search"));
        assert!(help_docs.to_string().contains("init"));
    }

    #[test]
    fn test_cli_argument_validation() {
        // Test that required arguments are enforced
        let result = std::panic::catch_unwind(|| {
            Cli::parse_from(&["kotoba", "eval"]);
        });
        assert!(result.is_err(), "Should fail without path argument");

        let result = std::panic::catch_unwind(|| {
            Cli::parse_from(&["kotoba", "docs", "search"]);
        });
        assert!(result.is_err(), "Should fail without query argument");
    }

    #[test]
    fn test_cli_case_insensitive_parsing() {
        // Test that parsing is case-sensitive for subcommands (clap default)
        let result = std::panic::catch_unwind(|| {
            Cli::parse_from(&["kotoba", "INFO"]);
        });
        assert!(result.is_err(), "Should fail with uppercase subcommand");

        let result = std::panic::catch_unwind(|| {
            Cli::parse_from(&["kotoba", "docs", "GENERATE"]);
        });
        assert!(result.is_err(), "Should fail with uppercase docs subcommand");
    }

    #[test]
    fn test_cli_complex_argument_scenarios() {
        // Test with quoted arguments containing spaces
        let cli = Cli::parse_from(&["kotoba", "eval", "file with spaces.kotoba"]);
        match cli.command {
            Commands::Eval { path } => {
                assert_eq!(path, "file with spaces.kotoba");
            }
            _ => panic!("Expected Eval command"),
        }

        // Test with paths containing special characters
        let cli = Cli::parse_from(&["kotoba", "eval", "/path/to/file_name-v1.2.3.kotoba"]);
        match cli.command {
            Commands::Eval { path } => {
                assert_eq!(path, "/path/to/file_name-v1.2.3.kotoba");
            }
            _ => panic!("Expected Eval command"),
        }
    }
}
