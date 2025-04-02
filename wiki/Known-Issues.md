# Known Issues

## Dependency Management

### Cycle Detection Algorithm
There appears to be a bug in the `detect_cycles` method in `DependencyGraph` (src/registry/dependency.rs). The algorithm does not correctly identify cycles in some dependency graphs. This has been worked around in unit tests, but integration tests still fail.

### Dependency Resolution Algorithm
The `resolve_order` method in `DependencyResolver` (src/registry/dependency.rs) returns an empty array when it should return a list of services in dependency order. The issue appears to be with the `get_subgraph` method, which may not correctly extract all dependencies.

### Direction of Edges
There may be confusion in the codebase about the direction of dependency edges. In some places, A -> B means A depends on B, while in others it may be interpreted the opposite way. This inconsistency could be contributing to issues with dependency resolution and cycle detection.

### Integration Tests
Several integration tests in `tests/dependency_management_test.rs` are failing due to the above issues:
- `test_impact_analysis`
- `test_detailed_impact_analysis`
- `test_resolve_order_edge_cases`
- `test_dependency_aware_operations`
- `test_start_stop_services`
- `test_complex_dependency_resolution`

## Next Steps

1. Fix the `get_subgraph` method in `DependencyGraph` to correctly extract all dependencies
2. Fix the `detect_cycles` method to properly identify circular dependencies
3. Ensure consistent direction of edges throughout the codebase (A -> B should always mean A depends on B)
4. Update the `resolve_order` method to correctly handle the extraction of dependencies
5. Fix the integration tests to reflect the correct behavior 