use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use crate::error::{AureaCoreError, Result};
use crate::registry::ServiceRegistry;
use crate::schema::validation::ValidationService;

#[derive(Debug, Clone)]
pub struct EdgeMetadata {
    pub required: bool,
    pub version_constraint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CycleInfo {
    pub cycle_path: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct ImpactInfo {
    pub service_name: String,
    pub is_required: bool,
    pub impact_path: Vec<String>,
    pub description: String,
}

#[derive(Debug)]
pub struct DependencyGraph {
    pub adjacency_list: HashMap<String, Vec<(String, EdgeMetadata)>>,
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

    pub fn add_node(&mut self, node: String) {
        self.adjacency_list.entry(node).or_default();
    }

    pub fn add_edge(&mut self, from: String, to: String, metadata: EdgeMetadata) {
        self.add_node(from.clone());
        self.add_node(to.clone());
        if let Some(edges) = self.adjacency_list.get_mut(&from) {
            edges.push((to, metadata));
        }
    }

    pub fn detect_cycles(&self) -> Option<CycleInfo> {
        // Track three states for nodes in DFS:
        // - Not visited: not in visited_set
        // - In current path: in path_set
        // - Visited but not in current path: in visited_set but not in path_set
        let mut visited_set = HashSet::new();
        let mut path_set = HashSet::new();
        let mut path = Vec::new();

        // Check each node that hasn't been visited yet
        for start_node in self.adjacency_list.keys() {
            if !visited_set.contains(start_node)
                && self.dfs_detect_cycle(start_node, &mut visited_set, &mut path, &mut path_set)
            {
                // Find where the cycle starts in the path
                let last = path.last().unwrap();
                let cycle_start = path.iter().position(|n| n == last).unwrap();
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

    pub fn find_impact_path(&self, _graph: &DependencyGraph, _service_name: &str) -> Vec<String> {
        Vec::new()
    }

    pub fn analyze_impact_details(
        &self,
        _graph: &DependencyGraph,
        _service_name: &str,
    ) -> Vec<ImpactInfo> {
        Vec::new()
    }

    pub fn resolve_order(
        &self,
        _graph: &DependencyGraph,
        _service_names: &[String],
    ) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
}

pub trait RegistryRef {
    fn registry_ref(&self) -> &RwLock<ServiceRegistry>;
}

impl RegistryRef for Arc<RwLock<ServiceRegistry>> {
    fn registry_ref(&self) -> &RwLock<ServiceRegistry> {
        self
    }
}

impl RegistryRef for Rc<RwLock<ServiceRegistry>> {
    fn registry_ref(&self) -> &RwLock<ServiceRegistry> {
        self
    }
}

/// Struct to manage dependencies between services
pub struct DependencyManager<T: RegistryRef = Arc<RwLock<ServiceRegistry>>> {
    registry: T,
    validation_service: Arc<ValidationService>,
}

impl<T: RegistryRef> DependencyManager<T> {
    pub fn new(registry: T, validation_service: Arc<ValidationService>) -> Self {
        Self { registry, validation_service }
    }

    pub fn build_dependency_graph(&self) -> Result<DependencyGraph> {
        let mut graph = DependencyGraph::new();

        // Get all services from the registry
        let services = {
            let registry = self.registry.registry_ref().read().unwrap();
            registry.list_services()?
        };

        // Add all services as nodes first
        for service_name in &services {
            graph.add_node(service_name.clone());
        }

        // Now add all dependencies as edges
        {
            let registry = self.registry.registry_ref().read().unwrap();

            for service_name in &services {
                let service = registry.get_service(service_name)?;

                if let Some(deps) = &service.config.dependencies {
                    for dep in deps {
                        // Only add edge if the dependency exists in the registry
                        if services.contains(&dep.service) {
                            graph.add_edge(
                                service_name.clone(),
                                dep.service.clone(),
                                EdgeMetadata {
                                    required: dep.required,
                                    version_constraint: dep.version_constraint.clone(),
                                },
                            );
                        }
                    }
                }
            }
        }

        Ok(graph)
    }

    pub fn resolve_dependencies(&self, service_names: &[String]) -> Result<Vec<String>> {
        // First check for circular dependencies
        if let Some(cycle) = self.check_circular_dependencies()? {
            return Err(AureaCoreError::CircularDependency(cycle.description));
        }

        // Build the dependency graph
        let graph = self.build_dependency_graph()?;

        // Create a resolver and get the dependency order
        let resolver = DependencyResolver::new();
        resolver.resolve_order(&graph, service_names)
    }

    pub fn check_circular_dependencies(&self) -> Result<Option<CycleInfo>> {
        let graph = self.build_dependency_graph()?;
        Ok(graph.detect_cycles())
    }

    pub fn analyze_impact(&self, service_name: &str) -> Result<Vec<String>> {
        // Check if the service exists in the registry
        {
            let registry = self.registry.registry_ref().read().unwrap();
            registry.get_service(service_name)?;
        }

        // Build the dependency graph
        let graph = self.build_dependency_graph()?;

        // Find all services that depend on this service (reverse lookup)
        let mut impacted = Vec::new();
        let mut visited = HashSet::new();

        // DFS to find all services that depend on this one
        Self::dfs_find_reverse_dependencies(&graph, service_name, &mut visited, &mut impacted);

        Ok(impacted)
    }

    pub fn analyze_impact_detailed(&self, service_name: &str) -> Result<Vec<ImpactInfo>> {
        // Check if the service exists in the registry
        {
            let registry = self.registry.registry_ref().read().unwrap();
            registry.get_service(service_name)?;
        }

        // Build the dependency graph
        let graph = self.build_dependency_graph()?;

        // Find all services that depend on this service with detailed paths
        let mut impacted = Vec::new();
        let mut visited = HashSet::new();
        let path = vec![service_name.to_string()];

        // DFS to find all services that depend on this one with paths
        Self::dfs_find_reverse_dependencies_with_path(
            &graph,
            service_name,
            &mut visited,
            &mut impacted,
            &path,
        );

        Ok(impacted)
    }

    pub fn analyze_critical_impact(&self, service_name: &str) -> Result<Vec<String>> {
        // Get detailed impact analysis
        let impact_details = self.analyze_impact_detailed(service_name)?;

        // Filter for only required dependencies
        let critical_services: Vec<String> = impact_details
            .into_iter()
            .filter(|info| info.is_required)
            .map(|info| info.service_name)
            .collect();

        Ok(critical_services)
    }

    pub fn validate_dependencies(
        &self,
        service_name: &str,
    ) -> Result<HashMap<String, Vec<String>>> {
        let mut result = HashMap::new();
        let mut warnings = Vec::new();

        // Get the service and its dependencies first
        let service_deps = {
            let registry = self.registry.registry_ref().read().unwrap();
            let service = registry.get_service(service_name)?;
            service.config.dependencies.clone()
        };

        // Now check each dependency
        if let Some(dependencies) = service_deps {
            let registry = self.registry.registry_ref().read().unwrap();

            for dep in dependencies {
                match registry.get_service(&dep.service) {
                    Ok(_) => {
                        // Service exists, check version compatibility if constraint provided
                        if let Some(constraint) = &dep.version_constraint {
                            let dep_service = registry.get_service(&dep.service)?;

                            // Use validation service to check version compatibility
                            let compatibility =
                                self.validation_service.check_version_compatibility(
                                    &dep_service.config.schema_version,
                                    constraint,
                                );

                            // Check compatibility result
                            match compatibility {
                                crate::schema::validation::VersionCompatibility::Compatible => {
                                    // Compatible - no warning needed
                                },
                                crate::schema::validation::VersionCompatibility::MinorIncompatible => {
                                    warnings.push(format!(
                                        "Minor version incompatibility for {}: required {} but found {}",
                                        dep.service, constraint, dep_service.config.schema_version
                                    ));
                                },
                                crate::schema::validation::VersionCompatibility::MajorIncompatible => {
                                    warnings.push(format!(
                                        "Major version incompatibility for {}: required {} but found {}",
                                        dep.service, constraint, dep_service.config.schema_version
                                    ));
                                }
                            }
                        }
                    }
                    Err(_) => {
                        if dep.required {
                            warnings.push(format!("Required dependency {} not found", dep.service));
                        } else {
                            warnings.push(format!("Optional dependency {} not found", dep.service));
                        }
                    }
                }
            }
        }

        if !warnings.is_empty() {
            result.insert(service_name.to_string(), warnings);
        }

        Ok(result)
    }

    pub fn validate_all_dependencies(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut all_warnings = HashMap::new();

        // Get all services
        let services = {
            let registry = self.registry.registry_ref().read().unwrap();
            registry.list_services()?
        };

        // Validate each service's dependencies
        for service_name in services {
            let warnings = self.validate_dependencies(&service_name)?;
            for (service, svc_warnings) in warnings {
                all_warnings.insert(service, svc_warnings);
            }
        }

        Ok(all_warnings)
    }

    // Helper method to find all services that depend on a given service
    fn dfs_find_reverse_dependencies(
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
                    Self::dfs_find_reverse_dependencies(graph, from, visited, impacted);
                }
            }
        }
    }

    // Helper method to find all services that depend on a given service with path information
    fn dfs_find_reverse_dependencies_with_path(
        graph: &DependencyGraph,
        node: &str,
        visited: &mut HashSet<String>,
        impacted: &mut Vec<ImpactInfo>,
        current_path: &[String],
    ) {
        if visited.contains(node) {
            return;
        }

        visited.insert(node.to_string());

        // Find all services that depend on this node
        for (from, edges) in &graph.adjacency_list {
            for (to, metadata) in edges {
                if to == node {
                    // Check if this service is already in the impacted list
                    let already_impacted = impacted.iter().any(|info| info.service_name == *from);

                    if !already_impacted {
                        // Create a new path that includes this service
                        let mut new_path = current_path.to_owned();
                        new_path.insert(0, from.clone());

                        // Create impact info for this service
                        let is_required = metadata.required;
                        let description = if is_required {
                            format!(
                                "Required dependency chain from '{}' to '{}'",
                                new_path.first().unwrap(),
                                new_path.last().unwrap()
                            )
                        } else {
                            format!(
                                "Optional dependency chain from '{}' to '{}'",
                                new_path.first().unwrap(),
                                new_path.last().unwrap()
                            )
                        };

                        let impact_info = ImpactInfo {
                            service_name: from.clone(),
                            is_required,
                            impact_path: new_path.clone(),
                            description,
                        };

                        impacted.push(impact_info);

                        // Continue DFS with this new path
                        Self::dfs_find_reverse_dependencies_with_path(
                            graph, from, visited, impacted, &new_path,
                        );
                    }
                }
            }
        }
    }
}
