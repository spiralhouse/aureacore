use std::path::Path;
use std::{fmt, fs};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;

use crate::error::{AureaCoreError, Result};
use crate::schema::validation::ValidationService;

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
    "1.0.0".to_string()
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
    /// Cached service schema data
    pub schema_data: Option<serde_json::Value>,
}

impl Service {
    /// Creates a new service instance
    pub fn new(name: String, config: ServiceConfig) -> Self {
        let now = Utc::now();
        Self {
            name,
            config,
            status: ServiceStatus::new(ServiceState::Inactive),
            last_updated: now,
            schema_data: None,
        }
    }

    /// Updates the service configuration
    pub fn update_config(&mut self, config: ServiceConfig) -> Result<()> {
        self.config = config;
        self.last_updated = Utc::now();
        self.status = ServiceStatus::new(ServiceState::Validating);
        self.schema_data = None;
        Ok(())
    }

    /// Loads the service schema data from the config path
    pub fn load_schema_data(&mut self) -> Result<&serde_json::Value> {
        if self.schema_data.is_none() {
            let config_path = Path::new(&self.config.config_path);

            if !config_path.exists() {
                return Err(AureaCoreError::Service(format!(
                    "Configuration file not found: {}",
                    self.config.config_path
                )));
            }

            let config_content = fs::read_to_string(config_path).map_err(|e| {
                AureaCoreError::Service(format!("Failed to read configuration file: {}", e))
            })?;

            // Parse the configuration content based on file extension
            let data = if config_path.extension().is_some_and(|ext| ext == "json") {
                serde_json::from_str::<serde_json::Value>(&config_content).map_err(|e| {
                    AureaCoreError::Service(format!("Failed to parse JSON configuration: {}", e))
                })?
            } else if config_path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml") {
                let yaml_value: serde_yaml::Value =
                    serde_yaml::from_str(&config_content).map_err(|e| {
                        AureaCoreError::Service(format!(
                            "Failed to parse YAML configuration: {}",
                            e
                        ))
                    })?;

                // Convert YAML to JSON value
                serde_json::to_value(yaml_value).map_err(|e| {
                    AureaCoreError::Service(format!("Failed to convert YAML to JSON: {}", e))
                })?
            } else {
                return Err(AureaCoreError::Service(format!(
                    "Unsupported configuration file format: {}",
                    self.config.config_path
                )));
            };

            self.schema_data = Some(data);
        }

        Ok(self.schema_data.as_ref().unwrap())
    }

    // Mock the load_schema_data method for testing
    #[cfg(test)]
    fn mock_schema_data(&mut self, schema_data: serde_json::Value) {
        self.schema_data = Some(schema_data);
    }

    /// Validates the service configuration
    pub fn validate(&mut self, validation_service: &mut ValidationService) -> Result<()> {
        self.status = ServiceStatus::new(ServiceState::Validating);

        // Load the schema data
        let schema_data = self.load_schema_data()?;

        // Validate the schema
        match validation_service.validate_service(schema_data) {
            Ok(_) => {
                self.status = ServiceStatus::new(ServiceState::Active);
                Ok(())
            }
            Err(err) => {
                let error_message = format!("Schema validation failed: {}", err);
                self.status = ServiceStatus::new(ServiceState::Error).with_error(error_message);
                Err(err)
            }
        }
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
    use serde_json::json;

    use super::*;
    use crate::schema::validation::ValidationService;

    fn create_test_config(config_path: &str) -> ServiceConfig {
        ServiceConfig {
            namespace: Some("test".to_string()),
            config_path: config_path.to_string(),
            schema_version: "1.0.0".to_string(),
        }
    }

    #[test]
    fn test_service_creation() {
        let config = create_test_config("test/config.yaml");
        let service = Service::new("test-service".to_string(), config);
        assert_eq!(service.name, "test-service");
        assert_eq!(service.status.state, ServiceState::Inactive);
    }

    #[test]
    fn test_service_update_config() {
        let config = create_test_config("test/config.yaml");
        let mut service = Service::new("test-service".to_string(), config);

        let new_config = ServiceConfig {
            namespace: Some("new-test".to_string()),
            config_path: "new-test/config.yaml".to_string(),
            schema_version: "1.1.0".to_string(),
        };
        service.update_config(new_config.clone()).unwrap();
        assert_eq!(service.config.namespace, Some("new-test".to_string()));
        assert_eq!(service.config.config_path, "new-test/config.yaml");
        assert_eq!(service.config.schema_version, "1.1.0");
        assert_eq!(service.status.state, ServiceState::Validating);
    }

    #[test]
    fn test_service_timestamps() {
        let config = create_test_config("test/config.yaml");
        let mut service = Service::new("test-service".to_string(), config);

        let initial_update = service.last_updated;
        let initial_check = service.status.last_checked;

        // Sleep briefly to ensure timestamps differ
        std::thread::sleep(std::time::Duration::from_millis(5));

        // Update should change both timestamps
        let new_config = create_test_config("test/config.yaml");
        service.update_config(new_config).unwrap();

        assert!(service.last_updated > initial_update);
        assert!(service.status.last_checked > initial_check);
    }

    #[test]
    fn test_service_validation() {
        let config = create_test_config("test/config.json");
        let mut service = Service::new("test-service".to_string(), config);
        let mut validation_service = ValidationService::new();

        // Mock the schema data to avoid file system access
        service.mock_schema_data(json!({
            "name": "test-service",
            "version": "1.0.0",
            "service_type": {
                "type": "rest"
            },
            "endpoints": [
                {
                    "name": "test",
                    "path": "/test"
                }
            ],
            "schema_version": "1.0.0"
        }));

        // Validation should pass for valid schema
        let result = service.validate(&mut validation_service);
        assert!(result.is_ok(), "Validation failed: {:?}", result);
        assert_eq!(service.status.state, ServiceState::Active);
    }

    #[test]
    fn test_service_validation_failure() {
        let config = create_test_config("test/config.json");
        let mut service = Service::new("test-service".to_string(), config);
        let mut validation_service = ValidationService::new();

        // Mock the schema data with invalid data
        service.mock_schema_data(json!({
            "name": "test-service",
            "version": "1.0.0"
            // Missing required fields
        }));

        // Validation should fail for invalid schema
        let result = service.validate(&mut validation_service);
        assert!(result.is_err());
        assert_eq!(service.status.state, ServiceState::Error);
        assert!(service.status.error_message.is_some());
    }
}
