//! Git version information captured at compile time

/// Git version info (tag and short SHA)
///
/// Version information is captured at compile time via build.rs, making the binary
/// portable without requiring git to be installed at runtime.
#[derive(Clone, Debug)]
pub struct GitVersion {
    /// Version tag (e.g., "v0.1.0" or crate version if no tag)
    pub tag: String,
    /// Short commit SHA (e.g., "abc1234")
    pub sha: String,
}

impl std::fmt::Display for GitVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.tag, self.sha)
    }
}

impl GitVersion {
    /// Get git version captured at compile time
    ///
    /// This uses environment variables set by build.rs, avoiding the need for
    /// git to be installed at runtime. Falls back to crate version if git info
    /// was not available at compile time.
    #[must_use]
    pub fn detect() -> Self {
        // These are set at compile time by build.rs
        let tag = env!("GIT_VERSION_TAG").to_string();
        let sha = env!("GIT_VERSION_SHA").to_string();
        Self { tag, sha }
    }
}

impl Default for GitVersion {
    fn default() -> Self {
        Self::detect()
    }
}
