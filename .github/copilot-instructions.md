# Superpull - Rust Implementation

A fast, parallel git repository puller written in Rust. This is a Rust port of the original C# superpull project.

## Project Overview

Superpull provides six main commands:

1. **super-pull**: Recursively finds and pulls all git repositories in a directory
4. **az-clone**: Clones all repositories from an Azure DevOps organization with parallel execution
3. **bb-clone**: Clones all repositories from a Bitbucket Cloud workspace with parallel execution
6. **gea-clone**: Clones all repositories from a self-hosted Gitea instance with parallel execution
2. **gh-clone**: Clones all repositories from a GitHub organization or user with parallel execution
5. **gl-clone**: Clones all repositories from a GitLab group or user with parallel execution

## Development

### Building

```bash
cargo build --release
```

The binary will be in `target/release/superpull`.

### Running Tests

```bash
cargo test
```

### Running Integration Tests

Integration tests for on-prem Git servers are available in the `tests/` directory. They use Docker containers and require the `SUPERPULL_INTEGRATION_TESTS=1` environment variable:

```bash
# Run all integration tests
SUPERPULL_INTEGRATION_TESTS=1 cargo test -- --ignored

# Run specific integration tests
SUPERPULL_INTEGRATION_TESTS=1 cargo test azure_clone_with_superpull -- --ignored
SUPERPULL_INTEGRATION_TESTS=1 cargo test bitbucket_clone_with_superpull -- --ignored
SUPERPULL_INTEGRATION_TESTS=1 cargo test gitea_clone_with_superpull -- --ignored
SUPERPULL_INTEGRATION_TESTS=1 cargo test github_clone_with_superpull -- --ignored
SUPERPULL_INTEGRATION_TESTS=1 cargo test gitlab_clone_with_superpull -- --ignored
```

See `tests/README.md` for detailed information about integration test setup and resource requirements.

### Code Organization

- `src/main.rs` - Entry point and command dispatcher
- `src/cli.rs` - Command-line argument parsing using clap
- `src/git.rs` - Git repository pulling logic
- `src/azure_devops.rs` - Azure DevOps API interaction and cloning logic
- `src/bitbucket.rs` - Bitbucket Cloud API interaction and cloning logic
- `src/gitea.rs` - Gitea API interaction and cloning logic
- `src/github.rs` - GitHub API interaction and cloning logic
- `src/gitlab.rs` - GitLab API interaction and cloning logic
- `src/clone_task_manager.rs` - Parallel clone task execution and management
- `src/clone_options.rs` - Clone operation configuration options
- `src/filter_options.rs` - Repository filtering criteria
- `src/symlink_manager.rs` - Git submodule symbolic link creation
- `src/process_manager.rs` - Process execution with timeout handling

## Key Dependencies

- **tokio**: Async runtime
- **reqwest**: HTTP client for GitHub API
- **clap**: Command-line argument parsing
- **serde/serde_json**: JSON serialization
- **regex**: Pattern matching for filtering
- **colored**: Terminal output coloring
- **walkdir**: Directory traversal for finding repos

## Features

- Parallel git operations with configurable throttling
- **Default super-pull command**: Just run `superpull <folder>` without needing a subcommand
- Azure DevOps API v7.1 integration with project-level enumeration (cloud & self-hosted)
- Bitbucket Cloud API v2.0 integration for workspace repo discovery
- Bitbucket Server/Data Center API v2.0 integration for project repo discovery
- Gitea API v1 integration for self-hosted instances
- GitHub API v2.0 integration for organization/user repo discovery (cloud & enterprise)
- GitLab API v4 integration for group and user repo discovery (cloud & self-hosted)
- Repository filtering by name patterns, size, and other criteria
- Timeout handling for long-running operations
- Symbolic link creation for git submodules (all 5 git servers)
- Bearer token and basic auth support
- Token-based clone URL injection (GitHub, Bitbucket, Azure DevOps, GitLab, Gitea)

## Authentication

- Azure DevOps: `AZURE_DEVOPS_TOKEN` environment variable or CLI token (PAT)
- Bitbucket: `BITBUCKET_TOKEN` environment variable or CLI token (with `-b` flag for bearer auth)
- Gitea: `GITEA_TOKEN` environment variable or CLI token
- GitHub: `GITHUB_TOKEN` environment variable or CLI token
- GitLab: `GITLAB_TOKEN` or `CI_JOB_TOKEN` environment variable or CLI token

## CI/CD

The project includes automated CI/CD pipelines via GitHub Actions:

- **build.yml**: Runs on every push/PR to main and develop branches
  - Unit tests on ubuntu-latest and macos-latest
  - Code formatting check with `cargo fmt`
  - Linting with `cargo clippy`
  - Multi-platform binary builds and releases on version tags (Linux x86_64/ARM64, macOS x86_64/ARM64)
  - Binaries are compressed as tar.gz files with maximum gzip compression (-9)

- **integration.yml**: Runs integration tests against all 5 git servers
  - Mock Azure DevOps (Azure DevOps clone with superpull)
  - Bitbucket Cloud (Bitbucket clone with superpull)
  - Gitea (Gitea clone with superpull)
  - Mock GitHub Enterprise (GitHub clone with superpull)
  - GitLab (GitLab clone with superpull)
  - Docker-based test containers for self-hosted services

## Future Improvements

- Implement progress indicators
- Add dry-run mode
- Cache API responses
- Configurable retry logic
- Support for additional self-hosted Git services (Forgejo, Gogs, etc.)
- Performance/load testing
