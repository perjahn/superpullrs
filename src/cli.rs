use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "superpull")]
#[command(about = "Faster git repo puller", long_about = None)]
pub struct Args {
    /// Use bearer token auth, instead of basic auth
    #[arg(short = 'b')]
    pub bearer_token: bool,

    /// Throttle parallel git pull/clone processes (default: 10)
    #[arg(short = 'p', default_value = "10")]
    pub throttle: usize,

    /// Timeout, in seconds (default: 60)
    #[arg(short = 't', default_value = "60")]
    pub timeout: u64,

    /// Root folder for super-pull (when no subcommand specified)
    #[arg(value_name = "FOLDER")]
    pub folder: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Pull all git repositories in a folder
    #[command(visible_alias = "pull")]
    SuperPull {
        /// Root folder to search for git repositories
        folder: String,

        /// Recurse into subfolders
        #[arg(short = 'r')]
        recurse: bool,
    },
    /// Clone all repositories from an Azure DevOps organization
    AzClone {
        /// Azure DevOps organization name
        organization: String,

        /// Target folder for cloning
        #[arg(default_value = ".")]
        folder: String,

        /// Azure DevOps Personal Access Token (PAT)
        #[arg(short = 'a')]
        token: Option<String>,

        /// Azure DevOps base URL for self-hosted (e.g., https://azuredevops.example.com)
        #[arg(short = 's')]
        server_url: Option<String>,

        /// Filter repos for specific name, using regex. Can be specified multiple times
        #[arg(short = 'n')]
        name_patterns: Vec<String>,

        /// Exclude filter repos for specific name, using regex. Can be specified multiple times
        #[arg(short = 'o')]
        exclude_patterns: Vec<String>,

        /// Filter repos for max size in KB
        #[arg(short = 'm', default_value = "-1")]
        max_size_kb: i32,

        /// Create symbolic links between repos, based on git submodules
        #[arg(short = 'l')]
        create_symlinks: bool,
    },
    /// Clone all repositories from a Bitbucket workspace (Cloud) or project (Server/Data Center)
    BbClone {
        /// Target folder for cloning
        #[arg(default_value = ".")]
        folder: String,

        /// Bitbucket API token (Cloud: personal token or app password, Server: personal token)
        #[arg(short = 'a')]
        token: Option<String>,

        /// For Server/Data Center: base URL (e.g., https://bitbucket.example.com)
        #[arg(short = 's')]
        server_url: Option<String>,

        /// Filter repos for specific name, using regex. Can be specified multiple times
        #[arg(short = 'n')]
        name_patterns: Vec<String>,

        /// Exclude filter repos for specific name, using regex. Can be specified multiple times
        #[arg(short = 'o')]
        exclude_patterns: Vec<String>,

        /// Filter repos for max size in KB
        #[arg(short = 'm', default_value = "-1")]
        max_size_kb: i32,

        /// Create symbolic links between repos, based on git submodules
        #[arg(short = 'l')]
        create_symlinks: bool,

        /// Use Bitbucket Server API v1.0 instead of v2.0
        #[arg(short = '1')]
        use_api_v1: bool,
    },
    /// Clone all repositories from a Forgejo organization
    FojClone {
        /// Forgejo base URL (e.g., https://forgejo.example.com)
        base_url: String,

        /// Forgejo organization name
        organization: String,

        /// Target folder for cloning
        #[arg(default_value = ".")]
        folder: String,

        /// Forgejo API token
        #[arg(short = 'a')]
        token: Option<String>,

        /// Filter repos for specific name, using regex. Can be specified multiple times
        #[arg(short = 'n')]
        name_patterns: Vec<String>,

        /// Exclude filter repos for specific name, using regex. Can be specified multiple times
        #[arg(short = 'o')]
        exclude_patterns: Vec<String>,

        /// Filter repos for max size in KB
        #[arg(short = 'm', default_value = "-1")]
        max_size_kb: i32,

        /// Create symbolic links between repos, based on git submodules
        #[arg(short = 'l')]
        create_symlinks: bool,
    },
    /// Clone all repositories from a Gitea organization
    GeaClone {
        /// Gitea base URL (e.g., https://gitea.example.com)
        base_url: String,

        /// Gitea organization name
        organization: String,

        /// Target folder for cloning
        #[arg(default_value = ".")]
        folder: String,

        /// Gitea API token
        #[arg(short = 'a')]
        token: Option<String>,

        /// Filter repos for specific name, using regex. Can be specified multiple times
        #[arg(short = 'n')]
        name_patterns: Vec<String>,

        /// Exclude filter repos for specific name, using regex. Can be specified multiple times
        #[arg(short = 'o')]
        exclude_patterns: Vec<String>,

        /// Filter repos for max size in KB
        #[arg(short = 'm', default_value = "-1")]
        max_size_kb: i32,

        /// Create symbolic links between repos, based on git submodules
        #[arg(short = 'l')]
        create_symlinks: bool,
    },
    /// Clone all repositories from a GitHub organization or user
    GhClone {
        /// GitHub entity: orgs/<orgname> or users/<username>
        entity: String,

        /// Target folder for cloning
        #[arg(default_value = ".")]
        folder: String,

        /// GitHub API base URL (for GitHub Enterprise: https://github.example.com/api/v3)
        #[arg(short = 's')]
        server_url: Option<String>,

        /// Filter repos for specific team. Can be specified multiple times
        #[arg(short = 'e')]
        teams: Vec<String>,

        /// Filter repos for specific name, using regex. Can be specified multiple times
        #[arg(short = 'n')]
        name_patterns: Vec<String>,

        /// Exclude filter repos for specific name, using regex. Can be specified multiple times
        #[arg(short = 'o')]
        exclude_patterns: Vec<String>,

        /// Filter repos for max size in KB of the .git folder
        #[arg(short = 'm', default_value = "-1")]
        max_size_kb: i32,

        /// Create symbolic links between repos, based on git submodules
        #[arg(short = 'l')]
        create_symlinks: bool,
    },
    /// Clone all repositories from a GitLab group or user
    GlClone {
        /// GitLab group path or username
        entity: String,

        /// Target folder for cloning
        #[arg(default_value = ".")]
        folder: String,

        /// GitLab personal access token or CI job token
        #[arg(short = 'a')]
        token: Option<String>,

        /// GitLab base URL for self-hosted (e.g., https://gitlab.example.com)
        #[arg(short = 's')]
        server_url: Option<String>,

        /// If set, treat entity as a group; otherwise treat as user
        #[arg(short = 'g')]
        is_group: bool,

        /// Filter repos for specific name, using regex. Can be specified multiple times
        #[arg(short = 'n')]
        name_patterns: Vec<String>,

        /// Exclude filter repos for specific name, using regex. Can be specified multiple times
        #[arg(short = 'o')]
        exclude_patterns: Vec<String>,

        /// Filter repos for max size in KB
        #[arg(short = 'm', default_value = "-1")]
        max_size_kb: i32,

        /// Create symbolic links between repos, based on git submodules
        #[arg(short = 'l')]
        create_symlinks: bool,
    },
}
