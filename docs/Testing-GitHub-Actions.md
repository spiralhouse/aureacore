# Testing GitHub Actions Locally

This document provides instructions on how to test GitHub Actions workflows locally using the `act` tool.

## Prerequisites

1. Install [act](https://github.com/nektos/act) on your system:
   - macOS: `brew install act`
   - Linux: Download from GitHub releases
   - Windows: Use WSL or download from GitHub releases

2. Docker must be installed and running on your system

## Setup

1. Create a `.act` directory in the project root (this is already included in the repository):
   ```
   mkdir -p .act
   ```

2. Create secrets file for authentication if needed:
   ```
   echo "GITHUB_TOKEN=your_github_token" > .act/.secrets
   ```

3. Add `.act/.secrets` to your `.gitignore` file to avoid committing sensitive tokens

## Available Test Workflows

The repository includes the following test workflows:

- `.act/release-test.yml`: Tests the release workflow with simplified steps
- `.act/changelog-script-test.yml`: Tests the update-changelog.js script

## Running Tests

To run a test workflow:

```bash
# On macOS ARM64 systems, specify the correct container architecture
act -W .act/changelog-script-test.yml --container-architecture linux/amd64 

# To use a specific event file
act -W .act/release-test.yml --container-architecture linux/amd64 --eventpath .act/event.json
```

## Debugging

If you encounter issues:

1. Run with `-v` for verbose output: `act -v -W .act/release-test.yml`
2. Check Docker logs with `docker logs [container-id]`
3. Use `-n` flag to dry-run: `act -n -W .act/release-test.yml`

## Limitations

- Some GitHub features may not be fully supported in the local environment
- Third-party actions may require real GitHub authentication
- Complex workflows might need to be simplified for local testing

## Troubleshooting

- If authentication issues occur, consider creating mock versions of actions
- For environment variables, create a `.env` file and use `--env-file`
- For Docker permissions issues, ensure your user is in the docker group 