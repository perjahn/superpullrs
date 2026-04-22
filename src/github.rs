use anyhow::{Context, Result};
use base64::Engine;
use colored::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

use crate::clone_options::CloneOptions;
use crate::clone_task_manager;
use crate::debug_utils;
use crate::filter_options::FilterOptions;
use crate::symlink_manager;

const GITHUB_SAAS_API: &str = "https://api.github.com";
const PER_PAGE: u32 = 100;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GithubRepository {
    name: String,
    clone_url: String,
    size: i32,
}

pub async fn super_clone(
    entity: &str,
    folder: &str,
    bearer_token: bool,
    server_url: Option<&str>,
    teams: Vec<String>,
    options: CloneOptions,
) -> Result<()> {
    let start = Instant::now();

    let target_folder = if folder.is_empty() { "." } else { folder };

    if !Path::new(target_folder).exists() {
        println!("Creating folder: '{}'", target_folder);
        std::fs::create_dir_all(target_folder)?;
    }

    let github_token = env::var("GITHUB_TOKEN").unwrap_or_default();

    let filter_opts = FilterOptions::new()
        .with_name_patterns(options.name_patterns.clone())
        .with_exclude_patterns(options.exclude_patterns.clone())
        .with_max_size_kb(options.max_size_kb);

    let repos = get_repos(
        entity,
        &github_token,
        bearer_token,
        server_url,
        &teams,
        filter_opts,
    )
    .await?;

    if repos.is_empty() {
        if github_token.is_empty() {
            println!("No git repos found. GITHUB_TOKEN environment variable isn't set, for access to private repos it must be set.");
        } else {
            println!("No git repos found.");
        }
        return Ok(());
    }

    let total_repos = repos.len();
    println!("Got {} repos.", total_repos);

    let mut repo_urls: Vec<String> = repos
        .into_iter()
        .map(|r| {
            r.clone_url
                .strip_suffix(".git")
                .unwrap_or(&r.clone_url)
                .to_string()
        })
        .collect();

    repo_urls.sort();

    let clone_repos: Vec<(String, String)> = repo_urls
        .iter()
        .map(|url| (clean_url(url), url.clone()))
        .collect();

    clone_task_manager::execute_clone_tasks(
        clone_repos.clone(),
        target_folder,
        options.clone(),
        |url| {
            let mut clone_url = url.to_string();
            if !github_token.is_empty() {
                if let Some(index) = clone_url.find("://") {
                    clone_url = format!(
                        "{}{}@{}",
                        &clone_url[..index + 3],
                        github_token,
                        &clone_url[index + 3..]
                    );
                }
            }
            clone_url
        },
    )
    .await?;

    if options.create_symlinks {
        symlink_manager::create_symbolic_links(&clone_repos, target_folder).await?;
    }

    println!("{}", format!("Done: {:?}", start.elapsed()).green());

    Ok(())
}

async fn get_repos(
    entity: &str,
    github_token: &str,
    bearer_token: bool,
    server_url: Option<&str>,
    teams: &[String],
    filter_opts: FilterOptions,
) -> Result<Vec<GithubRepository>> {
    let api_url = server_url
        .map(|url| url.trim_end_matches('/'))
        .unwrap_or(GITHUB_SAAS_API);
    let mut repos = Vec::new();

    if !teams.is_empty() {
        for team in teams {
            let address = format!(
                "{}/{}/teams/{}/repos?per_page={}",
                api_url, entity, team, PER_PAGE
            );
            repos.extend(get_repos_paginated(&address, github_token, bearer_token).await?);
        }
    } else {
        let address = format!("{}/{}/repos?per_page={}", api_url, entity, PER_PAGE);
        repos = get_repos_paginated(&address, github_token, bearer_token).await?;
    }
    println!();

    // Remove duplicates
    repos.sort_by(|a, b| a.name.cmp(&b.name));
    repos.dedup_by(|a, b| a.name == b.name);

    let total_repos = repos.len();

    // Apply filters using centralized FilterOptions methods
    repos.retain(|r| filter_opts.should_include(&r.name, r.size));

    println!("Found {} repos, filtered to {}.", total_repos, repos.len());

    Ok(repos)
}

async fn get_repos_paginated(
    address: &str,
    github_token: &str,
    bearer_token: bool,
) -> Result<Vec<GithubRepository>> {
    let mut repos = Vec::new();
    let mut next_address = Some(address.to_string());

    while let Some(url) = next_address {
        println!("Getting repos: '{}'", url);

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("superpull/1.0"));

        if !github_token.is_empty() {
            let auth_value = if bearer_token {
                format!("Bearer {}", github_token)
            } else {
                let credentials = format!("{}:x-oauth-basic", github_token);
                format!(
                    "Basic {}",
                    base64::engine::general_purpose::STANDARD.encode(&credentials)
                )
            };
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&auth_value).context("Failed to create auth header")?,
            );
        }

        let client = Client::new();
        let response = client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .context("Failed to fetch repos")?;

        let status = response.status();
        if !status.is_success() {
            if status == 404 {
                break;
            }
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to fetch repos: {} - {}", status, text);
        }

        let response_text = response.text().await.context("Failed to read response")?;
        print!(".");
        let _ = io::stdout().flush();
        let _ = debug_utils::save_api_response("github", "repos", &response_text);
        let page_repos: Vec<GithubRepository> =
            serde_json::from_str(&response_text).context("Failed to parse repos JSON")?;

        repos.extend(page_repos);

        // Check for Link header to get next page
        next_address = None;
    }

    Ok(repos)
}

fn clean_url(url: &str) -> String {
    url.rsplit('/')
        .next()
        .unwrap_or(url)
        .replace("%20", "_")
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_lowercase().to_string()
            } else if c == '-' || c == '.' {
                c.to_string()
            } else {
                "_".to_string()
            }
        })
        .collect::<String>()
        .split("__")
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}
