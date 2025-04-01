pub mod error;
pub mod registry;
pub mod schema;

pub use error::{AureaCoreError, Result};
pub use registry::{Service, ServiceConfig, ServiceState, ServiceStatus};
// Temporarily comment out the dependency exports until the module is properly implemented
// pub use registry::{
//     CycleInfo, DependencyGraph, DependencyManager, DependencyResolver, EdgeMetadata,
// };
pub use schema::service::{Dependency, Endpoint, ServiceSchema, ServiceType};
pub use schema::validation::{CompiledSchema, SchemaType, ValidationService, VersionCompatibility};
