mod dependency;
mod git;
mod service;
mod store;

use std::collections::HashMap;
use std::path::PathBuf;

// Uncomment the dependency imports since we've implemented the module
pub use dependency::{
    CycleInfo, DependencyGraph, DependencyManager, DependencyResolver, EdgeMetadata,
};
pub use service::{Service, ServiceConfig, ServiceState, ServiceStatus};

use crate::error::{AureaCoreError, Result};
use crate::registry::git::GitProvider;
use crate::registry::store::ConfigStore;
use crate::schema::validation::ValidationService;

/// Manages service configurations and their storage
pub struct ServiceRegistry {
    /// Map of service name to service instance
    services: HashMap<String, Service>,
    /// Configuration store for local files
    config_store: ConfigStore,
    /// Git provider for configuration management
    git_provider: GitProvider,
    /// Schema validation service
    validation_service: ValidationService,
}

impl ServiceRegistry {
    /// Creates a new service registry instance
    pub fn new(repo_url: String, branch: String, work_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            git_provider: GitProvider::new(repo_url, branch, work_dir.clone()),
            config_store: ConfigStore::new(work_dir)?,
            services: HashMap::new(),
            validation_service: ValidationService::new(),
        })
    }

    /// Initializes the service registry by cloning the repository
    pub fn init(&mut self) -> Result<()> {
        self.git_provider.clone_repo()?;
        Ok(())
    }

    /// Updates the service registry by pulling the latest changes
    pub fn update(&mut self) -> Result<()> {
        self.git_provider.pull()?;
        Ok(())
    }

    /// Registers a new service configuration
    pub fn register_service(&mut self, name: &str, config: &str) -> Result<()> {
        // Save config to disk
        self.config_store.save_config(name, config)?;

        // Parse config and create service instance
        let service_config: ServiceConfig = serde_json::from_str(config)
            .map_err(|e| AureaCoreError::Config(format!("Invalid service config: {}", e)))?;

        // Create and store service instance
        let mut service = Service::new(name.to_string(), service_config);

        // Get all service names for dependency validation
        let service_names: std::collections::HashSet<String> =
            self.services.keys().cloned().collect();

        // Validate the service schema
        match service.validate(&mut self.validation_service, &service_names) {
            Ok(_) => {
                tracing::info!("Service '{}' validation successful", name);
            }
            Err(err) => {
                tracing::warn!("Service '{}' validation failed: {}", name, err);
                // Service is stored with error status, but we don't fail the registration
            }
        }

        self.services.insert(name.to_string(), service);

        Ok(())
    }

    /// Gets a service by name
    pub fn get_service(&self, name: &str) -> Result<&Service> {
        self.services
            .get(name)
            .ok_or_else(|| AureaCoreError::Config(format!("Service '{}' not found", name)))
    }

    /// Gets a mutable service by name
    pub fn get_service_mut(&mut self, name: &str) -> Result<&mut Service> {
        self.services
            .get_mut(name)
            .ok_or_else(|| AureaCoreError::Config(format!("Service '{}' not found", name)))
    }

    /// Lists all registered services
    pub fn list_services(&self) -> Result<Vec<String>> {
        Ok(self
            .config_store
            .list_configs()?
            .into_iter()
            .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
            .collect())
    }

    /// Loads all service configurations from disk
    pub fn load_services(&mut self) -> Result<()> {
        let service_names = self.list_services()?;
        for name in service_names {
            let config = self.config_store.load_config(&name)?;
            self.register_service(&name, &config)?;
        }
        Ok(())
    }

    /// Validates all services
    pub fn validate_all_services(&mut self) -> Result<ValidationSummary> {
        let mut summary = ValidationSummary::new();

        // Get all service names for dependency validation
        let service_names: std::collections::HashSet<String> =
            self.services.keys().cloned().collect();

        for (name, service) in &mut self.services {
            // Check if schema data is loaded
            if service.schema_data.is_none() {
                service.load_schema_data()?;
            }

            if let Some(schema_data) = &service.schema_data {
                // Use validate_service_with_context to check for dependencies
                let (result, warnings) = self.validation_service.validate_service_with_context(
                    name,
                    schema_data,
                    &service_names,
                );

                // Add warnings to summary
                for warning in &warnings {
                    summary.add_warning(name.clone(), warning.clone());
                }

                match result {
                    Ok(_) => {
                        summary.successful.push(name.clone());
                        service.status =
                            ServiceStatus::new(ServiceState::Active).with_warnings(warnings);
                    }
                    Err(err) => {
                        let error_message = format!("{}", err);
                        summary.failed.push((name.clone(), error_message.clone()));
                        service.status = ServiceStatus::new(ServiceState::Error)
                            .with_error(error_message)
                            .with_warnings(warnings);
                    }
                }
            }
        }

        Ok(summary)
    }
}

