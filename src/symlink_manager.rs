use anyhow::Result;
use std::path::Path;

use crate::process_manager::ProcessManager;

/// Create symbolic links for git submodules across all cloned repositories.
///
/// Processes all repositories in the target folder and creates symlinks for any
/// git submodules defined in .gitmodules. This enables better organization of
/// submodule dependencies.
///
/// # Arguments
/// * `repos` - Vector of (repo_name, _) tuples (url is ignored)
/// * `target_folder` - Base folder where repositories were cloned
pub async fn create_symbolic_links(repos: &[(String, String)], target_folder: &str) -> Result<()> {
    for (repo_name, _) in repos {
        let repo_path = if target_folder == "." {
            repo_name.clone()
        } else {
            format!("{}/{}", target_folder, repo_name)
        };

        if !Path::new(&repo_path).exists() {
            println!("Warning: Folder not found: '{}'", repo_path);
            continue;
        }

        let pm = ProcessManager::new(1, 60);
        match pm
            .run_command_with_output(
                &repo_path,
                "git",
                &["config", "--file", ".gitmodules", "--get-regexp", "path"],
            )
            .await
        {
            Ok(output) => {
                if output.trim().is_empty() {
                    continue;
                }

                let submodules: Vec<&str> = output
                    .lines()
                    .filter_map(|line: &str| {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            Some(parts[1])
                        } else {
                            None
                        }
                    })
                    .collect();

                for submodule in submodules {
                    let target = format!("../{}", submodule);
                    let symlink_path = format!("{}/{}", repo_path, submodule);

                    if let Ok(metadata) = std::fs::symlink_metadata(&symlink_path) {
                        if metadata.is_symlink() {
                            if let Ok(link_target) = std::fs::read_link(&symlink_path) {
                                if link_target.to_string_lossy() == target {
                                    println!(
                                        "Existing symbolic link for submodule: '{}' '{}' -> '{}'",
                                        repo_path, submodule, target
                                    );
                                    continue;
                                }
                            }
                            let _ = std::fs::remove_file(&symlink_path);
                        }
                    }

                    println!(
                        "Creating symbolic link for submodule: '{}' '{}' -> '{}'",
                        repo_path, submodule, target
                    );

                    let _ = std::os::unix::fs::symlink(&target, &symlink_path);
                }
            }
            Err(_) => {
                // No submodules, continue
            }
        }
    }

    Ok(())
}
