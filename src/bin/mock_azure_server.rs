/// Mock Azure DevOps Server for integration testing
///
/// This server provides basic Azure DevOps API v7.1 compatibility for:
/// - List projects in an organization: GET /{org}/_apis/projects?api-version=7.1
/// - List repositories in a project: GET /{org}/{project}/_apis/git/repositories?api-version=7.1
/// - Clone URLs with HTTPS
///
/// Usage:
/// - Run on port 8091 (configurable via PORT env var)
/// - Authenticate with: Authorization: Basic <base64(PAT:token)>
/// - Returns mock projects and repositories with realistic structure
use serde_json::json;
use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8091".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16");

    // Listen on all interfaces (0.0.0.0) for Docker container compatibility
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Build router
    let app = axum::Router::new()
        .route("/:org/_apis/projects", axum::routing::get(list_projects))
        .route(
            "/:org/_apis/git/repositories",
            axum::routing::get(list_repositories),
        )
        .route("/health", axum::routing::get(health_check))
        .fallback(not_found);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    println!("Mock Azure DevOps Server listening on {}", addr);

    axum::serve(listener, app).await.expect("Server error");
}

async fn health_check() -> &'static str {
    "OK"
}

async fn list_projects(
    axum::extract::Path(org): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> (axum::http::HeaderMap, axum::Json<serde_json::Value>) {
    let continuation_token: Option<String> = params.get("continuationToken").cloned();

    let top: u32 = params
        .get("$top")
        .and_then(|p| p.parse().ok())
        .unwrap_or(100);

    // Decode continuation token to get skip value, or default to 0
    let skip = if let Some(token) = &continuation_token {
        token.parse::<usize>().unwrap_or(0)
    } else {
        0
    };

    // Azure API v7.1 returns projects in "value" array
    let projects = generate_mock_projects(&org, skip, top as usize);

    let mut headers = axum::http::HeaderMap::new();
    // Add continuation token if there are more results
    let total_projects = 3; // Fixed 3 projects
    if skip + projects.len() < total_projects {
        let next_token = (skip + projects.len()).to_string();
        if let Ok(header_value) = axum::http::HeaderValue::from_str(&next_token) {
            headers.insert("X-MS-ContinuationToken", header_value);
        }
    }

    (
        headers,
        axum::Json(json!({
            "value": projects,
            "count": projects.len()
        })),
    )
}

async fn list_repositories(
    axum::extract::Path(org): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> (axum::http::HeaderMap, axum::Json<serde_json::Value>) {
    let continuation_token: Option<String> = params.get("continuationToken").cloned();

    let top: u32 = params
        .get("$top")
        .and_then(|p| p.parse().ok())
        .unwrap_or(100);

    // Decode continuation token to get skip value, or default to 0
    let skip = if let Some(token) = &continuation_token {
        token.parse::<usize>().unwrap_or(0)
    } else {
        0
    };

    // Extract project from query parameter
    let project = params
        .get("project")
        .cloned()
        .unwrap_or_else(|| "default-project".to_string());

    let repos = generate_mock_repositories(&org, &project, skip, top as usize);

    let mut headers = axum::http::HeaderMap::new();
    // Add continuation token if there are more results
    let total_repos = 101; // 101 repos for pagination testing
    if skip + repos.len() < total_repos {
        let next_token = (skip + repos.len()).to_string();
        if let Ok(header_value) = axum::http::HeaderValue::from_str(&next_token) {
            headers.insert("X-MS-ContinuationToken", header_value);
        }
    }

    (
        headers,
        axum::Json(json!({
            "value": repos,
            "count": repos.len()
        })),
    )
}

fn generate_mock_projects(_org: &str, _skip: usize, _top: usize) -> Vec<serde_json::Value> {
    // Return test projects with pagination support (101 total for pagination testing)
    let all_projects = vec![
        json!({
            "id": "11111111-1111-1111-1111-111111111111",
            "name": "test-project-001",
            "description": "Test project 001"
        }),
        json!({
            "id": "22222222-2222-2222-2222-222222222222",
            "name": "test-project-002",
            "description": "Test project 002"
        }),
        json!({
            "id": "33333333-3333-3333-3333-333333333333",
            "name": "test-project-003",
            "description": "Test project 003"
        }),
    ];

    // For projects, just return a fixed set (not paginated in this test)
    all_projects
}

fn generate_mock_repositories(
    org: &str,
    project: &str,
    skip: usize,
    top: usize,
) -> Vec<serde_json::Value> {
    // Generate 101 mock repositories per project (to test pagination)
    let total_repos = 101;
    let start = skip;
    let end = (skip + top).min(total_repos);

    let mut repos = Vec::new();

    for i in start..end {
        let repo_num = i + 1;
        let repo_name = format!("{}-repo-{:03}", project, repo_num);

        repos.push(json!({
            "id": format!("repo-{:05}", repo_num),
            "name": repo_name,
            "url": format!("https://dev.azure.com/{}/{}/{}/_apis/git/repositories/{}", org, project, project, repo_num),
            "project": {
                "id": format!("proj-{:05}", repo_num),
                "name": project
            },
            "remoteUrl": format!("https://dev.azure.com/{}/{}/_git/{}", org, project, repo_name),
            "sshUrl": format!("git@ssh.dev.azure.com:v3/{}/{}/{}", org, project, repo_name),
            "webUrl": format!("https://dev.azure.com/{}/{}/{}/_git/{}", org, project, project, repo_name),
            "size": 1024 + (repo_num as i32 * 256),
            "isDisabled": false,
            "isPrivate": true,
            "defaultBranch": "refs/heads/main",
        }));
    }

    repos
}

async fn not_found() -> (axum::http::StatusCode, &'static str) {
    (axum::http::StatusCode::NOT_FOUND, "Not Found")
}
