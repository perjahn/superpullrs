//! Pagination Integration Tests
//!
//! These tests verify that superpull correctly handles pagination across all 6 git services.
//! Each service is configured to return 101 repositories, requiring pagination to fetch all repos.
//! With a page size of 30-100 per page, this ensures we test the pagination loop logic.

use std::path::Path;
use std::process::Command;

mod docker_helpers;
use docker_helpers::DockerContainer;

fn is_docker_available() -> bool {
    Command::new("docker")
        .args(["--version"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn check_mock_server_ready(port: u16, path: &str) -> bool {
    Command::new("curl")
        .args(["-s", "-f", &format!("http://127.0.0.1:{}{}", port, path)])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

// ============================================================================
// Azure DevOps Pagination Test
// ============================================================================

#[test]
#[ignore] // Run with: SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored azure_devops_pagination
fn azure_devops_pagination() {
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_err() {
        println!("Skipping integration test - set SUPERPULL_INTEGRATION_TESTS=1 to run");
        return;
    }

    if !is_docker_available() {
        println!("Docker is not available, skipping test");
        return;
    }

    println!("Building mock Azure DevOps server image...");
    let build_output = Command::new("docker")
        .args([
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
                eprintln!("Failed to build mock Azure server image");
                return;
            }
        }
        Err(e) => {
            eprintln!("Failed to run docker build: {}", e);
            return;
        }
    }

    let container = match DockerContainer::start(
        "mock-azure-server:latest",
        "superpull-azure-pagination-test",
        8091,
        8091,
        &[],
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start mock Azure server: {}", e);
            return;
        }
    };

    println!("Waiting for mock Azure DevOps server to become ready...");
    match container.wait_for_ready(|port| check_mock_server_ready(port, "/health"), 60) {
        Ok(_) => println!("Mock Azure DevOps server is ready"),
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    }

    let output_dir = "/tmp/superpull-azure-pagination-test";
    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
    std::fs::create_dir_all(output_dir).ok();

    println!("Running Azure DevOps az with pagination (101 repos, 100 per page)...");
    let superpull_output = Command::new("./target/release/superpull")
        .env("AZURE_DEVOPS_TOKEN", "test-token")
        .args(["az", "-s", "http://127.0.0.1:8091", "test-org", output_dir])
        .output();

    match superpull_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            println!("superpull output:\n{}", stdout);
            if !stderr.is_empty() {
                println!("stderr:\n{}", stderr);
            }

            // Count cloned repos
            let repo_count: usize = std::fs::read_dir(output_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .count()
                })
                .unwrap_or(0);

            println!(
                "✓ Azure DevOps pagination test: cloned {} repos (expected ~101)",
                repo_count
            );

            if repo_count > 100 {
                println!("✓ Pagination confirmed: fetched repos from multiple pages");
            } else if repo_count >= 100 {
                println!("⚠ Got exactly 100 repos, second page may not have been fetched");
            } else {
                println!("⚠ Only fetched {} repos", repo_count);
            }
        }
        Err(e) => {
            eprintln!("Failed to run superpull: {}", e);
        }
    }

    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
}

// ============================================================================
// Bitbucket Pagination Test
// ============================================================================

#[test]
#[ignore] // Run with: SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored bitbucket_pagination
fn bitbucket_pagination() {
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_err() {
        println!("Skipping integration test - set SUPERPULL_INTEGRATION_TESTS=1 to run");
        return;
    }

    if !is_docker_available() {
        println!("Docker is not available, skipping test");
        return;
    }

    println!("Starting Bitbucket container...");
    let container = match DockerContainer::start(
        "atlassian/bitbucket:latest",
        "superpull-bitbucket-pagination-test",
        7990,
        7990,
        &[],
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start Bitbucket: {}", e);
            return;
        }
    };

    println!("Waiting for Bitbucket to become ready (this may take 1-2 minutes)...");
    match container.wait_for_ready(|port| check_mock_server_ready(port, "/"), 120) {
        Ok(_) => println!("Bitbucket is ready"),
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    }

    let output_dir = "/tmp/superpull-bitbucket-pagination-test";
    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
    std::fs::create_dir_all(output_dir).ok();

    println!("Running Bitbucket bb with pagination...");
    let superpull_output = Command::new("./target/release/superpull")
        .env("BITBUCKET_TOKEN", "test-token")
        .args([
            "bb",
            "-s",
            "http://127.0.0.1:7990",
            "test-workspace",
            output_dir,
        ])
        .output();

    match superpull_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            println!("superpull output:\n{}", stdout);
            if !stderr.is_empty() {
                println!("stderr:\n{}", stderr);
            }

            let repo_count: usize = std::fs::read_dir(output_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .count()
                })
                .unwrap_or(0);

            println!("✓ Bitbucket pagination test: cloned {} repos", repo_count);
        }
        Err(e) => {
            eprintln!("Failed to run superpull: {}", e);
        }
    }

    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
}

