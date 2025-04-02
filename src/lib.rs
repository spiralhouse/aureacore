pub mod error;
pub mod registry;
pub mod schema;

pub use error::{AureaCoreError, Result};
// Uncomment the dependency exports now that the module is implemented
pub use registry::{
    CycleInfo, DependencyGraph, DependencyManager, DependencyResolver, EdgeMetadata, ImpactInfo,
};
pub use registry::{Service, ServiceConfig, ServiceState, ServiceStatus};
pub use schema::service::{Dependency, Endpoint, ServiceSchema, ServiceType};
pub use schema::validation::{CompiledSchema, SchemaType, ValidationService, VersionCompatibility};
