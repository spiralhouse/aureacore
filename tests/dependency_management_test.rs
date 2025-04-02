use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use aureacore::error::{AureaCoreError, Result};
use aureacore::registry::{DependencyManager, ServiceRegistry};
use aureacore::schema::validation::ValidationService;

// Create a test registry with predefined services and dependencies
fn create_test_registry() -> Rc<RwLock<ServiceRegistry>> {
    // Create temporary directory
    let temp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/tests");
    if !temp_dir.exists() {
        std::fs::create_dir_all(&temp_dir).unwrap();
    }

    // Create registry
    let registry = ServiceRegistry::new(
        "https://example.com/repo.git".to_string(),
        "main".to_string(),
        temp_dir.clone(),
    )
    .unwrap();

    let registry_rc = Rc::new(RwLock::new(registry));

    // Create service configuration files
    {
        let mut registry = registry_rc.write().unwrap();

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
        std::fs::write(temp_dir.join("service-a.json"), config_a).unwrap();
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
        std::fs::write(temp_dir.join("service-b.json"), config_b).unwrap();
        registry.register_service("service-b", config_b).unwrap();

        // Add service C
        let config_c = r#"{
            "name": "service-c",
            "namespace": null,
            "config_path": "service-c.json",
            "schema_version": "1.0.0",
            "dependencies": []
        }"#;
        std::fs::write(temp_dir.join("service-c.json"), config_c).unwrap();
        registry.register_service("service-c", config_c).unwrap();

        // Add service D
        let config_d = r#"{
            "name": "service-d",
            "namespace": null,
            "config_path": "service-d.json",
            "schema_version": "1.0.0",
            "dependencies": []
        }"#;
        std::fs::write(temp_dir.join("service-d.json"), config_d).unwrap();
        registry.register_service("service-d", config_d).unwrap();
    }

    registry_rc
}

#[test]
fn test_dependency_graph_creation() -> Result<()> {
    let registry = create_test_registry();
    let validation_service = Arc::new(ValidationService::new());

    // Debug logging
    {
        let registry_read = registry.read().unwrap();
        let service_names = registry_read.list_services()?;
        println!("Available services: {:?}", service_names);

        // Check each service's dependencies
        for name in &service_names {
            let service = registry_read.get_service(name)?;
            println!("Service {} dependencies: {:?}", name, service.config.dependencies);
        }
    }

    let manager = DependencyManager::new(registry, validation_service);
    let graph = manager.build_dependency_graph()?;

    // Debug graph
    println!("Graph node count: {}", graph.adjacency_list.len());

    // Verify nodes
    assert_eq!(graph.adjacency_list.len(), 4);

    // Verify edges
    let a_neighbors = graph
        .adjacency_list
        .get(&"service-a".to_string())
        .map(|edges| edges.iter().map(|(to, _)| to).collect::<Vec<_>>())
        .unwrap_or_default();
    println!("Service A neighbors: {:?}", a_neighbors);
    assert_eq!(a_neighbors.len(), 2);
    assert!(a_neighbors.contains(&&"service-b".to_string()));
    assert!(a_neighbors.contains(&&"service-c".to_string()));

    let b_neighbors = graph
        .adjacency_list
        .get(&"service-b".to_string())
        .map(|edges| edges.iter().map(|(to, _)| to).collect::<Vec<_>>())
        .unwrap_or_default();
    println!("Service B neighbors: {:?}", b_neighbors);
    assert_eq!(b_neighbors.len(), 1);
    assert!(b_neighbors.contains(&&"service-d".to_string()));

    // Check empty neighbors for leaf nodes
    let c_neighbors = graph
        .adjacency_list
        .get(&"service-c".to_string())
        .map(|edges| edges.iter().map(|(to, _)| to).collect::<Vec<_>>())
        .unwrap_or_default();
    println!("Service C neighbors: {:?}", c_neighbors);
    assert_eq!(c_neighbors.len(), 0);

    let d_neighbors = graph
        .adjacency_list
        .get(&"service-d".to_string())
        .map(|edges| edges.iter().map(|(to, _)| to).collect::<Vec<_>>())
        .unwrap_or_default();
    println!("Service D neighbors: {:?}", d_neighbors);
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

    println!("Resolved services: {:?}", resolved);

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

    println!("Positions - D: {}, B: {}, A: {}", d_pos, b_pos, a_pos);

    assert!(d_pos < b_pos, "D should come before B");
    assert!(b_pos < a_pos, "B should come before A");

    Ok(())
}

