use colored::*;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::task::JoinSet;

use crate::clone_options::CloneOptions;
use crate::process_manager::ProcessManager;

/// Execute clone tasks with throttling and service-specific token injection.
///
/// # Arguments
/// * `repos` - Vector of (repo_name, url) tuples to clone
/// * `target_folder` - Base folder to clone repositories into
/// * `options` - Clone options including throttle and timeout
/// * `token_injector` - Closure that transforms clone URL with service-specific auth
pub async fn execute_clone_tasks<F>(
    repos: Vec<(String, String)>,
    target_folder: &str,
    options: CloneOptions,
    mut token_injector: F,
) -> Result<(), anyhow::Error>
where
    F: FnMut(&str) -> String,
{
    let total_repos = repos.len();
    let mut task_set: JoinSet<(String, Result<(), anyhow::Error>, std::time::Duration, u64)> =
        JoinSet::new();
    let mut count = 0;
    let start_time = Instant::now();

    // Create log file
    let log_path = format!("{}/superpull.log", target_folder);
    let log_file = Arc::new(Mutex::new(
        OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&log_path)?,
    ));

    if let Ok(mut file) = log_file.lock() {
        let _ = writeln!(file, "=== Superpull Clone Log ===\n");
    }

    let mut repos_iter = repos.into_iter();
    let mut total_size: u64 = 0;

    loop {
        // Keep filling set up to throttle limit
        while task_set.len() < options.throttle {
            if let Some((repo_name, repo_url)) = repos_iter.next() {
                let target_path = if target_folder == "." {
                    repo_name.clone()
                } else {
                    format!("{}/{}", target_folder, repo_name)
                };

                if Path::new(&target_path).exists() {
                    println!("Folder already exists: '{}'", target_path);
                    continue;
                }

                count += 1;
                println!(
                    "{}",
                    format!(
                        "Cloning ({}/{}): '{}' -> '{}'",
                        count, total_repos, repo_url, target_path
                    )
                    .green()
                );

                let clone_url = token_injector(&repo_url);
                let pm_clone = ProcessManager::new(options.throttle, options.timeout);
                let target_path_clone = target_path.clone();
                let log_file_clone = Arc::clone(&log_file);

                task_set.spawn(async move {
                    let clone_start = Instant::now();

                    let (success, output, is_timeout) = match pm_clone
                        .run_git_command(".", &["clone", &clone_url, &target_path_clone])
                        .await
                    {
                        Ok((success, output, is_timeout)) => (success, output, is_timeout),
                        Err(e) => (false, e.to_string(), false),
                    };

                    let clone_elapsed = clone_start.elapsed();

                    // Log the git command output
                    if let Ok(mut file) = log_file_clone.lock() {
                        if !output.is_empty() {
                            let _ = writeln!(file, "{}", output);
                        }
                        let _ = file.flush();
                    }

                    let result = if success {
                        Ok(())
                    } else {
                        let error_msg = if is_timeout {
                            "Clone timeout".to_string()
                        } else {
                            output.lines().last().unwrap_or("Clone failed").to_string()
                        };
                        Err(std::io::Error::other(error_msg).into())
                    };

                    // Calculate size of cloned directory
                    let dir_size = if result.is_ok() {
                        calculate_directory_size(&target_path_clone).unwrap_or(0)
                    } else {
                        0
                    };

                    (target_path_clone, result, clone_elapsed, dir_size)
                });
            } else {
                break;
            }
        }

        // If no tasks left, we're done
        if task_set.is_empty() {
            break;
        }

        // Await the task that completes first
        if let Some(Ok((folder_name, result, elapsed, dir_size))) = task_set.join_next().await {
            total_size += dir_size;
            let throughput = if elapsed.as_secs_f64() > 0.0 {
                dir_size as f64 / elapsed.as_secs_f64()
            } else {
                0.0
            };

            match result {
                Ok(_) => {
                    let msg = format!(
                        "Cloned: {} | {:.2}s | {:.2} MB | {:.2} MB/s",
                        folder_name,
                        elapsed.as_secs_f64(),
                        dir_size as f64 / (1024.0 * 1024.0),
                        throughput / (1024.0 * 1024.0)
                    );
                    println!("{}", msg.green());
                }
                Err(e) => {
                    let msg = format!("Error: {} - {}", folder_name, e);
                    println!("{}", msg.red());
                }
            }
        }
    }

    // Calculate total bytes cloned
    let elapsed = start_time.elapsed();
    let throughput = if elapsed.as_secs_f64() > 0.0 {
        total_size as f64 / elapsed.as_secs_f64()
    } else {
        0.0
    };

    println!();
    println!("Clone operation completed:");
    println!("  Time: {:.2}s", elapsed.as_secs_f64());
    println!(
        "  Total size: {:.2} MB",
        total_size as f64 / (1024.0 * 1024.0)
    );
    println!("  Throughput: {:.2} MB/s", throughput / (1024.0 * 1024.0));

    // Log summary to file
    if let Ok(mut file) = log_file.lock() {
        let _ = writeln!(file, "\n=== Summary ===");
        let _ = writeln!(file, "Total time: {:.2}s", elapsed.as_secs_f64());
        let _ = writeln!(
            file,
            "Total size: {:.2} MB",
            total_size as f64 / (1024.0 * 1024.0)
        );
        let _ = writeln!(
            file,
            "Throughput: {:.2} MB/s",
            throughput / (1024.0 * 1024.0)
        );
        let _ = writeln!(file, "Log saved to: {}", log_path);
    }

    println!("Log saved to: {}", log_path);

    Ok(())
}

/// Calculate the total size of all files in a directory recursively
fn calculate_directory_size(path: &str) -> Result<u64, anyhow::Error> {
    let mut total_size: u64 = 0;

    fn walk_dir(path: &Path, total: &mut u64) -> std::io::Result<()> {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    *total += metadata.len();
                }
            } else if path.is_dir() {
                walk_dir(&path, total)?;
            }
        }
        Ok(())
    }

    if Path::new(path).exists() {
        walk_dir(Path::new(path), &mut total_size)?;
    }

    Ok(total_size)
}