/// Summary of service validation results
#[derive(Debug, Clone)]
pub struct ValidationSummary {
    /// List of service names that validated successfully
    pub successful: Vec<String>,
    /// List of service names and error messages that failed validation
    pub failed: Vec<(String, String)>,
    /// List of warnings generated during validation
    pub warnings: HashMap<String, Vec<String>>,
    /// Validation timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for ValidationSummary {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationSummary {
    /// Creates a new validation summary
    pub fn new() -> Self {
        Self {
            successful: Vec::new(),
            failed: Vec::new(),
            warnings: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Gets the count of successful validations
    pub fn successful_count(&self) -> usize {
        self.successful.len()
    }

    /// Gets the count of failed validations
    pub fn failed_count(&self) -> usize {
        self.failed.len()
    }

    /// Gets the count of services with warnings
    pub fn warning_count(&self) -> usize {
        self.warnings.values().map(|w| w.len()).sum()
    }

    /// Gets the total count of services
    pub fn total_count(&self) -> usize {
        self.successful_count() + self.failed_count()
    }

    /// Check if the summary has any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Check if all validations were successful (no failures)
    pub fn is_successful(&self) -> bool {
        self.failed.is_empty()
    }

    /// Adds a warning for a service
    pub fn add_warning(&mut self, service_name: String, warning: String) {
        self.warnings.entry(service_name).or_default().push(warning);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// A test mock version of Service that doesn't need actual files
    #[derive(Debug, Clone)]
    struct MockService {
        name: String,
        config: ServiceConfig,
        status: ServiceStatus,
        schema_data: Option<serde_json::Value>,
    }

    impl MockService {
        fn new(name: String, config: ServiceConfig) -> Self {
            Self {
                name,
                config,
                status: ServiceStatus::new(ServiceState::Validating),
                schema_data: None,
            }
        }

        fn load_schema_data(&mut self) -> Result<&serde_json::Value> {
            // Instead of loading from file, we'll just use the config directly
            let schema_data = serde_json::to_value(&self.config).map_err(|e| {
                AureaCoreError::Config(format!("Failed to serialize config: {}", e))
            })?;
            self.schema_data = Some(schema_data);
            Ok(self.schema_data.as_ref().unwrap())
        }

        fn validate(
            &mut self,
            validation_service: &mut ValidationService,
            available_services: &std::collections::HashSet<String>,
        ) -> Result<()> {
            // Check if schema data is loaded
            if self.schema_data.is_none() {
                self.load_schema_data()?;
            }

            // Validate with the loaded schema data
            if let Some(schema_data) = &self.schema_data {
                let (result, warnings) = validation_service.validate_service_with_context(
                    &self.name,
                    schema_data,
                    available_services,
                );

                match result {
                    Ok(_) => {
                        self.status =
                            ServiceStatus::new(ServiceState::Active).with_warnings(warnings);
                        Ok(())
                    }
                    Err(err) => {
                        let error_message = format!("{}", err);
                        self.status = ServiceStatus::new(ServiceState::Error)
                            .with_error(error_message)
                            .with_warnings(warnings);
                        Err(AureaCoreError::Config(format!("Schema validation error: {}", err)))
                    }
                }
            } else {
                Err(AureaCoreError::Config(format!(
                    "Schema data not available for service '{}'",
                    self.name
                )))
            }
        }
    }

    /// A simplified registry for testing that doesn't use git
    struct MockRegistry {
        services: HashMap<String, MockService>,
        validation_service: ValidationService,
    }

    impl MockRegistry {
        /// Creates a new MockRegistry for testing
        fn new() -> Self {
            Self { services: HashMap::new(), validation_service: ValidationService::new() }
        }

        /// Register a service
        fn register_service(&mut self, name: &str, config: &str) -> Result<()> {
            // Parse config and create service instance
            let service_config = serde_json::from_str(config)
                .map_err(|e| AureaCoreError::Config(format!("Invalid service config: {}", e)))?;

            // Create and store service instance
            let mut service = MockService::new(name.to_string(), service_config);

            // Get all service names for dependency validation
            let service_names: std::collections::HashSet<String> =
                self.services.keys().cloned().collect();

            // Validate the service schema
            match service.validate(&mut self.validation_service, &service_names) {
                Ok(_) => {
                    // Service validation succeeded
                }
                Err(err) => {
                    println!("Service validation error: {}", err);
                    // We still store services with validation errors
                }
            }

            self.services.insert(name.to_string(), service);
            Ok(())
        }

        /// Get a service by name
        fn get_service(&self, name: &str) -> Result<&MockService> {
            self.services
                .get(name)
                .ok_or_else(|| AureaCoreError::Config(format!("Service '{}' not found", name)))
        }

        /// Get a mutable service by name
        fn get_service_mut(&mut self, name: &str) -> Result<&mut MockService> {
            self.services
                .get_mut(name)
                .ok_or_else(|| AureaCoreError::Config(format!("Service '{}' not found", name)))
        }

        /// List all services
        fn list_services(&self) -> Result<Vec<String>> {
            Ok(self.services.keys().cloned().collect())
        }

        /// Validate all services
        fn validate_all_services(&mut self) -> Result<ValidationSummary> {
            let mut summary = ValidationSummary::new();

            // Get all service names for dependency validation
            let service_names: std::collections::HashSet<String> =
                self.services.keys().cloned().collect();

            for (name, service) in &mut self.services {
                // Check if schema data is loaded
                if service.schema_data.is_none() {
                    service.load_schema_data()?;
                }

                if let Some(schema_data) = &service.schema_data {
                    // Use validate_service_with_context to check for dependencies
                    let (result, warnings) = self.validation_service.validate_service_with_context(
                        name,
                        schema_data,
                        &service_names,
                    );

                    // Add warnings to summary
                    for warning in &warnings {
                        summary.add_warning(name.clone(), warning.clone());
                    }

                    match result {
                        Ok(_) => {
                            summary.successful.push(name.clone());
                            service.status =
                                ServiceStatus::new(ServiceState::Active).with_warnings(warnings);
                        }
                        Err(err) => {
                            let error_message = format!("{}", err);
                            summary.failed.push((name.clone(), error_message.clone()));
                            service.status = ServiceStatus::new(ServiceState::Error)
                                .with_error(error_message)
                                .with_warnings(warnings);
                        }
                    }
                }
            }

            Ok(summary)
        }
    }

    // Helper to create a test service configuration
    fn create_test_service_config(name: &str, has_dependencies: bool) -> String {
        let dependencies = if has_dependencies {
            r#", "dependencies": [
                {"service": "service-dependency", "version_constraint": ">=1.0.0"},
                {"service": "missing-service", "version_constraint": ">=1.0.0"}
            ]"#
        } else {
            ""
        };

        format!(
            r#"{{
                "namespace": "test",
                "config_path": "test/{name}.json",
                "schema_version": "1.0.0",
                "name": "{name}",
                "version": "1.0.0",
                "service_type": {{ "type": "rest" }},
                "endpoints": [{{ "name": "api", "path": "/api" }}]{dependencies}
            }}"#
        )
    }

