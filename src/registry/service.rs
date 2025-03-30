use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Status of a service
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    /// Service is active and running
    Active,
    /// Service is inactive or stopped
    Inactive,
    /// Service is in an error state
    Error,
}

impl fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceStatus::Active => write!(f, "active"),
            ServiceStatus::Inactive => write!(f, "inactive"),
            ServiceStatus::Error => write!(f, "error"),
        }
    }
}

/// Configuration for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Optional namespace for the service
    pub namespace: Option<String>,
    /// Path to the service configuration file
    pub config_path: String,
}

/// Represents a service in the registry
#[derive(Debug, Clone)]
pub struct Service {
    /// Name of the service
    pub name: String,
    /// Service configuration
    pub config: ServiceConfig,
    /// Current status of the service
    pub status: ServiceStatus,
}

impl Service {
    /// Creates a new service instance
    pub fn new(name: String, config: ServiceConfig) -> Self {
        Self { name, config, status: ServiceStatus::Inactive }
    }

    /// Updates the service configuration
    pub fn update_config(&mut self, config: ServiceConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let config = ServiceConfig {
            namespace: Some("test".to_string()),
            config_path: "test/config.yaml".to_string(),
        };
        let service = Service::new("test-service".to_string(), config.clone());

        assert_eq!(service.name, "test-service");
        assert_eq!(service.config.namespace, Some("test".to_string()));
        assert_eq!(service.config.config_path, "test/config.yaml");
        assert_eq!(service.status, ServiceStatus::Inactive);
    }

    #[test]
    fn test_service_update_config() {
        let config = ServiceConfig {
            namespace: Some("test".to_string()),
            config_path: "test/config.yaml".to_string(),
        };
        let mut service = Service::new("test-service".to_string(), config);

        let new_config = ServiceConfig {
            namespace: Some("new-test".to_string()),
            config_path: "new-test/config.yaml".to_string(),
        };
        service.update_config(new_config.clone());

        assert_eq!(service.config.namespace, Some("new-test".to_string()));
        assert_eq!(service.config.config_path, "new-test/config.yaml");
    }

    #[test]
    fn test_service_status_display() {
        assert_eq!(ServiceStatus::Active.to_string(), "active");
        assert_eq!(ServiceStatus::Inactive.to_string(), "inactive");
        assert_eq!(ServiceStatus::Error.to_string(), "error");
    }
}
