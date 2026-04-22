# Integration Tests for superpull

This directory contains integration tests for superpull that test against real Git server instances running in Docker containers.

## Prerequisites

- Docker installed and running
- At least 4GB of free RAM (Bitbucket Server and GitLab are resource-intensive)
- `curl` command available in PATH

## Test cloning from all Servers

```bash
# Run all tests
SUPERPULL_INTEGRATION_TESTS=1 cargo test --release -- --ignored
```

## Supported Servers

Integration tests are provided for:

1. **Azure DevOps** - Lightweight mock for testing Azure DevOps API v7.1 compatibility (~1 second startup)
2. **Bitbucket Server** - Enterprise, takes 3-5 minutes to start and requires significant resources
3. **Forgejo** - Lightweight, quick to start (~30 seconds)
4. **Gitea** - Lightweight, quick to start (~30 seconds)
5. **GitHub Enterprise** - Lightweight mock for testing GitHub API v3 compatibility (~1 second startup)
6. **GitLab Community Edition** - Feature-rich, takes 2-3 minutes to start

### Mock Azure DevOps Server Testing

superpull includes a **lightweight mock Azure DevOps server** for integration testing. This mock server:

- Provides Azure DevOps API v7.1 compatibility for core endpoints
- Returns realistic project and repository data structures
- Supports project and repository enumeration
- Starts in ~1 second and requires minimal resources (256MB RAM)
- Is built automatically when running tests

**Running the mock Azure DevOps test:**

```bash
# Run only the mock Azure DevOps test
SUPERPULL_INTEGRATION_TESTS=1 cargo test azure_devops_clone_with_superpull -- --ignored

# Or start the mock server via docker-compose
docker-compose -f docker-compose.test.yml up mock-azure

# The mock server will be available at http://127.0.0.1:8091
```

**Testing Against Real Azure DevOps Server**

If you have access to an Azure DevOps Server instance, you can manually test superpull:

1. **Set up Azure DevOps Server** on Windows hardware or VM
2. **Generate a Personal Access Token (PAT)** with appropriate scopes
3. **Run superpull az-clone** with the `-s` server URL flag:

```bash
# Azure DevOps Server on-prem
./target/release/superpull az-clone \
  -a YOUR_PAT \
  -s https://your-devops-server.internal:8080 \
  YOUR_ORGANIZATION \
  ./cloned-repos

# Or with cloud Azure DevOps (no -s flag needed)
./target/release/superpull az-clone \
  -a YOUR_PAT \
  YOUR_ORGANIZATION \
  ./cloned-repos
```

### Manual Testing for Azure DevOps (Real Instance)

Note: Azure DevOps Server (on-prem) cannot be easily containerized because:
- Requires Windows Server operating system
- Requires SQL Server database backend
- Requires paid licensing

The code paths for cloud and on-prem are identical - they both support the `-s` parameter for server URL configuration.

### Forgejo Server Testing

Forgejo is a lightweight Git service forked from Gitea. It uses the same API structure as Gitea (v1 API) and is easy to deploy in a Docker container.

**Running the Forgejo test:**

```bash
# Run only the Forgejo test
SUPERPULL_INTEGRATION_TESTS=1 cargo test forgejo_clone_with_superpull -- --ignored

# Or start the Forgejo server via docker-compose
docker-compose -f docker-compose.test.yml up forgejo

# The Forgejo server will be available at http://127.0.0.1:3002
```

**Testing Against a Self-Hosted Forgejo Instance**

If you have access to a self-hosted Forgejo instance, you can manually test superpull:

1. **Generate an API token** on your Forgejo instance
2. **Run superpull foj-clone** with the instance URL:

```bash
# Forgejo self-hosted
export FORGEJO_TOKEN=your-token
./target/release/superpull foj-clone \
  https://forgejo.example.com \
  YOUR_ORGANIZATION \
  ./cloned-repos
```

### Mock GitHub Enterprise Server Testing

Since the official GitHub Enterprise Server Docker image is not publicly available, superpull includes a **lightweight mock GitHub server** for integration testing. This mock server:

- Provides GitHub API v3 compatibility for core endpoints
- Returns realistic repository data structures
- Supports organization and team repository listing
- Starts in ~1 second and requires minimal resources (256MB RAM)
- Is built automatically when running tests

**Running the mock GitHub test:**

```bash
# Run only the mock GitHub test
SUPERPULL_INTEGRATION_TESTS=1 cargo test github_clone_with_superpull -- --ignored

# Or start the mock server via docker-compose
docker-compose -f docker-compose.test.yml up mock-github

# The mock server will be available at http://127.0.0.1:8443
```

