# superpull - Rust Edition

[![Build and Release](https://github.com/perjahn/superpullrs/actions/workflows/build.yml/badge.svg)](https://github.com/perjahn/superpullrs/actions/workflows/build.yml)
[![Integration Tests](https://github.com/perjahn/superpullrs/actions/workflows/integration.yml/badge.svg)](https://github.com/perjahn/superpullrs/actions/workflows/integration.yml)
[![Dependabot Updates](https://github.com/perjahn/superpullrs/actions/workflows/dependabot/dependabot-updates/badge.svg)](https://github.com/perjahn/superpullrs/actions/workflows/dependabot/dependabot-updates)

A fast, parallel git repository puller written in Rust.

## Features

- **pull**: Recursively find and pull all git repositories in a directory
- **az**: Clone all repositories from an Azure DevOps organization
- **bb**: Clone all repositories from a Bitbucket Cloud workspace or Server/Data Center project
- **foj**: Clone all repositories from a self-hosted Forgejo instance
- **gea**: Clone all repositories from a self-hosted Gitea instance
- **gh**: Clone all repositories from a GitHub organization or user
- **gl**: Clone all repositories from a GitLab group or user
- **Filtering**: Support for team filtering (GitHub), regex-based name patterns, size limits, and exclusions
- **Parallel Processing**: Configurable throttling for concurrent git operations
- **Timeout Handling**: Built-in timeout management for long-running operations
- **Symbolic Links**: Create symbolic links for git submodules
- **Authentication**: Support for Azure DevOps, Bitbucket, Forgejo, Gitea, GitHub, and GitLab API authentication

## Usage

All commands support parallel execution with global options:
- `-p`: Throttle parallel processes (default: 10)
- `-t`: Timeout in seconds (default: 60)

### Flag Compatibility Matrix

| Flag | Description | SuperPull | AzClone | BbClone | FojClone | GeaClone | GhClone | GlClone |
|------|-------------|:---------:|:-------:|:-------:|:--------:|:--------:|:-------:|:-------:|
| `-1` | API v1.0 (Bitbucket) | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| `-a` | API Token | ❌ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ |
| `-b` | Bearer token auth | ❌ | ❌ | ✅ | ❌ | ❌ | ✅ | ❌ |
| `-e` | Team filter (GitHub) | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| `-f` | Exclude forked repos | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `-g` | Group flag (GitLab) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| `-l` | Create symlinks | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `-m` | Max size in KB | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `-n` | Name pattern filter | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `-o` | Exclude pattern filter | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `-p` | Throttle parallel (global) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `-r` | Recurse into subfolders | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `-s` | Server/API base URL | ❌ | ✅ | ✅ | (arg)¹ | (arg)¹ | ✅ | ✅ |
| `-t` | Timeout (global) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

The `-b` (bearer token) flag is only for clone commands that need API authentication (AzClone, BbClone, GhClone) and is ignored by the default pull mode. The `-b` is for API authentication and varies by service: Bitbucket uses for clone URL injection, GitHub uses for HTTP header bearer auth

¹ Forgejo and Gitea use positional `<server_url>` argument instead of `-s` flag (e.g. `foj https://forgejo.example.com org-name` or `gea https://gitea.example.com org-name`)

### Pull all repositories in a folder

```bash
superpull [OPTIONS] <folder>
superpull [OPTIONS] pull <folder>
```

Options:
- `-r`: Recurse into subfolders

### Clone all repositories from Azure DevOps

```bash
superpull az [-a token] [-f] [-l] [-m size] [-n regex] [-o regex] [-s server_url] <organization> [folder]
```

Options:
- `-a`: Azure DevOps Personal Access Token (PAT)
- `-f`: Exclude forked repositories
- `-l`: Create symbolic links for git submodules
- `-m`: Max size in KB
- `-n`: Filter repos by name regex (can be specified multiple times)
- `-o`: Exclude repos by name regex (can be specified multiple times)
- `-s`: Self-hosted Azure DevOps base URL (e.g., https://azuredevops.example.com)

### Clone all repositories from Bitbucket

```bash
superpull bb [-1] [-a token] [-f] [-l] [-m size] [-n regex] [-o regex] [-s server_url] <workspace> [folder]
```

For Bitbucket Cloud, use workspace name. For Server/Data Center, use project key and `-s` for server URL.

Options:
- `-1`: Use Bitbucket Server API v1.0 instead of v2.0
- `-a`: Bitbucket API token (Cloud: personal token or app password, Server: personal token)
- `-b`: Use bearer token authentication (required for token-based cloning)
- `-f`: Exclude forked repositories
- `-l`: Create symbolic links for git submodules
- `-m`: Max size in KB
- `-n`: Filter repos by name regex (can be specified multiple times)
- `-o`: Exclude repos by name regex (can be specified multiple times)
- `-s`: Server/Data Center base URL (e.g., https://bitbucket.example.com)

### Clone all repositories from Forgejo

```bash
superpull foj [-a token] [-f] [-l] [-m size] [-n regex] [-o regex] <server_url> <organization> [folder]
```

Options:
- `-a`: Forgejo API token
- `-f`: Exclude forked repositories
- `-l`: Create symbolic links for git submodules
- `-m`: Max size in KB
- `-n`: Filter repos by name regex (can be specified multiple times)
- `-o`: Exclude repos by name regex (can be specified multiple times)

### Clone all repositories from Gitea

```bash
superpull gea [-a token] [-f] [-l] [-m size] [-n regex] [-o regex] <server_url> <organization> [folder]
```

Options:
- `-a`: Gitea API token
- `-f`: Exclude forked repositories
- `-l`: Create symbolic links for git submodules
- `-m`: Max size in KB
- `-n`: Filter repos by name regex (can be specified multiple times)
- `-o`: Exclude repos by name regex (can be specified multiple times)

### Clone all repositories from GitHub

```bash
superpull gh [-e team] [-f] [-l] [-m size] [-n regex] [-o regex] [-s server_url] <entity> [folder]
```

The `<entity>` parameter specifies where to clone repositories from:
- For organizations: `orgs/<orgname>` (e.g., `orgs/microsoft`)
- For users: `users/<username>` (e.g., `users/octocat`)

Options:
- `-b`: Use bearer token authentication instead of basic auth
- `-e`: Filter repos for specific team (can be specified multiple times)
- `-f`: Exclude forked repositories
- `-l`: Create symbolic links for git submodules
- `-m`: Max size in KB of the .git folder
- `-n`: Filter repos by name regex (can be specified multiple times)
- `-o`: Exclude repos by name regex (can be specified multiple times)
- `-s`: GitHub API base URL for GitHub Enterprise (e.g., https://github.example.com/api/v3)

### Clone all repositories from GitLab

```bash
superpull gl [-a token] [-f] [-g] [-l] [-m size] [-n regex] [-o regex] [-s server_url] <entity> [folder]
```

The `<entity>` parameter specifies what to clone repositories from:
- For groups: group path (e.g., `mygroup` or `parent/subgroup`) — use `-g` flag
- For users: username (e.g., `octocat`)

Options:
- `-a`: GitLab personal access token or CI job token
- `-f`: Exclude forked repositories
- `-g`: Treat entity as a group (otherwise treats as user)
- `-l`: Create symbolic links for git submodules
- `-m`: Max size in KB
- `-n`: Filter repos by name regex (can be specified multiple times)
- `-o`: Exclude repos by name regex (can be specified multiple times)
- `-s`: Self-hosted GitLab base URL (e.g., https://gitlab.example.com)

### Environment Variables

- `AZURE_DEVOPS_TOKEN`: Azure DevOps Personal Access Token (required for cloning private repos)
- `BITBUCKET_TOKEN`: Personal token or app password for Bitbucket Cloud API (required for cloning private repos)
- `FORGEJO_TOKEN`: Forgejo API token (required for cloning private repos)
- `GITEA_TOKEN`: Gitea API token (required for cloning private repos)
- `GITHUB_TOKEN`: Personal access token for GitHub API (required for cloning private repos)
- `GITLAB_TOKEN`: GitLab personal access token (required for cloning private repos)
- `CI_JOB_TOKEN`: GitLab CI job token (alternative to GITLAB_TOKEN)

## Examples

```bash
# Pull all repos in current directory (default to pull)
superpull .

# Pull all repos recursively
superpull -r .

# Pull with explicit pull subcommand
superpull pull -r .

# Clone all repos from an Azure DevOps organization (cloud)
export AZURE_DEVOPS_TOKEN=<your-pat>
superpull az myorg ./myorg-repos

# Clone all repos from self-hosted Azure DevOps
export AZURE_DEVOPS_TOKEN=<your-pat>
superpull az -s https://azuredevops.example.com myorg ./myorg-repos

# Clone repos from Azure DevOps with filtering
superpull az -p 5 -n "^backend-" myorg ./backend-repos

# Clone all repos from a Bitbucket Cloud workspace
export BITBUCKET_TOKEN=<your-token>
superpull -b bb -a $BITBUCKET_TOKEN myworkspace ./myworkspace-repos

# Clone all repos from on-prem Bitbucket Server/Data Center
export BITBUCKET_TOKEN=<your-token>
superpull -b bb -a $BITBUCKET_TOKEN -s https://bitbucket.example.com PROJECT ./project-repos

# Clone all repos from older Bitbucket Server using API v1.0
export BITBUCKET_TOKEN=<your-token>
superpull -b bb -a $BITBUCKET_TOKEN -s https://bitbucket.example.com -1 PROJECT ./project-repos

# Clone repos from Bitbucket Cloud with filtering
superpull -b bb -a $BITBUCKET_TOKEN -n "^sdk-" -p 5 myworkspace ./sdk-repos

# Clone repos from Bitbucket and exclude private repos
superpull -b bb -a $BITBUCKET_TOKEN -o "^internal-" myworkspace ./public-repos

# Clone all repos from a self-hosted Forgejo instance
export FORGEJO_TOKEN=<your-token>
superpull foj https://forgejo.example.com myorg ./myorg-repos

# Clone repos from Forgejo with filtering
superpull foj -p 5 -n "^api-" https://forgejo.example.com myorg ./api-repos

# Clone repos from Forgejo and create symlinks for submodules
superpull foj -l -n "^v2-" https://forgejo.example.com myorg ./v2-repos

# Clone all repos from a self-hosted Gitea instance
export GITEA_TOKEN=<your-token>
superpull gea https://gitea.example.com myorg ./myorg-repos

# Clone repos from Gitea with filtering
superpull gea -p 5 -n "^api-" https://gitea.example.com myorg ./api-repos

# Clone all repos from a GitHub organization
export GITHUB_TOKEN=<your-token>
superpull gh orgs/myorg ./myorg-repos

# Clone all repos from GitHub Enterprise
export GITHUB_TOKEN=<your-token>
superpull gh -s https://github.example.com/api/v3 orgs/myorg ./myorg-repos

# Clone repos from GitHub with filtering and throttling
superpull gh -p 5 -n "^backend-" orgs/myorg ./backend-repos

# Clone repos from GitHub and create symlinks for submodules
superpull gh -l -n "^v2-" users/myuser ./v2-repos

# Clone all repos from a GitLab group
export GITLAB_TOKEN=<your-token>
superpull gl -g mygroup ./mygroup-repos

# Clone all repos from self-hosted GitLab
export GITLAB_TOKEN=<your-token>
superpull gl -g -s https://gitlab.example.com mygroup ./mygroup-repos

# Clone all repos from a GitLab user
superpull gl myusername ./myuser-repos

# Clone repos from GitLab with filtering
superpull gl -g -n "^sdk-" -p 5 mygroup ./sdk-repos

# Clone all repos from GitHub excluding forks
export GITHUB_TOKEN=<your-token>
superpull gh -f orgs/myorg ./myorg-repos

# Clone repos from Azure DevOps excluding forks and filtering by name
export AZURE_DEVOPS_TOKEN=<your-token>
superpull az -f -n "^production-" myorg ./prod-repos

# Clone repos from GitLab excluding forks
export GITLAB_TOKEN=<your-token>
superpull gl -f -g mygroup ./mygroup-repos
```

## Installation

### Download Prebuilt Binaries

Prebuilt binaries for Linux (x86_64, ARM64) and macOS (x86_64, ARM64) are available on the [releases page](https://github.com/perjahn/superpullrs/releases). Each binary is compressed as a `.tar.gz` file. Extract and run it with:

```bash
tar -xf superpull-<platform>.tar.gz
./superpull --help
```

### Building from Source

```bash
cargo build --release
```

The binary will be in `target/release/superpull`.

## License

MIT