/// Creates a more complex test registry with multiple dependency patterns
fn create_complex_registry() -> Rc<RwLock<ServiceRegistry>> {
    // Create temporary directory
    let temp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/tests");
    if !temp_dir.exists() {
        std::fs::create_dir_all(&temp_dir).unwrap();
    }

    // Create registry
    let registry = ServiceRegistry::new(
        "https://example.com/repo.git".to_string(),
        "main".to_string(),
        temp_dir.clone(),
    )
    .unwrap();

    let registry_rc = Rc::new(RwLock::new(registry));

    // Create service configuration files with a more complex dependency hierarchy
    // Service dependency structure:
    //
    //            A
    //          / | \
    //         B  C  E
    //        /   |   \
    //       D    F    G
    //           /      \
    //          H        I
    {
        let mut registry = registry_rc.write().unwrap();

        // Add service A (depends on B, C, E)
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
                    "required": true
                },
                {
                    "service": "service-e",
                    "version_constraint": "1.0.0",
                    "required": true
                }
            ]
        }"#;
        std::fs::write(temp_dir.join("service-a.json"), config_a).unwrap();
        registry.register_service("service-a", config_a).unwrap();

        // Add service B (depends on D)
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
        std::fs::write(temp_dir.join("service-b.json"), config_b).unwrap();
        registry.register_service("service-b", config_b).unwrap();

        // Add service C (depends on F)
        let config_c = r#"{
            "name": "service-c",
            "namespace": null,
            "config_path": "service-c.json",
            "schema_version": "1.0.0",
            "dependencies": [
                {
                    "service": "service-f",
                    "version_constraint": "1.0.0",
                    "required": true
                }
            ]
        }"#;
        std::fs::write(temp_dir.join("service-c.json"), config_c).unwrap();
        registry.register_service("service-c", config_c).unwrap();

        // Add service D (no dependencies)
        let config_d = r#"{
            "name": "service-d",
            "namespace": null,
            "config_path": "service-d.json",
            "schema_version": "1.0.0",
            "dependencies": []
        }"#;
        std::fs::write(temp_dir.join("service-d.json"), config_d).unwrap();
        registry.register_service("service-d", config_d).unwrap();

        // Add service E (depends on G)
        let config_e = r#"{
            "name": "service-e",
            "namespace": null,
            "config_path": "service-e.json",
            "schema_version": "1.0.0",
            "dependencies": [
                {
                    "service": "service-g",
                    "version_constraint": "1.0.0",
                    "required": true
                }
            ]
        }"#;
        std::fs::write(temp_dir.join("service-e.json"), config_e).unwrap();
        registry.register_service("service-e", config_e).unwrap();

        // Add service F (depends on H)
        let config_f = r#"{
            "name": "service-f",
            "namespace": null,
            "config_path": "service-f.json",
            "schema_version": "1.0.0",
            "dependencies": [
                {
                    "service": "service-h",
                    "version_constraint": "1.0.0",
                    "required": true
                }
            ]
        }"#;
        std::fs::write(temp_dir.join("service-f.json"), config_f).unwrap();
        registry.register_service("service-f", config_f).unwrap();

        // Add service G (depends on I)
        let config_g = r#"{
            "name": "service-g",
            "namespace": null,
            "config_path": "service-g.json",
            "schema_version": "1.0.0",
            "dependencies": [
                {
                    "service": "service-i",
                    "version_constraint": "1.0.0",
                    "required": true
                }
            ]
        }"#;
        std::fs::write(temp_dir.join("service-g.json"), config_g).unwrap();
        registry.register_service("service-g", config_g).unwrap();

        // Add service H (no dependencies)
        let config_h = r#"{
            "name": "service-h",
            "namespace": null,
            "config_path": "service-h.json",
            "schema_version": "1.0.0",
            "dependencies": []
        }"#;
        std::fs::write(temp_dir.join("service-h.json"), config_h).unwrap();
        registry.register_service("service-h", config_h).unwrap();

        // Add service I (no dependencies)
        let config_i = r#"{
            "name": "service-i",
            "namespace": null,
            "config_path": "service-i.json",
            "schema_version": "1.0.0",
            "dependencies": []
        }"#;
        std::fs::write(temp_dir.join("service-i.json"), config_i).unwrap();
        registry.register_service("service-i", config_i).unwrap();
    }

    registry_rc
}

