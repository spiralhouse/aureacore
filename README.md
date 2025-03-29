# AureaCore

[![CI](https://github.com/spiralhouse/aureacore/actions/workflows/ci.yml/badge.svg)](https://github.com/spiralhouse/aureacore/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/spiralhouse/aureacore/branch/main/graph/badge.svg)](https://codecov.io/gh/spiralhouse/aureacore)
[![License](https://img.shields.io/badge/License-Apache%202.0%20with%20Commons%20Clause-blue.svg)](LICENSE)

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

AureaCore uses [Task](https://taskfile.dev/) to automate common development workflows. To get started:

1. Install Task:
   ```bash
   # macOS
   brew install go-task

   # Linux
   sh -c "$(curl --location https://taskfile.dev/install.sh)" -- -d -b ~/.local/bin

   # Windows (with scoop)
   scoop install task
   ```

2. Install development tools:
   ```bash
   task setup
   ```

3. Available development commands:
   ```bash
   task                 # Show all available tasks
   task format         # Check code formatting
   task format-fix     # Fix code formatting
   task lint          # Run clippy lints
   task test          # Run tests
   task coverage      # Generate code coverage report
   task deps          # Check dependencies and licenses
   task audit         # Run security audit
   task check-all     # Run all checks
   ```

For more details on each command, run `task --list`

### Dependency Management

AureaCore uses [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) to validate dependencies and licenses. The configuration in `deny.toml` ensures:

* All dependencies use approved licenses (MIT, Apache-2.0, BSD-3-Clause, ISC, MPL-2.0, Unicode-3.0)
* No dependencies from unknown registries or git repositories
* Advisory database checks for security vulnerabilities
* Multiple version warnings for duplicate dependencies

Run `task deps` to check dependencies and licenses.

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
