/// Mock GitHub Enterprise Server for integration testing
///
/// This server provides basic GitHub API v3 compatibility for:
/// - List repositories in an organization: GET /api/v3/{org}/repos
/// - List team repositories: GET /api/v3/{org}/teams/{team}/repos
/// - Clone URLs with SSH or HTTPS
///
/// Usage:
/// - Run on port 8443 (configurable via PORT env var)
/// - Authenticate with: Authorization: Bearer <token> or Basic auth
/// - Returns mock repositories with realistic structure
use serde_json::json;
use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8443".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16");

    // Listen on all interfaces (0.0.0.0) for Docker container compatibility
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Build router
    let app = axum::Router::new()
        .route("/api/v3/:org/repos", axum::routing::get(list_org_repos))
        .route(
            "/api/v3/:org/teams/:team/repos",
            axum::routing::get(list_team_repos),
        )
        .route("/health", axum::routing::get(health_check))
        .fallback(not_found);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    println!("Mock GitHub Enterprise Server listening on {}", addr);

    axum::serve(listener, app).await.expect("Server error");
}

async fn health_check() -> &'static str {
    "OK"
}

async fn list_org_repos(
    axum::extract::Path(org): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::Json<serde_json::Value> {
    let page: u32 = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);

    let per_page: u32 = params
        .get("per_page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(30);

    let repos = generate_mock_repos(&org, page, per_page);
    axum::Json(repos)
}

async fn list_team_repos(
    axum::extract::Path((org, team)): axum::extract::Path<(String, String)>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::Json<serde_json::Value> {
    let page: u32 = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);

    let per_page: u32 = params
        .get("per_page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(30);

    // For teams, return a subset of org repos
    let mut repos = generate_mock_repos(&org, page, per_page);

    // Filter to only repos that would belong to the team
    if let serde_json::Value::Array(ref mut arr) = repos {
        arr.retain(|repo| {
            if let Some(name) = repo.get("name").and_then(|n| n.as_str()) {
                // Only include repos with names matching the team
                name.starts_with(&format!("{}-", team))
            } else {
                false
            }
        });
    }

    axum::Json(repos)
}

fn generate_mock_repos(org: &str, page: u32, per_page: u32) -> serde_json::Value {
    // Generate deterministic mock repositories
    let total_repos = 25;
    let start = ((page - 1) * per_page) as usize;
    let end = (start + per_page as usize).min(total_repos);

    let mut repos = Vec::new();

    for i in start..end {
        let repo_num = i + 1;
        let repo_name = format!("test-repo-{:03}", repo_num);

        repos.push(json!({
            "id": 1000 + repo_num,
            "name": repo_name,
            "full_name": format!("{}/{}", org, format!("test-repo-{:03}", repo_num)),
            "private": false,
            "owner": {
                "login": org,
                "id": 1,
                "type": "Organization"
            },
            "html_url": format!("https://github.example.com/{}/test-repo-{:03}", org, repo_num),
            "description": format!("Mock test repository #{}", repo_num),
            "fork": false,
            "created_at": "2023-01-01T00:00:00Z",
            "updated_at": "2023-01-01T00:00:00Z",
            "pushed_at": "2023-01-01T00:00:00Z",
            "homepage": null,
            "size": 1024 + (repo_num as i32 * 100),
            "stargazers_count": 0,
            "watchers_count": 0,
            "language": "Rust",
            "has_issues": true,
            "has_projects": true,
            "has_downloads": true,
            "has_wiki": true,
            "has_pages": false,
            "forks_count": 0,
            "archived": false,
            "disabled": false,
            "open_issues_count": 0,
            "license": null,
            "forks": 0,
            "open_issues": 0,
            "watchers": 0,
            "default_branch": "main",
            "clone_url": format!("https://github.example.com/{}/test-repo-{:03}.git", org, repo_num),
            "ssh_url": format!("git@github.example.com:{}/test-repo-{:03}.git", org, repo_num),
            "git_url": format!("git://github.example.com/{}/test-repo-{:03}.git", org, repo_num),
            "svn_url": format!("https://github.example.com/{}/test-repo-{:03}", org, repo_num),
        }));
    }

    serde_json::Value::Array(repos)
}

async fn not_found() -> (axum::http::StatusCode, &'static str) {
    (axum::http::StatusCode::NOT_FOUND, "Not Found")
}
