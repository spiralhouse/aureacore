use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AureaCoreError, Result};
use crate::registry::service::ServiceConfig;

/// Manages the local storage of service configurations
pub struct ConfigStore {
    /// Base directory for configuration files
    base_path: PathBuf,
}

impl ConfigStore {
    /// Create a new ConfigStore with the given base path
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self { base_path: base_path.into() }
    }

    /// Initialize the configuration store
    pub fn init(&self) -> Result<()> {
        if !self.base_path.exists() {
            fs::create_dir_all(&self.base_path).map_err(|e| {
                AureaCoreError::ConfigStore(format!("Failed to create config directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Load a service configuration
    pub fn load_config(&self, service_name: &str) -> Result<ServiceConfig> {
        let config_path = self.get_config_path(service_name);

        let config_str = fs::read_to_string(&config_path).map_err(|e| {
            AureaCoreError::ConfigStore(format!(
                "Failed to read config file for service '{}': {}",
                service_name, e
            ))
        })?;

        serde_yaml::from_str(&config_str).map_err(|e| {
            AureaCoreError::ConfigStore(format!(
                "Failed to parse config for service '{}': {}",
                service_name, e
            ))
        })
    }

    /// Save a service configuration
    pub fn save_config(&self, service_name: &str, config: &ServiceConfig) -> Result<()> {
        let config_path = self.get_config_path(service_name);

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AureaCoreError::ConfigStore(format!(
                    "Failed to create directory for service '{}': {}",
                    service_name, e
                ))
            })?;
        }

        let config_str = serde_yaml::to_string(config).map_err(|e| {
            AureaCoreError::ConfigStore(format!(
                "Failed to serialize config for service '{}': {}",
                service_name, e
            ))
        })?;

        fs::write(&config_path, config_str).map_err(|e| {
            AureaCoreError::ConfigStore(format!(
                "Failed to write config file for service '{}': {}",
                service_name, e
            ))
        })
    }

    /// List all configuration files
    pub fn list_configs(&self) -> Result<Vec<String>> {
        let mut configs = Vec::new();

        if !self.base_path.exists() {
            return Ok(configs);
        }

        for entry in fs::read_dir(&self.base_path).map_err(|e| {
            AureaCoreError::ConfigStore(format!("Failed to read config directory: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                AureaCoreError::ConfigStore(format!("Failed to read directory entry: {}", e))
            })?;

            if entry.path().extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                if let Some(name) = entry.path().file_stem() {
                    if let Some(name_str) = name.to_str() {
                        configs.push(name_str.to_string());
                    }
                }
            }
        }

        Ok(configs)
    }

    /// Get the full path for a service's configuration file
    fn get_config_path(&self, service_name: &str) -> PathBuf {
        self.base_path.join(format!("{}.yaml", service_name))
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

    #[test]
    fn test_config_store_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let store = ConfigStore::new(temp_dir.path());

        assert!(store.init().is_ok());
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let store = ConfigStore::new(temp_dir.path());
        store.init().unwrap();

        let config = create_test_config();
        assert!(store.save_config("test-service", &config).is_ok());

        let loaded_config = store.load_config("test-service");
        assert!(loaded_config.is_ok());

        let loaded_config = loaded_config.unwrap();
        assert_eq!(loaded_config.version, "1.0");
    }

    #[test]
    fn test_list_configs() {
        let temp_dir = TempDir::new().unwrap();
        let store = ConfigStore::new(temp_dir.path());
        store.init().unwrap();

        let config = create_test_config();
        store.save_config("service1", &config).unwrap();
        store.save_config("service2", &config).unwrap();

        let configs = store.list_configs().unwrap();
        assert_eq!(configs.len(), 2);
        assert!(configs.contains(&"service1".to_string()));
        assert!(configs.contains(&"service2".to_string()));
    }

    #[test]
    fn test_load_nonexistent_config() {
        let temp_dir = TempDir::new().unwrap();
        let store = ConfigStore::new(temp_dir.path());
        store.init().unwrap();

        let result = store.load_config("nonexistent");
        assert!(result.is_err());
    }
}
