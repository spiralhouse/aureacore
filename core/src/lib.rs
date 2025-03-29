//! Core functionality for AureaCore service catalog

use serde::{Deserialize, Serialize};

/// Basic service definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    /// Unique identifier for the service
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Service version
    pub version: String,
}

impl Service {
    /// Create a new service definition
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self { name: name.into(), description: None, version: version.into() }
    }

    /// Add a description to the service
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_service_creation() {
        let service = Service::new("test-service", "1.0.0").with_description("A test service");

        assert_eq!(service.name, "test-service");
        assert_eq!(service.version, "1.0.0");
        assert_eq!(service.description, Some("A test service".to_string()));
    }
}