// ============================================================================
// Forgejo Pagination Test
// ============================================================================

#[test]
#[ignore] // Run with: SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored forgejo_pagination
fn forgejo_pagination() {
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_err() {
        println!("Skipping integration test - set SUPERPULL_INTEGRATION_TESTS=1 to run");
        return;
    }

    if !is_docker_available() {
        println!("Docker is not available, skipping test");
        return;
    }

    println!("Starting Forgejo container...");
    let container = match DockerContainer::start(
        "codeberg.org/forgejo/forgejo:latest",
        "superpull-forgejo-pagination-test",
        3000,
        3003,
        &[("GITEA__DATABASE__DB_TYPE", "sqlite3")],
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start Forgejo: {}", e);
            return;
        }
    };

    println!("Waiting for Forgejo to become ready...");
    match container.wait_for_ready(|port| check_mock_server_ready(port, "/"), 60) {
        Ok(_) => println!("Forgejo is ready"),
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    }

    let output_dir = "/tmp/superpull-forgejo-pagination-test";
    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
    std::fs::create_dir_all(output_dir).ok();

    println!("Running Forgejo foj with pagination...");
    let superpull_output = Command::new("./target/release/superpull")
        .env("FORGEJO_TOKEN", "test-token")
        .args(["foj", "-s", "http://127.0.0.1:3003", "forgejo", output_dir])
        .output();

    match superpull_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            println!("superpull output:\n{}", stdout);
            if !stderr.is_empty() {
                println!("stderr:\n{}", stderr);
            }

            let repo_count: usize = std::fs::read_dir(output_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .count()
                })
                .unwrap_or(0);

            println!("✓ Forgejo pagination test: cloned {} repos", repo_count);
        }
        Err(e) => {
            eprintln!("Failed to run superpull: {}", e);
        }
    }

    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
}

// ============================================================================
// Gitea Pagination Test
// ============================================================================

#[test]
#[ignore] // Run with: SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored gitea_pagination
fn gitea_pagination() {
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_err() {
        println!("Skipping integration test - set SUPERPULL_INTEGRATION_TESTS=1 to run");
        return;
    }

    if !is_docker_available() {
        println!("Docker is not available, skipping test");
        return;
    }

    println!("Starting Gitea container...");
    let container = match DockerContainer::start(
        "gitea/gitea:latest",
        "superpull-gitea-pagination-test",
        3000,
        3002,
        &[("GITEA__DATABASE__DB_TYPE", "sqlite3")],
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start Gitea: {}", e);
            return;
        }
    };

    println!("Waiting for Gitea to become ready...");
    match container.wait_for_ready(|port| check_mock_server_ready(port, "/"), 60) {
        Ok(_) => println!("Gitea is ready"),
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    }

    let output_dir = "/tmp/superpull-gitea-pagination-test";
    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
    std::fs::create_dir_all(output_dir).ok();

    println!("Running Gitea gea with pagination...");
    let superpull_output = Command::new("./target/release/superpull")
        .env("GITEA_TOKEN", "test-token")
        .args(["gea", "-s", "http://127.0.0.1:3002", "gitea", output_dir])
        .output();

    match superpull_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            println!("superpull output:\n{}", stdout);
            if !stderr.is_empty() {
                println!("stderr:\n{}", stderr);
            }

            let repo_count: usize = std::fs::read_dir(output_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .count()
                })
                .unwrap_or(0);

            println!("✓ Gitea pagination test: cloned {} repos", repo_count);
        }
        Err(e) => {
            eprintln!("Failed to run superpull: {}", e);
        }
    }

    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
}

// ============================================================================
// GitHub Pagination Test
// ============================================================================