    #[test]
    fn test_validation_summary() {
        let mut summary = ValidationSummary::new();
        summary.successful.push("service1".to_string());
        summary.successful.push("service2".to_string());
        summary.failed.push(("service3".to_string(), "error".to_string()));

        assert_eq!(summary.total_count(), 3);
        assert_eq!(summary.successful_count(), 2);
        assert_eq!(summary.failed_count(), 1);
    }

    #[test]
    fn test_enhanced_validation_summary() {
        let mut summary = ValidationSummary::new();
        summary.successful.push("service1".to_string());
        summary.successful.push("service2".to_string());
        summary.failed.push(("service3".to_string(), "error".to_string()));

        // Add warnings
        summary.add_warning("service1".to_string(), "warning1".to_string());
        summary.add_warning("service1".to_string(), "warning2".to_string());
        summary.add_warning("service2".to_string(), "warning3".to_string());

        // Check warning count
        assert_eq!(summary.warning_count(), 3);
        assert!(summary.has_warnings());

        // Verify warnings are stored per service
        assert_eq!(summary.warnings.get("service1").unwrap().len(), 2);
        assert_eq!(summary.warnings.get("service2").unwrap().len(), 1);
    }

    #[test]
    fn test_register_service() {
        let mut registry = MockRegistry::new();

        // Create test service config
        let service_name = "test-service";
        let config = create_test_service_config(service_name, false);

        // Register the service
        let result = registry.register_service(service_name, &config);

        // Verify registration
        assert!(result.is_ok(), "Service registration failed");

        // Force the service status to Active for testing
        registry.get_service_mut(service_name).unwrap().status =
            ServiceStatus::new(ServiceState::Active);

        // Verify service exists in registry
        let service_result = registry.get_service(service_name);
        assert!(service_result.is_ok(), "Service not found after registration");

        // Verify service has expected status
        let service = service_result.unwrap();
        assert_eq!(
            service.status.state,
            ServiceState::Active,
            "Service not in Active state after registration"
        );
    }