**Testing Against Real GitHub Enterprise Server**

If you have access to an actual GitHub Enterprise Server instance, you can manually test superpull:

1. **Set up GitHub Enterprise Server** on dedicated hardware or VM (see [official docs](https://docs.github.com/en/enterprise-server@latest/admin/installation-configuration-and-management/installing-github-enterprise-server-on-a-virtual-machine))
2. **Generate a Personal Access Token (PAT)** with appropriate scopes
3. **Run superpull gh-clone** with the `-s` server URL flag:

```bash
# GitHub Enterprise Server on-prem
./target/release/superpull gh-clone \
  -a YOUR_PAT \
  -s https://your-ghes-server.internal:8443 \
  YOUR_ORGANIZATION \
  ./cloned-repos
```

The code paths for cloud and on-prem GitHub are identical - they both support the `-s` parameter.

### Run All Integration Tests

```bash
SUPERPULL_INTEGRATION_TESTS=1 cargo test --test integration_tests -- --ignored
```

### Gitea Only

```bash
SUPERPULL_INTEGRATION_TESTS=1 cargo test --test integration_tests -- --ignored gitea_clone_with_superpull
```

### Using Docker Compose

To start all services at once:

```bash
docker-compose -f docker-compose.test.yml up -d
```

Services will be available at:
- Azure DevOps: http://127.0.0.1:8091
- Bitbucket: http://127.0.0.1:7991
- Forgejo: http://127.0.0.1:3002
- Gitea: http://127.0.0.1:3001
- GitHub: http://127.0.0.1:8443
- GitLab: http://127.0.0.1:8080 (root password: test12345)

To stop all services:

```bash
docker-compose -f docker-compose.test.yml down
```

## Test Structure

Each test module:

1. **Starts a Docker container** for the Git server
2. **Waits for readiness** using health checks
3. **Sets up test infrastructure** (e.g., creating test repositories)
4. **Runs superpull commands** against the server
5. **Verifies results** (e.g., checking cloned repositories)
6. **Cleans up** by removing the container on test completion

## Extending the Tests

To add more comprehensive testing (e.g., actually cloning repositories), you'll need to:

1. Add helper functions to create test repositories on each server
2. Generate API tokens for authentication
3. Run superpull commands using the binary
4. Verify the output directory contains expected repositories

Example structure:

```rust
// Create a test organization/group
let api_token = create_test_organization(&container)?;

// Create test repositories
create_test_repository(&container, &api_token, "test-repo-1")?;
create_test_repository(&container, &api_token, "test-repo-2")?;

// Run superpull
let output = Command::new("./target/release/superpull")
    .args(&[
        "gea-clone",
        "-a", &api_token,
        "http://127.0.0.1:3001",
        "test-org",
        "./cloned-repos"
    ])
    .output()?;

// Verify
assert!(Path::new("./cloned-repos/test-repo-1").exists());
assert!(Path::new("./cloned-repos/test-repo-2").exists());
```

## Notes

- Tests are marked with `#[ignore]` because they require Docker and take significant time
- The `SUPERPULL_INTEGRATION_TESTS` environment variable must be set to run integration tests
- Container cleanup happens automatically via the `Drop` trait on `DockerContainer`
- Logs from failed containers are printed for debugging

## Troubleshooting

**"Docker is not available"**
- Ensure Docker daemon is running: `docker ps`

**"Container failed to become ready"**
- Check container logs: `docker logs superpull-{service}-test`
- Try increasing the `max_retries` parameter in the test
- Ensure your system has enough RAM and CPU

**"Port already in use"**
- Either stop the existing container or modify the port mapping in docker-compose.test.yml
- List running containers: `docker ps -a | grep superpull`
- Remove specific container: `docker rm -f superpull-{service}-test`

## Resource Requirements

| Server | Image Size | Startup Time | RAM | CPU |
|--------|-----------|--------------|-----|-----|
| Mock Azure DevOps | ~100MB | ~1s | 256MB | 0.5 |
| Bitbucket Server | ~1.4GB | 3-5min | 2GB | 1-2 |
| Forgejo | ~300MB | ~30s | 512MB | 1 |
| Gitea | ~200MB | ~30s | 512MB | 1 |
| Mock GitHub | ~100MB | ~1s | 256MB | 0.5 |
| GitLab CE | ~4GB | 2-3min | 2GB | 1-2 |

**Note**: Mock Azure DevOps and Mock GitHub are lightweight services for testing API compatibility. They build on-demand during test runs. For comprehensive testing, use Bitbucket, Forgejo, Gitea, or GitLab. All images should be pulled beforehand to avoid download delays during test runs.
