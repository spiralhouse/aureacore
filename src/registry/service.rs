use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Current status of a service
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServiceStatus {
    /// Service is in the process of being configured
    Pending { message: String, last_checked: DateTime<Utc> },
    /// Service is properly configured and validated
    Configured { last_checked: DateTime<Utc> },
    /// Service has configuration or validation errors
    Error { message: String, last_checked: DateTime<Utc> },
}

impl fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceStatus::Pending { message, .. } => write!(f, "Pending: {}", message),
            ServiceStatus::Configured { .. } => write!(f, "Configured"),
            ServiceStatus::Error { message, .. } => write!(f, "Error: {}", message),
        }
    }
}

/// Configuration for a specific service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Version of the service configuration
    pub version: String,
    /// Service-specific parameters
    #[serde(default)]
    pub parameters: std::collections::HashMap<String, serde_yaml::Value>,
    /// Service dependencies
    #[serde(default)]
    pub dependencies: Vec<String>,
}

impl ServiceConfig {
    /// Validate the service configuration against its schema
    pub fn validate_schema(&self) -> Result<()> {
        // TODO: Implement schema validation
        Ok(())
    }
}

/// Represents a service in the system
#[derive(Debug, Clone)]
pub struct Service {
    /// Unique name of the service
    pub name: String,
    /// Namespace the service belongs to
    pub namespace: String,
    /// Current service configuration
    pub config: ServiceConfig,
    /// Current status of the service
    pub status: ServiceStatus,
    /// Last time the service was updated
    pub last_updated: DateTime<Utc>,
}

impl Service {
    /// Create a new service with the given configuration
    pub fn new(name: String, namespace: String, config: ServiceConfig) -> Self {
        let now = Utc::now();
        Self {
            name,
            namespace,
            config,
            status: ServiceStatus::Pending {
                message: "Initial configuration".to_string(),
                last_checked: now,
            },
            last_updated: now,
        }
    }

    /// Validate the service configuration
    pub fn validate(&self) -> Result<()> {
        self.config.validate_schema()
    }

    /// Update the service configuration
    pub fn update_config(&mut self, config: ServiceConfig) -> Result<()> {
        let now = Utc::now();

        // Validate the new configuration
        self.status = ServiceStatus::Pending {
            message: "Validating new configuration".to_string(),
            last_checked: now,
        };

        if let Err(e) = config.validate_schema() {
            self.status = ServiceStatus::Error {
                message: format!("Configuration validation failed: {}", e),
                last_checked: now,
            };
            return Err(e);
        }

        // Update configuration if valid
        self.config = config;
        self.status = ServiceStatus::Configured { last_checked: now };
        self.last_updated = now;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn create_test_config() -> ServiceConfig {
        ServiceConfig {
            version: "1.0".to_string(),
            parameters: HashMap::new(),
            dependencies: vec![],
        }
    }

    #[test]
    fn test_service_creation() {
        let config = create_test_config();
        let service =
            Service::new("test-service".to_string(), "default".to_string(), config.clone());

        assert_eq!(service.name, "test-service");
        assert_eq!(service.namespace, "default");
        assert_eq!(service.config.version, "1.0");

        match service.status {
            ServiceStatus::Pending { message, .. } => {
                assert_eq!(message, "Initial configuration");
            }
            _ => panic!("Expected Pending status"),
        }
    }

    #[test]
    fn test_service_update_config() {
        let mut service =
            Service::new("test-service".to_string(), "default".to_string(), create_test_config());

        let mut new_config = create_test_config();
        new_config.version = "2.0".to_string();
        new_config
            .parameters
            .insert("key".to_string(), serde_yaml::Value::String("value".to_string()));

        assert!(service.update_config(new_config.clone()).is_ok());
        assert_eq!(service.config.version, "2.0");

        match service.status {
            ServiceStatus::Configured { .. } => (),
            _ => panic!("Expected Configured status"),
        }
    }

    #[test]
    fn test_service_status_display() {
        let now = Utc::now();

        let pending = ServiceStatus::Pending { message: "Testing".to_string(), last_checked: now };
        assert_eq!(pending.to_string(), "Pending: Testing");

        let configured = ServiceStatus::Configured { last_checked: now };
        assert_eq!(configured.to_string(), "Configured");

        let error = ServiceStatus::Error { message: "Failed".to_string(), last_checked: now };
        assert_eq!(error.to_string(), "Error: Failed");
    }
}
