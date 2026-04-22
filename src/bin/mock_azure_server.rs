/// Mock Azure DevOps Server for integration testing
///
/// This server provides basic Azure DevOps API v7.1 compatibility for:
/// - List projects in an organization: GET /{org}/_apis/projects?api-version=7.1
/// - List repositories in a project: GET /{org}/{project}/_apis/git/repositories?api-version=7.1
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
) -> axum::Json<serde_json::Value> {
    let _page: u32 = params
        .get("$skip")
        .and_then(|p| p.parse().ok())
        .unwrap_or(0);

    let per_page: u32 = params
        .get("$top")
        .and_then(|p| p.parse().ok())
        .unwrap_or(100);

    // Azure API v7.1 returns projects in "value" array
    let projects = generate_mock_projects(&org, per_page as usize);

    axum::Json(json!({
        "value": projects,
        "count": projects.len()
    }))
}

async fn list_repositories(
    axum::extract::Path(org): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::Json<serde_json::Value> {
    let _page: u32 = params
        .get("$skip")
        .and_then(|p| p.parse().ok())
        .unwrap_or(0);

    let per_page: u32 = params
        .get("$top")
        .and_then(|p| p.parse().ok())
        .unwrap_or(100);

    // Extract project from query parameter
    let project = params
        .get("project")
        .cloned()
        .unwrap_or_else(|| "default-project".to_string());

    let repos = generate_mock_repositories(&org, &project, per_page as usize);

    axum::Json(json!({
        "value": repos,
        "count": repos.len()
    }))
}

fn generate_mock_projects(_org: &str, _per_page: usize) -> Vec<serde_json::Value> {
    // Return a few test projects
    vec![
        json!({
            "id": "11111111-1111-1111-1111-111111111111",
            "name": "test-project-1",
            "description": "First test project"
        }),
        json!({
            "id": "22222222-2222-2222-2222-222222222222",
            "name": "test-project-2",
            "description": "Second test project"
        }),
        json!({
            "id": "33333333-3333-3333-3333-333333333333",
            "name": "test-project-3",
            "description": "Third test project"
        }),
    ]
}

fn generate_mock_repositories(
    org: &str,
    project: &str,
    _per_page: usize,
) -> Vec<serde_json::Value> {
    // Generate deterministic mock repositories for the given project
    let mut repos = Vec::new();

    for i in 1..=5 {
        let repo_name = format!("{}-repo-{:02}", project, i);

        repos.push(json!({
            "id": format!("repo-{:05}", i),
            "name": repo_name,
            "url": format!("https://dev.azure.com/{}/{}/{}/_apis/git/repositories/{}", org, project, project, i),
            "project": {
                "id": format!("proj-{:05}", i),
                "name": project
            },
            "remoteUrl": format!("https://dev.azure.com/{}/{}/_git/{}", org, project, repo_name),
            "sshUrl": format!("git@ssh.dev.azure.com:v3/{}/{}/{}", org, project, repo_name),
            "webUrl": format!("https://dev.azure.com/{}/{}/{}/_git/{}", org, project, project, repo_name),
            "size": 1024 + (i as i32 * 256),
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
