use crate::clone_options::CloneOptions;
use crate::clone_task_manager;
use crate::debug_utils;
use crate::filter_options::FilterOptions;
use crate::symlink_manager;
use anyhow::{anyhow, Result};
use colored::Colorize;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::time::Instant;

const BITBUCKET_SAAS_API: &str = "https://api.bitbucket.org/2.0";

#[derive(Serialize, Deserialize, Debug)]
struct BitbucketRepository {
    name: String,
    full_name: String,
    links: RepositoryLinks,
    size: u32,
    #[serde(skip)]
    workspace: String,
}

// API v1.0 Response structures
#[derive(Serialize, Deserialize, Debug)]
struct PaginatedRepositoryV1 {
    size: u32,
    limit: u32,
    #[serde(rename = "isLastPage")]
    is_last_page: bool,
    start: u32,
    #[serde(rename = "nextPageStart")]
    next_page_start: Option<u32>,
    values: Vec<BitbucketRepositoryV1>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BitbucketRepositoryV1 {
    slug: String,
    name: String,
    links: RepositoryLinks,
    project: ProjectInfo,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProjectInfo {
    key: String,
    name: String,
}

// API v2.0 Response structures
#[derive(Serialize, Deserialize, Debug)]
struct PaginatedWorkspaceV2 {
    pagelen: u32,
    next: Option<String>,
    values: Vec<BitbucketWorkspaceV2>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BitbucketWorkspaceV2 {
    slug: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PaginatedRepositoryV2 {
    size: u32,
    page: u32,
    pagelen: u32,
    next: Option<String>,
    previous: Option<String>,
    values: Vec<BitbucketRepositoryV2>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BitbucketRepositoryV2 {
    name: String,
    full_name: String,
    links: RepositoryLinks,
    size: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RepositoryLinks {
    clone: Option<Vec<CloneLink>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CloneLink {
    name: String,
    href: String,
}

pub async fn super_clone(
    folder: &str,
    bearer_token: bool,
    token: Option<&str>,
    server_url: Option<&str>,
    use_api_v1: bool,
    options: CloneOptions,
) -> Result<()> {
    let start = Instant::now();
    fs::create_dir_all(folder)?;

    let filter_opts = FilterOptions::new()
        .with_name_patterns(options.name_patterns.clone())
        .with_exclude_patterns(options.exclude_patterns.clone())
        .with_max_size_kb(options.max_size_kb);

    let repos = get_repos(bearer_token, token, server_url, use_api_v1, filter_opts).await?;

    println!("Found {} repositories", repos.len());

    let clone_repos: Vec<(String, String)> = repos
        .iter()
        .filter_map(|repo| {
            extract_https_clone_url(repo).ok().map(|url| {
                let clean_url = url.strip_suffix(".git").unwrap_or(&url).to_string();
                // Prefix repo name with workspace to avoid conflicts across workspaces
                let prefixed_name = format!("{}-{}", repo.workspace.to_lowercase(), repo.name);
                (prefixed_name, clean_url)
            })
        })
        .collect();

    let token_owned = token.map(|t| t.to_string());

    let clone_repos_clone = clone_repos.clone();

    clone_task_manager::execute_clone_tasks(clone_repos, folder, options.clone(), |url| {
        let mut clone_url = url.to_string();
        if bearer_token {
            if let Some(ref token) = token_owned {
                // For Bitbucket, use x-token-auth prefix
                if let Some(index) = clone_url.find("://") {
                    clone_url = format!(
                        "{}x-token-auth:{}@{}",
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
        symlink_manager::create_symbolic_links(&clone_repos_clone, folder).await?;
    }

    println!("{}", format!("Done: {:?}", start.elapsed()).green());
    Ok(())
}

async fn get_repos(
    bearer_token: bool,
    token: Option<&str>,
    server_url: Option<&str>,
    use_api_v1: bool,
    filter_opts: FilterOptions,
) -> Result<Vec<BitbucketRepository>> {
    if use_api_v1 {
        get_repos_v1(bearer_token, token, server_url, filter_opts).await
    } else {
        get_repos_v2(bearer_token, token, server_url, filter_opts).await
    }
}

async fn get_repos_v1(
    bearer_token: bool,
    token: Option<&str>,
    server_url: Option<&str>,
    filter_opts: FilterOptions,
) -> Result<Vec<BitbucketRepository>> {
    let api_url = if let Some(url) = server_url {
        format!("{}/rest/api/1.0", url.trim_end_matches('/'))
    } else {
        return Err(anyhow!("API v1.0 requires server_url to be specified"));
    };

    let client = Client::new();
    let mut repos = Vec::new();
    let base_url = format!("{}/repos", api_url);
    let mut url = format!("{}?start=0&limit=100", base_url);
    let mut has_more = true;

    while has_more {
        let mut request = client.get(&url);

        if let Some(token) = token {
            if bearer_token {
                request = request.bearer_auth(token);
            } else {
                // For API tokens, use Basic auth
                request = request.basic_auth(token.split(':').next().unwrap_or(token), Some(token));
            }
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch repositories: {} - {}",
                response.status(),
                response.text().await?
            ));
        }

        let response_text = response.text().await?;
        print!(".");
        let _ = io::stdout().flush();
        let _ = debug_utils::save_api_response("bitbucket", "repos_v1", &response_text);
        let data: PaginatedRepositoryV1 = serde_json::from_str(&response_text)?;

        for repo in data.values {
            // v1.0 doesn't provide size, collect all repos for centralized filtering
            repos.push(BitbucketRepository {
                name: repo.slug,
                full_name: repo.name,
                links: repo.links,
                size: 0,
                workspace: repo.project.key.to_lowercase(),
            });
        }

        has_more = !data.is_last_page;
        if has_more {
            // Use same base_url pattern, only update start parameter
            url = format!(
                "{}?start={}&limit=100",
                base_url,
                data.next_page_start.unwrap_or(0)
            );
        }
    }
    println!();

    // Apply name-based filters only (v1.0 doesn't provide size)
    let total_repos = repos.len();
    repos.retain(|r| filter_opts.should_include_by_name(&r.name));

    println!("Found {} repos, filtered to {}.", total_repos, repos.len());

    Ok(repos)
}

async fn get_repos_v2(
    bearer_token: bool,
    token: Option<&str>,
    server_url: Option<&str>,
    filter_opts: FilterOptions,
) -> Result<Vec<BitbucketRepository>> {
    let api_url = if let Some(url) = server_url {
        format!("{}/rest/api/2.0", url.trim_end_matches('/'))
    } else {
        BITBUCKET_SAAS_API.to_string()
    };

    // First get all workspaces
    let workspaces = get_workspaces(bearer_token, token, &api_url).await?;
    println!("Found {} workspaces", workspaces.len());

    let mut all_repos = Vec::new();

    // Then for each workspace, get its repositories
    for workspace in workspaces {
        let repos = get_repos_for_workspace(bearer_token, token, &api_url, &workspace.slug).await?;
        all_repos.extend(repos);
    }
    println!();

    // Apply filters using centralized FilterOptions methods
    let total_repos = all_repos.len();
    all_repos.retain(|r| filter_opts.should_include_bytes(&r.name, r.size));

    if !filter_opts.name_patterns.is_empty() || all_repos.len() != total_repos {
        println!(
            "Found {} repos, filtered to {}.",
            total_repos,
            all_repos.len()
        );
    }

    Ok(all_repos)
}

async fn get_workspaces(
    bearer_token: bool,
    token: Option<&str>,
    api_url: &str,
) -> Result<Vec<BitbucketWorkspaceV2>> {
    let url = format!("{}/workspaces?pagelen=100", api_url);
    println!("Getting workspaces from: '{}'", url);

    let client = Client::new();
    let mut request = client.get(&url);

    if let Some(token) = token {
        if bearer_token {
            request = request.bearer_auth(token);
        } else {
            request = request.basic_auth(token.split(':').next().unwrap_or(token), Some(token));
        }
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch workspaces: {} - {}",
            response.status(),
            response.text().await?
        ));
    }

    let mut workspaces = Vec::new();
    let mut next_url = Some(url);

    while let Some(url) = next_url {
        let mut request = client.get(&url);

        if let Some(token) = token {
            if bearer_token {
                request = request.bearer_auth(token);
            } else {
                request = request.basic_auth(token.split(':').next().unwrap_or(token), Some(token));
            }
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch workspaces: {} - {}",
                response.status(),
                response.text().await?
            ));
        }

        let response_text = response.text().await?;
        print!(".");
        let _ = io::stdout().flush();
        let _ = debug_utils::save_api_response("bitbucket", "workspaces", &response_text);
        let data: PaginatedWorkspaceV2 = serde_json::from_str(&response_text)?;
        workspaces.extend(data.values);
        next_url = data.next;
    }

    Ok(workspaces)
}

async fn get_repos_for_workspace(
    bearer_token: bool,
    token: Option<&str>,
    api_url: &str,
    workspace_slug: &str,
) -> Result<Vec<BitbucketRepository>> {
    let url = format!("{}/repositories/{}?pagelen=100", api_url, workspace_slug);

    let client = Client::new();
    let mut repos = Vec::new();
    let mut next_url = Some(url);

    while let Some(url) = next_url {
        let mut request = client.get(&url);

        if let Some(token) = token {
            if bearer_token {
                request = request.bearer_auth(token);
            } else {
                request = request.basic_auth(token.split(':').next().unwrap_or(token), Some(token));
            }
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch repositories for workspace {}: {} - {}",
                workspace_slug,
                response.status(),
                response.text().await?
            ));
        }

        let response_text = response.text().await?;
        print!(".");
        let _ = io::stdout().flush();
        let _ = debug_utils::save_api_response(
            "bitbucket",
            &format!("repos_{}", workspace_slug),
            &response_text,
        );
        let data: PaginatedRepositoryV2 = serde_json::from_str(&response_text)?;

        for repo in data.values {
            repos.push(BitbucketRepository {
                name: repo.name,
                full_name: repo.full_name,
                links: repo.links,
                size: repo.size,
                workspace: workspace_slug.to_string(),
            });
        }

        next_url = data.next;
    }

    Ok(repos)
}

fn extract_https_clone_url(repo: &BitbucketRepository) -> Result<String> {
    repo.links
        .clone
        .as_ref()
        .and_then(|links| {
            // First try to find https, then fallback to http
            links
                .iter()
                .find(|l| l.name == "https")
                .or_else(|| links.iter().find(|l| l.name == "http"))
        })
        .map(|l| l.href.clone())
        .ok_or_else(|| {
            anyhow!(
                "No http/https clone url found for repository: {}",
                repo.name
            )
        })
}
