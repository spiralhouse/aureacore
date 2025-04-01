use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use aureacore::error::Result;
use aureacore::registry::{
    DependencyGraph, DependencyManager, DependencyResolver, EdgeMetadata, Service, ServiceConfig,
    ServiceRegistry, ServiceState, ServiceStatus,
};
use aureacore::schema::service::Dependency;
use aureacore::schema::validation::ValidationService;

// Create a test registry with predefined services and dependencies
fn create_test_registry() -> Arc<RwLock<ServiceRegistry>> {
    let temp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/tests");
    let registry = ServiceRegistry::new(
        "https://example.com/repo.git".to_string(),
        "main".to_string(),
        temp_dir,
    )
    .unwrap();

    let registry_arc = Arc::new(RwLock::new(registry));

    {
        let mut registry = registry_arc.write().unwrap();

        // Add service A
        let config_a = r#"{
            "name": "service-a",
            "namespace": null,
            "config_path": "service-a.json",
            "schema_version": "1.0.0",
            "dependencies": [
                {
                    "service": "service-b",
                    "version_constraint": "1.0.0",
                    "required": true
                },
                {
                    "service": "service-c",
                    "version_constraint": "1.0.0",
                    "required": false
                }
            ]
        }"#;
        registry.register_service("service-a", config_a).unwrap();

        // Add service B
        let config_b = r#"{
            "name": "service-b",
            "namespace": null,
            "config_path": "service-b.json",
            "schema_version": "1.0.0",
            "dependencies": [
                {
                    "service": "service-d",
                    "version_constraint": "1.0.0",
                    "required": true
                }
            ]
        }"#;
        registry.register_service("service-b", config_b).unwrap();

        // Add service C
        let config_c = r#"{
            "name": "service-c",
            "namespace": null,
            "config_path": "service-c.json",
            "schema_version": "1.0.0",
            "dependencies": []
        }"#;
        registry.register_service("service-c", config_c).unwrap();

        // Add service D
        let config_d = r#"{
            "name": "service-d",
            "namespace": null,
            "config_path": "service-d.json",
            "schema_version": "1.0.0",
            "dependencies": []
        }"#;
        registry.register_service("service-d", config_d).unwrap();
    }

    registry_arc
}

#[test]
fn test_dependency_graph_creation() -> Result<()> {
    let registry = create_test_registry();
    let validation_service = Arc::new(ValidationService::new());

    let manager = DependencyManager::new(registry, validation_service);
    let graph = manager.build_dependency_graph()?;

    // Verify nodes
    assert_eq!(graph.node_count(), 4);

    // Verify edges
    let a_neighbors = graph.get_neighbors(&"service-a".to_string());
    assert_eq!(a_neighbors.len(), 2);
    assert!(a_neighbors.contains(&&"service-b".to_string()));
    assert!(a_neighbors.contains(&&"service-c".to_string()));

    let b_neighbors = graph.get_neighbors(&"service-b".to_string());
    assert_eq!(b_neighbors.len(), 1);
    assert!(b_neighbors.contains(&&"service-d".to_string()));

    // Check empty neighbors for leaf nodes
    let c_neighbors = graph.get_neighbors(&"service-c".to_string());
    assert_eq!(c_neighbors.len(), 0);

    let d_neighbors = graph.get_neighbors(&"service-d".to_string());
    assert_eq!(d_neighbors.len(), 0);

    Ok(())
}

#[test]
fn test_dependency_resolution() -> Result<()> {
    let registry = create_test_registry();
    let validation_service = Arc::new(ValidationService::new());

    let manager = DependencyManager::new(registry, validation_service);

    // Request service A (should include B, C, and D in correct order)
    let resolved = manager.resolve_dependencies(&["service-a".to_string()])?;

    // Verify all dependencies are included
    assert_eq!(resolved.len(), 4);
    assert!(resolved.contains(&"service-a".to_string()));
    assert!(resolved.contains(&"service-b".to_string()));
    assert!(resolved.contains(&"service-c".to_string()));
    assert!(resolved.contains(&"service-d".to_string()));

    // Verify topological order: D before B, B before A
    let d_pos = resolved.iter().position(|x| x == "service-d").unwrap();
    let b_pos = resolved.iter().position(|x| x == "service-b").unwrap();
    let a_pos = resolved.iter().position(|x| x == "service-a").unwrap();

    assert!(d_pos < b_pos);
    assert!(b_pos < a_pos);

    Ok(())
}

