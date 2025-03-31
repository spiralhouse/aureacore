use std::collections::HashMap;
use std::sync::Arc;

use jsonschema::JSONSchema;
use schemars::schema_for;
use semver::Version;

use crate::error::{AureaCoreError as Error, Result};
use crate::schema::service::ServiceSchema;

/// Current schema version used by the system
pub const CURRENT_SCHEMA_VERSION: &str = "1.0.0";

/// Type of schema to validate against
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SchemaType {
    /// Root configuration schema
    Root,
    /// Service configuration schema
    Service,
    /// Custom schema with specified name
    Custom(String),
}

/// Result of version compatibility check
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum VersionCompatibility {
    /// Versions are compatible
    Compatible,
    /// Minor incompatibility (forward-compatible)
    MinorIncompatible,
    /// Major incompatibility (breaking changes)
    MajorIncompatible,
}

/// Checks compatibility between versions (standalone function)
pub fn check_version_compatibility(version: &str, current: &str) -> VersionCompatibility {
    // Parse versions
    let v1 = match Version::parse(version) {
        Ok(v) => v,
        Err(_) => return VersionCompatibility::MajorIncompatible,
    };

    let v2 = match Version::parse(current) {
        Ok(v) => v,
        Err(_) => return VersionCompatibility::MajorIncompatible,
    };

    // Compare major and minor versions
    if v1.major != v2.major {
        VersionCompatibility::MajorIncompatible
    } else if v1.minor != v2.minor {
        VersionCompatibility::MinorIncompatible
    } else {
        VersionCompatibility::Compatible
    }
}

/// A compiled JSON schema validator
#[derive(Clone)]
pub struct CompiledSchema {
    schema: Arc<JSONSchema>,
}

impl CompiledSchema {
    /// Creates a new compiled schema
    pub fn new(schema: JSONSchema) -> Self {
        Self { schema: Arc::new(schema) }
    }

    /// Validates a value against the schema
    pub fn validate(&self, value: &serde_json::Value) -> std::result::Result<(), Vec<String>> {
        match self.schema.validate(value) {
            Ok(_) => Ok(()),
            Err(errors) => {
                let error_strings: Vec<String> = errors.map(|err| format!("{}", err)).collect();
                Err(error_strings)
            }
        }
    }
}

/// Service for validating configuration against schemas
pub struct ValidationService {
    schema_cache: HashMap<SchemaType, CompiledSchema>,
}

impl Default for ValidationService {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationService {
    /// Creates a new validation service
    pub fn new() -> Self {
        Self { schema_cache: HashMap::new() }
    }

    /// Gets or compiles a schema of the specified type
    pub fn get_or_compile_schema(&mut self, schema_type: SchemaType) -> Result<&CompiledSchema> {
        if !self.schema_cache.contains_key(&schema_type) {
            let compiled = self.compile_schema(&schema_type)?;
            self.schema_cache.insert(schema_type.clone(), compiled);
        }

        Ok(self.schema_cache.get(&schema_type).unwrap())
    }

    /// Compiles a schema of the specified type
    pub fn compile_schema(&self, schema_type: &SchemaType) -> Result<CompiledSchema> {
        let schema_value = match schema_type {
            SchemaType::Service => {
                serde_json::to_value(schema_for!(ServiceSchema)).map_err(|e| {
                    Error::SchemaCompilationError(format!("Failed to generate schema: {}", e))
                })?
            }
            SchemaType::Root => {
                // Root schema will be implemented later
                return Err(Error::NotImplemented("Root schema not yet implemented".to_string()));
            }
            SchemaType::Custom(name) => {
                return Err(Error::NotImplemented(format!(
                    "Custom schema {} not implemented",
                    name
                )));
            }
        };

        let schema = JSONSchema::compile(&schema_value).map_err(|e| {
            Error::SchemaCompilationError(format!("Failed to compile schema: {}", e))
        })?;

        Ok(CompiledSchema::new(schema))
    }

