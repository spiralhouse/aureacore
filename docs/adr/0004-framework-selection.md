# ADR 0004: Framework and Library Selection

## Status
Proposed

## Context
Following our architectural decisions in ADR 0002 (Rust as backend) and ADR 0003 (CAP considerations), we need to select specific frameworks and libraries that will form the foundation of AureaCore. Key requirements:

- High performance for frequent service lookups
- Strong async support for non-blocking operations
- Redis integration for caching
- Clean, maintainable API design
- Active community and maintenance
- Good documentation and examples

## Decision
We will use the following core frameworks and libraries:

### 1. Web Framework: Axum
- Built on Tokio and Tower ecosystem
- Excellent performance characteristics
- Clean, modern API design
- Strong middleware support
- Growing community (18.3k stars)
- Good balance of performance and developer experience

Reasons:
- Second best performance after Actix
- More approachable than Actix (less complex generics)
- Better middleware system for our needs
- Natural fit with Tokio ecosystem
- Strong typing with clear error handling

### 2. Redis Integration: redis-rs with bb8
- Main Redis library (redis-rs)
  - De facto standard Redis library for Rust
  - High-level API with flexible type conversion
  - Strong async support
  - Cluster support for scaling
  - Active maintenance (3.8k stars)
- Connection pooling (bb8-redis)
  - Async connection pooling
  - Tokio compatibility
  - Automatic connection management

### 3. Supporting Libraries
- `tokio`: Async runtime
  - Industry standard async runtime
  - Powers both Axum and redis-rs
  - Excellent performance
  - Comprehensive tooling

- `tower`: Middleware framework
  - Service abstractions
  - Rich middleware ecosystem
  - Built-in rate limiting
  - Request tracing

- `serde`: Serialization
  - Type-safe serialization
  - Automatic derive macros
  - YAML and JSON support
  - Custom format support

- `tracing`: Observability
  - Structured logging
  - Distributed tracing
  - Performance metrics
  - Debug tooling

## Technical Details

### 1. Core Dependencies
```toml
[dependencies]
# Web Framework
axum = { version = "0.7", features = ["macros"] }
tower = { version = "0.4", features = ["full"] }
tower-http = { version = "0.5", features = ["full"] }

# Redis
redis = { version = "0.29", features = ["tokio-comp", "cluster"] }
bb8-redis = "0.21"

# Async Runtime
tokio = { version = "1.36", features = ["full"] }

# Utilities
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### 2. Basic Application Structure
```rust
use axum::{
    routing::{get, post},
    Router,
};
use bb8_redis::RedisConnectionManager;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    // Redis connection pool
    let manager = RedisConnectionManager::new("redis://localhost").unwrap();
    let pool = bb8::Pool::builder()
        .build(manager)
        .await
        .unwrap();

    // Application router
    let app = Router::new()
        .route("/services", get(list_services))
        .route("/services/:name", get(get_service))
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

    // Start server
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

## Consequences

### Positive
- High-performance foundation with Axum
- Clean, modern async code with Tokio
- Production-ready Redis integration
- Strong typing and error handling
- Excellent middleware support
- Good developer experience

### Negative
- Learning curve for async Rust
- More complex than simpler frameworks
- Need to manage connection pools
- Potential for deadlocks if not careful with async

### Risks
1. **Async Complexity**
   - Mitigation: Clear patterns and documentation
   - Careful error handling
   - Proper timeout configuration

2. **Redis Integration**
   - Mitigation: Connection pooling
   - Circuit breakers
   - Fallback mechanisms

3. **Framework Maturity**
   - Mitigation: Stick to stable versions
   - Monitor issue trackers
   - Contribute fixes upstream

## Implementation Strategy

1. Phase 1: Basic Setup
   - Core application structure
   - Redis connection pooling
   - Basic health endpoints

2. Phase 2: Service Layer
   - Service CRUD operations
   - Cache integration
   - Error handling

3. Phase 3: Advanced Features
   - Middleware stack
   - Monitoring
   - Circuit breakers

4. Phase 4: Performance
   - Connection tuning
   - Cache optimization
   - Load testing

## References
- [Axum Documentation](https://docs.rs/axum)
- [redis-rs Documentation](https://docs.rs/redis)
- [Tokio Documentation](https://tokio.rs)
- [Tower Documentation](https://docs.rs/tower) 