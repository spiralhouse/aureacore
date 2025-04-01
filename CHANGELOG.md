# Changelog

All notable changes to AureaCore will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - ReleaseDate

### Added
- Initial project setup
- Core service registry implementation
- Service Registry Foundation milestone completed
- Schema validation foundation (in progress)
- Dependency tracking between services (in progress)

### Changed
- Updated documentation and README
- Removed MongoDB dependency

### Fixed
- N/A

## [v0.5.0] - 2025-04-01

### Added
- Service-type specific validation checks

### Changed
- Enhanced service validation framework

### Fixed
- Added wiki repository to .gitignore

## [v0.4.0] - 2025-04-01

### Added
- Enhanced validation summary
- Improved mock services for testing

### Changed
- Enhanced service validation with dependency checking
- Improved warning handling in validation process

## [v0.3.6] - 2025-03-31

### Changed
- Updated git2 requirement from 0.18 to 0.20

## [v0.3.5] - 2025-03-31

### Changed
- Maintenance release with synchronization fixes

## [v0.3.4] - 2025-03-31

### Changed
- Updated tower requirement from 0.4 to 0.5
- Updated jsonschema requirement from 0.17 to 0.29

### Fixed
- Updated code for jsonschema 0.29 API changes
- Added MIT-0 license exception for borrow-or-share dependency

## [v0.3.3] - 2025-03-31

### Changed
- Maintenance release with internal improvements

## [v0.3.2] - 2025-03-31

### Changed
- Updated axum requirement from 0.7 to 0.8
- Updated tower-http requirement from 0.5 to 0.6

## [v0.3.1] - 2025-03-31

### Fixed
- Simplified CI workflow by skipping codecov upload for dependabot PRs
- Improved handling of dependabot PRs in codecov workflow

## [v0.3.0] - 2025-03-31

### Added
- Enhanced validation summary with warnings
- Improved display of validation results

### Fixed
- Fixed dependency validation and schema compatibility tests

### Changed
- Enabled Dependabot for automated dependency updates

## [v0.2.1] - 2025-03-31

### Fixed
- Used valid GitHub Actions bot email for releases
- Improved GitHub CI workflows

## [v0.2.0] - 2025-03-31

### Added
- Implemented service schema validation

### Changed
- Added codecov configuration to ignore import lines

## [v0.1.0] - 2025-03-31

### Added
- Initial release
- Project foundation and structure
- Basic documentation and development setup 