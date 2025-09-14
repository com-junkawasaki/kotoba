//! # Kotoba Kotobanet
//!
//! Kotoba-specific Jsonnet extensions extending the base Jsonnet implementation.
//! This crate provides Kotoba-specific functionality:
//!
//! - HTTP Parser: .kotoba.json configuration parsing
//! - Frontend Framework: React component definitions
//! - Deploy Configuration: Deployment settings
//! - Config Management: General configuration handling

pub mod error;
pub mod http_parser;
pub mod frontend;
pub mod deploy;
pub mod config;

pub use error::*;
pub use http_parser::*;
pub use frontend::*;
pub use deploy::*;
pub use config::*;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Evaluate Kotoba Jsonnet code with extensions
pub fn evaluate_kotoba(code: &str) -> Result<kotoba_jsonnet::JsonnetValue> {
    // TODO: Add Kotoba-specific extensions
    Ok(kotoba_jsonnet::evaluate(code)?)
}

/// Evaluate Kotoba Jsonnet to JSON with extensions
pub fn evaluate_kotoba_to_json(code: &str) -> Result<String> {
    // TODO: Add Kotoba-specific extensions
    Ok(kotoba_jsonnet::evaluate_to_json(code)?)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_kotoba_evaluation() {
        // Test basic Jsonnet functionality still works
        let result = evaluate_kotoba(r#"{ name: "Kotoba", version: 1 }"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_kotoba_to_json() {
        let result = evaluate_kotoba_to_json(r#"{ message: "Hello Kotobanet" }"#);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("Hello Kotobanet"));
    }

    #[test]
    fn test_evaluate_kotoba_simple_expressions() {
        // Test simple object
        let result = evaluate_kotoba(r#"{ name: "test", value: 42 }"#);
        assert!(result.is_ok());

        // Test simple array
        let result = evaluate_kotoba(r#"[1, 2, 3, "four", true]"#);
        assert!(result.is_ok());

        // Test simple string
        let result = evaluate_kotoba(r#""Hello World""#);
        assert!(result.is_ok());

        // Test simple number
        let result = evaluate_kotoba(r#"3.14159"#);
        assert!(result.is_ok());

        // Test simple boolean
        let result = evaluate_kotoba(r#"true"#);
        assert!(result.is_ok());

        // Test null
        let result = evaluate_kotoba(r#"null"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_kotoba_complex_expressions() {
        // Test arithmetic operations
        let result = evaluate_kotoba(r#"(2 + 3) * 4"#);
        assert!(result.is_ok());

        // Test string concatenation
        let result = evaluate_kotoba(r#""Hello " + "World""#);
        assert!(result.is_ok());

        // Test object field access
        let result = evaluate_kotoba(r#"{ a: 1, b: 2 }.a"#);
        assert!(result.is_ok());

        // Test array indexing
        let result = evaluate_kotoba(r#"[10, 20, 30][1]"#);
        assert!(result.is_ok());

        // Test conditional expressions
        let result = evaluate_kotoba(r#"if true then "yes" else "no""#);
        assert!(result.is_ok());

        // Test function definition and call
        let result = evaluate_kotoba(r#"(function(x) x * 2)(5)"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_kotoba_jsonnet_features() {
        // Test object comprehension
        let result = evaluate_kotoba(r#"{ [x]: x * x for x in [1, 2, 3] }"#);
        assert!(result.is_ok());

        // Test array comprehension
        let result = evaluate_kotoba(r#"[x * 2 for x in [1, 2, 3]]"#);
        assert!(result.is_ok());

        // Test local variables
        let result = evaluate_kotoba(r#"local x = 5; local y = 10; x + y"#);
        assert!(result.is_ok());

        // Test imports (basic)
        let result = evaluate_kotoba(r#"local std = { sqrt: function(x) x * x }; std.sqrt(4)"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_kotoba_to_json_simple() {
        // Test object to JSON
        let result = evaluate_kotoba_to_json(r#"{ name: "test", count: 5 }"#);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains(r#""name": "test""#));
        assert!(json.contains(r#""count": 5"#));

        // Test array to JSON
        let result = evaluate_kotoba_to_json(r#"[1, "two", true, null]"#);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains(r#"[1, "two", true, null]"#));

        // Test primitive values to JSON
        let result = evaluate_kotoba_to_json(r#""string value""#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#""string value""#);

        let result = evaluate_kotoba_to_json(r#"42"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"42"#);

        let result = evaluate_kotoba_to_json(r#"true"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"true"#);

        let result = evaluate_kotoba_to_json(r#"null"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"null"#);
    }

    #[test]
    fn test_evaluate_kotoba_to_json_complex() {
        // Test nested objects
        let result = evaluate_kotoba_to_json(r#"
        {
            user: {
                id: 123,
                name: "John Doe",
                email: "john@example.com",
                active: true
            },
            settings: {
                theme: "dark",
                notifications: {
                    email: true,
                    push: false
                }
            },
            tags: ["admin", "premium"]
        }
        "#);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains(r#""id": 123"#));
        assert!(json.contains(r#""name": "John Doe""#));
        assert!(json.contains(r#""theme": "dark""#));
        assert!(json.contains(r#"["admin", "premium"]"#));
    }

    #[test]
    fn test_evaluate_kotoba_to_json_formatting() {
        let result = evaluate_kotoba_to_json(r#"{ a: 1, b: "test", c: true }"#);
        assert!(result.is_ok());
        let json = result.unwrap();

        // Should be valid JSON
        assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok());

        // Should contain expected content
        assert!(json.contains(r#""a": 1"#));
        assert!(json.contains(r#""b": "test""#));
        assert!(json.contains(r#""c": true"#));
    }

    #[test]
    fn test_evaluate_kotoba_error_cases() {
        // Test syntax error
        let result = evaluate_kotoba(r#"{ invalid syntax "#);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KotobaNetError::Jsonnet(_)));

        // Test undefined variable
        let result = evaluate_kotoba(r#"undefined_variable"#);
        assert!(result.is_err());

        // Test type error
        let result = evaluate_kotoba(r#"1 + "string""#);
        assert!(result.is_err());

        // Test division by zero
        let result = evaluate_kotoba(r#"1 / 0"#);
        assert!(result.is_err());

        // Test invalid array access
        let result = evaluate_kotoba(r#"[1, 2, 3][10]"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_kotoba_to_json_error_cases() {
        // Test syntax error
        let result = evaluate_kotoba_to_json(r#"{ invalid syntax "#);
        assert!(result.is_err());

        // Test function cannot be converted to JSON
        let result = evaluate_kotoba_to_json(r#"function(x) x * 2"#);
        assert!(result.is_err());

        // Test builtin cannot be converted to JSON
        let result = evaluate_kotoba_to_json(r#"std.sqrt"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_kotoba_edge_cases() {
        // Test empty object
        let result = evaluate_kotoba(r#"{}"#);
        assert!(result.is_ok());

        // Test empty array
        let result = evaluate_kotoba(r#"[]"#);
        assert!(result.is_ok());

        // Test very large number
        let result = evaluate_kotoba(r#"999999999999999"#);
        assert!(result.is_ok());

        // Test very small number
        let result = evaluate_kotoba(r#"0.000000000000001"#);
        assert!(result.is_ok());

        // Test unicode strings
        let result = evaluate_kotoba(r#""Hello 世界 🌍""#);
        assert!(result.is_ok());

        // Test nested functions
        let result = evaluate_kotoba(r#"(function(f) function(x) f(f(x)))(function(y) y + 1)(5)"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_kotoba_to_json_edge_cases() {
        // Test empty object
        let result = evaluate_kotoba_to_json(r#"{}"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"{}"#);

        // Test empty array
        let result = evaluate_kotoba_to_json(r#"[]"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"[]"#);

        // Test unicode strings
        let result = evaluate_kotoba_to_json(r#""Hello 世界 🌍""#);
        assert!(result.is_ok());
        let json_str = result.unwrap();
        assert!(json_str.contains("世界"));
        assert!(json_str.contains("🌍"));
    }

    #[test]
    fn test_evaluate_kotoba_with_comments() {
        // Test Jsonnet comments are handled
        let result = evaluate_kotoba(r#"
        // This is a comment
        {
            /* Multi-line
               comment */
            value: 42,  // Inline comment
            # Another comment style
            message: "test"
        }
        "#);
        assert!(result.is_ok());

        let result_json = evaluate_kotoba_to_json(r#"
        // This is a comment
        {
            /* Multi-line
               comment */
            value: 42,  // Inline comment
            # Another comment style
            message: "test"
        }
        "#);
        assert!(result_json.is_ok());
        let json = result_json.unwrap();
        assert!(json.contains(r#""value": 42"#));
        assert!(json.contains(r#""message": "test""#));
    }

    #[test]
    fn test_evaluate_kotoba_large_expressions() {
        // Test with a moderately large expression
        let large_expr = format!("{{ {} }}", (0..100).map(|i| format!("field_{}: {}", i, i)).collect::<Vec<_>>().join(", "));
        let result = evaluate_kotoba(&large_expr);
        assert!(result.is_ok());

        let json_result = evaluate_kotoba_to_json(&large_expr);
        assert!(json_result.is_ok());
        let json = json_result.unwrap();
        assert!(json.contains("field_0"));
        assert!(json.contains("field_99"));
    }

    #[test]
    fn test_evaluate_kotoba_special_characters() {
        // Test with special characters in strings
        let result = evaluate_kotoba(r#""special chars: \n\t\"\\""#);
        assert!(result.is_ok());

        let json_result = evaluate_kotoba_to_json(r#""special chars: \n\t\"\\""#);
        assert!(json_result.is_ok());
    }

    #[test]
    fn test_evaluate_kotoba_mathematical_operations() {
        // Test various mathematical operations
        let operations = vec![
            r#"2 + 3"#,
            r#"10 - 4"#,
            r#"3 * 7"#,
            r#"20 / 4"#,
            r#"2 ^ 3"#,
            r#"10 % 3"#,
            r#"-5"#,
            r#"3.14 * 2"#,
            r#"std.max(5, 10)"#,
            r#"std.min(5, 10)"#,
            r#"std.abs(-5)"#,
            r#"std.floor(3.7)"#,
            r#"std.ceil(3.2)"#,
        ];

        for op in operations {
            let result = evaluate_kotoba(op);
            assert!(result.is_ok(), "Failed to evaluate: {}", op);
        }
    }

    #[test]
    fn test_evaluate_kotoba_string_operations() {
        // Test string operations
        let operations = vec![
            r#""hello" + " " + "world""#,
            r#"std.length("hello")"#,
            r#"std.substr("hello", 1, 3)"#,
            r#"std.startsWith("hello", "he")"#,
            r#"std.endsWith("hello", "lo")"#,
            r#"std.contains("hello", "ell")"#,
            r#"std.stringChars("abc")"#,
        ];

        for op in operations {
            let result = evaluate_kotoba(op);
            assert!(result.is_ok(), "Failed to evaluate: {}", op);
        }
    }

    #[test]
    fn test_evaluate_kotoba_array_operations() {
        // Test array operations
        let operations = vec![
            r#"[1, 2, 3] + [4, 5]"#,
            r#"std.length([1, 2, 3])"#,
            r#"[1, 2, 3][1]"#,
            r#"std.slice([1, 2, 3, 4, 5], 1, 3)"#,
            r#"std.map(function(x) x * 2, [1, 2, 3])"#,
            r#"std.filter(function(x) x > 2, [1, 2, 3, 4])"#,
            r#"std.foldl(function(acc, x) acc + x, 0, [1, 2, 3])"#,
        ];

        for op in operations {
            let result = evaluate_kotoba(op);
            assert!(result.is_ok(), "Failed to evaluate: {}", op);
        }
    }

    #[test]
    fn test_evaluate_kotoba_object_operations() {
        // Test object operations
        let operations = vec![
            r#"{ a: 1, b: 2 } + { c: 3 }"#,
            r#"std.objectFields({ a: 1, b: 2 })"#,
            r#"std.objectValues({ a: 1, b: 2 })"#,
            r#"std.objectHas({ a: 1, b: 2 }, "a")"#,
            r#"{ [x]: x * x for x in ["a", "b", "c"] }"#,
        ];

        for op in operations {
            let result = evaluate_kotoba(op);
            assert!(result.is_ok(), "Failed to evaluate: {}", op);
        }
    }

    #[test]
    fn test_version_constant() {
        // Test that VERSION constant is accessible and is a non-empty string
        assert!(!VERSION.is_empty());
        assert!(VERSION.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-'));
    }

    #[test]
    fn test_evaluate_kotoba_with_imports() {
        // Test basic std library usage
        let result = evaluate_kotoba(r#"std.join(" ", ["hello", "world"])"#);
        assert!(result.is_ok());

        let json_result = evaluate_kotoba_to_json(r#"std.join(" ", ["hello", "world"])"#);
        assert!(json_result.is_ok());
        assert_eq!(json_result.unwrap(), r#""hello world""#);
    }

    #[test]
    fn test_evaluate_kotoba_with_std_functions() {
        // Test various std functions
        let std_tests = vec![
            (r#"std.length("hello")"#, r#"5"#),
            (r#"std.length([1, 2, 3])"#, r#"3"#),
            (r#"std.length({ a: 1, b: 2 })"#, r#"2"#),
            (r#"std.toString(42)"#, r#""42""#),
            (r#"std.toString(true)"#, r#""true""#),
            (r#"std.parseInt("123")"#, r#"123"#),
            (r#"std.parseJson("{\"a\": 1}")"#, r#"{"a":1}"#),
            (r#"std.base64("hello")"#, r#""aGVsbG8=""#),
        ];

        for (expr, expected_json) in std_tests {
            let result = evaluate_kotoba_to_json(expr);
            assert!(result.is_ok(), "Failed to evaluate: {}", expr);
            let json = result.unwrap();
            assert_eq!(json, expected_json);
        }
    }

    #[test]
    fn test_evaluate_kotoba_performance() {
        // Test that evaluation doesn't take too long (basic performance test)
        let start = std::time::Instant::now();
        let result = evaluate_kotoba(r#"std.foldl(function(acc, x) acc + x, 0, std.range(1, 1000))"#);
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        // Should complete in reasonable time (less than 1 second for this simple operation)
        assert!(elapsed.as_millis() < 1000, "Took too long: {:?}", elapsed);
    }

    #[test]
    fn test_evaluate_kotoba_memory_safety() {
        // Test that we don't crash with deeply nested structures
        let deep_nested = r#"
        local makeNested = function(depth) if depth == 0 then 42 else { value: makeNested(depth - 1) };
        makeNested(10)
        "#;

        let result = evaluate_kotoba(deep_nested);
        assert!(result.is_ok());

        let json_result = evaluate_kotoba_to_json(deep_nested);
        assert!(json_result.is_ok());
    }

    #[test]
    fn test_integration_file_parsing_http() {
        use crate::http_parser::HttpParser;

        let http_config = r#"
        {
            routes: [
                {
                    path: "/api/users",
                    method: "GET",
                    handler: "getUsers",
                    middleware: ["auth"],
                    authRequired: true,
                }
            ],
            middleware: {
                auth: {
                    type: "jwt",
                }
            }
        }
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(http_config.as_bytes()).unwrap();
        let file_path = temp_file.path();

        let result = HttpParser::parse_file(file_path);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.routes.len(), 1);
        assert_eq!(config.routes[0].path, "/api/users");
        assert_eq!(config.routes[0].method, crate::http_parser::HttpMethod::GET);
    }

    #[test]
    fn test_integration_file_parsing_frontend() {
        use crate::frontend::FrontendParser;

        let frontend_config = r#"
        {
            components: {
                Button: {
                    props: {
                        text: { type: "string", required: true }
                    },
                    render: "<button>{props.text}</button>",
                    imports: ["React"]
                }
            },
            pages: [
                {
                    path: "/",
                    component: "HomePage"
                }
            ]
        }
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(frontend_config.as_bytes()).unwrap();
        let file_path = temp_file.path();

        let result = FrontendParser::parse_file(file_path);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert!(config.components.contains_key("Button"));
        assert_eq!(config.pages.len(), 1);
        assert_eq!(config.pages[0].path, "/");
    }

    #[test]
    fn test_integration_file_parsing_deploy() {
        use crate::deploy::DeployParser;

        let deploy_config = r#"
        {
            name: "test-app",
            version: "1.0.0",
            environment: "production",
            scaling: {
                minInstances: 2,
                maxInstances: 10,
            },
            regions: [{
                name: "us-east-1",
                provider: "AWS",
                instanceType: "t3.medium",
            }]
        }
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(deploy_config.as_bytes()).unwrap();
        let file_path = temp_file.path();

        let result = DeployParser::parse_file(file_path);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.name, "test-app");
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.regions.len(), 1);
        assert_eq!(config.regions[0].name, "us-east-1");
    }

    #[test]
    fn test_integration_file_parsing_config() {
        use crate::config::ConfigParser;

        let app_config = r#"
        {
            app: {
                name: "TestApp",
                version: "1.0.0",
            },
            database: {
                enabled: true,
                driver: "PostgreSQL",
                host: "localhost",
                database: "testdb",
                username: "user",
            },
            features: {
                testFeature: true,
            }
        }
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(app_config.as_bytes()).unwrap();
        let file_path = temp_file.path();

        let result = ConfigParser::parse_file(file_path);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.app.name, "TestApp");
        assert!(config.database.enabled);
        assert!(config.features.flags.get("testFeature").unwrap());
    }

    #[test]
    fn test_integration_file_parsing_complex() {
        use crate::http_parser::HttpParser;
        use crate::frontend::FrontendParser;
        use crate::deploy::DeployParser;
        use crate::config::ConfigParser;

        // Create temporary directory for test files
        let temp_dir = tempfile::tempdir().unwrap();

        // HTTP config file
        let http_path = temp_dir.path().join("http.jsonnet");
        let http_config = r#"
        {
            routes: [
                {
                    path: "/api/users",
                    method: "GET",
                    handler: "getUsers",
                }
            ]
        }
        "#;
        std::fs::write(&http_path, http_config).unwrap();

        // Frontend config file
        let frontend_path = temp_dir.path().join("frontend.jsonnet");
        let frontend_config = r#"
        {
            components: {
                Button: {
                    render: "<button>Click</button>",
                    imports: ["React"]
                }
            }
        }
        "#;
        std::fs::write(&frontend_path, frontend_config).unwrap();

        // Deploy config file
        let deploy_path = temp_dir.path().join("deploy.jsonnet");
        let deploy_config = r#"
        {
            name: "integration-test",
            version: "1.0.0",
            environment: "production",
            regions: [{
                name: "us-east-1",
                provider: "AWS",
                instanceType: "t3.medium",
            }]
        }
        "#;
        std::fs::write(&deploy_path, deploy_config).unwrap();

        // App config file
        let config_path = temp_dir.path().join("config.jsonnet");
        let app_config = r#"
        {
            app: {
                name: "IntegrationTest",
                version: "1.0.0",
            },
            database: {
                enabled: true,
                driver: "PostgreSQL",
                host: "localhost",
                database: "test",
                username: "user",
            }
        }
        "#;
        std::fs::write(&config_path, app_config).unwrap();

        // Test all parsers can read their respective files
        let http_result = HttpParser::parse_file(&http_path);
        assert!(http_result.is_ok());

        let frontend_result = FrontendParser::parse_file(&frontend_path);
        assert!(frontend_result.is_ok());

        let deploy_result = DeployParser::parse_file(&deploy_path);
        assert!(deploy_result.is_ok());

        let config_result = ConfigParser::parse_file(&config_path);
        assert!(config_result.is_ok());

        // Verify parsed content
        let http_config = http_result.unwrap();
        assert_eq!(http_config.routes[0].path, "/api/users");

        let frontend_config = frontend_result.unwrap();
        assert!(frontend_config.components.contains_key("Button"));

        let deploy_config = deploy_result.unwrap();
        assert_eq!(deploy_config.name, "integration-test");

        let app_config = config_result.unwrap();
        assert_eq!(app_config.app.name, "IntegrationTest");
        assert!(app_config.database.enabled);
    }

    #[test]
    fn test_integration_file_parsing_errors() {
        use crate::http_parser::HttpParser;

        // Test non-existent file
        let result = HttpParser::parse_file("/nonexistent/file.jsonnet");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KotobaNetError::Io(_)));

        // Test invalid JSON in file
        let invalid_config = r#"{ invalid syntax "#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(invalid_config.as_bytes()).unwrap();
        let file_path = temp_file.path();

        let result = HttpParser::parse_file(file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_integration_file_parsing_different_extensions() {
        use crate::http_parser::HttpParser;

        let config = r#"
        {
            routes: [
                {
                    path: "/test",
                    method: "GET",
                    handler: "testHandler",
                }
            ]
        }
        "#;

        // Test different file extensions
        let extensions = vec!["jsonnet", "json", "kotoba"];

        for ext in extensions {
            let file_name = format!("test.{}", ext);
            let temp_path = std::env::temp_dir().join(file_name);
            std::fs::write(&temp_path, config).unwrap();

            let result = HttpParser::parse_file(&temp_path);
            assert!(result.is_ok(), "Failed to parse file with extension: {}", ext);

            // Clean up
            let _ = std::fs::remove_file(&temp_path);
        }
    }

    #[test]
    fn test_integration_file_parsing_large_files() {
        use crate::config::ConfigParser;

        // Create a large configuration file
        let mut large_config = r#"
        {
            app: {
                name: "LargeTestApp",
                version: "1.0.0",
            },
            features: {
        "#.to_string();

        // Add many feature flags
        for i in 0..1000 {
            large_config.push_str(&format!("feature_{}: {},\n", i, i % 2 == 0));
        }

        large_config.push_str(r#"
            }
        }
        "#);

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(large_config.as_bytes()).unwrap();
        let file_path = temp_file.path();

        let result = ConfigParser::parse_file(file_path);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.app.name, "LargeTestApp");
        assert_eq!(config.features.flags.len(), 1000);
    }

    #[test]
    fn test_integration_file_parsing_unicode() {
        use crate::frontend::FrontendParser;

        let unicode_config = r#"
        {
            components: {
                "🌟Button": {
                    props: {
                        "📝text": {
                            type: "string",
                            required: true,
                        }
                    },
                    render: "<button>{props.📝text}</button>",
                    imports: ["React"]
                }
            },
            pages: [
                {
                    path: "/🏠",
                    component: "HomePage",
                    meta: {
                        title: "🏠 Home",
                        description: "🏡 Welcome to our site"
                    }
                }
            ]
        }
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(unicode_config.as_bytes()).unwrap();
        let file_path = temp_file.path();

        let result = FrontendParser::parse_file(file_path);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert!(config.components.contains_key("🌟Button"));
        assert_eq!(config.pages[0].path, "/🏠");
        assert_eq!(config.pages[0].meta.as_ref().unwrap()["title"], "🏠 Home");
    }
}
