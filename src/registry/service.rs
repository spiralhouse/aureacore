use std::collections::HashSet;
use std::path::Path;
use std::{fmt, fs};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use {serde_json, tracing};

use crate::error::{AureaCoreError, Result};
use crate::schema::service::Dependency;
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
    /// Dependencies on other services
    #[serde(default)]
    pub dependencies: Option<Vec<Dependency>>,
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
    /// Warning messages (e.g., missing dependencies or minor version issues)
    pub warnings: Vec<String>,
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
        Self { state, last_checked: Utc::now(), error_message: None, warnings: Vec::new() }
    }

    /// Updates the status with an error
    pub fn with_error(mut self, message: String) -> Self {
        self.state = ServiceState::Error;
        self.error_message = Some(message);
        self.last_checked = Utc::now();
        self
    }

    /// Updates the status with warnings
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
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
    pub fn validate(
        &mut self,
        validation_service: &mut ValidationService,
        available_services: &HashSet<String>,
    ) -> Result<()> {
        self.status = ServiceStatus::new(ServiceState::Validating);

        // Avoid borrow checker issues by cloning values we need for logging
        let service_name = self.name.clone();

        // Load the schema data
        let schema_data = match self.load_schema_data() {
            Ok(data) => {
                let mut data_value = data.clone();

                // Synchronize dependencies from ServiceConfig to schema_data if present in config
                if let Some(dependencies) = &self.config.dependencies {
                    // Create a mutable version of the schema data
                    if let Some(obj) = data_value.as_object_mut() {
                        // Convert dependencies to serde_json::Value
                        let deps_value = serde_json::to_value(dependencies)
                            .unwrap_or(serde_json::Value::Array(vec![]));
                        // Update schema data with dependencies
                        obj.insert("dependencies".to_string(), deps_value);
                    }
                }

                data_value
            }
            Err(err) => return Err(err),
        };

        // Validate the schema with context for dependency validation
        let (result, warnings) = validation_service.validate_service_with_context(
            &service_name,
            &schema_data,
            available_services,
        );

        // Process validation result
        match result {
            Ok(_) => {
                // Service validated successfully but may have warnings
                self.status =
                    ServiceStatus::new(ServiceState::Active).with_warnings(warnings.clone());

                // Log any warnings
                for warning in &warnings {
                    tracing::warn!("Service '{}' validation warning: {}", service_name, warning);
                }

                Ok(())
            }
            Err(err) => {
                let error_message = format!("Schema validation failed: {}", err);
                self.status = ServiceStatus::new(ServiceState::Error)
                    .with_error(error_message)
                    .with_warnings(warnings.clone());

                Err(err)
            }
        }
    }

    /// Gets the current service status
    pub fn status(&self) -> &ServiceStatus {
        &self.status
    }

    /// Sets an error for the service
    pub fn set_error(&mut self, message: String) {
        self.status = ServiceStatus::new(ServiceState::Error).with_error(message);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde_json::json;

    use super::*;

    fn create_test_config(config_path: &str) -> ServiceConfig {
        ServiceConfig {
            namespace: None,
            config_path: config_path.to_string(),
            schema_version: "1.0.0".to_string(),
            dependencies: None,
        }
    }

    #[test]
    fn test_service_creation() {
        let config = create_test_config("test.json");
        let service = Service::new("test-service".to_string(), config);
        assert_eq!(service.name, "test-service");
        assert_eq!(service.status.state, ServiceState::Inactive);
    }

    #[test]
    fn test_service_update_config() {
        let config = create_test_config("test.json");
        let mut service = Service::new("test-service".to_string(), config);

        let new_config = create_test_config("updated.json");
        let result = service.update_config(new_config);

        assert!(result.is_ok());
        assert_eq!(service.config.config_path, "updated.json");
        assert_eq!(service.status.state, ServiceState::Validating);
        assert!(service.schema_data.is_none());
    }

    #[test]
    fn test_service_timestamps() {
        let config = create_test_config("test.json");
        let service = Service::new("test-service".to_string(), config);
        let initial_timestamp = service.last_updated;

        // Sleep briefly to ensure timestamps are different
        std::thread::sleep(std::time::Duration::from_millis(5));

        let mut service = service;
        let new_config = create_test_config("updated.json");
        let _ = service.update_config(new_config);

        assert!(service.last_updated > initial_timestamp);
    }

    #[test]
    fn test_service_validation() {
        let config = create_test_config("test.json");
        let mut service = Service::new("test-service".to_string(), config);

        // Mock schema data
        service.mock_schema_data(json!({
            "name": "test-service",
            "version": "1.0.0",
            "schema_version": "1.0.0",
            "service_type": {
                "type": "rest"
            },
            "endpoints": [
                {
                    "name": "api",
                    "path": "/api"
                }
            ]
        }));

        let mut validation_service = ValidationService::new();
        let available_services = HashSet::new();
        let result = service.validate(&mut validation_service, &available_services);

        assert!(result.is_ok(), "Validation failed: {:?}", result);
        assert_eq!(service.status.state, ServiceState::Active);
    }

    #[test]
    fn test_service_validation_failure() {
        let config = create_test_config("test.json");
        let mut service = Service::new("test-service".to_string(), config);

        // Mock invalid schema data (missing required fields)
        service.mock_schema_data(json!({
            "name": "test-service",
            "version": "1.0.0"
            // Missing service_type and endpoints
        }));

        let mut validation_service = ValidationService::new();
        let available_services = HashSet::new();
        let result = service.validate(&mut validation_service, &available_services);

        assert!(result.is_err());
        assert_eq!(service.status.state, ServiceState::Error);
        assert!(service.status.error_message.is_some());
    }

    #[test]
    fn test_service_validation_with_warnings() {
        let config = create_test_config("test.json");
        let mut service = Service::new("test-service".to_string(), config);

        // Mock schema data with dependencies
        service.mock_schema_data(json!({
            "name": "test-service",
            "version": "1.0.0",
            "schema_version": "1.0.0",
            "service_type": {
                "type": "rest"
            },
            "endpoints": [
                {
                    "name": "api",
                    "path": "/api"
                }
            ],
            "dependencies": [
                {
                    "service": "missing-service",
                    "version_constraint": "1.0.0"
                }
            ]
        }));

        let mut validation_service = ValidationService::new();
        let available_services = HashSet::new(); // Empty set - dependency won't be found
        let result = service.validate(&mut validation_service, &available_services);

        assert!(result.is_ok(), "Validation failed: {:?}", result);
        assert_eq!(service.status.state, ServiceState::Active);
        assert!(!service.status.warnings.is_empty());
        assert!(service.status.warnings[0].contains("missing-service"));
    }

    #[test]
    fn test_service_validation_with_config_dependencies() {
        use crate::schema::service::Dependency;

        // Create a config with dependencies
        let mut config = create_test_config("test.json");
        config.dependencies = Some(vec![Dependency {
            service: "config-dependency".to_string(),
            version_constraint: Some("1.0.0".to_string()),
            required: true,
        }]);

        let mut service = Service::new("test-service".to_string(), config);

        // Mock basic schema data without dependencies field
        service.mock_schema_data(json!({
            "name": "test-service",
            "version": "1.0.0",
            "schema_version": "1.0.0",
            "service_type": {
                "type": "rest"
            },
            "endpoints": [
                {
                    "name": "api",
                    "path": "/api"
                }
            ]
        }));

        let mut validation_service = ValidationService::new();
        let available_services = HashSet::new(); // Empty set - dependency won't be found
        let result = service.validate(&mut validation_service, &available_services);

        assert!(result.is_ok(), "Validation failed: {:?}", result);
        assert_eq!(service.status.state, ServiceState::Active);
        assert!(!service.status.warnings.is_empty());
        assert!(service.status.warnings[0].contains("config-dependency"));
    }
}
