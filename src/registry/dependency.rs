use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use crate::error::{AureaCoreError, Result};
use crate::registry::ServiceRegistry;
use crate::schema::validation::{ValidationService, VersionCompatibility};

/// Metadata for edges in the dependency graph
#[derive(Debug, Clone)]
pub struct EdgeMetadata {
    /// Whether this dependency is required
    pub required: bool,
    /// Version constraint (semver expression)
    pub version_constraint: Option<String>,
}

/// Information about a detected dependency cycle
#[derive(Debug, Clone)]
pub struct CycleInfo {
    /// Path of the cycle (list of service names in cycle order)
    pub cycle_path: Vec<String>,
    /// Description of the cycle for display/reporting
    pub description: String,
}

/// Dependency graph representation with nodes as service names
#[derive(Debug)]
pub struct DependencyGraph {
    /// Adjacency list representation of the graph
    adjacency_list: HashMap<String, Vec<(String, EdgeMetadata)>>,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    /// Creates a new empty dependency graph
    pub fn new() -> Self {
        Self { adjacency_list: HashMap::new() }
    }

    /// Adds a node (service) to the graph if it doesn't already exist
    pub fn add_node(&mut self, node: String) {
        self.adjacency_list.entry(node).or_default();
    }

    /// Adds a directed edge from one service to its dependency
    pub fn add_edge(&mut self, from: String, to: String, metadata: EdgeMetadata) {
        // Ensure both nodes exist
        self.add_node(from.clone());
        self.add_node(to.clone());

        // Add directed edge
        if let Some(edges) = self.adjacency_list.get_mut(&from) {
            edges.push((to, metadata));
        }
    }

    /// Gets all neighbors (dependencies) of a node
    pub fn get_neighbors(&self, node: &String) -> Vec<&String> {
        self.adjacency_list
            .get(node)
            .map(|edges| edges.iter().map(|(to, _)| to).collect())
            .unwrap_or_default()
    }

    /// Gets the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.adjacency_list.len()
    }

    /// Detects cycles in the dependency graph using DFS
    pub fn detect_cycles(&self) -> Option<CycleInfo> {
        // For each node, perform DFS and track visited nodes
        for start_node in self.adjacency_list.keys() {
            let mut visited = HashSet::new();
            let mut path = Vec::new();
            let mut path_set = HashSet::new();

            if self.dfs_detect_cycle(start_node, &mut visited, &mut path, &mut path_set) {
                // Extract the cycle from the path
                let cycle_start = path.iter().position(|n| n == path.last().unwrap()).unwrap();
                let cycle = path[cycle_start..].to_vec();

                return Some(CycleInfo {
                    cycle_path: cycle.clone(),
                    description: format!("Circular dependency detected: {}", cycle.join(" -> ")),
                });
            }
        }

        None
    }

    /// Helper method for cycle detection using DFS
    fn dfs_detect_cycle(
        &self,
        node: &String,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
        path_set: &mut HashSet<String>,
    ) -> bool {
        // If the node is already in the current path, we found a cycle
        if path_set.contains(node) {
            path.push(node.clone());
            return true;
        }

        // If the node has been visited but is not in the current path, no cycle through this node
        if visited.contains(node) {
            return false;
        }

        // Mark as visited and add to current path
        visited.insert(node.clone());
        path.push(node.clone());
        path_set.insert(node.clone());

        // Check all neighbors
        if let Some(edges) = self.adjacency_list.get(node) {
            for (neighbor, _) in edges {
                if self.dfs_detect_cycle(neighbor, visited, path, path_set) {
                    return true;
                }
            }
        }

        // Remove from current path when backtracking
        path.pop();
        path_set.remove(node);

        false
    }

    /// Performs topological sort on the graph
    /// Returns ordered list of nodes or error if cycles exist
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        // If there are cycles, topological sort is not possible
        if let Some(cycle_info) = self.detect_cycles() {
            return Err(AureaCoreError::CircularDependency(cycle_info.description));
        }

        // Count incoming edges for each node
        let mut in_degree = HashMap::new();
        for node in self.adjacency_list.keys() {
            in_degree.insert(node.clone(), 0);
        }

        for edges in self.adjacency_list.values() {
            for (to, _) in edges {
                *in_degree.entry(to.clone()).or_insert(0) += 1;
            }
        }

        // Start with nodes that have no incoming edges
        let mut queue = VecDeque::new();
        for (node, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(node.clone());
            }
        }

        let mut sorted = Vec::new();

        // Process nodes in order
        while let Some(node) = queue.pop_front() {
            sorted.push(node.clone());

            // Reduce in-degree of neighbors
            if let Some(edges) = self.adjacency_list.get(&node) {
                for (to, _) in edges {
                    if let Some(degree) = in_degree.get_mut(to) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(to.clone());
                        }
                    }
                }
            }
        }

        // Verify all nodes are included
        if sorted.len() != self.adjacency_list.len() {
            return Err(AureaCoreError::Internal(
                "Topological sort failed despite no detected cycles".to_string(),
            ));
        }

        // Reverse the order to get dependencies first
        sorted.reverse();

        Ok(sorted)
    }

    /// Creates a subgraph containing only the specified roots and their dependencies
    pub fn get_subgraph(&self, roots: &[String]) -> Self {
        let mut subgraph = DependencyGraph::new();
        let mut visited = HashSet::new();

        // Use DFS to extract all reachable nodes from roots
        for root in roots {
            self.dfs_extract_subgraph(root, &mut subgraph, &mut visited);
        }

        subgraph
    }

    /// Helper for subgraph extraction using DFS
    fn dfs_extract_subgraph(
        &self,
        node: &String,
        subgraph: &mut DependencyGraph,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(node) {
            return;
        }

        visited.insert(node.clone());
        subgraph.add_node(node.clone());

        // Add all edges from this node
        if let Some(edges) = self.adjacency_list.get(node) {
            for (to, metadata) in edges {
                subgraph.add_edge(node.clone(), to.clone(), metadata.clone());
                self.dfs_extract_subgraph(to, subgraph, visited);
            }
        }
    }
}

