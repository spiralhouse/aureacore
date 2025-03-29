//! API layer for AureaCore service catalog

use async_graphql::{EmptySubscription, Object, Schema};
use aureacore_core::Service;

/// GraphQL Query root
pub struct Query;

#[Object]
impl Query {
    /// Get a service by name
    async fn service(&self, name: String) -> Option<Service> {
        // This is just a placeholder implementation
        Some(Service::new(name, "0.1.0"))
    }

    /// List all services
    async fn services(&self) -> Vec<Service> {
        // This is just a placeholder implementation
        vec![Service::new("example-service", "1.0.0").with_description("An example service")]
    }
}

/// GraphQL Mutation root
pub struct Mutation;

#[Object]
impl Mutation {
    /// Create a new service
    async fn create_service(
        &self,
        name: String,
        version: String,
        description: Option<String>,
    ) -> Service {
        // This is just a placeholder implementation
        let mut service = Service::new(name, version);
        if let Some(desc) = description {
            service = service.with_description(desc);
        }
        service
    }
}

/// Create the GraphQL schema
pub fn create_schema() -> Schema<Query, Mutation, EmptySubscription> {
    Schema::build(Query, Mutation, EmptySubscription).finish()
}

#[cfg(test)]
mod tests {
    use async_graphql::Value;

    use super::*;

    #[tokio::test]
    async fn test_service_query() {
        let schema = create_schema();
        let query = r#"
            query {
                service(name: "test") {
                    name
                    version
                }
            }
        "#;

        let res = schema.execute(query).await;
        assert_eq!(res.data.to_string(), "{service: {name: \"test\", version: \"0.1.0\"}}");
    }
}
