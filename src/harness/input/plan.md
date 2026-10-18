# Plan: [One-Sentence Description of the plan which achieves intent]

## 1. Overview

- **Objective:** [What the implementation will accomplish]
- **Source Intent:** [Runtime-supplied reference to the approved Intent]
- **Source Specification:** [Runtime-supplied reference to the approved Specification]
- **Expected Outcome:** [Observable result when implementation is complete]
- **Requirements Addressed:** [Requirement IDs from the Specification]
- **Implementation Strategy:** [Concise summary of the approach, including important dependencies or architectural choices]

The approved Specification defines what must be built. This plan defines how the approved scope will be implemented and verified.

Do not introduce requirements, features, or scope that are not justified by the approved inputs.

## 2. Context & References

### 2.1 Source References

- **Approved Intent:** [Supplied artifact reference]
- **Approved Specification:** [Supplied artifact reference]
- **Applicable Repository Instructions:** [Verified instructions or applicable configuration]
- **Applicable Policies and Constraints:** [Relevant requirements and mandatory restrictions]

Use the artifact references supplied by the runtime. Do not assume fixed artifact directories or machine-specific paths.

### 2.2 Requirements and Assumptions

Summarize the requirements that materially affect implementation.

| Requirement ID | Required Behavior or Constraint | Planning Implication |
|---|---|---|
| [FR-001] | [Approved requirement] | [Implementation or verification consequence] |

List only requirements relevant to the task. Preserve the identifiers used by the Specification; if identifiers are absent, assign stable planning references without changing the approved requirements.

- **Verified Facts:** [Repository or specification facts that are established]
- **Assumptions:** [Unverified assumptions and how they will be resolved]
- **Missing Information:** [Information requiring inspection or clarification]
- **Unresolved Decisions:** [Decisions requiring human or product-owner input]

Distinguish verified facts from assumptions and proposals. Do not silently resolve decisions reserved for a human.

## 3. Proposed Changes

### 3.1 Authoritative File-Change Inventory

Identify every repository file expected to be created, modified, or deleted to implement the approved Specification.

This inventory is authoritative for implementation scope. Every implementation action and execution step must reference files listed here.

#### Files to Create

| File Path | Purpose | Requirements Addressed |
|---|---|---|
| `path/to/new_file.ext` | [Why the file is necessary] | [Requirement IDs] |

If no files need to be created, state `None`.

#### Files to Modify

| File Path | Current Responsibility | Expected Change | Requirements Addressed |
|---|---|---|---|
| `path/to/existing_file.ext` | [Verified existing responsibility] | [Specific proposed change] | [Requirement IDs] |

If no files need to be modified, state `None`.

#### Files to Delete

| File Path | Reason for Removal | Dependencies or References to Update |
|---|---|---|
| `path/to/obsolete_file.ext` | [Why deletion is necessary] | [Verified affected references or dependencies] |

If no files need to be deleted, state `None`.

#### Inventory Rules

- Use repository-relative paths for files inside the repository.
- Verify existing paths before identifying files for modification or deletion.
- Identify proposed new files explicitly as new.
- Include tests, configuration, documentation, migrations, and other supporting files when necessary.
- Prefer existing extension points over unnecessary abstractions and redundant files.
- Do not invent paths, APIs, modules, or test locations.
- If inspection is needed, mark the path `Requires repository inspection` and describe what must be verified.
- Justify every file against an approved requirement or necessary implementation dependency.
- Exclude unrelated files and temporary artifacts that are not deliverables.

### 3.2 Implementation Design

Describe the proposed changes by meaningful component or behavior. Each change must reference files from Section 3.1.

#### [Component or Change Name]

- **Purpose:** [Why this change is necessary]
- **Files:** [Paths from Section 3.1]
- **Action:** [Create, Modify, or Delete]
- **Requirements Addressed:** [Requirement IDs]
- **Existing Architecture:** [Verified components, interfaces, or extension points to reuse]
- **Proposed Changes:** [Concrete structural and behavioral changes, including relevant types, APIs, configuration, or integration points]
- **Dependencies:** [Prerequisite changes and dependent components]
- **Compatibility Considerations:** [Relevant existing behavior or supported environments]
- **Expected Behavior:** [Observable result after implementation]

Repeat for each meaningful implementation change.

Do not write implementation code or pseudo-code. Do not present unverified design assumptions as existing repository facts.

If inspection reveals that additional files are necessary, update the authoritative inventory and its justification before finalizing the plan.

## 4. Verification & Testing Strategy

Define the checks needed to demonstrate that the proposed implementation satisfies the approved requirements.

All planned verification commands must also appear in the machine-readable contract in Section 6.2.

### 4.1 Unit Tests

| Test Location | Behavior Under Test | Expected Result | Requirements Addressed |
|---|---|---|---|
| [Verified existing or proposed test path] | [Specific behavior] | [Observable success condition] | [Requirement IDs] |

