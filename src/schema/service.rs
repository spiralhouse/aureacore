use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Schema for a service configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServiceSchema {
    /// Name of the service
    pub name: String,
    /// Version of the service
    pub version: String,
    /// Description of the service
    pub description: Option<String>,
    /// Owner of the service
    pub owner: Option<String>,
    /// Documentation URL for the service
    pub documentation_url: Option<String>,
    /// Service type
    pub service_type: ServiceType,
    /// Service endpoints
    pub endpoints: Vec<Endpoint>,
    /// Dependencies on other services
    pub dependencies: Option<Vec<Dependency>>,
    /// Extensible metadata for additional attributes
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of services
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "custom_type")]
pub enum ServiceType {
    /// HTTP/REST service
    Rest,
    /// gRPC service
    Grpc,
    /// GraphQL service
    GraphQL,
    /// Event-driven service
    EventDriven,
    /// Other type of service
    #[serde(rename = "other")]
    Other(String),
}

/// Endpoint definition
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Endpoint {
    /// Name of the endpoint
    pub name: String,
    /// Path or address of the endpoint
    pub path: String,
    /// Method or protocol for the endpoint
    pub method: Option<String>,
    /// Documentation about the endpoint
    pub description: Option<String>,
}

/// Dependency on another service
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Dependency {
    /// Name of the service dependency
    pub service: String,
    /// Version constraint for the dependency
    pub version_constraint: Option<String>,
    /// Whether this dependency is required
    #[serde(default = "default_true")]
    pub required: bool,
}

/// Default function to set dependency as required by default
fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use jsonschema::JSONSchema;
    use schemars::schema_for;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_valid_service_schema() {
        let schema = serde_json::to_value(schema_for!(ServiceSchema)).unwrap();
        let validator = JSONSchema::compile(&schema).unwrap();

        let config = json!({
            "name": "auth-service",
            "version": "1.0.0",
            "description": "Authentication service",
            "owner": "Platform Team",
            "documentation_url": "https://docs.example.com/auth",
            "service_type": {
                "type": "rest"
            },
            "endpoints": [
                {
                    "name": "login",
                    "path": "/api/v1/login",
                    "method": "POST",
                    "description": "User login endpoint"
                }
            ],
            "dependencies": [
                {
                    "service": "user-service",
                    "version_constraint": ">=1.0.0",
                    "required": true
                }
            ],
            "metadata": {
                "team_slack_channel": "#auth-team",
                "priority": 1
            }
        });

        let validation = validator.validate(&config);
        assert!(validation.is_ok(), "Validation failed");
    }

    #[test]
    fn test_invalid_service_schema() {
        let schema = serde_json::to_value(schema_for!(ServiceSchema)).unwrap();
        let validator = JSONSchema::compile(&schema).unwrap();

        let config = json!({
            "name": "auth-service",
            "version": "1.0.0",
            // Missing required field "service_type"
            "endpoints": []
        });

        let validation = validator.validate(&config);
        assert!(validation.is_err(), "Expected validation to fail");
    }

    #[test]
    fn test_service_schema_with_custom_service_type() {
        let schema = serde_json::to_value(schema_for!(ServiceSchema)).unwrap();
        let validator = JSONSchema::compile(&schema).unwrap();

        let config = json!({
            "name": "legacy-service",
            "version": "1.0.0",
            "service_type": {
                "type": "other",
                "custom_type": "legacy-soap"
            },
            "endpoints": [
                {
                    "name": "get-data",
                    "path": "/data"
                }
            ]
        });

        let validation = validator.validate(&config);
        assert!(validation.is_ok(), "Validation failed");
    }

    #[test]
    fn test_service_schema_with_metadata() {
        let schema = serde_json::to_value(schema_for!(ServiceSchema)).unwrap();
        let validator = JSONSchema::compile(&schema).unwrap();

        let config = json!({
            "name": "metrics-service",
            "version": "1.0.0",
            "service_type": {
                "type": "rest"
            },
            "endpoints": [],
            "metadata": {
                "team": "Platform",
                "priority": 2,
                "tags": ["metrics", "monitoring"],
                "config": {
                    "retention_days": 30,
                    "sampling_rate": 0.1
                }
            }
        });

        let validation = validator.validate(&config);
        assert!(validation.is_ok(), "Validation failed");
    }
}