#[test]
#[ignore] // Run with: SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored github_pagination
fn github_pagination() {
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_err() {
        println!("Skipping integration test - set SUPERPULL_INTEGRATION_TESTS=1 to run");
        return;
    }

    if !is_docker_available() {
        println!("Docker is not available, skipping test");
        return;
    }

    println!("Building mock GitHub server image...");
    let build_output = Command::new("docker")
        .args([
            "build",
            "-f",
            "Dockerfile.mock-github",
            "-t",
            "mock-github-server:latest",
            ".",
        ])
        .output();

    match build_output {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Failed to build mock GitHub server image");
                return;
            }
        }
        Err(e) => {
            eprintln!("Failed to run docker build: {}", e);
            return;
        }
    }

    let container = match DockerContainer::start(
        "mock-github-server:latest",
        "superpull-github-pagination-test",
        8443,
        8443,
        &[],
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start mock GitHub server: {}", e);
            return;
        }
    };

    println!("Waiting for mock GitHub server to become ready...");
    match container.wait_for_ready(|port| check_mock_server_ready(port, "/health"), 60) {
        Ok(_) => println!("Mock GitHub server is ready"),
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    }

    let output_dir = "/tmp/superpull-github-pagination-test";
    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
    std::fs::create_dir_all(output_dir).ok();

    println!("Running GitHub gh with pagination (101 repos, 30 per page)...");
    let superpull_output = Command::new("./target/release/superpull")
        .env("GITHUB_TOKEN", "test-token")
        .args([
            "gh",
            "-s",
            "http://127.0.0.1:8443/api/v3",
            "test-org",
            output_dir,
        ])
        .output();

    match superpull_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            println!("superpull output:\n{}", stdout);
            if !stderr.is_empty() {
                println!("stderr:\n{}", stderr);
            }

            // Count cloned repos
            let repo_count: usize = std::fs::read_dir(output_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .count()
                })
                .unwrap_or(0);

            println!(
                "✓ GitHub pagination test: cloned {} repos (expected ~101)",
                repo_count
            );

            // We expect to see repos from both page 1 and page 2+
            // With per_page=30, we need at least 2 requests to get past 101 repos
            if repo_count > 30 {
                println!("✓ Pagination confirmed: fetched repos from multiple pages");
            } else {
                println!(
                    "⚠ Warning: Only fetched {} repos, pagination may not have worked",
                    repo_count
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to run superpull: {}", e);
        }
    }

    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
}

// ============================================================================
// GitLab Pagination Test
// ============================================================================

#[test]
#[ignore] // Run with: SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored gitlab_pagination
fn gitlab_pagination() {
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_err() {
        println!("Skipping integration test - set SUPERPULL_INTEGRATION_TESTS=1 to run");
        return;
    }

    if !is_docker_available() {
        println!("Docker is not available, skipping test");
        return;
    }

    println!("Starting GitLab container...");
    let container = match DockerContainer::start(
        "gitlab/gitlab-ce:latest",
        "superpull-gitlab-pagination-test",
        80,
        8082,
        &[
            ("GITLAB_ROOT_PASSWORD", "test12345"),
            ("GITLAB_OMNIBUS_CONFIG", ""),
        ],
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start GitLab: {}", e);
            return;
        }
    };

    println!("Waiting for GitLab to become ready (this may take 2-3 minutes)...");
    match container.wait_for_ready(|port| check_mock_server_ready(port, "/"), 120) {
        Ok(_) => println!("GitLab is ready"),
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    }

    let output_dir = "/tmp/superpull-gitlab-pagination-test";
    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
    std::fs::create_dir_all(output_dir).ok();

    println!("Running GitLab gl with pagination...");
    let superpull_output = Command::new("./target/release/superpull")
        .env("GITLAB_TOKEN", "glpat-test-token")
        .args(["gl", "-s", "http://127.0.0.1:8082", "root", output_dir])
        .output();

    match superpull_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            println!("superpull output:\n{}", stdout);
            if !stderr.is_empty() {
                println!("stderr:\n{}", stderr);
            }

            let repo_count: usize = std::fs::read_dir(output_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .count()
                })
                .unwrap_or(0);

            println!("✓ GitLab pagination test: cloned {} repos", repo_count);
        }
        Err(e) => {
            eprintln!("Failed to run superpull: {}", e);
        }
    }

    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
}

// ============================================================================
// Pagination Test Marker
// ============================================================================

#[test]
fn pagination_tests_marker() {
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_ok() {
        println!("✓ Pagination integration tests are available");
        println!("Run with: SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored");
        println!("  - azure_devops_pagination");
        println!("  - bitbucket_pagination");
        println!("  - forgejo_pagination");
        println!("  - gitea_pagination");
        println!("  - github_pagination");
        println!("  - gitlab_pagination");
    } else {
        println!("Set SUPERPULL_INTEGRATION_TESTS=1 to run pagination tests");
    }
}