/// Resolver for dependency operations like ordering and impact analysis
pub struct DependencyResolver;

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyResolver {
    /// Creates a new dependency resolver
    pub fn new() -> Self {
        Self
    }

    /// Resolves dependencies in topological order
    pub fn resolve_order(
        &self,
        graph: &DependencyGraph,
        service_names: &[String],
    ) -> Result<Vec<String>> {
        // Create a subgraph containing only the requested services and their dependencies
        let subgraph = graph.get_subgraph(service_names);

        // Perform topological sort on the subgraph
        // This should return services in order where dependencies come before dependents
        subgraph.topological_sort()
    }

    /// Find services impacted by a change to the specified service
    pub fn find_impact_path(&self, graph: &DependencyGraph, service_name: &str) -> Vec<String> {
        // Find all services that depend on the specified service
        let mut impacted = Vec::new();
        let mut visited = HashSet::new();

        Self::dfs_reverse_deps(graph, service_name, &mut visited, &mut impacted);

        impacted
    }

    /// Helper for finding reverse dependencies using DFS
    fn dfs_reverse_deps(
        graph: &DependencyGraph,
        node: &str,
        visited: &mut HashSet<String>,
        impacted: &mut Vec<String>,
    ) {
        if visited.contains(node) {
            return;
        }

        visited.insert(node.to_string());

        // Find all services that depend on this node
        for (from, edges) in &graph.adjacency_list {
            for (to, _) in edges {
                if to == node && !impacted.contains(from) {
                    impacted.push(from.clone());
                    Self::dfs_reverse_deps(graph, from, visited, impacted);
                }
            }
        }
    }
}

// Define a local trait for registry reference
pub trait RegistryRef {
    fn registry_ref(&self) -> &RwLock<ServiceRegistry>;
}

// Implement RegistryRef for Arc<RwLock<ServiceRegistry>>
impl RegistryRef for Arc<RwLock<ServiceRegistry>> {
    fn registry_ref(&self) -> &RwLock<ServiceRegistry> {
        self
    }
}

// Implement RegistryRef for Rc<RwLock<ServiceRegistry>>
impl RegistryRef for Rc<RwLock<ServiceRegistry>> {
    fn registry_ref(&self) -> &RwLock<ServiceRegistry> {
        self
    }
}

/// Manages dependencies between services in the registry.
pub struct DependencyManager<T: RegistryRef = Arc<RwLock<ServiceRegistry>>> {
    registry: T,
    validation_service: Arc<ValidationService>,
}

