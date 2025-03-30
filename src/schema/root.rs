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
    use jsonschema::JSONSchema;
    use schemars::schema_for;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_valid_root_config() {
        let schema =
            serde_json::to_value(schema_for!(RootConfig)).expect("Failed to compile schema");
        let validator = JSONSchema::compile(&schema).expect("Failed to compile schema");

        let config = json!({
            "version": "1.0",
            "global": {
                "config_dir": "/etc/aureacore/configs",
                "default_namespace": "default"
            },
            "services": [
                {
                    "name": "test-service",
                    "config_path": "services/test-service/config.yaml",
                    "namespace": "test"
                }
            ]
        });

        let validation = validator.validate(&config);
        assert!(validation.is_ok());
    }

    #[test]
    fn test_invalid_root_config_missing_required() {
        let schema =
            serde_json::to_value(schema_for!(RootConfig)).expect("Failed to compile schema");
        let validator = JSONSchema::compile(&schema).expect("Failed to compile schema");

        let config = json!({
            "version": "1.0",
            "services": []
            // Missing global section
        });

        let validation = validator.validate(&config);
        assert!(validation.is_err(), "Expected validation to fail");
    }

    #[test]
    fn test_invalid_service_ref() {
        let schema =
            serde_json::to_value(schema_for!(RootConfig)).expect("Failed to compile schema");
        let validator = JSONSchema::compile(&schema).expect("Failed to compile schema");

        let config = json!({
            "version": "1.0",
            "global": {
                "config_dir": "/etc/aureacore/configs",
                "default_namespace": "default"
            },
            "services": [
                {
                    "name": "auth-service"
                    // Missing required config_path
                }
            ]
        });

        let validation = validator.validate(&config);
        assert!(validation.is_err(), "Expected validation to fail");
    }
}
