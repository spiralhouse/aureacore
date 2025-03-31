use std::collections::{HashMap, HashSet};
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

    /// Validates dependencies between services
    pub fn validate_dependencies(
        &self,
        service_name: &str,
        config: &serde_json::Value,
        available_services: &HashSet<String>,
    ) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Extract dependencies from the configuration
        if let Some(dependencies) = config.get("dependencies").and_then(|d| d.as_array()) {
            for dep in dependencies {
                if let Some(name) = dep.get("service").and_then(|n| n.as_str()) {
                    if !available_services.contains(name) {
                        let warning = format!(
                            "Service '{}' depends on '{}', which is not registered in the catalog",
                            service_name, name
                        );
                        warnings.push(warning);
                    }
                }
            }
        }

        Ok(warnings)
    }

    /// Gets schema and performs validation with version compatibility check
    fn perform_schema_validation(
        &mut self,
        config: &serde_json::Value,
    ) -> (Result<()>, Option<String>) {
        // Get the service schema
        let schema_result = self.get_or_compile_schema(SchemaType::Service);
        if let Err(err) = schema_result {
            return (Err(err), None);
        }
        let schema = schema_result.unwrap();

        // Extract version from config for compatibility check
        let config_version =
            config.get("schema_version").and_then(|v| v.as_str()).unwrap_or("1.0.0");

        // Check version compatibility
        let compatibility = check_version_compatibility(config_version, CURRENT_SCHEMA_VERSION);

        match compatibility {
            VersionCompatibility::Compatible => {
                // Perform validation
                match schema.validate(config) {
                    Ok(_) => (Ok(()), None),
                    Err(errors) => (
                        Err(Error::ValidationError(format!(
                            "Schema validation failed: {}",
                            errors.join(", ")
                        ))),
                        None,
                    ),
                }
            }
            VersionCompatibility::MinorIncompatible => {
                // Create warning but continue with validation
                let warning = format!(
                    "Minor schema version incompatibility: config version {} vs current {}",
                    config_version, CURRENT_SCHEMA_VERSION
                );

                match schema.validate(config) {
                    Ok(_) => (Ok(()), Some(warning)),
                    Err(errors) => (
                        Err(Error::ValidationError(format!(
                            "Schema validation failed: {}",
                            errors.join(", ")
                        ))),
                        None,
                    ),
                }
            }
            VersionCompatibility::MajorIncompatible => (
                Err(Error::IncompatibleVersion(format!(
                    "Schema version {} is incompatible with current version {}",
                    config_version, CURRENT_SCHEMA_VERSION
                ))),
                None,
            ),
        }
    }

    /// Validates a service configuration with additional context
    pub fn validate_service_with_context(
        &mut self,
        service_name: &str,
        config: &serde_json::Value,
        available_services: &HashSet<String>,
    ) -> (Result<()>, Vec<String>) {
        // First, validate dependencies which uses &self (immutable borrow)
        let dependency_warnings = self
            .validate_dependencies(service_name, config, available_services)
            .unwrap_or_default();

        // Next, perform schema validation which uses &mut self
        let (validation_result, version_warning) = self.perform_schema_validation(config);

        // Combine warnings
        let mut all_warnings = dependency_warnings;
        if let Some(warning) = version_warning {
            all_warnings.push(warning);
        }

        (validation_result, all_warnings)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

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
            "schema_version": "1.0.0",
            "service_type": {
                "type": "rest"
            },
            "endpoints": [
                {
                    "name": "api",
                    "path": "/api"
                }
            ],
            "metadata": {
                "owner": "Test Team",
                "description": "Test service"
            }
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
            "schema_version": "2.0.0", // Major version incompatibility
            "service_type": {
                "type": "rest"
            },
            "endpoints": [],
            "metadata": {
                "owner": "Test Team",
                "description": "Test service"
            }
        });

        let result = service.validate_service(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(Error::IncompatibleVersion(_))));
    }

    #[test]
    fn test_dependency_validation() {
        let service = ValidationService::new();

        // Create a service with dependencies
        let config = json!({
            "name": "test-service",
            "schema_version": "1.0.0",
            "dependencies": [
                {"service": "existing-service", "version_constraint": "1.0.0"},
                {"service": "missing-service", "version_constraint": "1.0.0"}
            ]
        });

        // Create a set of available services
        let mut available_services = HashSet::new();
        available_services.insert("existing-service".to_string());
        available_services.insert("another-service".to_string());

        // Validate dependencies
        let warnings =
            service.validate_dependencies("test-service", &config, &available_services).unwrap();

        // Should have one warning for the missing service
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("missing-service"));

        // Add all required services
        available_services.insert("missing-service".to_string());

        // Validate again
        let warnings =
            service.validate_dependencies("test-service", &config, &available_services).unwrap();

        // Should have no warnings now
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_validate_service_with_context() {
        let mut service = ValidationService::new();

        // Create a valid service with dependencies that matches the schema
        let config = json!({
            "name": "test-service",
            "version": "1.0.0",
            "schema_version": "1.0.0",
            "service_type": {
                "type": "rest"
            },
            "endpoints": [
                {
                    "name": "api",
                    "path": "/api"
                }
            ],
            "metadata": {
                "owner": "Test Team",
                "description": "Test service"
            },
            "dependencies": [
                {
                    "service": "existing-service",
                    "version_constraint": "1.0.0"
                },
                {
                    "service": "missing-service",
                    "version_constraint": "1.0.0"
                }
            ]
        });

        // Create a set of available services
        let mut available_services = HashSet::new();
        available_services.insert("existing-service".to_string());

        // Validate with context
        let (result, warnings) =
            service.validate_service_with_context("test-service", &config, &available_services);

        // Validation should succeed but with warnings
        assert!(result.is_ok(), "Validation failed: {:?}", result);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("missing-service"));

        // Test with incompatible version
        let incompatible_config = json!({
            "name": "test-service",
            "version": "1.0.0",
            "schema_version": "2.0.0",  // Major version incompatibility
            "service_type": {
                "type": "rest"
            },
            "endpoints": [
                {
                    "name": "api",
                    "path": "/api"
                }
            ],
            "metadata": {
                "owner": "Test Team",
                "description": "Test service"
            }
        });

        let (result, warnings) = service.validate_service_with_context(
            "test-service",
            &incompatible_config,
            &available_services,
        );

        // Validation should fail due to version incompatibility
        assert!(result.is_err());
        assert_eq!(warnings.len(), 0);

        // Test with minor incompatible version
        let minor_incompatible_config = json!({
            "name": "test-service",
            "version": "1.0.0",
            "schema_version": "1.1.0",  // Minor version incompatibility
            "service_type": {
                "type": "rest"
            },
            "endpoints": [
                {
                    "name": "api",
                    "path": "/api"
                }
            ],
            "metadata": {
                "owner": "Test Team",
                "description": "Test service"
            }
        });

        let (result, warnings) = service.validate_service_with_context(
            "test-service",
            &minor_incompatible_config,
            &available_services,
        );

        // Validation should succeed with warning for minor version incompatibility
        assert!(result.is_ok(), "Validation failed: {:?}", result);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Minor schema version incompatibility"));
    }
}
