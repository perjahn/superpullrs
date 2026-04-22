/// Common options for all clone operations
#[derive(Debug, Clone)]
pub struct CloneOptions {
    /// Throttle parallel git pull/clone processes
    pub throttle: usize,
    /// Timeout in seconds
    pub timeout: u64,
    /// Filter repos for specific name, using regex
    pub name_patterns: Vec<String>,
    /// Exclude filter repos for specific name, using regex
    pub exclude_patterns: Vec<String>,
    /// Filter repos for max size in KB
    pub max_size_kb: i32,
    /// Create symbolic links between repos, based on git submodules
    pub create_symlinks: bool,
}

impl Default for CloneOptions {
    fn default() -> Self {
        Self {
            throttle: 10,
            timeout: 60,
            name_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            max_size_kb: -1,
            create_symlinks: false,
        }
    }
}

impl CloneOptions {
    /// Create a new CloneOptions with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set throttle value
    pub fn with_throttle(mut self, throttle: usize) -> Self {
        self.throttle = throttle;
        self
    }

    /// Set timeout value
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set name patterns
    pub fn with_name_patterns(mut self, patterns: Vec<String>) -> Self {
        self.name_patterns = patterns;
        self
    }

    /// Set exclude patterns
    pub fn with_exclude_patterns(mut self, patterns: Vec<String>) -> Self {
        self.exclude_patterns = patterns;
        self
    }

    /// Set max size
    pub fn with_max_size_kb(mut self, size: i32) -> Self {
        self.max_size_kb = size;
        self
    }

    /// Set create symlinks flag
    pub fn with_create_symlinks(mut self, create: bool) -> Self {
        self.create_symlinks = create;
        self
    }
}
