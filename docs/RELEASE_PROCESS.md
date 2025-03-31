# Release Process

AureaCore follows [Semantic Versioning](https://semver.org/) and uses automated release processes based on [Conventional Commits](https://www.conventionalcommits.org/).

## Automated Releases

Releases are automatically created when commits are merged into the `main` branch. The version number is determined based on the conventional commit messages:

- `fix:` commits trigger a PATCH release (e.g., 1.0.0 → 1.0.1)
- `feat:` commits trigger a MINOR release (e.g., 1.0.0 → 1.1.0)
- Commits with `BREAKING CHANGE:` in the body trigger a MAJOR release (e.g., 1.0.0 → 2.0.0)

The GitHub Actions workflow handles:
1. Checking if a new release is needed
2. Updating the changelog
3. Incrementing the version number
4. Creating a git tag
5. Publishing a GitHub Release

## Manual Release Process

In some cases, you may need to create a release manually. Follow these steps:

### Prerequisites

Install the required tools:

```bash
npm run install-release-tools
```

or directly:

```bash
cargo install cargo-release cargo-conventional-commits
```

### Preparing for Release

1. Make sure all changes are committed using conventional commits format
2. Ensure the CHANGELOG.md is up to date

### Creating a Release

Run one of the following commands depending on the type of release:

```bash
# For a patch release (bug fixes)
cargo release patch --no-dev-version --execute

# For a minor release (new features)
cargo release minor --no-dev-version --execute

# For a major release (breaking changes)
cargo release major --no-dev-version --execute
```

This will:
- Update version in Cargo.toml
- Update CHANGELOG.md
- Create a git commit with the version changes
- Create a git tag
- Push the changes and tag to GitHub

### Post-Release

After a release, a GitHub Action workflow will automatically create a GitHub Release with the changelog contents.

## Commit Message Guidelines

For the automated versioning to work correctly, all commits should follow the Conventional Commits format:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Common types include:
- `feat`: A new feature (triggers MINOR version bump)
- `fix`: A bug fix (triggers PATCH version bump)
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc)
- `refactor`: Code changes that neither fix bugs nor add features
- `test`: Adding or updating tests
- `chore`: Routine tasks, maintenance, etc.

Breaking changes must be indicated in the footer with `BREAKING CHANGE:` or in the type with an exclamation mark (`feat!:`).

### Examples

```
feat: add service registry validation

fix: prevent registry corruption when services are deleted

docs: update README with new installation instructions

feat!: remove deprecated API endpoint

fix: resolve service lookup issue

BREAKING CHANGE: service lookup now requires authentication
``` 