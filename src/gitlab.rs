use anyhow::{anyhow, Result};
use colored::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use std::time::Instant;

use crate::clone_options::CloneOptions;
use crate::clone_task_manager;
use crate::filter_options::FilterOptions;
use crate::symlink_manager;

const GITLAB_API: &str = "https://gitlab.com/api/v4";
const PER_PAGE: u32 = 100;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GitlabProject {
    id: i32,
    name: String,
    #[serde(rename = "http_url_to_repo")]
    http_url_to_repo: String,
    size: Option<i32>,
}

pub async fn super_clone(
    group_or_user: &str,
    folder: &str,
    is_group: bool,
    token: Option<&str>,
    server_url: Option<&str>,
    options: CloneOptions,
) -> Result<()> {
    let start = Instant::now();

    let target_folder = if folder.is_empty() { "." } else { folder };

    if !Path::new(target_folder).exists() {
        println!("Creating folder: '{}'", target_folder);
        std::fs::create_dir_all(target_folder)?;
    }

    let gitlab_token = token
        .map(|t| t.to_string())
        .or_else(|| env::var("GITLAB_TOKEN").ok())
        .or_else(|| env::var("CI_JOB_TOKEN").ok());

    let filter_opts = FilterOptions::new()
        .with_name_patterns(options.name_patterns.clone())
        .with_exclude_patterns(options.exclude_patterns.clone())
        .with_max_size_kb(options.max_size_kb);

    let repos = get_projects(
        group_or_user,
        &gitlab_token,
        is_group,
        server_url,
        filter_opts,
    )
    .await?;

    if repos.is_empty() {
        if gitlab_token.is_none() {
            println!("No git repos found. GITLAB_TOKEN environment variable isn't set, for access to private projects it must be set.");
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
                .http_url_to_repo
                .strip_suffix(".git")
                .unwrap_or(&r.http_url_to_repo)
                .to_string();
            (r.name, url)
        })
        .collect();

    repo_urls.sort_by(|a, b| a.1.cmp(&b.1));

    let clone_repos = repo_urls.clone();

    clone_task_manager::execute_clone_tasks(repo_urls, target_folder, options.clone(), |url| {
        let mut clone_url = url.to_string();
        if let Some(token) = &gitlab_token {
            if !token.is_empty() {
                if let Some(index) = clone_url.find("://") {
                    clone_url = format!(
                        "{}oauth2:{}@{}",
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

async fn get_projects(
    group_or_user: &str,
    token: &Option<String>,
    is_group: bool,
    server_url: Option<&str>,
    filter_opts: FilterOptions,
) -> Result<Vec<GitlabProject>> {
    let endpoint = if is_group {
        format!("groups/{}/projects", group_or_user)
    } else {
        format!("users/{}/projects", group_or_user)
    };

    let mut projects = get_projects_paginated(&endpoint, token, server_url).await?;

    // Apply filters using centralized FilterOptions methods
    let total_projects = projects.len();
    projects.retain(|p| filter_opts.should_include(&p.name, p.size.unwrap_or(0)));

    if !filter_opts.name_patterns.is_empty() || projects.len() != total_projects {
        println!(
            "Found {} projects, filtered to {}.",
            total_projects,
            projects.len()
        );
    }

    Ok(projects)
}

async fn get_projects_paginated(
    endpoint: &str,
    token: &Option<String>,
    server_url: Option<&str>,
) -> Result<Vec<GitlabProject>> {
    let api_url = if let Some(url) = server_url {
        format!("{}/api/v4", url.trim_end_matches('/'))
    } else {
        GITLAB_API.to_string()
    };

    let mut projects = Vec::new();
    let mut page = 1;

    loop {
        let url = format!(
            "{}/{}?page={}&per_page={}",
            api_url, endpoint, page, PER_PAGE
        );
        println!("Getting projects: '{}'", url);

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("superpull/1.0"));

        if let Some(token) = token {
            if !token.is_empty() {
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Private-Token {}", token))?,
                );
            }
        }

        let response = Client::new().get(&url).headers(headers).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch projects: {} - {}",
                response.status(),
                response.text().await?
            ));
        }

        let page_projects: Vec<GitlabProject> = response.json().await?;
        if page_projects.is_empty() {
            break;
        }

        projects.extend(page_projects);
        page += 1;
    }

    Ok(projects)
}
