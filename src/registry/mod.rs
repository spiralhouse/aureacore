mod git;
mod service;
mod store;

use std::collections::HashMap;
use std::path::PathBuf;

use git::GitProvider;
pub use service::{Service, ServiceConfig, ServiceStatus};
use store::ConfigStore;

use crate::error::{AureaCoreError, Result};

/// Central registry for managing services and their configurations
pub struct ServiceRegistry {
    /// Map of service name to service instance
    services: HashMap<String, Service>,
    /// Git provider for configuration management
    git_provider: GitProvider,
    /// Configuration store for local files
    config_store: ConfigStore,
}

impl ServiceRegistry {
    /// Create a new ServiceRegistry
    pub fn new(repo_url: String, branch: String, config_dir: impl Into<PathBuf>) -> Self {
        let config_dir = config_dir.into();
        Self {
            services: HashMap::new(),
            git_provider: GitProvider::new(repo_url, branch, config_dir.join("repo")),
            config_store: ConfigStore::new(config_dir.join("configs")),
        }
    }

    /// Initialize the registry
    pub async fn init(&mut self) -> Result<()> {
        // Initialize Git repository
        self.git_provider.clone_repo()?;

        // Initialize config store
        self.config_store.init()?;

        // Initial sync of configurations
        self.sync_configurations()?;

        Ok(())
    }

    /// Register a new service
    pub fn register_service(&mut self, service: Service) -> Result<()> {
        // Validate the service configuration
        service.validate()?;

        // Save the configuration
        self.config_store.save_config(&service.name, &service.config)?;

        // Add to registry
        self.services.insert(service.name.clone(), service);

        Ok(())
    }

    /// Get a service by name
    pub fn get_service(&self, name: &str) -> Option<&Service> {
        self.services.get(name)
    }

    /// Get a mutable reference to a service
    pub fn get_service_mut(&mut self, name: &str) -> Option<&mut Service> {
        self.services.get_mut(name)
    }

    /// List all registered services
    pub fn list_services(&self) -> Vec<&Service> {
        self.services.values().collect()
    }

    /// Synchronize configurations with Git repository
    pub fn sync_configurations(&mut self) -> Result<()> {
        // Pull latest changes
        self.git_provider.pull_changes()?;

        // Load all configurations
        let configs = self.config_store.list_configs()?;

        for service_name in configs {
            let config = self.config_store.load_config(&service_name)?;

            match self.services.get_mut(&service_name) {
                Some(service) => {
                    // Update existing service
                    service.update_config(config)?;
                }
                None => {
                    // Create new service
                    let service = Service::new(
                        service_name.clone(),
                        "default".to_string(), // Use default namespace
                        config,
                    );
                    self.services.insert(service_name, service);
                }
            }
        }

        Ok(())
    }

    /// Validate a service's configuration
    pub fn validate_service(&self, name: &str) -> Result<()> {
        let service = self
            .services
            .get(name)
            .ok_or_else(|| AureaCoreError::ConfigStore(format!("Service '{}' not found", name)))?;

        service.validate()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempfile::TempDir;

    use super::*;

    fn create_test_config() -> ServiceConfig {
        ServiceConfig {
            version: "1.0".to_string(),
            parameters: HashMap::new(),
            dependencies: vec![],
        }
    }

    #[tokio::test]
    async fn test_registry_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let mut registry = ServiceRegistry::new(
            "https://example.com/repo.git".to_string(),
            "main".to_string(),
            temp_dir.path(),
        );

        assert!(registry.init().await.is_ok());
    }

    #[test]
    fn test_service_registration() {
        let temp_dir = TempDir::new().unwrap();
        let mut registry = ServiceRegistry::new(
            "https://example.com/repo.git".to_string(),
            "main".to_string(),
            temp_dir.path(),
        );

        let service =
            Service::new("test-service".to_string(), "default".to_string(), create_test_config());

        assert!(registry.register_service(service.clone()).is_ok());
        assert!(registry.get_service("test-service").is_some());
    }

    #[test]
    fn test_service_listing() {
        let temp_dir = TempDir::new().unwrap();
        let mut registry = ServiceRegistry::new(
            "https://example.com/repo.git".to_string(),
            "main".to_string(),
            temp_dir.path(),
        );

        let service1 =
            Service::new("service1".to_string(), "default".to_string(), create_test_config());
        let service2 =
            Service::new("service2".to_string(), "default".to_string(), create_test_config());

        registry.register_service(service1).unwrap();
        registry.register_service(service2).unwrap();

        let services = registry.list_services();
        assert_eq!(services.len(), 2);
    }
}
