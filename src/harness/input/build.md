# Build Skill

## Purpose

Execute the implementation plan for the current SDLC feature.

The Build stage is responsible for turning the intent, specification, and implementation plan into a working implementation, including the executable tests described by the plan.

The goal is not merely to modify code or make the project compile.

The goal is to make the **specified behavior work and prove that behavior with tests**.

---

## Required Context

Before changing code, read the complete set of artifacts for the current feature:

```text
intent.md
spec.md
plan.md
```

These files establish:

- **Intent** — why the feature exists and what goal it serves.
- **Specification** — what behavior is required.
- **Plan** — where and how the implementation should be changed.
- **Tests** — how the required behavior will be proven.

Do not begin implementation from the Plan alone.

The implementation must remain consistent with the Intent and Specification.

---

## Step 1 — Understand the Goal

Read the artifacts and determine:

1. What problem is being solved?
2. What behavior is being introduced or changed?
3. What existing behavior must remain unchanged?
4. What files/components are expected to change?
5. What tests demonstrate that the requested behavior works?

Before modifying code, inspect the existing implementation surrounding the planned changes.

Do not assume the plan perfectly describes the current repository state.

If the repository differs materially from the plan, adapt the implementation to the actual codebase while preserving the specified behavior.

---

## Step 2 — Start With the Tests

Write the tests before implementing the behavior whenever practical.

Use the **Verification & Testing Strategy** and **Execution Work Order** in `plan.md`
as the source of truth for the required tests.

For each planned test:

1. Identify the behavior being requested.
2. Translate it into an executable test.
3. Write the test against the public behavior/API that should exist.
4. Run the test.
5. Confirm that the test fails for the expected reason before implementing the feature.

The initial failure should demonstrate that the desired behavior does not yet exist
or is not yet correct.

---

## Step 3 — Implement the Behavior

After establishing the tests, implement the smallest coherent change that makes the required behavior work.

Follow the implementation plan where appropriate, but use the actual repository structure as the source of truth for implementation details.

Prefer:

- existing abstractions,
- existing project conventions,
- small cohesive changes,
- explicit state transitions,
- deterministic behavior,
- reusable domain logic.

Avoid:

- unrelated refactors,
- speculative abstractions,
- duplicating existing functionality,
- changing behavior outside the specification,
- modifying tests solely to accommodate an incorrect implementation.

---

## Step 4 — Iterate Test → Implement → Verify

Work in a tight loop:

```text
Read artifacts
    ↓
Write test
    ↓
Run test
    ↓
Observe failure
    ↓
Implement behavior
    ↓
Run test
    ↓
Fix implementation
    ↓
Run related tests
```

When a test fails:

1. Determine whether the failure is caused by the implementation.
2. If implementation is wrong, fix the implementation.
3. If the test contradicts the specification, reconcile it with the specification.
4. If the specification is ambiguous, do not invent requirements; document the ambiguity or request intervention.

Never simply delete or weaken a failing test because implementation is difficult.

---

## Step 5 — Test the Actual Behavior

Tests should verify the behavior sought by the feature.

Examples of behavior that may require explicit tests:

- state transitions,
- returned values,
- errors,
- persistence,
- serialization/deserialization,
- filesystem changes,
- CLI behavior,
- pipeline decisions,
- retries,
- boundary conditions,
- invalid inputs,
- failure handling,
- integration between components.

For persisted state, verify the actual persisted representation when persistence is part of the specification.

For state machines or pipelines, verify the transition and resulting state rather than only testing that a function returned `Ok(())`.

---

## Step 6 — Run Broader Regression Tests

Once the feature-specific tests pass:

1. Run the relevant module/package tests.
2. Run integration tests.
3. Run the broader project test suite when practical.
4. Run formatting/linting checks required by the project.

A feature is not complete merely because its newly added test passes.

Existing behavior must remain intact unless the specification explicitly changes it.

---

## Step 7 — Review Against the Specification

Before declaring Build complete, reread:

spec/<feature-slug>.md
plan/<feature-slug>.md

Check each requirement against the implementation and executable tests.

Confirm:

- [ ] Required behavior is implemented.
- [ ] Tests exist for the required behavior.
- [ ] Tests initially demonstrated the missing/incorrect behavior where practical.
- [ ] Feature-specific tests pass.
- [ ] Relevant existing tests pass.
- [ ] Failure and boundary behavior is covered where required.
- [ ] Persistence behavior is verified where required.
- [ ] No unrelated behavior was changed.
- [ ] The implementation does not merely satisfy the tests while violating the specification.

---

## Completion Criteria

The Build stage is complete only when:

1. The implementation described by the specification exists.
2. The feature-specific tests have been written.
3. The tests verify the behavior sought by the feature.
4. The tests pass.
5. Relevant regression tests pass.
6. The implementation has been reviewed against the specification.
7. Any deviations, limitations, or unresolved issues are documented.

The Build stage should leave the repository in a state that the Verify stage can independently validate.

## Important Principle

**Do not build code and then ask what to test.**

First understand the requested behavior.

Then write the tests that express that behavior.

Then build the implementation that makes those tests pass.

Tests are not documentation added after implementation. They are an executable expression of the behavior required by the specification.
