use regex::Regex;

#[derive(Debug, Clone)]
pub struct FilterOptions {
    pub name_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_size_kb: i32,
}

impl FilterOptions {
    pub fn new() -> Self {
        Self {
            name_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            max_size_kb: -1,
        }
    }

    pub fn with_name_patterns(mut self, patterns: Vec<String>) -> Self {
        self.name_patterns = patterns;
        self
    }

    pub fn with_exclude_patterns(mut self, patterns: Vec<String>) -> Self {
        self.exclude_patterns = patterns;
        self
    }

    pub fn with_max_size_kb(mut self, max_size: i32) -> Self {
        self.max_size_kb = max_size;
        self
    }

    /// Check if a repository should be included based on name patterns and exclusions.
    pub fn should_include_by_name(&self, name: &str) -> bool {
        // Check exclude patterns
        for pattern in &self.exclude_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(name) {
                    return false;
                }
            }
        }

        // Check name patterns (if specified, repo must match at least one)
        if self.name_patterns.is_empty() {
            return true;
        }

        for pattern in &self.name_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(name) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a repository should be included based on size in KB.
    pub fn should_include_by_size(&self, size_kb: i32) -> bool {
        if self.max_size_kb >= 0 {
            size_kb <= self.max_size_kb
        } else {
            true
        }
    }

    /// Check if a repository should be included based on name and size (in KB).
    pub fn should_include(&self, name: &str, size_kb: i32) -> bool {
        self.should_include_by_size(size_kb) && self.should_include_by_name(name)
    }

    /// Check if a repository should be included based on name and size (in bytes).
    /// Useful for systems that report size in bytes (e.g., Bitbucket).
    pub fn should_include_bytes(&self, name: &str, size_bytes: u32) -> bool {
        let size_kb = (size_bytes / 1024) as i32;
        self.should_include(name, size_kb)
    }
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self::new()
    }
}
