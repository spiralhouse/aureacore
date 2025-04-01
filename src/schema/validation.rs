use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use jsonschema::{validator_for, Validator};
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
    schema: Arc<Validator>,
}

impl CompiledSchema {
    /// Creates a new compiled schema
    pub fn new(schema: Validator) -> Self {
        Self { schema: Arc::new(schema) }
    }

    /// Validates a value against the schema
    pub fn validate(&self, value: &serde_json::Value) -> std::result::Result<(), Vec<String>> {
        match self.schema.validate(value) {
            Ok(_) => Ok(()),
            Err(error) => {
                // In the new version, errors are not directly iterable
                // We need to convert a single error to a Vec
                Err(vec![format!("{}", error)])
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

        let schema = validator_for(&schema_value).map_err(|e| {
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

    /// Validates a service based on its specific service type
    fn validate_service_type(&self, service_name: &str, config: &serde_json::Value) -> Vec<String> {
        let mut warnings = Vec::new();

        // Extract service type
        let service_type =
            match config.get("service_type").and_then(|st| st.get("type")).and_then(|t| t.as_str())
            {
                Some(t) => t,
                None => return warnings, // No service type, nothing to validate
            };

        // Validate based on service type
        match service_type {
            "rest" => {
                // Validate REST-specific requirements
                if let Some(endpoints) = config.get("endpoints").and_then(|e| e.as_array()) {
                    for (i, endpoint) in endpoints.iter().enumerate() {
                        // Check if REST endpoints have method specified
                        if endpoint.get("method").is_none() {
                            warnings.push(format!(
                                "Service '{}' is a REST service but endpoint #{} doesn't specify an HTTP method",
                                service_name, i+1
                            ));
                        }
                    }
                }
            }
            "graphql" => {
                // Validate GraphQL-specific requirements
                let has_schema =
                    config.get("metadata").and_then(|m| m.get("graphql_schema")).is_some();

                if !has_schema {
                    warnings.push(format!(
                        "Service '{}' is a GraphQL service but doesn't specify a graphql_schema in metadata",
                        service_name
                    ));
                }
            }
            "grpc" => {
                // Validate gRPC-specific requirements
                let has_proto = config.get("metadata").and_then(|m| m.get("proto_files")).is_some();

                if !has_proto {
                    warnings.push(format!(
                        "Service '{}' is a gRPC service but doesn't specify proto_files in metadata",
                        service_name
                    ));
                }
            }
            "event_driven" => {
                // Validate event-driven service requirements
                let has_topics = config.get("metadata").and_then(|m| m.get("topics")).is_some();

                if !has_topics {
                    warnings.push(format!(
                        "Service '{}' is an event-driven service but doesn't specify topics in metadata",
                        service_name
                    ));
                }
            }
            _ => {
                // For custom types, just verify they have a description
                if config.get("description").is_none() {
                    warnings.push(format!(
                        "Service '{}' uses a custom service type '{}' but doesn't provide a description",
                        service_name, service_type
                    ));
                }
            }
        }

        warnings
    }

    /// Validates a service configuration with context for dependency validation
    /// Returns a tuple of (Result, Vec<Warnings>)
    pub fn validate_service_with_context(
        &mut self,
        service_name: &str,
        config: &serde_json::Value,
        available_services: &HashSet<String>,
    ) -> (Result<()>, Vec<String>) {
        let mut warnings = Vec::new();

        // Extract version from config for compatibility check
        let config_version =
            config.get("schema_version").and_then(|v| v.as_str()).unwrap_or("1.0.0");

        // Check version compatibility
        let compatibility = check_version_compatibility(config_version, CURRENT_SCHEMA_VERSION);

        match compatibility {
            VersionCompatibility::Compatible => {
                // Compatible, proceed with validation
            }
            VersionCompatibility::MinorIncompatible => {
                // Minor incompatibility, add warning but continue
                let warning_msg = format!(
                    "Service '{}' uses schema version {} which has minor differences from the current version {}. Some features may not be validated correctly.",
                    service_name,
                    config_version,
                    CURRENT_SCHEMA_VERSION
                );
                warnings.push(warning_msg);
                // Log the warning
                tracing::warn!(
                    "Minor schema version incompatibility for service '{}': config version {} vs current {}",
                    service_name,
                    config_version,
                    CURRENT_SCHEMA_VERSION
                );
            }
            VersionCompatibility::MajorIncompatible => {
                // Major incompatibility, return error
                return (
                    Err(Error::IncompatibleVersion(format!(
                        "Schema version {} is incompatible with current version {}",
                        config_version, CURRENT_SCHEMA_VERSION
                    ))),
                    warnings,
                );
            }
        }

        // Validate dependencies
        if let Ok(dependency_warnings) =
            self.validate_dependencies(service_name, config, available_services)
        {
            warnings.extend(dependency_warnings);
        }

        // Validate service-specific fields
        warnings.extend(self.validate_service_type(service_name, config));

        // Get schema and perform validation
        let (validation_result, schema_warning) = self.perform_schema_validation(config);

        // If we have a schema warning, add it
        if let Some(warning) = schema_warning {
            warnings.push(warning);
        }

        // Return the result and all warnings
        (validation_result, warnings)
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
        // Now we get two warnings: one for missing service, one for missing HTTP method in REST endpoint
        assert_eq!(warnings.len(), 2);
        // Verify the warnings contain what we expect
        assert!(
            warnings.iter().any(|w| w.contains("missing-service")),
            "Expected warning about missing service"
        );
        assert!(
            warnings.iter().any(|w| w.contains("HTTP method")),
            "Expected warning about missing HTTP method"
        );

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

        // Validation should succeed with warnings for minor version incompatibility
        // and REST endpoint without HTTP method
        assert!(result.is_ok(), "Validation failed: {:?}", result);
        assert_eq!(warnings.len(), 3);
        // Verify we have the expected warnings
        assert!(
            warnings.iter().any(|w| w.contains("minor differences")),
            "Expected warning about minor version differences"
        );
        assert!(
            warnings.iter().any(|w| w.contains("HTTP method")),
            "Expected warning about missing HTTP method"
        );
    }

    #[test]
    fn test_service_type_validation() {
        let mut validator = ValidationService::new();

        // Test REST service without methods
        let service_name = "test-rest-service";
        let config = json!({
            "name": "test-rest-service",
            "version": "1.0.0",
            "service_type": {
                "type": "rest"
            },
            "endpoints": [
                {
                    "name": "api",
                    "path": "/api"
                    // Missing method
                }
            ]
        });

        let (result, warnings) =
            validator.validate_service_with_context(service_name, &config, &HashSet::new());

        // Validation should pass but with warnings
        assert!(result.is_ok(), "REST service validation failed");
        assert!(!warnings.is_empty(), "Expected warnings for REST service without methods");
        assert!(
            warnings.iter().any(|w| w.contains("HTTP method")),
            "Expected warning about missing HTTP method"
        );

        // Test GraphQL service without schema
        let service_name = "test-graphql-service";
        let config = json!({
            "name": "test-graphql-service",
            "version": "1.0.0",
            "service_type": {
                "type": "graphql"
            },
            "endpoints": [
                {
                    "name": "api",
                    "path": "/graphql"
                }
            ]
            // Missing graphql_schema in metadata
        });

        let (result, warnings) =
            validator.validate_service_with_context(service_name, &config, &HashSet::new());

        // Validation should pass but with warnings
        assert!(result.is_ok(), "GraphQL service validation failed");
        assert!(!warnings.is_empty(), "Expected warnings for GraphQL service without schema");
        assert!(
            warnings.iter().any(|w| w.contains("graphql_schema")),
            "Expected warning about missing GraphQL schema"
        );

        // Test service with custom type but missing description
        let service_name = "test-custom-service";
        let config = json!({
            "name": "test-custom-service",
            "version": "1.0.0",
            "service_type": {
                "type": "other",
                "custom_type": "custom-protocol"
            },
            "endpoints": [
                {
                    "name": "api",
                    "path": "/api"
                }
            ]
            // Missing description
        });

        let (result, warnings) =
            validator.validate_service_with_context(service_name, &config, &HashSet::new());

        // Validation should pass but with warnings
        assert!(result.is_ok(), "Custom service validation failed");
        assert!(!warnings.is_empty(), "Expected warnings for custom service without description");
        assert!(
            warnings.iter().any(|w| w.contains("description")),
            "Expected warning about missing description for custom service type"
        );
    }

    #[test]
    fn test_version_compatibility_warnings() {
        let mut validator = ValidationService::new();

        // Configure a different current version to test minor incompatibility
        let service_name = "test-version-service";
        let config = json!({
            "name": "test-version-service",
            "schema_version": "1.1.0", // Minor version difference
            "version": "1.0.0",
            "service_type": {
                "type": "rest"
            },
            "endpoints": [
                {
                    "name": "api",
                    "path": "/api",
                    "method": "GET"
                }
            ]
        });

        // The constant CURRENT_SCHEMA_VERSION is "1.0.0"

        let (result, warnings) =
            validator.validate_service_with_context(service_name, &config, &HashSet::new());

        // Validation should pass with minor version incompatibility warnings
        assert!(result.is_ok(), "Version compatibility validation failed");
        assert!(!warnings.is_empty(), "Expected warnings for minor version incompatibility");
        assert!(
            warnings.iter().any(|w| w.contains("minor differences")),
            "Expected warning about minor version differences"
        );
    }
}
