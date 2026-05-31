use regex::Regex;

#[derive(Debug, Clone)]
pub struct FilterOptions {
    pub name_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub exclude_forked: bool,
    pub max_size_kb: i32,
}

impl FilterOptions {
    pub fn new() -> Self {
        Self {
            name_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            exclude_forked: false,
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

    pub fn with_exclude_forked(mut self, exclude_forked: bool) -> Self {
        self.exclude_forked = exclude_forked;
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
            if let Ok(re) = Regex::new(pattern)
                && re.is_match(name)
            {
                return false;
            }
        }

        // Check name patterns (if specified, repo must match at least one)
        if self.name_patterns.is_empty() {
            return true;
        }

        for pattern in &self.name_patterns {
            if let Ok(re) = Regex::new(pattern)
                && re.is_match(name)
            {
                return true;
            }
        }

        false
    }

    /// Check if a repository should be included based on fork status.
    pub fn should_include_by_fork(&self, is_forked: bool) -> bool {
        if self.exclude_forked {
            !is_forked
        } else {
            true
        }
    }

    /// Check if a repository should be included based on size in KB.
    pub fn should_include_by_size(&self, size_kb: i32) -> bool {
        if self.max_size_kb >= 0 {
            size_kb <= self.max_size_kb
        } else {
            true
        }
    }

    /// Check if a repository should be included based on name, fork status, and size (in KB).
    pub fn should_include(&self, name: &str, is_forked: bool, size_kb: i32) -> bool {
        self.should_include_by_size(size_kb)
            && self.should_include_by_name(name)
            && self.should_include_by_fork(is_forked)
    }

    /// Check if a repository should be included based on name, fork status, and size (in bytes).
    /// Useful for systems that report size in bytes (e.g., Bitbucket).
    pub fn should_include_bytes(&self, name: &str, is_forked: bool, size_bytes: u32) -> bool {
        let size_kb = (size_bytes / 1024) as i32;
        self.should_include(name, is_forked, size_kb)
    }
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_include_by_fork_when_exclude_forked_false() {
        let filter = FilterOptions::new();
        assert!(filter.should_include_by_fork(true));
        assert!(filter.should_include_by_fork(false));
    }

    #[test]
    fn test_should_include_by_fork_when_exclude_forked_true() {
        let filter = FilterOptions::new().with_exclude_forked(true);
        assert!(!filter.should_include_by_fork(true));
        assert!(filter.should_include_by_fork(false));
    }

    #[test]
    fn test_should_include_with_fork_excluded() {
        let filter = FilterOptions::new()
            .with_exclude_forked(true)
            .with_name_patterns(vec!["my.*".to_string()]);

        // Forked repo - should be excluded
        assert!(!filter.should_include("my_repo", true, 100));

        // Non-forked repo with matching name - should be included
        assert!(filter.should_include("my_repo", false, 100));

        // Non-forked repo without matching name - should be excluded
        assert!(!filter.should_include("other_repo", false, 100));
    }

    #[test]
    fn test_should_include_with_fork_and_size_filter() {
        let filter = FilterOptions::new()
            .with_exclude_forked(true)
            .with_max_size_kb(500);

        // Forked repo - excluded regardless of size
        assert!(!filter.should_include("repo", true, 100));

        // Non-forked repo, under size limit - included
        assert!(filter.should_include("repo", false, 100));

        // Non-forked repo, over size limit - excluded
        assert!(!filter.should_include("repo", false, 600));
    }

    #[test]
    fn test_should_include_with_fork_and_exclude_pattern() {
        let filter = FilterOptions::new()
            .with_exclude_forked(true)
            .with_exclude_patterns(vec!["test.*".to_string()]);

        // Forked repo - excluded
        assert!(!filter.should_include("my_repo", true, 100));

        // Non-forked but excluded by pattern - excluded
        assert!(!filter.should_include("test_repo", false, 100));

        // Non-forked, not excluded by pattern - included
        assert!(filter.should_include("prod_repo", false, 100));
    }

    #[test]
    fn test_should_include_bytes_with_fork() {
        let filter = FilterOptions::new().with_exclude_forked(true);

        // Forked repo - excluded
        assert!(!filter.should_include_bytes("repo", true, 1024));

        // Non-forked repo - included
        assert!(filter.should_include_bytes("repo", false, 1024));
    }
}
