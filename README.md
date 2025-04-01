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

Detailed documentation can be found in the [GitHub Wiki](https://github.com/spiralhouse/aureacore/wiki):

* [Getting Started](https://github.com/spiralhouse/aureacore/wiki/Getting-Started) - Installation and quick start guides
* [Architecture](https://github.com/spiralhouse/aureacore/wiki/Architecture) - System architecture and design
* [Guides](https://github.com/spiralhouse/aureacore/wiki/Guides) - How-to guides for common tasks
* [API Documentation](https://github.com/spiralhouse/aureacore/wiki/API-Documentation) - API reference
* [Implementation Plan](https://github.com/spiralhouse/aureacore/wiki/Implementation-Plan) - Phased approach with milestones and tasks
* [Architecture Decision Records](https://github.com/spiralhouse/aureacore/wiki/Architecture-Decision-Records) - Records of architecture decisions
* [Contributing](https://github.com/spiralhouse/aureacore/wiki/Contributing) - Guidelines for contributing to the project

## Getting Started

### Prerequisites

* Rust 1.75 or later
* Node.js 20.0 or later (for UI development)

### Installation

Coming soon

## Development Status

This project is currently in active development. See our [Implementation Plan](https://github.com/spiralhouse/aureacore/wiki/Implementation-Plan) for detailed milestones and progress tracking.

Current progress:
- [x] Initial project setup
- [x] Core architectural decisions
- [x] Service registry foundation
- [x] Schema validation
- [x] Dependency management
- [ ] UI implementation
- [ ] Integration interfaces

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](https://github.com/spiralhouse/aureacore/wiki/Contributing) before submitting a pull request.

### Development Setup

AureaCore supports two development workflows:

#### Option 1: Dev Container (Recommended)

The easiest way to get started is using [Dev Containers](https://containers.dev/). This approach provides a consistent, pre-configured development environment with all required tools and dependencies.

Requirements:
- [Docker](https://www.docker.com/get-started)
- [VS Code](https://code.visualstudio.com/) with the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)

To start developing:
1. Clone the repository
2. Open in VS Code
3. When prompted "Reopen in Container", click "Yes"
   - Or press `F1`, type "Dev Containers: Reopen in Container"

The container includes:
- Rust nightly toolchain with required components
- All development tools (cargo-deny, grcov, etc.)
- Redis server
- Task runner
- Recommended VS Code extensions
- Git and GitHub CLI configuration

#### Option 2: Local Setup

If you prefer a local setup, you'll need to install the required tools manually:

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

### Git Hooks

AureaCore uses Git hooks to ensure code quality:

* **pre-commit**: Runs before each commit
  - Checks code formatting with `cargo fmt`
  - Runs linting with `cargo clippy`

* **pre-push**: Runs before each push
  - Runs all tests with `cargo test`

The hooks are automatically configured when using the dev container. For local setup:

```bash
# Configure Git to use the project's hooks
git config core.hooksPath .hooks
```

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
* `feat`: A new feature (triggers minor version bump)
* `fix`: A bug fix (triggers patch version bump)
* `docs`: Documentation changes
* `style`: Code style changes (formatting, etc)
* `refactor`: Code changes that neither fix bugs nor add features
* `perf`: Performance improvements
* `test`: Adding or updating tests
* `chore`: Routine tasks, maintenance, etc.

Breaking changes indicated with `BREAKING CHANGE:` in the footer or `!` after the type (e.g., `feat!:`) will trigger major version bumps.

For more information on the release process, see [Release Process](docs/RELEASE_PROCESS.md).

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
