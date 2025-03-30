use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AureaCoreError {
    SchemaValidation(String),
    // We'll add more error types as we implement more features
}

impl fmt::Display for AureaCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AureaCoreError::SchemaValidation(msg) => write!(f, "Schema validation error: {}", msg),
        }
    }
}

impl Error for AureaCoreError {}

pub type Result<T> = std::result::Result<T, AureaCoreError>;
