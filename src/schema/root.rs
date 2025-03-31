use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Root configuration schema for AureaCore
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RootConfig {
    /// Version of the configuration schema
    pub version: String,
    /// Global settings that apply to all services
    pub global: GlobalConfig,
    /// List of services managed by AureaCore
    pub services: Vec<ServiceRef>,
}

/// Global configuration settings
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GlobalConfig {
    /// Base directory for service configurations
    pub config_dir: String,
    /// Default namespace for services
    pub default_namespace: String,
}

/// Reference to a service configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServiceRef {
    /// Name of the service
    pub name: String,
    /// Path to the service configuration file, relative to config_dir
    pub config_path: String,
    /// Optional namespace override for the service
    pub namespace: Option<String>,
}

#[cfg(test)]
mod tests {
    use jsonschema::validator_for;
    use schemars::schema_for;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_valid_root_config() {
        let schema =
            serde_json::to_value(schema_for!(RootConfig)).expect("Failed to compile schema");
        let validator = validator_for(&schema).expect("Failed to compile schema");

        let config = json!({
            "version": "1.0.0",
            "global": {
                "config_dir": "/etc/aureacore/configs",
                "default_namespace": "default"
            },
            "services": [
                {
                    "name": "service1",
                    "config_path": "services/service1/config.yaml",
                    "namespace": "test"
                }
            ]
        });

        let result = validator.validate(&config);
        assert!(result.is_ok(), "Validation failed: {:?}", result);
    }

    #[test]
    fn test_invalid_root_config_missing_required() {
        let schema =
            serde_json::to_value(schema_for!(RootConfig)).expect("Failed to compile schema");
        let validator = validator_for(&schema).expect("Failed to compile schema");

        let config = json!({
            "version": "1.0.0",
            // Missing global section
            "services": []
        });

        let result = validator.validate(&config);
        assert!(result.is_err(), "Validation should fail");
    }

    #[test]
    fn test_invalid_service_ref() {
        let schema =
            serde_json::to_value(schema_for!(RootConfig)).expect("Failed to compile schema");
        let validator = validator_for(&schema).expect("Failed to compile schema");

        let config = json!({
            "version": "1.0.0",
            "global": {
                "config_dir": "/etc/aureacore/configs",
                "default_namespace": "default"
            },
            "services": [
                {
                    // Missing name
                    "config_path": "services/service1/config.yaml",
                    "namespace": "test"
                }
            ]
        });

        let result = validator.validate(&config);
        assert!(result.is_err(), "Validation should fail");
    }
}