#[test]
fn test_impact_analysis() -> Result<()> {
    let registry = create_test_registry();
    let validation_service = Arc::new(ValidationService::new());

    let manager = DependencyManager::new(registry, validation_service);

    // Analyze impact of D (should impact B and A)
    let d_impact = manager.analyze_impact("service-d")?;
    assert_eq!(d_impact.len(), 2);
    assert!(d_impact.contains(&"service-b".to_string()));
    assert!(d_impact.contains(&"service-a".to_string()));

    // Analyze impact of B (should impact A)
    let b_impact = manager.analyze_impact("service-b")?;
    assert_eq!(b_impact.len(), 1);
    assert!(b_impact.contains(&"service-a".to_string()));

    // Analyze impact of C (should impact A, but as optional dependency)
    let c_impact = manager.analyze_impact("service-c")?;
    assert_eq!(c_impact.len(), 1);
    assert!(c_impact.contains(&"service-a".to_string()));

    // Analyze impact of A (should impact nothing)
    let a_impact = manager.analyze_impact("service-a")?;
    assert_eq!(a_impact.len(), 0);

    Ok(())
}

#[test]
#[ignore]
fn test_circular_dependency_detection() -> Result<()> {
    // Create a test registry with circular dependencies
    let temp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/tests");
    let registry = ServiceRegistry::new(
        "https://example.com/repo.git".to_string(),
        "main".to_string(),
        temp_dir,
    )
    .unwrap();

    let registry_arc = Arc::new(RwLock::new(registry));

    {
        let mut registry = registry_arc.write().unwrap();

        // Add service X
        let config_x = r#"{
            "name": "service-x",
            "namespace": null,
            "config_path": "service-x.json",
            "schema_version": "1.0.0",
            "dependencies": [
                {
                    "service": "service-y",
                    "version_constraint": "1.0.0",
                    "required": true
                }
            ]
        }"#;
        registry.register_service("service-x", config_x).unwrap();

        // Add service Y
        let config_y = r#"{
            "name": "service-y",
            "namespace": null,
            "config_path": "service-y.json",
            "schema_version": "1.0.0",
            "dependencies": [
                {
                    "service": "service-z",
                    "version_constraint": "1.0.0",
                    "required": true
                }
            ]
        }"#;
        registry.register_service("service-y", config_y).unwrap();

        // Add service Z with circular dependency back to X
        let config_z = r#"{
            "name": "service-z",
            "namespace": null,
            "config_path": "service-z.json",
            "schema_version": "1.0.0",
            "dependencies": [
                {
                    "service": "service-x",
                    "version_constraint": "1.0.0",
                    "required": true
                }
            ]
        }"#;
        registry.register_service("service-z", config_z).unwrap();
    }

    let validation_service = Arc::new(ValidationService::new());
    let manager = DependencyManager::new(registry_arc, validation_service);

    // Check for circular dependencies
    let cycle_info = manager.check_circular_dependencies()?;
    assert!(cycle_info.is_some());

    let cycle = cycle_info.unwrap();
    assert!(cycle.cycle_path.len() >= 3);

    // Ensure all three services are in the cycle path
    assert!(cycle.cycle_path.contains(&"service-x".to_string()));
    assert!(cycle.cycle_path.contains(&"service-y".to_string()));
    assert!(cycle.cycle_path.contains(&"service-z".to_string()));

    // Attempt to resolve dependencies (should fail due to circular dependency)
    let resolve_result = manager.resolve_dependencies(&["service-x".to_string()]);
    assert!(resolve_result.is_err());

    Ok(())
}
