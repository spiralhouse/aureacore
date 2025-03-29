//! Plugin system for AureaCore service catalog

use aureacore_core::Service;
use std::error::Error;

/// Trait for implementing service discovery plugins
#[async_trait::async_trait]
pub trait ServiceDiscovery: Send + Sync {
    /// Discover services from the plugin's source
    async fn discover(&self) -> Result<Vec<Service>, Box<dyn Error>>;
}

/// Example plugin implementation for testing
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct TestPlugin;

    #[async_trait]
    impl ServiceDiscovery for TestPlugin {
        async fn discover(&self) -> Result<Vec<Service>, Box<dyn Error>> {
            Ok(vec![Service::new("test-service", "1.0.0").with_description("A test service")])
        }
    }

    #[tokio::test]
    async fn test_plugin_discovery() {
        let plugin = TestPlugin;
        let services = plugin.discover().await.unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "test-service");
    }
}
