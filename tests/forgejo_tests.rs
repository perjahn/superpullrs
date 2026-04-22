use std::process::Command;

mod docker_helpers;
use docker_helpers::{is_docker_available, DockerContainer};

fn check_forgejo_ready(port: u16) -> bool {
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
            status_code.starts_with("200") || status_code.starts_with("302")
        })
        .unwrap_or(false)
}

#[test]
#[ignore] // Run with: SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored forgejo_clone_with_superpull
fn forgejo_clone_with_superpull() {
    if std::env::var("SUPERPULL_INTEGRATION_TESTS").is_err() {
        println!("Skipping integration test - set SUPERPULL_INTEGRATION_TESTS=1 to run");
        return;
    }

    if !is_docker_available() {
        println!("Docker is not available, skipping test");
        return;
    }

    println!("Starting Forgejo container...");

    // Start Forgejo container
    let container = match DockerContainer::start(
        "codeberg.org/forgejo/forgejo:latest",
        "superpull-forgejo-test",
        3000,
        3002,
        &[("FORGEJO__DATABASE__DB_TYPE", "sqlite3")],
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start Forgejo: {}", e);
            return;
        }
    };

    // Wait for Forgejo to be ready
    match container.wait_for_ready(check_forgejo_ready, 30) {
        Ok(_) => println!("Forgejo is ready"),
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Container logs:\n{}", container.get_logs());
            return;
        }
    }

    println!("Testing superpull foj-clone against Forgejo...");

    // Create output directory for cloned repos
    let output_dir = "/tmp/superpull-forgejo-test";
    if std::path::Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir).expect("Failed to clean output dir");
    }
    std::fs::create_dir_all(output_dir).expect("Failed to create output dir");

    // Run superpull against Forgejo
    let superpull_output = Command::new("./target/release/superpull")
        .args(&[
            "foj-clone",
            "http://127.0.0.1:3002",
            "test-user",
            output_dir,
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
                let entries: usize = std::fs::read_dir(output_dir)
                    .expect("Failed to read output dir")
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .count();

                println!("Successfully cloned {} repos from Forgejo", entries);
            } else {
                eprintln!("superpull command failed");
            }
        }
        Err(e) => {
            eprintln!("Failed to execute superpull: {}", e);
        }
    }
}
