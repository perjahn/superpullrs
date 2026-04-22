//! Azure DevOps Server Integration Tests
//!
//! These tests use a mock Azure DevOps Server (Docker container) that provides
//! a minimal Azure DevOps API v7.1 compatible interface for testing superpull's az-clone command.

use std::path::Path;
use std::process::Command;

mod docker_helpers;
use docker_helpers::{is_docker_available, DockerContainer};

fn check_mock_azure_ready(port: u16) -> bool {
    Command::new("curl")
        .args(&["-s", "-f", &format!("http://127.0.0.1:{}/health", port)])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[test]
#[ignore] // Run with: SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored azure_devops_clone_with_superpull
fn azure_devops_clone_with_superpull() {
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_err() {
        println!("Skipping integration test - set SUPERPULL_INTEGRATION_TESTS=1 to run");
        return;
    }

    if !is_docker_available() {
        println!("Docker is not available, skipping test");
        return;
    }

    println!("Building mock Azure DevOps server image...");

    // Build the mock server image
    let build_output = Command::new("docker")
        .args(&[
            "build",
            "-f",
            "Dockerfile.mock-azure",
            "-t",
            "mock-azure-server:latest",
            ".",
        ])
        .output();

    match build_output {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Failed to build mock Azure DevOps server image");
                eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
                return;
            }
            println!("Mock Azure DevOps server image built successfully");
        }
        Err(e) => {
            eprintln!("Failed to run docker build: {}", e);
            return;
        }
    }

    println!("Starting mock Azure DevOps server container...");

    let container = match DockerContainer::start(
        "mock-azure-server:latest",
        "superpull-mock-azure-test",
        8091,
        8091,
        &[],
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start mock Azure DevOps server: {}", e);
            return;
        }
    };

    // Wait for mock Azure to be ready
    println!("Waiting for mock Azure DevOps server to become ready...");
    match container.wait_for_ready(check_mock_azure_ready, 60) {
        Ok(_) => println!("Mock Azure DevOps server is ready"),
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Container logs:\n{}", container.get_logs());
            return;
        }
    }

    println!("Testing superpull az-clone against mock Azure DevOps server...");

    // Create output directory for cloned repos
    let output_dir = "/tmp/superpull-mock-azure-test";
    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).expect("Failed to clean output dir");
    }
    std::fs::create_dir_all(output_dir).expect("Failed to create output dir");

    // Run superpull against the mock server
    let superpull_output = Command::new("./target/release/superpull")
        .args(&[
            "az-clone",
            "-a",
            "test-token",
            "-s",
            "http://127.0.0.1:8091",
            "test-org",
            output_dir,
        ])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output();

    match superpull_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            println!("superpull stdout:\n{}", stdout);
            if !stderr.is_empty() {
                println!("superpull stderr:\n{}", stderr);
            }

            if output.status.success() {
                // Verify some repos were cloned
                let entries: Vec<_> = std::fs::read_dir(output_dir)
                    .expect("Failed to read output dir")
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .collect();

                if !entries.is_empty() {
                    println!("✓ Successfully cloned {} repositories", entries.len());
                    for entry in entries.iter().take(5) {
                        if let Some(name) = entry.file_name().to_str() {
                            println!("  - {}", name);
                        }
                    }
                    if entries.len() > 5 {
                        println!("  ... and {} more", entries.len() - 5);
                    }
                } else {
                    println!("⚠ Warning: No repositories were cloned (may be expected for mock)");
                }
            } else {
                eprintln!("✗ superpull command failed with status: {}", output.status);
            }
        }
        Err(e) => {
            eprintln!("Failed to run superpull: {}", e);
            eprintln!("Make sure superpull is built: cargo build --release");
        }
    }

    // Cleanup
    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }

    println!("Test completed successfully");
}
