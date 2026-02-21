//! Application builder for composing components.

/// Builder for constructing the application.
pub struct Builder {
    config_path: Option<std::path::PathBuf>,
}

impl Builder {
    pub fn new() -> Self {
        Self { config_path: None }
    }

    pub fn config_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.config_path = Some(path.into());
        self
    }

    // Add more builder methods as needed based on existing patterns
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
