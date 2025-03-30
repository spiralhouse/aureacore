use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AureaCoreError, Result};

/// Manages service configuration storage
pub struct ConfigStore {
    /// Base directory for configuration files
    config_dir: PathBuf,
}

impl ConfigStore {
    /// Creates a new ConfigStore instance
    pub fn new(config_dir: impl Into<PathBuf>) -> Result<Self> {
        let config_dir = config_dir.into();
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).map_err(|e| {
                AureaCoreError::Config(format!("Failed to create config directory: {}", e))
            })?;
        }
        Ok(Self { config_dir })
    }

    /// Loads a configuration file
    pub fn load_config(&self, path: impl AsRef<Path>) -> Result<String> {
        let path = self.config_dir.join(path);
        if !path.exists() {
            return Err(AureaCoreError::Config(format!(
                "Configuration file not found: {}",
                path.display()
            )));
        }

        fs::read_to_string(&path).map_err(|e| {
            AureaCoreError::Config(format!(
                "Failed to read configuration file {}: {}",
                path.display(),
                e
            ))
        })
    }

    /// Saves a configuration file
    pub fn save_config(&self, path: impl AsRef<Path>, content: &str) -> Result<()> {
        let path = self.config_dir.join(path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| {
                    AureaCoreError::Config(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }
        }

        fs::write(&path, content).map_err(|e| {
            AureaCoreError::Config(format!(
                "Failed to write configuration file {}: {}",
                path.display(),
                e
            ))
        })
    }

    /// Lists all configuration files
    pub fn list_configs(&self) -> Result<Vec<PathBuf>> {
        let mut configs = Vec::new();
        let dir = fs::read_dir(&self.config_dir).map_err(|e| {
            AureaCoreError::Config(format!("Failed to read config directory: {}", e))
        })?;

        for entry in dir {
            let entry = entry.map_err(|e| {
                AureaCoreError::Config(format!("Failed to read directory entry: {}", e))
            })?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                configs.push(path.strip_prefix(&self.config_dir).unwrap().to_path_buf());
            }
        }

        Ok(configs)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_config_store_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let store = ConfigStore::new(temp_dir.path()).unwrap();
        assert!(store.config_dir.exists());
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let store = ConfigStore::new(temp_dir.path()).unwrap();
        let config_path = PathBuf::from("test/config.json");
        let content = "test: value";

        store.save_config(&config_path, content).unwrap();
        let loaded = store.load_config(&config_path).unwrap();
        assert_eq!(loaded, content);
    }

    #[test]
    fn test_load_nonexistent_config() {
        let temp_dir = TempDir::new().unwrap();
        let store = ConfigStore::new(temp_dir.path()).unwrap();
        let result = store.load_config("nonexistent.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_configs() {
        let temp_dir = TempDir::new().unwrap();
        let store = ConfigStore::new(temp_dir.path()).unwrap();
        let config1 = PathBuf::from("test1.json");
        let config2 = PathBuf::from("test2.json");

        store.save_config(&config1, "test1").unwrap();
        store.save_config(&config2, "test2").unwrap();

        let configs = store.list_configs().unwrap();
        assert_eq!(configs.len(), 2);
        assert!(configs.contains(&config1));
        assert!(configs.contains(&config2));
    }
}
