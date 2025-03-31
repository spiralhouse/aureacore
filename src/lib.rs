pub mod error;
pub mod registry;
pub mod schema;

pub use error::{AureaCoreError, Result};
pub use registry::{
    Service, ServiceConfig, ServiceRegistry, ServiceState, ServiceStatus, ValidationSummary,
};
pub use schema::service::{Dependency, Endpoint, ServiceSchema, ServiceType};
pub use schema::validation::{CompiledSchema, SchemaType, ValidationService, VersionCompatibility};
