use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use aureacore::error::Result;
use aureacore::registry::{Service, ServiceRegistry, ServiceState, ServiceStatus};
use aureacore::schema::service::Dependency;
use serde_json::json;

/// Mock implementation of ServiceRegistry for testing without file IO
struct MockRegistry {
    services: HashMap<String, aureacore::registry::Service>,
}

impl MockRegistry {
    fn new() -> Self {
        Self { services: HashMap::new() }
    }

    fn register_service(&mut self, name: &str, config_json: &str) -> Result<()> {
        // Parse config JSON
        let config = serde_json::from_str(config_json).map_err(|e| {
            aureacore::error::AureaCoreError::Config(format!("Invalid config: {}", e))
        })?;

        // Create service
        let mut service = aureacore::registry::Service::new(name.to_string(), config);

        // Parse the full JSON to get schema data
        let schema_data: serde_json::Value = serde_json::from_str(config_json).map_err(|e| {
            aureacore::error::AureaCoreError::Config(format!("Invalid JSON: {}", e))
        })?;

        // Set schema data directly
        service.schema_data = Some(schema_data);

        // Add service to registry
        self.services.insert(name.to_string(), service);

        Ok(())
    }

    fn get_service(&self, name: &str) -> Result<&aureacore::registry::Service> {
        self.services.get(name).ok_or_else(|| {
            aureacore::error::AureaCoreError::Config(format!("Service '{}' not found", name))
        })
    }

    fn validate_all_services(&mut self) -> Result<aureacore::registry::ValidationSummary> {
        // Create validation summary
        let mut summary = aureacore::registry::ValidationSummary::new();

        // Build dependency graph
        let mut dep_graph = aureacore::registry::DependencyGraph::new();

        // Add all services to the graph
        for service_name in self.services.keys() {
            dep_graph.add_node(service_name.clone());
        }

        // Add dependencies as edges
        for (service_name, service) in &self.services {
            if let Some(dependencies) = &service.config.dependencies {
                for dependency in dependencies {
                    if self.services.contains_key(&dependency.service) {
                        let metadata = aureacore::registry::EdgeMetadata {
                            required: dependency.required,
                            version_constraint: dependency.version_constraint.clone(),
                        };
                        dep_graph.add_edge(
                            service_name.clone(),
                            dependency.service.clone(),
                            metadata,
                        );
                    }
                }
            }
        }

        // Check for circular dependencies
        if let Some(cycle) = dep_graph.detect_cycles() {
            summary.add_warning(
                "system".to_string(),
                format!("Circular dependency detected: {}", cycle.description),
            );
        }

        // Validate dependencies
        let mut services_with_errors = Vec::new();
        let mut dependency_warnings = HashMap::new();

        for (service_name, service) in &self.services {
            let mut service_warnings = Vec::new();
            let mut has_critical_error = false;
            let mut error_message = String::new();

            if let Some(dependencies) = &service.config.dependencies {
                for dep in dependencies {
                    let dep_name = &dep.service;

                    // Check if dependency exists
                    if let Some(dep_service) = self.services.get(dep_name) {
                        // Skip version check if no constraint provided
                        if let Some(version_constraint) = &dep.version_constraint {
                            // Get version from schema data
                            if let Some(schema) = &dep_service.schema_data {
                                if let Some(version) =
                                    schema.get("version").and_then(|v| v.as_str())
                                {
                                    // Create a validation service to check version compatibility
                                    let validation_service =
                                        aureacore::schema::validation::ValidationService::new();
                                    let compatibility = validation_service
                                        .check_version_compatibility(version, version_constraint);

                                    match compatibility {
                                        aureacore::schema::validation::VersionCompatibility::Compatible => {
                                            // Compatible - no warning needed
                                        },
                                        aureacore::schema::validation::VersionCompatibility::MinorIncompatible => {
                                            // Add a warning for minor incompatibility
                                            service_warnings.push(format!(
                                                "Minor version incompatibility for dependency '{}': expected {} but found {}",
                                                dep_name, version_constraint, version
                                            ));
                                        },
                                        aureacore::schema::validation::VersionCompatibility::MajorIncompatible => {
                                            let msg = format!(
                                                "Major version incompatibility for dependency '{}': expected {} but found {}",
                                                dep_name, version_constraint, version
                                            );
                                            if dep.required {
                                                // Critical error for required dependency
                                                has_critical_error = true;
                                                error_message = msg.clone();
                                                summary.failed.push((service_name.clone(), msg));
                                            } else {
                                                // Warning for optional dependency
                                                service_warnings.push(format!(
                                                    "Optional dependency '{}' has incompatible version: {}",
                                                    dep_name, msg
                                                ));
                                            }
                                        }
                                    }
                                } else {
                                    service_warnings.push(format!(
                                        "Dependency '{}' has missing or invalid version in schema",
                                        dep_name
                                    ));
                                }
                            }
                        }
                    } else {
                        // Dependency not found
                        if dep.required {
                            let msg = format!("Required dependency '{}' not found", dep_name);
                            has_critical_error = true;
                            error_message = msg.clone();
                            summary.failed.push((service_name.clone(), msg));
                        } else {
                            service_warnings
                                .push(format!("Optional dependency '{}' not found", dep_name));
                        }
                    }
                }
            }

            // Add warnings for this service if any
            if !service_warnings.is_empty() {
                dependency_warnings.insert(service_name.clone(), service_warnings);
            }

            // Collect services with critical errors for status updates
            if has_critical_error {
                services_with_errors.push((service_name.clone(), error_message));
            } else {
                // If no errors, mark as successful
                summary.successful.push(service_name.clone());
            }
        }

        // Update service statuses for services with errors
        for (service_name, error_message) in &services_with_errors {
            if let Some(service) = self.services.get_mut(service_name) {
                service.status =
                    ServiceStatus::new(ServiceState::Error).with_error(error_message.clone());
            }
        }

        // Create a HashSet of service names with errors for quick lookups
        let services_with_errors_set: std::collections::HashSet<String> =
            services_with_errors.iter().map(|(name, _)| name.clone()).collect();

        // Set remaining services as active
        for (name, service) in &mut self.services {
            if !services_with_errors_set.contains(name) {
                service.status = ServiceStatus::new(ServiceState::Active);
            }
        }

        // Merge dependency warnings into the summary
        for (service_name, warnings) in dependency_warnings {
            for warning in warnings {
                summary.add_warning(service_name.clone(), warning);
            }
        }

        Ok(summary)
    }
}