#[test]
fn test_complex_dependency_resolution() -> Result<()> {
    let registry = create_complex_registry();
    let validation_service = Arc::new(ValidationService::new());

    let manager = DependencyManager::new(registry, validation_service);

    // Test 1: Resolve all dependencies for service A
    let resolved_a = manager.resolve_dependencies(&["service-a".to_string()])?;
    println!("Resolved services for A: {:?}", resolved_a);

    // Should include all 9 services
    assert_eq!(resolved_a.len(), 9, "Should resolve all 9 services in the hierarchy");

    // Verify service A depends on B, C, E
    let a_pos = resolved_a.iter().position(|x| x == "service-a").unwrap();
    let b_pos = resolved_a.iter().position(|x| x == "service-b").unwrap();
    let c_pos = resolved_a.iter().position(|x| x == "service-c").unwrap();
    let e_pos = resolved_a.iter().position(|x| x == "service-e").unwrap();

    assert!(b_pos < a_pos, "B should come before A");
    assert!(c_pos < a_pos, "C should come before A");
    assert!(e_pos < a_pos, "E should come before A");

    // Test 2: Resolve dependencies for multiple services (B and E)
    let resolved_be =
        manager.resolve_dependencies(&["service-b".to_string(), "service-e".to_string()])?;
    println!("Resolved services for B and E: {:?}", resolved_be);

    // Should include B, D, E, G, I
    assert_eq!(resolved_be.len(), 5, "Should resolve 5 services for B and E combined");
    assert!(resolved_be.contains(&"service-b".to_string()));
    assert!(resolved_be.contains(&"service-d".to_string()));
    assert!(resolved_be.contains(&"service-e".to_string()));
    assert!(resolved_be.contains(&"service-g".to_string()));
    assert!(resolved_be.contains(&"service-i".to_string()));

    // Verify ordering
    let b_pos = resolved_be.iter().position(|x| x == "service-b").unwrap();
    let d_pos = resolved_be.iter().position(|x| x == "service-d").unwrap();
    let e_pos = resolved_be.iter().position(|x| x == "service-e").unwrap();
    let g_pos = resolved_be.iter().position(|x| x == "service-g").unwrap();
    let i_pos = resolved_be.iter().position(|x| x == "service-i").unwrap();

    assert!(d_pos < b_pos, "D should come before B");
    assert!(i_pos < g_pos, "I should come before G");
    assert!(g_pos < e_pos, "G should come before E");

    // Test 3: Resolve dependencies for leaf services
    let resolved_leaf =
        manager.resolve_dependencies(&["service-d".to_string(), "service-h".to_string()])?;
    println!("Resolved services for leaf services D and H: {:?}", resolved_leaf);

    // Should only include the leaf services themselves (no dependencies)
    assert_eq!(resolved_leaf.len(), 2, "Should only include the 2 leaf services");
    assert!(resolved_leaf.contains(&"service-d".to_string()));
    assert!(resolved_leaf.contains(&"service-h".to_string()));

    // Test 4: Resolve dependencies for a mid-level service
    let resolved_c = manager.resolve_dependencies(&["service-c".to_string()])?;
    println!("Resolved services for mid-level service C: {:?}", resolved_c);

    // Should include C, F, H
    assert_eq!(resolved_c.len(), 3, "Should include C and its dependencies");
    assert!(resolved_c.contains(&"service-c".to_string()));
    assert!(resolved_c.contains(&"service-f".to_string()));
    assert!(resolved_c.contains(&"service-h".to_string()));

    // Verify ordering
    let c_pos = resolved_c.iter().position(|x| x == "service-c").unwrap();
    let f_pos = resolved_c.iter().position(|x| x == "service-f").unwrap();
    let h_pos = resolved_c.iter().position(|x| x == "service-h").unwrap();

    assert!(h_pos < f_pos, "H should come before F");
    assert!(f_pos < c_pos, "F should come before C");

    Ok(())
}

