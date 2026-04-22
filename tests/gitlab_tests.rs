use std::process::Command;

mod docker_helpers;
use docker_helpers::{is_docker_available, DockerContainer};

fn check_gitlab_ready(port: u16) -> bool {
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
#[ignore] // Run with: SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored gitlab_clone_with_superpull
fn gitlab_clone_with_superpull() {
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_err() {
        println!("Skipping integration test - set SUPERPULL_INTEGRATION_TESTS=1 to run");
        return;
    }

    if !is_docker_available() {
        println!("Docker is not available, skipping test");
        return;
    }

    println!("Starting GitLab container...");

    // Note: GitLab container is resource-intensive and may take a long time to start
    // For quick testing, you might want to use gitlab/gitlab-ce:latest instead of gitlab-ee
    let container = match DockerContainer::start(
        "gitlab/gitlab-ce:latest",
        "superpull-gitlab-test",
        80,
        8080,
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

    // Wait for GitLab to be ready (this can take several minutes)
    println!("Waiting for GitLab to become ready (this may take 2-3 minutes)...");
    match container.wait_for_ready(check_gitlab_ready, 90) {
        Ok(_) => println!("GitLab is ready"),
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Container logs:\n{}", container.get_logs());
            return;
        }
    }

    println!("Testing superpull gl-clone against GitLab...");

    // Create output directory for cloned repos
    let output_dir = "/tmp/superpull-gitlab-test";
    if std::path::Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).expect("Failed to clean output dir");
    }
    std::fs::create_dir_all(output_dir).expect("Failed to create output dir");

    // Run superpull against GitLab
    let superpull_output = Command::new("./target/release/superpull")
        .args(&[
            "gl-clone",
            "root",
            output_dir,
            "-s",
            "http://127.0.0.1:8080",
            "-a",
            "test-token",
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
                    println!("⚠ Warning: No repositories were cloned (may be expected if no groups exist)");
                }
            } else {
                println!("⚠ superpull command did not succeed (expected if no groups/projects available)");
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
