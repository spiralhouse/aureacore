# ADR 0003: CAP Theorem Considerations and Cache Strategy

## Status
Proposed

## Context
AureaCore needs to serve as a reliable service catalog that can be deployed independently of phicd. Key considerations:
- High availability is critical for service lookups
- Eventual consistency is acceptable for service metadata
- Infrastructure teams may build tools requiring frequent service lookups
- System should work well with existing GitOps tools (FluxCD, ArgoCD)
- Service catalog should be virtually stateless for reliability

## Decision
We will prioritize Availability and Partition Tolerance over Consistency (AP over CP) in our architecture, with the following key decisions:

### 1. Cache-First Architecture
- Redis as primary cache layer (replacing MongoDB)
  - In-memory performance for high-frequency lookups
  - Built-in replication for high availability
  - Pub/sub for cache invalidation
  - TTL support for stale data management
  - Atomic operations for cache updates

### 2. Service Definition Storage
- Git remains source of truth for service definitions
- Redis cache stores:
  - Parsed and validated service definitions
  - Computed dependency graphs
  - Frequently accessed metadata
  - Search indices
- Cache invalidation strategy:
  - Time-based TTL as baseline
  - Webhook-triggered immediate updates
  - Background refresh for stale data
  - Version tracking for cache entries

### 3. Stateless Service Design
- All service instances are equal
- No instance-specific state
- Cache layer handles all shared state
- Each instance maintains:
  - Local memory cache for ultra-high-performance
  - Background Git synchronization
  - Health check status

### 4. High Availability Features
- Multiple read replicas for Redis
- Circuit breakers for Git operations
- Fallback to cached data when Git is unavailable
- Stale cache acceptable over no data
- Background refresh mechanisms

### 5. Consistency Model
- Eventual consistency for service definitions
- Real-time notifications for critical updates
- Version vectors for conflict detection
- Clear staleness indicators in API responses
- Optional strong consistency mode for critical operations

### 6. API Enhancements
- Cache status in response headers
  - Cache hit/miss status
  - Data freshness indicators
  - Version information
- Consistency level requests
  - Allow clients to specify needed consistency
  - Default to eventual consistency
- Bulk operation support
  - Batch queries for efficiency
  - Parallel processing capabilities

## Technical Changes Required

1. **Data Store Changes**
   - Replace MongoDB with Redis
   - Implement cache management layer
   - Design cache data structures

2. **API Layer Updates**
   - Add cache headers
   - Implement consistency controls
   - Add bulk operations
   - Include staleness indicators

3. **Service Architecture**
   - Implement stateless design
   - Add local caching layer
   - Enhance health checks
   - Add cache synchronization

4. **Monitoring Additions**
   - Cache hit/miss rates
   - Data freshness metrics
   - Git sync status
   - Instance health status

## Consequences

### Positive
- Higher availability for service lookups
- Better performance for frequent queries
- Simpler horizontal scaling
- More resilient to Git outages
- Clear consistency expectations

### Negative
- More complex cache invalidation logic
- Potential for stale data
- Need for cache warming
- Additional infrastructure (Redis)

### Risks
1. **Cache Coherence**
   - Mitigation: Clear invalidation strategy
   - Version tracking
   - Background validation

2. **Redis Dependency**
   - Mitigation: Redis cluster for HA
   - Fallback to local cache
   - Circuit breakers

3. **Stale Data Impact**
   - Mitigation: Clear staleness indicators
   - Configurable TTL
   - Priority refresh for critical services

## Implementation Strategy

1. Phase 1: Core Cache Infrastructure
   - Redis integration
   - Basic cache layer
   - Cache invalidation

2. Phase 2: Enhanced Availability
   - Redis clustering
   - Local caching
   - Circuit breakers

3. Phase 3: Consistency Controls
   - Version tracking
   - Staleness indicators
   - Consistency level API

4. Phase 4: Performance Optimization
   - Bulk operations
   - Cache warming
   - Performance monitoring

## References
- [Redis Documentation](https://redis.io/documentation)
- [CAP Theorem](https://en.wikipedia.org/wiki/CAP_theorem)
- [Eventual Consistency](https://en.wikipedia.org/wiki/Eventual_consistency)
- [Circuit Breaker Pattern](https://martinfowler.com/bliki/CircuitBreaker.html) 