Include new or modified test files in the authoritative file inventory.

If unit tests are not applicable, explain why and identify the alternative verification method.

### 4.2 Behavioral, Integration & End-to-End Tests

| Test ID | Command or Procedure | Expected Result | Requirements Addressed |
|---|---|---|---|
| [TEST-001] | [Verified command or reproducible procedure] | [Observable result] | [Requirement IDs] |

Include relevant runtime behavior, CLI execution, application interaction, IPC, persistence, error handling, and integration boundaries.

Do not substitute compilation or a smoke test for required behavioral testing. Include every necessary behavioral check justified by the Specification.

When a required test depends on unavailable infrastructure or requires manual interaction, state the prerequisite and describe the procedure needed to verify it.

### 4.3 Regression & Compatibility Checks

- [ ] **Existing behavior:** [Behavior that must remain functional]
- [ ] **Integration boundary:** [Relevant component or service boundary]
- [ ] **Compatibility:** [Applicable platform, configuration, or API constraints]
- [ ] **Failure handling:** [Relevant error or recovery behavior]

Include only checks relevant to the approved changes.

### 4.4 Risks and Verification Limitations

For each material risk:

- **Risk:** [Potential failure or regression]
- **Impact:** [Consequence]
- **Mitigation:** [Planned mitigation]
- **Verification:** [Check that evaluates the risk]

Identify missing test infrastructure, unresolved commands, skipped checks, and decisions that could block implementation.

A planned check is not a passing check. This document describes intended verification, not completed verification.

## 5. Execution Work Order

List implementation steps in dependency order. Each step must reference the authoritative file inventory in Section 3.1 and contribute to the verification strategy in Section 4.

### Step 1: [Action Name]

- **Files:** [Paths from Section 3.1]
- **Action:** [Concrete implementation work]
- **Requirements Addressed:** [Requirement IDs]
- **Prerequisites:** [Required earlier steps, or None]
- **Verification:** [Specific check and expected result]
- **Completion Condition:** [Observable condition for considering this step complete]

### Step 2: [Action Name]

- **Files:** [Paths from Section 3.1]
- **Action:** [Concrete implementation work]
- **Requirements Addressed:** [Requirement IDs]
- **Prerequisites:** [Required earlier steps]
- **Verification:** [Specific check and expected result]
- **Completion Condition:** [Observable condition for considering this step complete]

Continue until all planned changes and required verification are covered.

### Work-Order Rules

- Order steps according to actual dependencies.
- Include prerequisite changes before dependent work.
- Group independent changes where doing so is practical and safe.
- Include test implementation, configuration, integration, and migration work where required.
- Reference relevant inventory entries rather than creating a separate, conflicting list of files.
- Use actionable steps with observable completion conditions.
- Do not prescribe commands merely because they are familiar; use commands appropriate to the verified repository and host environment.
- If a dependency or decision blocks a step, identify the blocker explicitly.
- Do not claim that any step has been executed or passed.

## 6. Completion Criteria & Machine-Readable Build Contract

### 6.1 Completion Criteria

The implementation may be reported as complete only when:

- [ ] Every approved in-scope requirement has been addressed or explicitly reported as blocked.
- [ ] All planned file changes have been implemented or their status has been documented.
- [ ] Actual changes have been reconciled against the authoritative file inventory.
- [ ] Required unit, behavioral, integration, and regression checks have been executed, with results recorded.
- [ ] Failures, skipped checks, and verification limitations have been disclosed.
- [ ] No unapproved changes or unexplained file additions remain.
- [ ] Relevant unresolved decisions and limitations have been documented.
- [ ] The final implementation and verification evidence supports the completion claim.

These are completion criteria for a future implementation and verification process. Do not mark them complete during planning.

### 6.2 Machine-Readable Build Contract

Provide a valid JSON object using the following schema:

```json
{
  "expected_outputs": [
    "path/to/source_file",
    "path/to/test_file"
  ],
  "verification_commands": [
    {
      "id": "VERIFY-001",
      "argv": ["program", "argument1", "argument2"],
      "purpose": "Describe the behavior or requirement being verified.",
      "requirements": ["FR-001"]
    }
  ]
}
```

#### Contract Requirements

**`expected_outputs`**

- List every repository file expected to exist after implementation that is created or modified by the task.
- Include all planned test files, configuration, documentation, migrations, and other deliverables.
- Use repository-relative paths.
- Keep the list consistent with the Create and Modify inventories in Section 3.1.
- Exclude deleted files and non-deliverable temporary artifacts.
- Use an empty array if no repository files are expected to be created or modified.

**`verification_commands`**

- Include every executable verification command specified in Section 4.
- Represent each command as an `argv` array: the executable first, followed by its individual arguments.
- Specify commands that can be invoked from the repository root.
- Do not include