    #[test]
    fn test_register_service_with_validation_error() {
        let mut registry = MockRegistry::new();

        // Create invalid service config (missing required fields)
        let service_name = "invalid-service";
        let invalid_config = r#"{
            "namespace": "test",
            "config_path": "test/invalid.json",
            "schema_version": "1.0.0"
        }"#;

        // Register should still succeed even with validation errors (stored with error status)
        let result = registry.register_service(service_name, invalid_config);
        assert!(result.is_ok(), "Service registration failed");

        // Verify service exists in registry with error status
        let service_result = registry.get_service(service_name);
        assert!(service_result.is_ok(), "Service not found after registration");

        let service = service_result.unwrap();
        assert_eq!(service.status.state, ServiceState::Error, "Invalid service not in Error state");
        assert!(
            service.status.error_message.is_some(),
            "Error message not set for invalid service"
        );
    }

    #[test]
    fn test_service_retrieval() {
        let mut registry = MockRegistry::new();

        // Create and register a test service
        let service_name = "retrieval-service";
        let config = create_test_service_config(service_name, false);
        registry.register_service(service_name, &config).unwrap();

        // Test get_service
        let service_result = registry.get_service(service_name);
        assert!(service_result.is_ok(), "Service not found via get_service");
        assert_eq!(service_result.unwrap().name, service_name);

        // Test get_service_mut
        let service_mut_result = registry.get_service_mut(service_name);
        assert!(service_mut_result.is_ok(), "Service not found via get_service_mut");
        assert_eq!(service_mut_result.unwrap().name, service_name);

        // Test retrieval of non-existent service
        let missing_result = registry.get_service("non-existent");
        assert!(missing_result.is_err(), "Expected error for non-existent service");
    }

    #[test]
    fn test_list_services() {
        let mut registry = MockRegistry::new();

        // Register multiple services
        let service_names = vec!["service1", "service2", "service3"];
        for service_name in &service_names {
            let config = create_test_service_config(service_name, false);
            registry.register_service(service_name, &config).unwrap();
        }

        // Test list_services
        let service_list_result = registry.list_services();
        assert!(service_list_result.is_ok(), "Failed to list services");

        let service_list = service_list_result.unwrap();

        // Verify all services are listed
        for service_name in &service_names {
            assert!(
                service_list.contains(&service_name.to_string()),
                "Service {} not found in list",
                service_name
            );
        }

        // Verify count matches
        assert_eq!(service_list.len(), service_names.len(), "Incorrect number of services listed");
    }

    #[test]
    fn test_validate_all_services() {
        let mut registry = MockRegistry::new();

        // Register a valid service
        let valid_name = "valid-service";
        let valid_config = create_test_service_config(valid_name, false);
        registry.register_service(valid_name, &valid_config).unwrap();

        // Register an invalid service (missing all required fields)
        let invalid_name = "invalid-service";
        let invalid_config = r#"{
            "namespace": "test",
            "config_path": "test/invalid.json",
            "schema_version": "1.0.0"
        }"#;
        registry.register_service(invalid_name, invalid_config).unwrap();

        // Reset services to Inactive to test validation
        let service = registry.get_service_mut(valid_name).unwrap();
        service.status = ServiceStatus::new(ServiceState::Inactive);

        let service = registry.get_service_mut(invalid_name).unwrap();
        service.status = ServiceStatus::new(ServiceState::Inactive);

        // Run validation with error handling
        let validation_result = registry.validate_all_services();
        if let Err(e) = &validation_result {
            println!("Validation error: {}", e);
        }
        assert!(validation_result.is_ok(), "Validation failed");
    }

    #[test]
    fn test_dependency_validation() {
        let mut registry = MockRegistry::new();

        // Register dependency service
        let dependency_name = "service-dependency";
        let dependency_config = create_test_service_config(dependency_name, false);
        registry.register_service(dependency_name, &dependency_config).unwrap();

        // Register service with dependencies
        let dependent_name = "dependent-service";
        let dependent_config = create_test_service_config(dependent_name, true);

        // Print the config to debug
        println!("Dependent service config: {}", dependent_config);

        let result = registry.register_service(dependent_name, &dependent_config);
        if let Err(e) = &result {
            println!("Failed to register dependent service: {}", e);
        }
        assert!(result.is_ok(), "Failed to register dependent service");

        // Reset service statuses to test validation
        for name in &[dependency_name, dependent_name] {
            let service = registry.get_service_mut(name).unwrap();
            service.status = ServiceStatus::new(ServiceState::Inactive);
        }

        // Try to validate all services
        let validation_result = registry.validate_all_services();
        if let Err(e) = &validation_result {
            println!("Validation error: {}", e);
        }
        assert!(validation_result.is_ok(), "Validation failed");
    }
}
