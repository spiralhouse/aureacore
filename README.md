# AureaCore

[![codecov](https://codecov.io/gh/spiralhouse/aureacore/branch/main/graph/badge.svg)](https://codecov.io/gh/spiralhouse/aureacore)

A powerful service catalog solution that helps teams discover, organize, and manage their microservices ecosystem with mathematical precision and intuitive design.

## Overview

AureaCore is part of SpiralHouse's suite of open-source tools dedicated to enhancing the developer experience. It serves as the service catalog component of phicd, providing teams with a comprehensive view of their microservices landscape. Through GitOps principles and intuitive design, AureaCore helps engineering teams understand, manage, and evolve their service architecture effectively.

## Features

* **GitOps-Based Service Management**
  - Centralized root configuration
  - Distributed service-specific definitions
  - Version-controlled service metadata
  - Automated service discovery

* **Service Metadata Management**
  - Rich service documentation
  - Dependency tracking
  - Integration points
  - Deployment configurations
  - Custom metadata fields

* **Dependency Analysis**
  - Real-time dependency visualization
  - Impact analysis
  - Circular dependency detection
  - Version compatibility checking
  - Cross-service relationship mapping

* **Integration Capabilities**
  - Native phicd integration
  - Kubernetes service discovery
  - Git repository integration
  - CI/CD pipeline connectivity
  - Observability platform integration

* **Modern User Interface**
  - Interactive dependency graphs
  - Real-time updates
  - Advanced search and filtering
  - Role-based access control
  - GitOps-aware editing

## Documentation

Detailed documentation can be found in the `docs` directory:

* [Architecture Decisions](docs/adr) - Architectural decision records
* System Overview (coming soon)
* Integration Guide (coming soon)
* User Guide (coming soon)
* Operations Guide (coming soon)

## Getting Started

### Prerequisites

* Rust 1.75 or later
* MongoDB 7.0 or later
* Node.js 20.0 or later (for UI development)

### Installation

Coming soon

## Development Status

This project is currently in the architectural design phase. Implementation will begin once the core architectural decisions are finalized. Track our progress:

- [x] Initial project setup
- [x] Core architectural decisions
- [ ] Service definition schema
- [ ] Core service registry
- [ ] UI implementation
- [ ] Integration interfaces

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](CONTRIBUTING.md) before submitting a pull request.

### Development Setup

Coming soon

### Commit Guidelines

This project follows [Conventional Commits](https://www.conventionalcommits.org/) to automate versioning and changelog generation. All commits must follow this format:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Types include:
* `feat`: A new feature
* `fix`: A bug fix
* `docs`: Documentation changes
* `style`: Code style changes (formatting, etc)
* `refactor`: Code changes that neither fix bugs nor add features
* `perf`: Performance improvements
* `test`: Adding or updating tests
* `chore`: Routine tasks, maintenance, etc.

## License

This software is released under the Apache License 2.0 with Commons Clause.

### What You Can Do
* ✅ Use the software freely in your business
* ✅ Deploy internally in your organization
* ✅ View and modify the source code
* ✅ Share your modifications
* ✅ Use our software to run your business operations

### What You Cannot Do
* ❌ Sell the software
* ❌ Offer the software as a commercial managed service
* ❌ Resell hosted or cloud versions
* ❌ Commercially distribute the software

## Related Projects

* [phicd](https://github.com/spiralhouse/phicd) - Continuous Delivery tracking system