impl<T: RegistryRef> DependencyManager<T> {
    /// Creates a new dependency manager with the given service registry.
    pub fn new(registry: T, validation_service: Arc<ValidationService>) -> Self {
        Self { registry, validation_service }
    }

    /// Builds a dependency graph for the current state of the service registry.
    pub fn build_dependency_graph(&self) -> Result<DependencyGraph> {
        let registry = self.registry.registry_ref().read().unwrap();
        let service_names = registry.list_services()?;

        let mut graph = DependencyGraph::new();

        // Add all services to the graph
        for service_name in &service_names {
            graph.add_node(service_name.clone());
        }

        // Add dependencies as edges (from service to its dependency)
        for service_name in &service_names {
            if let Ok(service) = registry.get_service(service_name) {
                // Extract dependencies from service config
                if let Some(dependencies) = &service.config.dependencies {
                    for dependency in dependencies {
                        if service_names.contains(&dependency.service) {
                            let metadata = EdgeMetadata {
                                required: dependency.required,
                                version_constraint: dependency.version_constraint.clone(),
                            };
                            graph.add_edge(
                                service_name.clone(),
                                dependency.service.clone(),
                                metadata,
                            );
                        }
                    }
                }
            }
        }

        Ok(graph)
    }

    /// Resolves dependencies for the specified services
    pub fn resolve_dependencies(&self, service_names: &[String]) -> Result<Vec<String>> {
        let graph = self.build_dependency_graph()?;
        let resolver = DependencyResolver::new();
        resolver.resolve_order(&graph, service_names)
    }

    /// Checks for circular dependencies in the service graph
    pub fn check_circular_dependencies(&self) -> Result<Option<CycleInfo>> {
        let graph = self.build_dependency_graph()?;
        Ok(graph.detect_cycles())
    }

    /// Analyzes which services would be impacted by a change to the specified service
    pub fn analyze_impact(&self, service_name: &str) -> Result<Vec<String>> {
        let graph = self.build_dependency_graph()?;
        let resolver = DependencyResolver::new();
        Ok(resolver.find_impact_path(&graph, service_name))
    }

