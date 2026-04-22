use colored::*;
use std::collections::VecDeque;
use std::path::Path;
use tokio::task::JoinHandle;

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
    let mut tasks: VecDeque<JoinHandle<(String, Result<(), anyhow::Error>)>> = VecDeque::new();
    let mut count = 0;

    for (repo_name, repo_url) in repos {
        // Wait if we have too many tasks running
        while tasks.len() >= options.throttle {
            if let Some(task) = tasks.pop_front() {
                if let Ok((folder_name, result)) = task.await {
                    match result {
                        Ok(_) => println!("{}", format!("Cloned: {}", folder_name).green()),
                        Err(e) => println!("{}", format!("Error: {} - {}", folder_name, e).red()),
                    }
                }
            }
        }

        count += 1;

        let target_path = if target_folder == "." {
            repo_name.clone()
        } else {
            format!("{}/{}", target_folder, repo_name)
        };

        if Path::new(&target_path).exists() {
            println!("Folder already exists: '{}'", target_path);
            continue;
        }

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

        let task = tokio::spawn(async move {
            let result = pm_clone
                .run_git_command(".", &["clone", &clone_url, &target_path_clone])
                .await
                .map(|_| ());
            (target_path_clone, result)
        });

        tasks.push_back(task);
    }

    // Wait for remaining tasks
    while let Some(task) = tasks.pop_front() {
        if let Ok((folder_name, result)) = task.await {
            match result {
                Ok(_) => println!("{}", format!("Cloned: {}", folder_name).green()),
                Err(e) => println!("{}", format!("Error: {} - {}", folder_name, e).red()),
            }
        }
    }

    Ok(())
}
