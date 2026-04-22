use crate::clone_options::CloneOptions;
use crate::clone_task_manager;
use crate::filter_options::FilterOptions;
use crate::symlink_manager;
use anyhow::{anyhow, Result};
use colored::Colorize;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Instant;

const BITBUCKET_API: &str = "https://api.bitbucket.org/2.0";

// API v2.0 Response structures
#[derive(Serialize, Deserialize, Debug)]
struct PaginatedResponseV2 {
    size: u32,
    page: u32,
    pagelen: u32,
    next: Option<String>,
    previous: Option<String>,
    values: Vec<BitbucketRepositoryV2>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BitbucketWorkspaceV2 {
    slug: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PaginatedWorkspacesResponse {
    pagelen: u32,
    next: Option<String>,
    values: Vec<BitbucketWorkspaceV2>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BitbucketRepositoryV2 {
    name: String,
    full_name: String,
    links: RepositoryLinks,
    size: u32,
}

// API v1.0 Response structures
#[derive(Serialize, Deserialize, Debug)]
struct PaginatedResponseV1 {
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
}

#[derive(Serialize, Deserialize, Debug)]
struct BitbucketRepository {
    name: String,
    full_name: String,
    links: RepositoryLinks,
    size: u32,
}

impl BitbucketRepositoryV1 {
    fn to_v2(&self) -> BitbucketRepository {
        BitbucketRepository {
            name: self.slug.clone(),
            full_name: self.name.clone(),
            links: RepositoryLinks {
                clone: self.links.clone.clone(),
            },
            size: 0, // v1.0 doesn't provide size, default to 0
        }
    }
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

    let repos =
        get_bitbucket_repos(bearer_token, token, server_url, use_api_v1, filter_opts).await?;

    println!("Found {} repositories", repos.len());

    let clone_repos: Vec<(String, String)> = repos
        .iter()
        .filter_map(|repo| {
            extract_https_clone_url(repo)
                .ok()
                .map(|url| (repo.name.clone(), url))
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

async fn get_bitbucket_repos(
    bearer_token: bool,
    token: Option<&str>,
    server_url: Option<&str>,
    use_api_v1: bool,
    filter_opts: FilterOptions,
) -> Result<Vec<BitbucketRepository>> {
    if use_api_v1 {
        get_bitbucket_repos_v1(bearer_token, token, server_url, filter_opts).await
    } else {
        get_bitbucket_repos_v2(bearer_token, token, server_url, filter_opts).await
    }
}

async fn get_bitbucket_repos_v1(
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

        let data: PaginatedResponseV1 = response.json().await?;

        for repo in data.values {
            // v1.0 doesn't provide size, so we default to 0 and include all
            if filter_opts.should_include_bytes(&repo.slug, 0) {
                repos.push(repo.to_v2());
            }
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

    Ok(repos)
}

async fn get_bitbucket_repos_v2(
    bearer_token: bool,
    token: Option<&str>,
    server_url: Option<&str>,
    filter_opts: FilterOptions,
) -> Result<Vec<BitbucketRepository>> {
    let api_url = if let Some(url) = server_url {
        format!("{}/rest/api/2.0", url.trim_end_matches('/'))
    } else {
        BITBUCKET_API.to_string()
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

        let data: PaginatedWorkspacesResponse = response.json().await?;
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

        let data: PaginatedResponseV2 = response.json().await?;

        for repo in data.values {
            repos.push(BitbucketRepository {
                name: repo.name,
                full_name: repo.full_name,
                links: repo.links,
                size: repo.size,
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
        .and_then(|links| links.iter().find(|l| l.name == "https"))
        .map(|l| l.href.clone())
        .ok_or_else(|| anyhow!("No HTTPS clone link found for repository: {}", repo.name))
}
