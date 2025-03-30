use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Configuration for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Optional namespace for the service
    pub namespace: Option<String>,
    /// Path to the service configuration file
    pub config_path: String,
    /// Schema version for validation
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
}

fn default_schema_version() -> String {
    "1.0".to_string()
}

/// Status of a service
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    /// Current state of the service
    pub state: ServiceState,
    /// Last time the service was checked
    pub last_checked: DateTime<Utc>,
    /// Optional error message
    pub error_message: Option<String>,
}

/// State of a service
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceState {
    /// Service is active and running
    Active,
    /// Service is inactive or stopped
    Inactive,
    /// Service configuration is being validated
    Validating,
    /// Service is in an error state
    Error,
}

impl fmt::Display for ServiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceState::Active => write!(f, "Active"),
            ServiceState::Inactive => write!(f, "Inactive"),
            ServiceState::Validating => write!(f, "Validating"),
            ServiceState::Error => write!(f, "Error"),
        }
    }
}

impl ServiceStatus {
    /// Creates a new service status
    pub fn new(state: ServiceState) -> Self {
        Self { state, last_checked: Utc::now(), error_message: None }
    }

    /// Updates the status with an error
    pub fn with_error(mut self, message: String) -> Self {
        self.state = ServiceState::Error;
        self.error_message = Some(message);
        self.last_checked = Utc::now();
        self
    }

    /// Updates the status state
    pub fn with_state(mut self, state: ServiceState) -> Self {
        self.state = state;
        self.last_checked = Utc::now();
        self.error_message = None;
        self
    }
}

/// Represents a service in the registry
#[derive(Debug)]
pub struct Service {
    /// Name of the service
    pub name: String,
    /// Service configuration
    pub config: ServiceConfig,
    /// Current status of the service
    pub status: ServiceStatus,
    /// When the service was last updated
    pub last_updated: DateTime<Utc>,
}

impl Service {
    /// Creates a new service instance
    pub fn new(name: String, config: ServiceConfig) -> Self {
        let now = Utc::now();
        Self { name, config, status: ServiceStatus::new(ServiceState::Inactive), last_updated: now }
    }

    /// Updates the service configuration
    pub fn update_config(&mut self, config: ServiceConfig) -> Result<()> {
        self.config = config;
        self.last_updated = Utc::now();
        self.status = ServiceStatus::new(ServiceState::Validating);
        Ok(())
    }

    /// Validates the service configuration
    pub fn validate(&mut self) -> Result<()> {
        // TODO: Implement schema validation
        self.status = ServiceStatus::new(ServiceState::Active);
        Ok(())
    }

    /// Gets the current service status
    pub fn status(&self) -> &ServiceStatus {
        &self.status
    }

    /// Sets an error status with a message
    pub fn set_error(&mut self, message: String) {
        self.status = ServiceStatus::new(ServiceState::Error).with_error(message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> ServiceConfig {
        ServiceConfig {
            namespace: Some("test".to_string()),
            config_path: "test/config.yaml".to_string(),
            schema_version: "1.0".to_string(),
        }
    }

    #[test]
    fn test_service_creation() {
        let config = create_test_config();
        let service = Service::new("test-service".to_string(), config);
        assert_eq!(service.name, "test-service");
        assert_eq!(service.status.state, ServiceState::Inactive);
    }

    #[test]
    fn test_service_update_config() {
        let config = create_test_config();
        let mut service = Service::new("test-service".to_string(), config);

        let new_config = ServiceConfig {
            namespace: Some("new-test".to_string()),
            config_path: "new-test/config.yaml".to_string(),
            schema_version: "1.1".to_string(),
        };
        service.update_config(new_config.clone()).unwrap();
        assert_eq!(service.config.namespace, Some("new-test".to_string()));
        assert_eq!(service.config.config_path, "new-test/config.yaml");
        assert_eq!(service.status.state, ServiceState::Validating);
    }

    #[test]
    fn test_service_status_transitions() {
        let config = create_test_config();
        let mut service = Service::new("test-service".to_string(), config);

        // Initial state
        assert_eq!(service.status.state, ServiceState::Inactive);
        assert!(service.status.error_message.is_none());

        // Set error
        service.set_error("test error".to_string());
        assert_eq!(service.status.state, ServiceState::Error);
        assert_eq!(service.status.error_message, Some("test error".to_string()));

        // Validate (moves to Active)
        service.validate().unwrap();
        assert_eq!(service.status.state, ServiceState::Active);
        assert!(service.status.error_message.is_none());
    }

    #[test]
    fn test_service_timestamps() {
        let config = create_test_config();
        let mut service = Service::new("test-service".to_string(), config);

        let initial_update = service.last_updated;
        let initial_check = service.status.last_checked;

        // Sleep briefly to ensure timestamps differ
        std::thread::sleep(std::time::Duration::from_millis(5));

        // Update should change both timestamps
        let new_config = create_test_config();
        service.update_config(new_config).unwrap();

        assert!(service.last_updated > initial_update);
        assert!(service.status.last_checked > initial_check);
    }
}
