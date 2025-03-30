use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AureaCoreError {
    /// Error during Git operations
    Git(String),
    /// Error during file system operations
    Io(std::io::Error),
    /// Error during configuration parsing
    Config(String),
    /// Error during service operations
    Service(String),
    // We'll add more error types as we implement more features
}

impl fmt::Display for AureaCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AureaCoreError::Git(msg) => write!(f, "Git error: {}", msg),
            AureaCoreError::Io(err) => write!(f, "IO error: {}", err),
            AureaCoreError::Config(msg) => write!(f, "Configuration error: {}", msg),
            AureaCoreError::Service(msg) => write!(f, "Service error: {}", msg),
        }
    }
}

impl Error for AureaCoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AureaCoreError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AureaCoreError {
    fn from(err: std::io::Error) -> Self {
        AureaCoreError::Io(err)
    }
}

impl From<git2::Error> for AureaCoreError {
    fn from(err: git2::Error) -> Self {
        AureaCoreError::Git(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AureaCoreError>;
