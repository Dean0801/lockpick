use std::io;

/// Central error type for lockpick
#[derive(Debug, thiserror::Error)]
pub enum LockpickError {
    /// I/O errors (file read/write)
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Parse errors (lockfile, config, JSON, YAML)
    #[error("Parse error: {0}")]
    Parse(String),

    /// No lockfile found in project
    #[error("No lockfile found: {0}")]
    NoLockfile(String),

    /// Configuration errors
    #[error("Config error: {0}")]
    Config(String),

    /// Network errors (OSV API, etc.)
    #[error("Network error: {0}")]
    Network(String),

    /// Report generation errors
    #[error("Report error: {0}")]
    Report(String),
}
