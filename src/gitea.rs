use anyhow::{anyhow, Result};
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

const PER_PAGE: u32 = 50;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GiteaRepository {
    id: i32,
    name: String,
    #[serde(rename = "clone_url")]
    clone_url: String,
    size: i32,
}

pub async fn super_clone(
    base_url: &str,
    organization: &str,
    folder: &str,
    token: Option<&str>,
    options: CloneOptions,
) -> Result<()> {
    let start = Instant::now();

    let target_folder = if folder.is_empty() { "." } else { folder };

    if !Path::new(target_folder).exists() {
        println!("Creating folder: '{}'", target_folder);
        std::fs::create_dir_all(target_folder)?;
    }

    let gitea_token = token
        .map(|t| t.to_string())
        .or_else(|| env::var("GITEA_TOKEN").ok());

    let filter_opts = FilterOptions::new()
        .with_name_patterns(options.name_patterns.clone())
        .with_exclude_patterns(options.exclude_patterns.clone())
        .with_max_size_kb(options.max_size_kb);

    let repos = get_repos(base_url, organization, &gitea_token, filter_opts).await?;

    if repos.is_empty() {
        if gitea_token.is_none() {
            println!("No git repos found. GITEA_TOKEN environment variable isn't set, for access to private repos it must be set.");
        } else {
            println!("No git repos found.");
        }
        return Ok(());
    }

    let total_repos = repos.len();
    println!("Got {} repos.", total_repos);

    let mut repo_urls: Vec<(String, String)> = repos
        .into_iter()
        .map(|r| {
            let url = r
                .clone_url
                .strip_suffix(".git")
                .unwrap_or(&r.clone_url)
                .to_string();
            (r.name, url)
        })
        .collect();

    repo_urls.sort_by(|a, b| a.1.cmp(&b.1));

    let clone_repos = repo_urls.clone();

    clone_task_manager::execute_clone_tasks(repo_urls, target_folder, options.clone(), |url| {
        let mut clone_url = url.to_string();
        if let Some(token) = &gitea_token {
            if !token.is_empty() {
                if let Some(index) = clone_url.find("://") {
                    clone_url = format!(
                        "{}token:{}@{}",
                        &clone_url[..index + 3],
                        token,
                        &clone_url[index + 3..]
                    );
                }
            }
        }
        clone_url
    })
    .await?;

    if options.create_symlinks {
        symlink_manager::create_symbolic_links(&clone_repos, target_folder).await?;
    }

    println!("{}", format!("Done: {:?}", start.elapsed()).green());
    Ok(())
}

async fn get_repos(
    base_url: &str,
    organization: &str,
    token: &Option<String>,
    filter_opts: FilterOptions,
) -> Result<Vec<GiteaRepository>> {
    let mut repos = get_repos_paginated(base_url, organization, token).await?;
    println!();

    // Apply filters using centralized FilterOptions methods
    let total_repos = repos.len();
    repos.retain(|r| filter_opts.should_include(&r.name, r.size));

    println!("Found {} repos, filtered to {}.", total_repos, repos.len());

    Ok(repos)
}

async fn get_repos_paginated(
    base_url: &str,
    organization: &str,
    token: &Option<String>,
) -> Result<Vec<GiteaRepository>> {
    let mut repos = Vec::new();
    let mut page = 1;

    let base_url = base_url.trim_end_matches('/');

    loop {
        let url = format!(
            "{}/api/v1/orgs/{}/repos?page={}&limit={}",
            base_url, organization, page, PER_PAGE
        );
        println!("Getting repos: '{}'", url);

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("superpull/1.0"));

        if let Some(token) = token {
            if !token.is_empty() {
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("token {}", token))?,
                );
            }
        }

        let response = Client::new().get(&url).headers(headers).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch repos: {} - {}",
                response.status(),
                response.text().await?
            ));
        }

        let response_text = response.text().await?;
        print!(".");
        let _ = io::stdout().flush();
        let _ = debug_utils::save_api_response("gitea", "repos", &response_text);
        let page_repos: Vec<GiteaRepository> = serde_json::from_str(&response_text)?;
        if page_repos.is_empty() {
            break;
        }

        repos.extend(page_repos);
        page += 1;
    }

    Ok(repos)
}
