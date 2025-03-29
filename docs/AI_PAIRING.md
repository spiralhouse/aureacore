# AI Pair Programming Guide

This document serves as a quick reference for AI assistants (particularly Claude) working on the AureaCore project. It provides context and preferences that would otherwise need to be re-explained after session resets.

## Project Context

AureaCore is a service catalog component of the broader phicd project (https://github.com/spiralhouse/phicd). It helps teams discover, organize, and manage their microservices ecosystem through:
- GitOps-based service definitions
- Centralized and distributed configuration
- Rich metadata management
- Dependency analysis
- Integration with phicd

## Development Environment

### Tools and Configuration
- Operating System: darwin 24.3.0
- Shell: /bin/zsh
- GitHub CLI: Installed and authenticated
- Workspace: /Users/$USER/Projects/aureacore

### Related Repositories
- phicd: /Users/$USER/Projects/phicd
- AureaCore: Current repository

## Development Preferences

### Git Workflow
1. Create feature branches from main
2. Use conventional commits
3. Create focused, single-purpose PRs
4. Include comprehensive PR descriptions

### PR Creation Guidelines
- Use `gh pr create` for creating PRs
- Due to CLI limitations with newlines, use one of these approaches:
  ```bash
  # Approach 1: Using shell variables (preferred)
  pr_description="A description with newlines\nAnd more lines here\nand here..."
  gh pr create --title "type: descriptive title" --body "$pr_description"

  # Approach 2: Using temporary files
  cat > pr-description.txt << EOL
  Your PR description here
  EOL
  cat pr-description.txt | gh pr create --title "type: descriptive title" -F -
  rm pr-description.txt  # cleanup needed
  ```
- Keep PR descriptions focused and well-structured

### Code Organization
- ADRs in docs/adr/
- Documentation in docs/
- Source code structure (coming soon)

### Documentation Standards
- Use markdown for all documentation
- Include links to related documents
- Keep documentation close to code
- Update docs alongside code changes

## AI Assistant Preferences

### Communication Style
- Professional but conversational
- Explain actions before executing them
- Provide options for next steps
- Ask for clarification when needed

### Tool Usage
- Prefer using available tools over asking for manual steps
- Explain tool usage before execution
- Handle errors gracefully
- Clean up temporary files

### Code Changes
- Make focused, purposeful changes
- Explain changes before making them
- Follow project conventions
- Consider impact on existing code

## Common Tasks

### Creating a New Feature Branch
```bash
git checkout -b feature/descriptive-name
```

### Creating a PR
```bash
# Create PR with shell variable
pr_description="Summary of changes\n\nDetailed description of changes:\n- Change 1\n- Change 2\n\nAdditional context or notes"

# Create PR using GitHub CLI
gh pr create --title "type: descriptive title" --body "$pr_description"
```

### Commit Messages
```bash
git commit -m "type(scope): descriptive message"
```

## Project Status

Current phase:
- Initial architecture design
- Core ADRs in review
- Documentation setup
- Preparing for implementation

Next steps:
- Service definition schema
- Core service registry
- UI implementation
- Integration interfaces

## References

- [Project README](README.md)
- [Architecture Decisions](docs/adr/)
- [phicd Repository](https://github.com/spiralhouse/phicd)

## Maintenance

This document should be updated when:
- Development preferences change
- New common tasks are identified
- Project status changes significantly
- New AI-relevant context is needed 