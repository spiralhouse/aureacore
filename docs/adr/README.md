# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for the AureaCore project.

## What is an ADR?

An Architecture Decision Record (ADR) is a document that captures an important architectural decision made along with its context and consequences. For more information about ADRs, see [ADR-0000: Record Architecture Decisions](0000-record-architecture-decisions.md).

## ADR Index

| Number | Title | Status | Description |
|--------|-------|--------|-------------|
| [0000](0000-record-architecture-decisions.md) | Record Architecture Decisions | Accepted | Establishes the ADR process and format |
| [0001](0001-initial-architecture.md) | Initial Architecture | Proposed | Defines the core architecture and components |
| [0002](0002-backend-language-choice.md) | Backend Language Choice | Proposed | Selection of Rust as the backend language |
| [0003](0003-cap-considerations.md) | CAP Considerations | Proposed | Cache strategy and consistency model |
| [0004](0004-framework-selection.md) | Framework Selection | Proposed | Selection of core frameworks and libraries |

## Statuses

- **Proposed**: The ADR is under discussion
- **Accepted**: The ADR has been approved and is in effect
- **Deprecated**: The ADR is no longer in effect but kept for historical context
- **Superseded**: The ADR has been replaced by another ADR

## Contributing

When adding a new ADR:

1. Copy `template.md` to create your ADR
2. Name it using the format `NNNN-title-with-dashes.md`
3. Update this README to include your ADR in the index
4. Submit the ADR as a pull request for review

See [ADR-0000](0000-record-architecture-decisions.md) for more details on the ADR process.
