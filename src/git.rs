use crate::process_manager::ProcessManager;
use anyhow::Result;
use colored::*;
use std::collections::VecDeque;
use std::path::Path;
use std::time::Instant;
use tokio::task::JoinHandle;
use walkdir::WalkDir;

pub async fn super_pull(folder: &str, recurse: bool, throttle: usize, timeout: u64) -> Result<()> {
    let start = Instant::now();

    let search_folder = if folder.is_empty() { "." } else { folder };

    if !Path::new(search_folder).exists() {
        anyhow::bail!("Folder not found: '{}'", search_folder);
    }

    // Find all git repositories
    let mut repos = Vec::new();

    if recurse {
        for entry in WalkDir::new(search_folder)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == ".git" && entry.file_type().is_dir() {
                if let Some(parent) = entry.path().parent() {
                    repos.push(parent.to_string_lossy().to_string());
                }
            }
        }
    } else {
        for entry in std::fs::read_dir(search_folder)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_dir() && path.join(".git").exists() {
                repos.push(path.to_string_lossy().into_owned());
            }
        }
    }

    repos.sort();

    println!("Found {} repos.", repos.len());

    let mut tasks: VecDeque<JoinHandle<(String, Result<bool>)>> = VecDeque::new();
    let mut count = 0;

    for repo_folder in &repos {
        // Wait if we have too many tasks running
        while tasks.len() >= throttle {
            if let Some(task) = tasks.pop_front() {
                if let Ok((folder_name, result)) = task.await {
                    match result {
                        Ok(_) => println!("{}", format!("Done: {}", folder_name).green()),
                        Err(e) => println!("{}", format!("Error: {} - {}", folder_name, e).red()),
                    }
                }
            }
        }

        count += 1;
        let total = repos.len();
        let repo_name = Path::new(repo_folder)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        println!(
            "{}",
            format!("Pulling ({}/{}) {}...", count, total, repo_name).green()
        );

        let pm_clone = ProcessManager::new(throttle, timeout);
        let repo_folder_clone = repo_folder.clone();
        let repo_name_clone = repo_name.clone();

        let task = tokio::spawn(async move {
            let result = pm_clone
                .run_git_command(&repo_folder_clone, &["pull", "-r"])
                .await;
            (repo_name_clone, result)
        });

        tasks.push_back(task);
    }

    // Wait for remaining tasks
    while let Some(task) = tasks.pop_front() {
        if let Ok((folder_name, result)) = task.await {
            match result {
                Ok(_) => println!("{}", format!("Done: {}", folder_name).green()),
                Err(e) => println!("{}", format!("Error: {} - {}", folder_name, e).red()),
            }
        }
    }

    println!("{}", format!("Done: {:?}", start.elapsed()).green());

    Ok(())
}