    /// Validates a service configuration
    pub fn validate_service(&mut self, config: &serde_json::Value) -> Result<()> {
        // Get the service schema
        let schema = self.get_or_compile_schema(SchemaType::Service)?;

        // Extract version from config for compatibility check
        let config_version =
            config.get("schema_version").and_then(|v| v.as_str()).unwrap_or("1.0.0");

        // Check using the standalone function
        let compatibility = check_version_compatibility(config_version, CURRENT_SCHEMA_VERSION);

        match compatibility {
            VersionCompatibility::Compatible => {
                // Perform validation
                match schema.validate(config) {
                    Ok(_) => Ok(()),
                    Err(errors) => Err(Error::ValidationError(format!(
                        "Schema validation failed: {}",
                        errors.join(", ")
                    ))),
                }
            }
            VersionCompatibility::MinorIncompatible => {
                // Log warning but continue with validation
                tracing::warn!(
                    "Minor schema version incompatibility: config version {} vs current {}",
                    config_version,
                    CURRENT_SCHEMA_VERSION
                );

                match schema.validate(config) {
                    Ok(_) => Ok(()),
                    Err(errors) => Err(Error::ValidationError(format!(
                        "Schema validation failed: {}",
                        errors.join(", ")
                    ))),
                }
            }
            VersionCompatibility::MajorIncompatible => Err(Error::IncompatibleVersion(format!(
                "Schema version {} is incompatible with current version {}",
                config_version, CURRENT_SCHEMA_VERSION
            ))),
        }
    }

    /// Checks compatibility between versions
    pub fn check_version_compatibility(
        &self,
        version: &str,
        current: &str,
    ) -> VersionCompatibility {
        // Parse versions
        let v1 = match Version::parse(version) {
            Ok(v) => v,
            Err(_) => return VersionCompatibility::MajorIncompatible,
        };

        let v2 = match Version::parse(current) {
            Ok(v) => v,
            Err(_) => return VersionCompatibility::MajorIncompatible,
        };

        // Compare major and minor versions
        if v1.major != v2.major {
            VersionCompatibility::MajorIncompatible
        } else if v1.minor != v2.minor {
            VersionCompatibility::MinorIncompatible
        } else {
            VersionCompatibility::Compatible
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_version_compatibility() {
        let service = ValidationService::new();

        // Compatible versions
        assert_eq!(
            service.check_version_compatibility("1.0.0", "1.0.1"),
            VersionCompatibility::Compatible
        );

        // Minor incompatible
        assert_eq!(
            service.check_version_compatibility("1.0.0", "1.1.0"),
            VersionCompatibility::MinorIncompatible
        );

        // Major incompatible
        assert_eq!(
            service.check_version_compatibility("1.0.0", "2.0.0"),
            VersionCompatibility::MajorIncompatible
        );

        // Invalid version
        assert_eq!(
            service.check_version_compatibility("invalid", "1.0.0"),
            VersionCompatibility::MajorIncompatible
        );
    }

    #[test]
    fn test_schema_compilation() {
        let mut service = ValidationService::new();

        // Service schema should compile successfully
        let schema_result = service.get_or_compile_schema(SchemaType::Service);
        assert!(schema_result.is_ok());
    }

    #[test]
    fn test_service_validation_success() {
        let mut service = ValidationService::new();

        let config = json!({
            "name": "test-service",
            "version": "1.0.0",
            "service_type": {
                "type": "rest"
            },
            "endpoints": [
                {
                    "name": "test",
                    "path": "/test"
                }
            ],
            "schema_version": "1.0.0"
        });

        let result = service.validate_service(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_service_validation_failure() {
        let mut service = ValidationService::new();

        // Missing required fields
        let config = json!({
            "name": "test-service",
            "version": "1.0.0"
            // Missing service_type and endpoints
        });

        let result = service.validate_service(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(Error::ValidationError(_))));
    }

    #[test]
    fn test_service_validation_incompatible_version() {
        let mut service = ValidationService::new();

        let config = json!({
            "name": "test-service",
            "version": "1.0.0",
            "service_type": {
                "type": "rest"
            },
            "endpoints": [],
            "schema_version": "2.0.0" // Major version incompatibility
        });

        let result = service.validate_service(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(Error::IncompatibleVersion(_))));
    }
}
