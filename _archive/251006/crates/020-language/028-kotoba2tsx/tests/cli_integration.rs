//! Integration tests for the kotoba2tsx CLI

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

// Skip CLI integration tests if binary is not available
// These tests require the CLI binary to be built first with: cargo build --release --features cli

const TEST_DATA_DIR: &str = "tests/test_data";
const BINARY_PATH: &str = "../../../target/release/kotoba2tsx";

#[test]
fn test_cli_convert_basic() {
    // Skip if binary doesn't exist
    if !Path::new(BINARY_PATH).exists() {
        println!("Skipping CLI test - binary not found at {}", BINARY_PATH);
        return;
    }

    // Setup test data
    setup_test_data();

    let input_path = format!("{}/basic.kotoba", TEST_DATA_DIR);
    let output_path = format!("{}/basic_output.tsx", TEST_DATA_DIR);

    // Run CLI command
    let output = Command::new(BINARY_PATH)
        .args(&["convert", "--input", &input_path, "--output", &output_path])
        .output()
        .expect("Failed to execute CLI command");

    // Check command succeeded
    assert!(output.status.success(), "CLI command failed: {:?}", output);

    // Check output file exists and contains expected content
    assert!(Path::new(&output_path).exists(), "Output file was not created");

    let content = fs::read_to_string(&output_path).expect("Failed to read output file");
    assert!(content.contains("import React"), "Missing React import");
    assert!(content.contains("const Button:"), "Missing Button component");
    assert!(content.contains("export default App"), "Missing default export");

    // Cleanup
    cleanup_test_data();
}

#[test]
fn test_cli_convert_with_options() {
    // Skip if binary doesn't exist
    if !Path::new(BINARY_PATH).exists() {
        println!("Skipping CLI test - binary not found at {}", BINARY_PATH);
        return;
    }

    setup_test_data();

    let input_path = format!("{}/basic.kotoba", TEST_DATA_DIR);
    let output_path = format!("{}/custom_output.tsx", TEST_DATA_DIR);

    // Run CLI command with custom options
    let output = Command::new(BINARY_PATH)
        .args(&[
            "convert",
            "--input", &input_path,
            "--output", &output_path,
            "--types=false",
            "--functional=false",
            "--prop-types=false"
        ])
        .output()
        .expect("Failed to execute CLI command");

    assert!(output.status.success(), "CLI command failed: {:?}", output);

    let content = fs::read_to_string(&output_path).expect("Failed to read output file");
    assert!(!content.contains("interface"), "Should not contain interfaces when types=false");
    assert!(content.contains("class "), "Should contain class components when functional=false");

    cleanup_test_data();
}

#[test]
fn test_cli_pipe_mode() {
    // Skip if binary doesn't exist
    if !Path::new(BINARY_PATH).exists() {
        println!("Skipping CLI test - binary not found at {}", BINARY_PATH);
        return;
    }

    setup_test_data();

    let input_path = format!("{}/basic.kotoba", TEST_DATA_DIR);

    // Read input file content
    let input_content = fs::read_to_string(&input_path).expect("Failed to read input file");

    // Run CLI command in pipe mode
    let output = Command::new(BINARY_PATH)
        .args(&["pipe", "--types=true", "--functional=true"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn CLI command");

    // Write to stdin
    output.stdin.as_ref().unwrap().write_all(input_content.as_bytes()).unwrap();

    let output_result = output.wait_with_output().expect("Failed to wait for command");
    assert!(output_result.status.success(), "CLI pipe command failed");

    let stdout_content = String::from_utf8(output_result.stdout).expect("Invalid UTF-8 output");

    assert!(stdout_content.contains("import React"), "Missing React import in piped output");
    assert!(stdout_content.contains("const Button:"), "Missing Button component in piped output");
}

#[test]
fn test_cli_invalid_input_file() {
    let output = Command::new(BINARY_PATH)
        .args(&["convert", "--input", "nonexistent.kotoba", "--output", "dummy.tsx"])
        .output()
        .expect("Failed to execute CLI command");

    // Should fail with error
    assert!(!output.status.success(), "CLI should fail with nonexistent input file");
}

#[test]
fn test_cli_help() {
    let output = Command::new(BINARY_PATH)
        .arg("--help")
        .output()
        .expect("Failed to execute help command");

    assert!(output.status.success(), "Help command failed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8 help output");
    assert!(stdout.contains("kotoba2tsx"), "Help should contain program name");
    assert!(stdout.contains("convert"), "Help should contain convert subcommand");
    assert!(stdout.contains("pipe"), "Help should contain pipe subcommand");
}

fn setup_test_data() {
    // Create test data directory
    fs::create_dir_all(TEST_DATA_DIR).expect("Failed to create test data directory");

    // Create basic test .kotoba file
    let basic_kotoba = r#"{
        "config": {
            "name": "TestApp",
            "version": "1.0.0",
            "theme": "light"
        },
        "components": {
            "Button": {
                "type": "component",
                "name": "Button",
                "component_type": "button",
                "props": {
                    "className": "btn",
                    "disabled": false
                },
                "children": []
            },
            "App": {
                "type": "component",
                "name": "App",
                "component_type": "div",
                "props": {
                    "className": "app"
                },
                "children": ["Button"]
            }
        },
        "handlers": {},
        "states": {}
    }"#;

    fs::write(format!("{}/basic.kotoba", TEST_DATA_DIR), basic_kotoba)
        .expect("Failed to write test kotoba file");
}

fn cleanup_test_data() {
    // Remove test data directory and all contents
    if Path::new(TEST_DATA_DIR).exists() {
        fs::remove_dir_all(TEST_DATA_DIR).expect("Failed to cleanup test data");
    }
}
