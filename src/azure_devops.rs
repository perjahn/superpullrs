use anyhow::{anyhow, Result};
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

const AZURE_SAAS_API: &str = "https://dev.azure.com";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AzureProject {
    id: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AzureRepository {
    id: String,
    name: String,
    #[serde(rename = "remoteUrl")]
    remote_url: String,
    size: i32,
    #[serde(skip)]
    project: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AzureProjectsResponse {
    value: Vec<AzureProject>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AzureRepositoriesResponse {
    value: Vec<AzureRepository>,
}

pub async fn super_clone(
    organization: &str,
    folder: &str,
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

    let azure_token = token
        .map(|t| t.to_string())
        .or_else(|| env::var("AZURE_DEVOPS_TOKEN").ok())
        .ok_or_else(|| {
            anyhow!(
                "Azure DevOps token not provided. Set via --token or AZURE_DEVOPS_TOKEN env var"
            )
        })?;

    let filter_opts = FilterOptions::new()
        .with_name_patterns(options.name_patterns.clone())
        .with_exclude_patterns(options.exclude_patterns.clone())
        .with_max_size_kb(options.max_size_kb);

    let repos = get_repos(organization, &azure_token, server_url, filter_opts).await?;

    if repos.is_empty() {
        println!("No git repos found.");
        return Ok(());
    }

    let total_repos = repos.len();
    println!("Got {} repos.", total_repos);

    let mut repo_urls: Vec<(String, String)> = repos
        .into_iter()
        .map(|r| {
            let url = r
                .remote_url
                .strip_suffix(".git")
                .unwrap_or(&r.remote_url)
                .to_string();
            // Prefix repo name with project name to avoid conflicts across projects
            let prefixed_name = format!("{}-{}", r.project.to_lowercase(), r.name);
            (prefixed_name, url)
        })
        .collect();

    repo_urls.sort_by(|a, b| a.1.cmp(&b.1));

    let clone_repos = repo_urls.clone();

    clone_task_manager::execute_clone_tasks(repo_urls, target_folder, options.clone(), |url| {
        let mut clone_url = url.to_string();
        if !azure_token.is_empty() {
            if let Some(index) = clone_url.find("://") {
                // Azure DevOps uses PAT:token format
                let encoded = base64::engine::general_purpose::STANDARD
                    .encode(format!("PAT:{}", azure_token));
                clone_url = format!(
                    "{}{}@{}",
                    &clone_url[..index + 3],
                    encoded,
                    &clone_url[index + 3..]
                );
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
    organization: &str,
    token: &str,
    server_url: Option<&str>,
    filter_opts: FilterOptions,
) -> Result<Vec<AzureRepository>> {
    // First get all projects
    let projects = get_projects(organization, token, server_url).await?;
    println!("Found {} projects", projects.len());

    let mut all_repos = Vec::new();

    // Then for each project, get its repositories
    for project in projects {
        let repos = get_repos_for_project(organization, &project.name, token, server_url).await?;
        all_repos.extend(repos);
    }
    println!();

    // Apply filters using centralized FilterOptions methods
    let total_repos = all_repos.len();
    all_repos.retain(|r| filter_opts.should_include(&r.name, r.size));

    println!(
        "Found {} repos, filtered to {}.",
        total_repos,
        all_repos.len()
    );

    Ok(all_repos)
}

async fn get_projects(
    organization: &str,
    token: &str,
    server_url: Option<&str>,
) -> Result<Vec<AzureProject>> {
    let base_url = if let Some(url) = server_url {
        url.trim_end_matches('/')
    } else {
        AZURE_SAAS_API
    };

    let url = format!(
        "{}/{}/_apis/projects?api-version=7.1",
        base_url, organization
    );
    println!("Getting projects from: '{}'", url);

    let client = Client::new();
    let auth = base64::engine::general_purpose::STANDARD.encode(format!("PAT:{}", token));

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("superpull/1.0"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Basic {}", auth))?,
    );

    let response = client.get(&url).headers(headers).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch projects: {} - {}",
            response.status(),
            response.text().await?
        ));
    }

    let response_text = response.text().await?;
    print!(".");
    let _ = io::stdout().flush();
    let _ = debug_utils::save_api_response("azure_devops", "projects", &response_text);
    let projects_response: AzureProjectsResponse = serde_json::from_str(&response_text)?;

    Ok(projects_response.value)
}

async fn get_repos_for_project(
    organization: &str,
    project: &str,
    token: &str,
    server_url: Option<&str>,
) -> Result<Vec<AzureRepository>> {
    let base_url = if let Some(url) = server_url {
        url.trim_end_matches('/')
    } else {
        AZURE_SAAS_API
    };

    let url = format!(
        "{}/{}/_apis/git/repositories?project={}&api-version=7.1",
        base_url, organization, project
    );
    println!("Getting repos for project '{}': '{}'", project, url);

    let client = Client::new();
    let auth = base64::engine::general_purpose::STANDARD.encode(format!("PAT:{}", token));

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("superpull/1.0"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Basic {}", auth))?,
    );

    let response = client.get(&url).headers(headers).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch repositories: {} - {}",
            response.status(),
            response.text().await?
        ));
    }

    let response_text = response.text().await?;
    let _ = debug_utils::save_api_response(
        "azure_devops",
        &format!("repos_{}", project),
        &response_text,
    );
    let repos_response: AzureRepositoriesResponse = serde_json::from_str(&response_text)?;

    // Add project name to each repository
    let repos = repos_response
        .value
        .into_iter()
        .map(|mut r| {
            r.project = project.to_string();
            r
        })
        .collect();

    Ok(repos)
}