    /// Checks version compatibility between a service and its dependency
    pub fn check_version_compatibility(
        &self,
        constraint: &str,
        version: &str,
    ) -> VersionCompatibility {
        self.validation_service.check_version_compatibility(constraint, version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a test graph
    fn create_test_graph() -> DependencyGraph {
        let mut graph = DependencyGraph::new();

        // Add nodes
        graph.add_node("A".to_string());
        graph.add_node("B".to_string());
        graph.add_node("C".to_string());
        graph.add_node("D".to_string());

        // Add edges: A -> B -> D, A -> C
        graph.add_edge(
            "A".to_string(),
            "B".to_string(),
            EdgeMetadata { required: true, version_constraint: None },
        );
        graph.add_edge(
            "A".to_string(),
            "C".to_string(),
            EdgeMetadata { required: false, version_constraint: None },
        );
        graph.add_edge(
            "B".to_string(),
            "D".to_string(),
            EdgeMetadata { required: true, version_constraint: None },
        );

        graph
    }

    // Helper to create a graph with cycle
    fn create_cycle_graph() -> DependencyGraph {
        let mut graph = DependencyGraph::new();

        // Add nodes
        graph.add_node("X".to_string());
        graph.add_node("Y".to_string());
        graph.add_node("Z".to_string());

        // Add edges: X -> Y -> Z -> X (cycle)
        graph.add_edge(
            "X".to_string(),
            "Y".to_string(),
            EdgeMetadata { required: true, version_constraint: None },
        );
        graph.add_edge(
            "Y".to_string(),
            "Z".to_string(),
            EdgeMetadata { required: true, version_constraint: None },
        );
        graph.add_edge(
            "Z".to_string(),
            "X".to_string(),
            EdgeMetadata { required: true, version_constraint: None },
        );

        graph
    }

    #[test]
    fn test_graph_node_creation() {
        let graph = create_test_graph();
        assert_eq!(graph.node_count(), 4);
    }

    #[test]
    fn test_graph_neighbors() {
        let graph = create_test_graph();

        let a_neighbors = graph.get_neighbors(&"A".to_string());
        assert_eq!(a_neighbors.len(), 2);
        assert!(a_neighbors.contains(&&"B".to_string()));
        assert!(a_neighbors.contains(&&"C".to_string()));

        let b_neighbors = graph.get_neighbors(&"B".to_string());
        assert_eq!(b_neighbors.len(), 1);
        assert!(b_neighbors.contains(&&"D".to_string()));

        let c_neighbors = graph.get_neighbors(&"C".to_string());
        assert_eq!(c_neighbors.len(), 0);

        let d_neighbors = graph.get_neighbors(&"D".to_string());
        assert_eq!(d_neighbors.len(), 0);
    }

    #[test]
    fn test_cycle_detection() {
        let acyclic_graph = create_test_graph();
        let cycle_info = acyclic_graph.detect_cycles();
        assert!(cycle_info.is_none(), "Acyclic graph should not detect cycles");

        let cyclic_graph = create_cycle_graph();
        let cycle_info = cyclic_graph.detect_cycles();
        assert!(cycle_info.is_some(), "Cyclic graph should detect cycles");

        if let Some(cycle) = cycle_info {
            assert_eq!(cycle.cycle_path.len(), 4); // X -> Y -> Z -> X cycle has 4 nodes including the repeated first node
                                                   // Due to the direction of edges (from service to dependency), the cycle detection
                                                   // finds cycles starting from Z, not X
            assert_eq!(cycle.cycle_path[0], "Z");
        }
    }

    #[test]
    fn test_topological_sort() {
        let graph = create_test_graph();
        let sorted = graph.topological_sort().unwrap();

        // Verify D comes before B, and B comes before A
        let d_pos = sorted.iter().position(|x| x == "D").unwrap();
        let b_pos = sorted.iter().position(|x| x == "B").unwrap();
        let a_pos = sorted.iter().position(|x| x == "A").unwrap();

        assert!(d_pos < b_pos, "D should come before B");
        assert!(b_pos < a_pos, "B should come before A");
    }

    #[test]
    fn test_topological_sort_with_cycles() {
        let graph = create_cycle_graph();
        let result = graph.topological_sort();

        assert!(result.is_err(), "Topological sort should fail with cycles");
    }

    #[test]
    fn test_subgraph_extraction() {
        let graph = create_test_graph();

        // Create subgraph with only A and its dependencies
        let subgraph = graph.get_subgraph(&["A".to_string()]);

        // Should contain all 4 nodes
        assert_eq!(subgraph.node_count(), 4);

        // Create subgraph with only B and its dependencies
        let subgraph = graph.get_subgraph(&["B".to_string()]);

        // Should contain B and D but not A or C
        assert_eq!(subgraph.node_count(), 2);
        assert!(subgraph.adjacency_list.contains_key("B"));
        assert!(subgraph.adjacency_list.contains_key("D"));
        assert!(!subgraph.adjacency_list.contains_key("A"));
        assert!(!subgraph.adjacency_list.contains_key("C"));
    }

    #[test]
    fn test_dependency_resolver() {
        let graph = create_test_graph();
        let resolver = DependencyResolver::new();

        // Test resolution order for A
        let order = resolver.resolve_order(&graph, &["A".to_string()]).unwrap();

        // Should contain all 4 nodes in topological order
        assert_eq!(order.len(), 4);

        // D should come before B, and B should come before A
        let d_pos = order.iter().position(|x| x == "D").unwrap();
        let b_pos = order.iter().position(|x| x == "B").unwrap();
        let a_pos = order.iter().position(|x| x == "A").unwrap();

        assert!(d_pos < b_pos, "D should come before B");
        assert!(b_pos < a_pos, "B should come before A");
    }

    #[test]
    fn test_impact_analysis() {
        let graph = create_test_graph();
        let resolver = DependencyResolver::new();

        // Test impact of changing D
        let impact = resolver.find_impact_path(&graph, "D");

        // Should affect B and A
        assert_eq!(impact.len(), 2);
        assert!(impact.contains(&"B".to_string()));
        assert!(impact.contains(&"A".to_string()));

        // Test impact of changing B
        let impact = resolver.find_impact_path(&graph, "B");

        // Should affect only A
        assert_eq!(impact.len(), 1);
        assert!(impact.contains(&"A".to_string()));

        // Test impact of changing A
        let impact = resolver.find_impact_path(&graph, "A");

        // Should affect nothing
        assert_eq!(impact.len(), 0);
    }
}
