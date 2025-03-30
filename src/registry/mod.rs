mod git;
mod service;
mod store;

use std::collections::HashMap;
use std::path::PathBuf;

pub use service::{Service, ServiceConfig, ServiceStatus};

use crate::error::{AureaCoreError, Result};
use crate::registry::git::GitProvider;
use crate::registry::store::ConfigStore;

/// Manages service configurations and their storage
pub struct ServiceRegistry {
    /// Map of service name to service instance
    services: HashMap<String, Service>,
    /// Configuration store for local files
    config_store: ConfigStore,
    /// Git provider for configuration management
    git_provider: GitProvider,
}

impl ServiceRegistry {
    /// Creates a new service registry instance
    pub fn new(repo_url: String, branch: String, work_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            git_provider: GitProvider::new(repo_url, branch, work_dir.clone()),
            config_store: ConfigStore::new(work_dir)?,
            services: HashMap::new(),
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
        let service = Service::new(name.to_string(), service_config);
        self.services.insert(name.to_string(), service);

        Ok(())
    }

    /// Gets a service by name
    pub fn get_service(&self, name: &str) -> Result<&Service> {
        self.services
            .get(name)
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
}

#[cfg(test)]
mod tests {
    use std::fs;

    use git2::build::CheckoutBuilder;
    use git2::Repository;
    use tempfile::TempDir;

    use super::*;

    fn setup_test_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("test-repo");
        let repo = Repository::init(&repo_path).unwrap();

        // Create an initial commit
        fs::create_dir_all(&repo_path).unwrap();
        let readme_path = repo_path.join("README.md");
        fs::write(&readme_path, "# Test Repository").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("README.md")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("test", "test@example.com").unwrap();

        // Create initial commit
        repo.commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[]).unwrap();

        // Create main branch
        let mut checkout = CheckoutBuilder::new();
        checkout.force();
        repo.checkout_head(Some(&mut checkout)).unwrap();

        // Set HEAD to refs/heads/main
        repo.set_head("refs/heads/main").unwrap();

        (temp_dir, repo_path)
    }

    #[test]
    fn test_registry_initialization() {
        let (_temp_dir, repo_path) = setup_test_repo();
        let work_dir = repo_path.parent().unwrap().join("work-dir");
        let mut registry = ServiceRegistry::new(
            repo_path.to_str().unwrap().to_string(),
            "main".to_string(),
            work_dir.clone(),
        )
        .unwrap();

        let result = registry.init();
        assert!(result.is_ok());
        assert!(work_dir.join(".git").exists());

        // Test service registration
        let test_config = r#"{"namespace": "test", "config_path": "test/config.yaml"}"#;
        let register_result = registry.register_service("test-service", test_config);
        assert!(register_result.is_ok());

        // Test service retrieval
        let service = registry.get_service("test-service").unwrap();
        assert_eq!(service.name, "test-service");
        assert_eq!(service.config.namespace, Some("test".to_string()));

        // Test service listing
        let services = registry.list_services().unwrap();
        assert!(!services.is_empty());
        assert!(services.contains(&"test-service".to_string()));
    }
}
