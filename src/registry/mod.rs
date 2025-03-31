mod git;
mod service;
mod store;

use std::collections::HashMap;
use std::path::PathBuf;

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

        // Perform validation on all services
        for (name, service) in &mut self.services {
            if let Some(schema_data) = &service.schema_data {
                // Use the full validation with context
                let (result, warnings) = self.validation_service.validate_service_with_context(
                    name,
                    schema_data,
                    &service_names,
                );

                // Add any warnings to the summary
                for warning in &warnings {
                    summary.add_warning(name.clone(), warning.clone());
                    tracing::warn!("Service '{}' validation warning: {}", name, warning);
                }

                match result {
                    Ok(_) => {
                        summary.successful.push(name.clone());
                        service.status =
                            ServiceStatus::new(ServiceState::Active).with_warnings(warnings);
                        tracing::info!("Service '{}' validation successful", name);
                    }
                    Err(err) => {
                        let error_message = format!("{}", err);
                        summary.failed.push((name.clone(), error_message.clone()));
                        service.status = ServiceStatus::new(ServiceState::Error)
                            .with_error(error_message)
                            .with_warnings(warnings);
                        tracing::warn!("Service '{}' validation failed: {}", name, err);
                    }
                }
            } else {
                // Load schema data if not already loaded
                match service.load_schema_data() {
                    Ok(schema_data) => {
                        let (result, warnings) = self
                            .validation_service
                            .validate_service_with_context(name, schema_data, &service_names);

                        // Add any warnings to the summary
                        for warning in &warnings {
                            summary.add_warning(name.clone(), warning.clone());
                            tracing::warn!("Service '{}' validation warning: {}", name, warning);
                        }

                        match result {
                            Ok(_) => {
                                summary.successful.push(name.clone());
                                service.status = ServiceStatus::new(ServiceState::Active)
                                    .with_warnings(warnings);
                                tracing::info!("Service '{}' validation successful", name);
                            }
                            Err(err) => {
                                let error_message = format!("{}", err);
                                summary.failed.push((name.clone(), error_message.clone()));
                                service.status = ServiceStatus::new(ServiceState::Error)
                                    .with_error(error_message)
                                    .with_warnings(warnings);
                                tracing::warn!("Service '{}' validation failed: {}", name, err);
                            }
                        }
                    }
                    Err(err) => {
                        let error_message = format!("Failed to load schema data: {}", err);
                        summary.failed.push((name.clone(), error_message.clone()));
                        service.status =
                            ServiceStatus::new(ServiceState::Error).with_error(error_message);
                        tracing::error!("Service '{}' schema data loading failed: {}", name, err);
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
    pub warnings: Vec<(String, String)>,
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
            warnings: Vec::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Returns the number of successful validations
    pub fn successful_count(&self) -> usize {
        self.successful.len()
    }

    /// Returns the number of failed validations
    pub fn failed_count(&self) -> usize {
        self.failed.len()
    }

    /// Returns the number of warnings
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// Returns the total number of validation attempts
    pub fn total_count(&self) -> usize {
        self.successful.len() + self.failed.len()
    }

    /// Adds a warning to the summary
    pub fn add_warning(&mut self, service_name: String, warning: String) {
        self.warnings.push((service_name, warning));
    }

    /// Checks if the validation was successful (no failures)
    pub fn is_successful(&self) -> bool {
        self.failed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use git2::Repository;

    use super::*;

    #[test]
    fn test_registry_initialization() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().join("test-repo");
        let work_dir = temp_dir.path().join("work-dir");

        // Create a test repository with proper initialization
        fs::create_dir_all(&repo_path).unwrap();
        let repo = Repository::init(&repo_path).unwrap();

        // Create and add a README file
        let readme_path = repo_path.join("README.md");
        fs::write(&readme_path, "# Test Repository").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("README.md")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("test", "test@example.com").unwrap();

        // Create initial commit on main branch
        repo.commit(Some("refs/heads/main"), &signature, &signature, "Initial commit", &tree, &[])
            .unwrap();

        // Set HEAD to refs/heads/main
        repo.set_head("refs/heads/main").unwrap();

        // Initialize ServiceRegistry with file:// URL
        let repo_url = format!("file://{}", repo_path.to_str().unwrap());
        let mut registry =
            ServiceRegistry::new(repo_url, "main".to_string(), work_dir.clone()).unwrap();

        // Test initialization
        let result = registry.init();
        assert!(result.is_ok(), "Registry initialization failed");
    }

    #[test]
    fn test_validation_summary() {
        let mut summary = ValidationSummary::new();

        summary.successful.push("service1".to_string());
        summary.successful.push("service2".to_string());
        summary.failed.push(("service3".to_string(), "error".to_string()));

        assert_eq!(summary.successful_count(), 2);
        assert_eq!(summary.failed_count(), 1);
        assert_eq!(summary.total_count(), 3);
    }

    #[test]
    fn test_enhanced_validation_summary() {
        let mut summary = ValidationSummary::new();

        // Add successful services
        summary.successful.push("service1".to_string());
        summary.successful.push("service2".to_string());

        // Add warnings
        summary.add_warning(
            "service2".to_string(),
            "Optional dependency 'service3' not found".to_string(),
        );
        summary.add_warning(
            "service1".to_string(),
            "Minor schema version incompatibility".to_string(),
        );

        // Add failures
        summary.failed.push(("service3".to_string(), "Schema validation failed".to_string()));

        // Verify counts
        assert_eq!(summary.successful_count(), 2);
        assert_eq!(summary.failed_count(), 1);
        assert_eq!(summary.warning_count(), 2);
        assert_eq!(summary.total_count(), 3);

        // Verify successful flag
        assert!(!summary.is_successful());

        // Create a successful summary
        let mut successful_summary = ValidationSummary::new();
        successful_summary.successful.push("service1".to_string());
        successful_summary.add_warning("service1".to_string(), "Minor warning".to_string());

        // Verify it's considered successful (warnings don't count as failures)
        assert!(successful_summary.is_successful());
    }
}