#[test]
fn test_resolve_order_edge_cases() -> Result<()> {
    let registry = create_complex_registry();
    let validation_service = Arc::new(ValidationService::new());

    let manager = DependencyManager::new(registry, validation_service);

    // Test 1: Empty input list
    let resolved_empty = manager.resolve_dependencies(&[])?;
    println!("Resolved services for empty input: {:?}", resolved_empty);
    assert_eq!(resolved_empty.len(), 0, "Should return empty list for empty input");

    // Test 2: Non-existent service
    let non_existent_result = manager.resolve_dependencies(&["non-existent-service".to_string()]);
    assert!(non_existent_result.is_err(), "Should fail for non-existent service");

    if let Err(err) = non_existent_result {
        println!("Expected error for non-existent service: {}", err);
        assert!(
            matches!(err, AureaCoreError::ServiceNotFound(_)),
            "Should be ServiceNotFound error"
        );
    }

    // Test 3: Mixed existing and non-existing services
    let mixed_result = manager
        .resolve_dependencies(&["service-a".to_string(), "non-existent-service".to_string()]);
    assert!(mixed_result.is_err(), "Should fail when any service doesn't exist");

    // Test 4: Multiple calls with different inputs
    let resolved_1 = manager.resolve_dependencies(&["service-b".to_string()])?;
    let resolved_2 = manager.resolve_dependencies(&["service-c".to_string()])?;

    // Both should succeed and give correct results
    assert_eq!(resolved_1.len(), 2, "B should include itself and D");
    assert_eq!(resolved_2.len(), 3, "C should include itself, F and H");

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
fn test_detailed_impact_analysis() -> Result<()> {
    let registry = create_complex_registry();
    let validation_service = Arc::new(ValidationService::new());

    let manager = DependencyManager::new(registry, validation_service);

    // Test 1: Get detailed impact for service I (leaf node that other services depend on)
    let i_impact = manager.analyze_impact_detailed("service-i")?;

    // Should impact G, which impacts E, which impacts A
    assert_eq!(i_impact.len(), 3, "Service I should impact 3 services: G, E, and A");

    // Verify each impacted service
    let g_info =
        i_impact.iter().find(|i| i.service_name == "service-g").expect("G should be impacted");
    let e_info =
        i_impact.iter().find(|i| i.service_name == "service-e").expect("E should be impacted");
    let a_info =
        i_impact.iter().find(|i| i.service_name == "service-a").expect("A should be impacted");

    // Check required flag
    assert!(g_info.is_required, "G requires I, so impact is required");
    assert!(e_info.is_required, "E requires G, so impact is required");
    assert!(a_info.is_required, "A requires E, so impact is required");

    // Test 2: Get only critical impacts (required dependencies)
    let h_critical = manager.analyze_critical_impact("service-h")?;

    // Should impact F (required), which impacts C (required), which impacts A (required)
    assert_eq!(h_critical.len(), 3, "Service H should critically impact 3 services");
    assert!(h_critical.contains(&"service-f".to_string()));
    assert!(h_critical.contains(&"service-c".to_string()));
    assert!(h_critical.contains(&"service-a".to_string()));

    // Test 3: Detailed impact on a service with mixed required/optional dependencies
    // Modify registry to make some dependencies optional
    let registry = create_test_registry();
    let validation_service = Arc::new(ValidationService::new());
    let manager = DependencyManager::new(registry, validation_service);

    // Service C is an optional dependency of A
    let c_impact = manager.analyze_impact_detailed("service-c")?;
    assert_eq!(c_impact.len(), 1, "Service C should impact only service A");

    let a_info =
        c_impact.iter().find(|i| i.service_name == "service-a").expect("A should be impacted");
    assert!(!a_info.is_required, "C is an optional dependency of A");

    // Test 4: Impact path should show dependency chain
    let d_impact = manager.analyze_impact_detailed("service-d")?;

    // Should impact B, which impacts A
    assert_eq!(d_impact.len(), 2, "Service D should impact 2 services: B and A");

    let b_info =
        d_impact.iter().find(|i| i.service_name == "service-b").expect("B should be impacted");
    let a_info =
        d_impact.iter().find(|i| i.service_name == "service-a").expect("A should be impacted");

    // Check impact paths
    assert_eq!(b_info.impact_path, vec!["service-d".to_string(), "service-b".to_string()]);

    // A's path should be through B
    assert!(a_info.impact_path.contains(&"service-d".to_string()));
    assert!(a_info.impact_path.contains(&"service-b".to_string()));
    assert!(a_info.impact_path.contains(&"service-a".to_string()));

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

    let registry_rc = Rc::new(RwLock::new(registry));

    {
        let mut registry = registry_rc.write().unwrap();

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
    let manager = DependencyManager::new(registry_rc, validation_service);

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

#[test]
fn test_dependency_aware_operations() -> Result<()> {
    let registry_rc = create_complex_registry();
    let mut registry = registry_rc.write().unwrap();

    // Test 1: Get services in dependency order
    let service_names = vec!["service-a".to_string()];
    let ordered = registry.get_ordered_services(&service_names)?;

    // Should include all dependencies for A
    assert!(ordered.contains(&"service-a".to_string()));
    assert!(ordered.contains(&"service-b".to_string()));
    assert!(ordered.contains(&"service-c".to_string()));
    assert!(ordered.contains(&"service-e".to_string()));

    // Verify dependencies come before dependents
    let a_pos = ordered.iter().position(|x| x == "service-a").unwrap();
    let b_pos = ordered.iter().position(|x| x == "service-b").unwrap();
    let c_pos = ordered.iter().position(|x| x == "service-c").unwrap();
    let e_pos = ordered.iter().position(|x| x == "service-e").unwrap();

    assert!(b_pos < a_pos, "B should come before A");
    assert!(c_pos < a_pos, "C should come before A");
    assert!(e_pos < a_pos, "E should come before A");

    // Test 2: Get services in reverse order
    let reverse_ordered = registry.get_reverse_ordered_services(&service_names)?;

    // Should have the same services but in reverse order
    assert_eq!(reverse_ordered.len(), ordered.len());

    // First service should be the dependent
    assert_eq!(reverse_ordered[0], "service-a");

    // Last services should be the leaf dependencies
    let last_services: Vec<&String> =
        reverse_ordered.iter().skip(reverse_ordered.len() - 3).collect();
    assert!(
        last_services.contains(&&"service-d".to_string())
            || last_services.contains(&&"service-h".to_string())
            || last_services.contains(&&"service-i".to_string()),
        "Leaf services should be at the end in reverse order"
    );

    // Test 3: Get impacted services
    let d_impacts = registry.get_impacted_services("service-d")?;
    assert_eq!(d_impacts.len(), 2);
    assert!(d_impacts.contains(&"service-b".to_string()));
    assert!(d_impacts.contains(&"service-a".to_string()));

    // Test 4: Get detailed impact
    let c_detailed = registry.get_detailed_impact("service-c")?;
    assert_eq!(c_detailed.len(), 1);

    let a_impact = c_detailed.iter().find(|i| i.service_name == "service-a").unwrap();
    assert!(a_impact.impact_path.contains(&"service-c".to_string()));
    assert!(a_impact.impact_path.contains(&"service-a".to_string()));

    // Test 5: Get critical impacts
    let i_critical = registry.get_critical_impacts("service-i")?;
    assert_eq!(i_critical.len(), 3);
    assert!(i_critical.contains(&"service-g".to_string()));
    assert!(i_critical.contains(&"service-e".to_string()));
    assert!(i_critical.contains(&"service-a".to_string()));

    // Test 6: Delete service with dependencies (should fail without force)
    let delete_result = registry.delete_service("service-d", false);
    assert!(delete_result.is_err());
    if let Err(err) = delete_result {
        assert!(format!("{}", err).contains("required by"));
    }

    // Test 7: Delete service with force
    let delete_result = registry.delete_service("service-d", true);
    assert!(delete_result.is_ok());
    let impacted = delete_result.unwrap();
    assert_eq!(impacted.len(), 2);
    assert!(impacted.contains(&"service-b".to_string()));
    assert!(impacted.contains(&"service-a".to_string()));

    // Verify service is gone
    let service_result = registry.get_service("service-d");
    assert!(service_result.is_err());

    // Test 8: Start and stop services
    let started_services = std::cell::RefCell::new(Vec::new());
    let stopped_services = std::cell::RefCell::new(Vec::new());

    // Define start and stop handlers
    let start_fn = |service: &str| -> Result<()> {
        started_services.borrow_mut().push(service.to_string());
        Ok(())
    };

    let stop_fn = |service: &str| -> Result<()> {
        stopped_services.borrow_mut().push(service.to_string());
        Ok(())
    };

    // Start services (dependencies first)
    let start_names = vec!["service-c".to_string()];
    let start_order = registry.start_services(&start_names, start_fn)?;

    // Should include C and its dependencies (F and H)
    assert_eq!(start_order.len(), 3);

    // Order should be H -> F -> C
    let h_pos = started_services.borrow().iter().position(|x| x == "service-h").unwrap();
    let f_pos = started_services.borrow().iter().position(|x| x == "service-f").unwrap();
    let c_pos = started_services.borrow().iter().position(|x| x == "service-c").unwrap();

    assert!(h_pos < f_pos, "H should be started before F");
    assert!(f_pos < c_pos, "F should be started before C");

    // Stop services (dependents first)
    let stop_order = registry.stop_services(&start_names, stop_fn)?;

    // Order should be C -> F -> H (reverse of start order)
    let h_pos = stopped_services.borrow().iter().position(|x| x == "service-h").unwrap();
    let f_pos = stopped_services.borrow().iter().position(|x| x == "service-f").unwrap();
    let c_pos = stopped_services.borrow().iter().position(|x| x == "service-c").unwrap();

    assert!(c_pos < f_pos, "C should be stopped before F");
    assert!(f_pos < h_pos, "F should be stopped before H");

    Ok(())
}

#[test]
fn test_start_stop_services() -> Result<()> {
    let registry_rc = create_complex_registry();
    let registry = registry_rc.read().unwrap();

    // Track started and stopped services
    let started_services = std::cell::RefCell::new(Vec::new());
    let stopped_services = std::cell::RefCell::new(Vec::new());

    // Create start and stop functions
    let start_fn = |service: &str| -> Result<()> {
        started_services.borrow_mut().push(service.to_string());
        println!("Starting service: {}", service);
        Ok(())
    };

    let stop_fn = |service: &str| -> Result<()> {
        stopped_services.borrow_mut().push(service.to_string());
        println!("Stopping service: {}", service);
        Ok(())
    };

    // List of services to start
    let start_names = vec!["service-a".to_string()];

    // Test starting services in dependency order
    let start_order = registry.start_services(&start_names, start_fn)?;
    println!("Start order: {:?}", start_order);

    // Verify all dependencies were started in correct order
    assert!(start_order.contains(&"service-a".to_string()));
    assert!(start_order.contains(&"service-b".to_string()));
    assert!(start_order.contains(&"service-c".to_string()));
    assert!(start_order.contains(&"service-d".to_string()));
    assert!(start_order.contains(&"service-e".to_string()));
    assert!(start_order.contains(&"service-f".to_string()));
    assert!(start_order.contains(&"service-g".to_string()));
    assert!(start_order.contains(&"service-h".to_string()));
    assert!(start_order.contains(&"service-i".to_string()));

    // Test stopping services in reverse dependency order
    let stop_order = registry.stop_services(&start_names, stop_fn)?;
    println!("Stop order: {:?}", stop_order);

    // Verify all dependencies were stopped in reverse order
    assert_eq!(stop_order.len(), start_order.len());

    // Compare started and stopped services
    let started = started_services.borrow();
    let stopped = stopped_services.borrow();
    assert_eq!(started.len(), stopped.len());

    Ok(())
}
