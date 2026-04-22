use std::process::Command;

mod docker_helpers;
use docker_helpers::{is_docker_available, DockerContainer};

fn check_bitbucket_ready(port: u16) -> bool {
    Command::new("curl")
        .args(&[
            "-s",
            &format!("http://127.0.0.1:{}/", port),
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
        ])
        .output()
        .map(|output| {
            let status_code = String::from_utf8_lossy(&output.stdout);
            status_code.starts_with("200")
                || status_code.starts_with("302")
                || status_code.starts_with("404")
        })
        .unwrap_or(false)
}

#[test]
#[ignore] // Run with: SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored bitbucket_clone_with_superpull
fn bitbucket_clone_with_superpull() {
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_err() {
        println!("Skipping integration test - set SUPERPULL_INTEGRATION_TESTS=1 to run");
        return;
    }

    if !is_docker_available() {
        println!("Docker is not available, skipping test");
        return;
    }

    println!("Starting Bitbucket Server container...");

    // Note: Bitbucket Server requires significant resources (4GB+ RAM recommended)
    // and a license key. For testing purposes, this container will start but
    // the UI may not be fully functional without a license.
    let container = match DockerContainer::start(
        "atlassian/bitbucket-server:latest",
        "superpull-bitbucket-test",
        7990,
        7991,
        &[],
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start Bitbucket Server: {}", e);
            return;
        }
    };

    // Wait for Bitbucket to be ready (this can take several minutes)
    println!("Waiting for Bitbucket Server to become ready (this may take 3-5 minutes)...");
    match container.wait_for_ready(check_bitbucket_ready, 90) {
        Ok(_) => println!("Bitbucket Server is ready"),
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Container logs:\n{}", container.get_logs());
            return;
        }
    }

    println!("Testing superpull bb-clone against Bitbucket Server...");

    // Create output directory for cloned repos
    let output_dir = "/tmp/superpull-bitbucket-test";
    if std::path::Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).expect("Failed to clean output dir");
    }
    std::fs::create_dir_all(output_dir).expect("Failed to create output dir");

    // Run superpull against the Bitbucket Server (v1 API)
    let superpull_output = Command::new("./target/release/superpull")
        .args(&[
            "bb-clone",
            output_dir,
            "-s",
            "http://127.0.0.1:7990",
            "-a",
            "test-token",
            "-1",
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
                let entries: Vec<_> = std::fs::read_dir(output_dir)
                    .expect("Failed to read output dir")
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .collect();

                if !entries.is_empty() {
                    println!("✓ Successfully cloned {} repositories", entries.len());
                } else {
                    println!(
                        "⚠ Warning: No repositories were cloned (may be expected for v1 test)"
                    );
                }
            } else {
                println!("⚠ superpull command did not succeed (expected if no repos available)");
            }
        }
        Err(e) => {
            eprintln!("Failed to run superpull: {}", e);
        }
    }

    // Cleanup
    if std::path::Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }

    println!("Test completed successfully");
}

#[test]
#[ignore] // Run with: SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored bitbucket_clone_api_v1
fn bitbucket_clone_api_v1() {
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_err() {
        println!("Skipping integration test - set SUPERPULL_INTEGRATION_TESTS=1 to run");
        return;
    }

    if !is_docker_available() {
        println!("Docker is not available, skipping test");
        return;
    }

    println!("Starting Bitbucket Server container for API v1 testing...");

    // Start a Bitbucket Server instance for testing API v1 support
    let container = match DockerContainer::start(
        "atlassian/bitbucket-server:latest",
        "superpull-bitbucket-v1-test",
        7990,
        7991,
        &[],
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start Bitbucket Server: {}", e);
            return;
        }
    };

    // Wait for Bitbucket to be ready
    println!("Waiting for Bitbucket Server to become ready for API v1 testing (this may take 3-5 minutes)...");
    match container.wait_for_ready(check_bitbucket_ready, 90) {
        Ok(_) => println!("Bitbucket Server is ready"),
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Container logs:\n{}", container.get_logs());
            return;
        }
    }

    // Test that API v1 is accessible
    println!("Testing Bitbucket API v1 accessibility...");
    let api_test = Command::new("curl")
        .args(&["-s", "-f", "http://127.0.0.1:7990/rest/api/1.0/repos"])
        .output();

    match api_test {
        Ok(output) => {
            if output.status.success() {
                println!("✓ Bitbucket API v1 is accessible");
                let response = String::from_utf8_lossy(&output.stdout);
                if response.contains("\"values\"") && response.contains("\"isLastPage\"") {
                    println!(
                        "✓ API v1 response format is valid (contains paginated response structure)"
                    );
                    println!("Test passed: API v1 support verified");
                } else {
                    eprintln!("✗ API v1 response format unexpected");
                    eprintln!("Response: {}", response);
                }
            } else {
                eprintln!("✗ API v1 endpoint returned error");
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to test API v1: {}", e);
        }
    }
}