/// Helper function to create a test service configuration
fn create_test_config(
    name: &str,
    version: &str,
    dependencies: Option<Vec<Dependency>>,
) -> serde_json::Value {
    let deps = dependencies.map(|deps| {
        deps.into_iter()
            .map(|dep| {
                json!({
                    "service": dep.service,
                    "version_constraint": dep.version_constraint,
                    "required": dep.required
                })
            })
            .collect::<Vec<serde_json::Value>>()
    });

    let mut config = json!({
        "name": name,
        "version": version,
        "namespace": "test",
        "config_path": format!("/tmp/{}.json", name),
        "schema_version": "1.0.0",
        "service_type": { "type": "rest" },
        "endpoints": []
    });

    if let Some(deps) = deps {
        config["dependencies"] = json!(deps);
    }

    config
}

#[test]
fn test_registry_dependency_validation() {
    // Create a mock registry
    let mut registry = MockRegistry::new();

    // Register services with dependencies
    let service_a = create_test_config(
        "service-a",
        "1.0.0",
        Some(vec![
            Dependency {
                service: "service-b".to_string(),
                version_constraint: Some("1.0.0".to_string()),
                required: true,
            },
            Dependency {
                service: "service-c".to_string(),
                version_constraint: Some("1.0.0".to_string()),
                required: false,
            },
        ]),
    );

    let service_b = create_test_config("service-b", "1.0.0", None);
    let service_c = create_test_config("service-c", "2.0.0", None); // Different version than expected

    // Register services
    registry.register_service("service-a", &service_a.to_string()).unwrap();
    registry.register_service("service-b", &service_b.to_string()).unwrap();
    registry.register_service("service-c", &service_c.to_string()).unwrap();

    // Validate all services
    let validation_result = registry.validate_all_services().unwrap();

    // Check validation summary
    assert!(validation_result.is_successful(), "Validation should succeed with warnings");
    assert!(validation_result.has_warnings(), "Validation should have warnings");

    // There should be warnings for service-a due to optional dependency service-c version mismatch
    assert!(validation_result.warnings.contains_key("service-a"), "service-a should have warnings");

    // Check that service-a warnings mention service-c
    if let Some(warnings) = validation_result.warnings.get("service-a") {
        assert!(
            warnings.iter().any(|w| w.contains("service-c") && w.contains("version")),
            "Warning should mention service-c version mismatch"
        );
    } else {
        panic!("Expected warnings for service-a");
    }

    // service-b should have no warnings
    assert!(
        !validation_result.warnings.contains_key("service-b"),
        "service-b should have no warnings"
    );

    // All services should be active
    let service_a = registry.get_service("service-a").unwrap();
    let service_b = registry.get_service("service-b").unwrap();
    let service_c = registry.get_service("service-c").unwrap();

    assert_eq!(service_a.status.state, ServiceState::Active);
    assert_eq!(service_b.status.state, ServiceState::Active);
    assert_eq!(service_c.status.state, ServiceState::Active);
}

