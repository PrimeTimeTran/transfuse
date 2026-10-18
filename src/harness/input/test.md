# Test Planning Skill

## Purpose

For every SDLC feature, change, or sprint, create a dedicated test plan that explains **what must be verified, how it will be verified, and what evidence proves the implementation is complete**.

The test plan is a required artifact of the Plan stage and is consumed by the Build and Verify stages.

## Required Artifact

Create:

```text
tests/<feature-slug>.md
```

The `<feature-slug>` must identify the specific feature or sprint being implemented.

Example:

```text
tests/create-sdlc-pipeline.md
```

Do not create a generic `tests.md` for unrelated features.

## Test Planning Rules

Before writing tests:

1. Read `intent/<slug>.md`.
2. Read `spec/<slug>.md`.
3. Read `plan/<slug>.md`.
4. Identify every externally observable requirement in the specification.
5. Map every requirement to at least one verification.
6. Identify failure, boundary, and invalid-input cases where applicable.
7. Identify integration behavior that cannot be adequately verified with a unit test.
8. Prefer deterministic tests over tests that depend on timing, network access, machine state, or external services.

Every important requirement must have a corresponding test or explicit verification.

## Test Categories

Use the categories that apply.

### Unit Tests

Verify isolated functions, methods, types, state transitions, serialization, parsing, and business rules.

For each unit test document:

- What behavior is being tested.
- Given/input state.
- Action being performed.
- Expected result.
- Failure condition being prevented.

### Integration Tests

Verify multiple components working together.

Examples:

- Pipeline → persistence.
- Session → filesystem.
- Evaluator → outcome.
- CLI → pipeline.
- Configuration → runtime behavior.

Document the required setup and expected persisted/output state.

### End-to-End Tests

Use when the feature crosses the complete application boundary.

Document:

- Starting state.
- Command or user action.
- Expected observable behavior.
- Expected artifacts/files/state.
- Cleanup requirements.

### Regression Tests

For every bug fixed during implementation, add a regression test when practical.

Document:

- Original failure.
- Triggering condition.
- Expected corrected behavior.

## Test Case Format

Every concrete test should use this structure:

```markdown
### Test: <name>

**Requirement:** <specification requirement being verified>

**Given:**

- Initial state.

**When:**

- Action performed.

**Then:**

- Expected result.
- Expected persisted state/output.

**Failure indicates:**

- What implementation behavior is incorrect.
```

## Coverage Requirements

Before completing the Plan stage, verify:

- [ ] Every significant specification requirement has a test or explicit verification.
- [ ] Happy-path behavior is covered.
- [ ] Relevant failure paths are covered.
- [ ] Relevant boundary conditions are covered.
- [ ] Persistence/serialization is verified when the feature persists state.
- [ ] State transitions are verified when the feature changes state.
- [ ] Regression cases are included for known bugs.
- [ ] Tests identify the exact expected behavior rather than merely asserting that code runs.

## Implementation Guidance

The Build agent must treat `tests/<feature-slug>.md` as the testing contract.

The Build agent should:

1. Read the test plan before implementing.
2. Implement the feature.
3. Add or update executable tests corresponding to the documented test cases.
4. Run the relevant test suite.
5. Record failures and fixes.
6. Do not mark the feature complete merely because compilation succeeds.

If a documented test cannot be implemented as written, the agent must explain why and either:

- adapt the test to the actual supported behavior, or
- request human intervention if the specification is ambiguous.

## Verification Evidence

The Verify stage must report:

- Tests executed.
- Tests passed.
- Tests failed.
- Commands used.
- Relevant output/evidence.
- Any documented tests that could not be executed.

A feature is not considered verified solely because the application builds.

## Relationship to SDLC Artifacts

The artifacts form this chain:

```text
intent/<slug>.md
       ↓
spec/<slug>.md
       ↓
plan/<slug>.md
       ↓
tests/<slug>.md
       ↓
implementation
       ↓
verification
```

The test plan must remain specific to the feature being implemented.