#[test]
fn test_registry_missing_required_dependency() {
    // Create a mock registry
    let mut registry = MockRegistry::new();

    // Register a service with a required dependency that doesn't exist
    let service_x = create_test_config(
        "service-x",
        "1.0.0",
        Some(vec![Dependency {
            service: "missing-service".to_string(),
            version_constraint: Some("1.0.0".to_string()),
            required: true, // Required!
        }]),
    );

    // Register service
    registry.register_service("service-x", &service_x.to_string()).unwrap();

    // Validate all services
    let validation_result = registry.validate_all_services().unwrap();

    // Check validation summary
    assert!(!validation_result.is_successful(), "Validation should fail");

    // service-x should be in the failed list
    assert_eq!(validation_result.failed_count(), 1);
    assert!(
        validation_result.failed.iter().any(|(name, _)| name == "service-x"),
        "service-x should be in the failed list"
    );

    // service-x status should be Error
    let service_x = registry.get_service("service-x").unwrap();
    assert_eq!(service_x.status.state, ServiceState::Error);

    // Error message should mention the missing dependency
    assert!(
        service_x.status.error_message.as_ref().unwrap().contains("missing-service"),
        "Error should mention the missing dependency"
    );
}

#[test]
fn test_registry_circular_dependencies() {
    // Create a mock registry
    let mut registry = MockRegistry::new();

    // Register services that form a circular dependency: X -> Y -> Z -> X
    let service_x = create_test_config(
        "service-x",
        "1.0.0",
        Some(vec![Dependency {
            service: "service-y".to_string(),
            version_constraint: Some("1.0.0".to_string()),
            required: true,
        }]),
    );

    let service_y = create_test_config(
        "service-y",
        "1.0.0",
        Some(vec![Dependency {
            service: "service-z".to_string(),
            version_constraint: Some("1.0.0".to_string()),
            required: true,
        }]),
    );

    let service_z = create_test_config(
        "service-z",
        "1.0.0",
        Some(vec![Dependency {
            service: "service-x".to_string(),
            version_constraint: Some("1.0.0".to_string()),
            required: true,
        }]),
    );

    // Register services
    registry.register_service("service-x", &service_x.to_string()).unwrap();
    registry.register_service("service-y", &service_y.to_string()).unwrap();
    registry.register_service("service-z", &service_z.to_string()).unwrap();

    // Validate all services
    let validation_result = registry.validate_all_services().unwrap();

    // Check validation summary - should succeed with warnings
    assert!(
        validation_result.is_successful(),
        "Validation should succeed with circular dependency warnings"
    );
    assert!(validation_result.has_warnings(), "Validation should have warnings");

    // There should be a system warning about circular dependencies
    assert!(validation_result.warnings.contains_key("system"), "Should have system-level warnings");

    // The warning should mention circular dependency
    if let Some(warnings) = validation_result.warnings.get("system") {
        assert!(
            warnings.iter().any(|w| w.contains("Circular dependency")),
            "Warning should mention circular dependency"
        );
    } else {
        panic!("Expected system warnings about circular dependencies");
    }

    // All services should still be active since circular dependencies are warnings, not errors
    let service_x = registry.get_service("service-x").unwrap();
    let service_y = registry.get_service("service-y").unwrap();
    let service_z = registry.get_service("service-z").unwrap();

    assert_eq!(service_x.status.state, ServiceState::Active);
    assert_eq!(service_y.status.state, ServiceState::Active);
    assert_eq!(service_z.status.state, ServiceState::Active);
}
