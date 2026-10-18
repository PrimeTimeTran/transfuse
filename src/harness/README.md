	 ## AI-Native SDLC playbook
	
	 A reimagined software development process that combines the control objectives
	 of the traditional SDLC with automated enforcement and continuous feedback.
	
	 Instead of treating development as a linear sequence of handoffs, the
	 AI-native SDLC treats it as a controlled loop. AI participates throughout
	 the lifecycle, producing artifacts, executing work, evaluating evidence,
	 and triggering the next stage when its exit criteria are satisfied.
	
	 The fundamental principle is:
	
	 ```text
	 Human intent
	      ↓
	   intent.md     WHY
	      ↓
	    spec.md      WHAT
	      ↓
	    plan.md      HOW
	      ↓
	 Implementation
	      ↓
	 Tests / Evals / Checks
	      ↓
	 Verification
	      │
	      ├── fail ──→ Build → Verify
	      │
	      └── pass
	           ↓
	         Deploy
	           ↓
	        Maintain
	           │
	           └──── new problem / opportunity ────→ intent.md
	 ```
	
	 The process therefore becomes a feedback system rather than a collection
	 of disconnected development phases.
	
	 ### Artifacts
	
	 Each session produces a durable chain of artifacts connecting intent to
	 implementation and evidence:
	
	 - `intent.md` — **why**
	   - Human-defined goal, motivation, scope, constraints, and unknowns.
	 - `spec.md` — **what**
	   - Concrete requirements and behavioral/design specification derived from
	     the intent.
	 - `plan.md` — **how**
	   - Ordered implementation strategy, affected files, dependencies, risks,
	     alternatives, and verification strategy.
	 - `code` — **implementation**
	   - The source changes required to satisfy the specification.
	 - `tests/evals` — **evidence**
	   - Deterministic tests, builds, type checks, linting, browser checks,
	     semantic evaluations, and other verification signals.
	 - `progress.md` — **execution state**
	   - What actually happened during the session, including deviations,
	     failures, retries, discoveries, and verification results.
	
	 The artifacts form a chain of accountability:
	
	 ```text
	 intent.md
	    │
	    ▼
	 spec.md
	    │
	    ▼
	 plan.md
	    │
	    ▼
	 code
	    │
	    ▼
	 evidence
	    │
	    ▼
	 deployment
	 ```
	
	 `progress.md` records the execution of that chain rather than replacing
	 any of the artifacts above.
	
	 ### Session
	
	 A session is one complete attempt to move a unit of work from intent
	 through deployment.
	
	 A session is stored under:
	
	 ```text
	 ./log/sessions/26-09-21.intent-title/
	     intent.md
	     spec.md
	     plan.md
	     progress.md
	 ```
	
	 The directory is the authoritative record of the session.
	
	 `./log/sessions.json` is an index of sessions and their outcomes. It should
	 contain enough metadata to discover and summarize sessions without having
	 to read every artifact.
	
	 A session must:
	
	 - Create a uniquely named directory using the format
	   `YY-MM-DD.intent-title`.
	 - Materialize the session templates from `./ai/template/`.
	 - Adapt those templates to the specific session.
	 - Maintain `progress.md` throughout execution.
	 - Record the final outcome in `./log/sessions.json`.
	
	 The session index is an index, not the source of truth. The session
	 directory contains the authoritative artifacts.
	
	 ### 1. Plan
	
	 Traditional development begins with requirements gathered by committees,
	 workshops, and sign-offs and eventually written into a document.
	
	 The AI-native process begins with an explicit human intent that can be
	 consumed by both humans and agents.
	
	 #### Outcome
	
	 `intent.md` aligns humans and AI on:
	
	 - Goal
	 - Motivation
	 - Scope
	 - Out of scope
	 - Constraints
	 - Success criteria
	 - Known unknowns
	
	 The stage must:
	
	 - Create a new directory in `./log/sessions/` using the session naming
	   convention.
	 - Create `intent.md` from `./ai/template/intent.md`.
	 - Populate it with the intent specific to the session.
	 - Establish the human-approved starting point for the session.
	
	 The agent may clarify or structure the intent, but the human remains the
	 authority on what the system is intended to accomplish.
	
	 ### 2. Design
	
	 The accepted intent is transformed into an explicit specification.
	
	 #### Outcome
	
	 `spec.md` defines what must be true when the work is complete.
	
	 It should describe:
	
	 - Functional requirements
	 - Behavioral requirements
	 - Interfaces and constraints
	 - Relevant architecture
	 - Non-goals
	 - Acceptance criteria
	
	 The stage must:
	
	 - Create `spec.md` from `./ai/template/spec.md`.
	 - Derive the specification from `intent.md`.
	 - Identify ambiguities before implementation begins.
	 - Establish requirements that can later be verified.
	
	 The specification is the contract against which implementation is
	 evaluated.
	
	 ### 3. Build
	
	 The accepted specification is converted into an implementation plan.
	
	 The agent then implements the plan while continuously validating local
	 assumptions.
	
	 #### Outcome
	
	 - `plan.md` describing the implementation strategy.
	 - Source changes implementing the plan.
	 - Tests/evals covering the requirements.
	 - Documentation updated where required.
	 - `progress.md` recording actual execution.
	
	 `plan.md` should describe:
	
	 - Files and components expected to change.
	 - Order of implementation.
	 - Dependencies.
	 - Testing strategy.
	 - Risks.
	 - Alternatives considered.
	 - Expected verification signals.
	
	 Build is not complete merely because code exists. It is complete when
	 there is a candidate implementation that can be submitted to verification.
	
	 The agent may discover information during implementation that invalidates
	 the original plan. Such discoveries should be recorded in `progress.md`
	 and may require the plan or specification to be revised before continuing.
	
	 ### 4. Verify
	
	 Verification is the control system of the SDLC.
	
	 The agent does not get to declare the session complete. The verification
	 system determines whether the implementation satisfies the specification.
	
	 ```text
	                 Build
	                   │
	                   ▼
	             deterministic
	                checks
	                   │
	                   ▼
	             semantic evals
	                   │
	                   ▼
	               evaluate
	              ↙         ↘
	           failure       pass
	             │             │
	             ▼             ▼
	         return to       Deploy
	           Build
	 ```
	
	 Verification should combine multiple kinds of evidence.
	
	 #### Deterministic evidence
	
	 Evidence whose result can be established directly by software:
	
	 - Unit tests
	 - Integration tests
	 - Type checking
	 - Linting
	 - Formatting
	 - Build success
	 - Schema validation
	 - Browser/E2E checks
	 - Security checks
	 - Artifact validation
	
	 These checks should remain deterministic wherever possible.
	
	 #### Semantic evidence
	
	 Some requirements cannot be reduced to simple assertions.
	
	 Examples include:
	
	 - Does the implementation actually satisfy the stated intent?
	 - Does the behavior match the specification?
	 - Is the resulting documentation understandable?
	 - Does the implementation introduce an obvious requirement mismatch?
	
	 These can be evaluated using structured AI evaluation.
	
	 JEV-style evaluations belong here.
	
	 For example:
	
	 ```text
	 intent.md
	     │
	 spec.md
	     │
	 implementation / diff
	     │
	 verification evidence
	     │
	     ▼
	   JEV evaluation
	     │
	     ├── requirements satisfied?
	     ├── intent aligned?
	     ├── blocking issue?
	     └── remaining risk?
	 ```
	
	 JEV provides semantic evidence; it does not replace deterministic tests
	 or human accountability.
	
	 #### Verification loop
	
	 When verification fails:
	
	 ```text
	 Verify
	   │
	   ├── deterministic failure ──→ Build
	   │
	   ├── semantic failure ───────→ Build
	   │
	   └── human-required decision ─→ Human
	                                  │
	                                  ▼
	                                Build
	 ```
	
	 The loop continues until the required verification gates pass or a human
	 explicitly intervenes.
	
	 #### Outcome
	
	 A successful verification produces an evidence record containing the
	 results of the checks and evaluations.
	
	 For example:
	
	 ```text
	 tests:             passed
	 build:             passed
	 typecheck:         passed
	 lint:              passed
	 intent_alignment:  evaluated
	 requirements:      evaluated
	 blocking_issue:    evaluated
	 risk:              evaluated
	 ```
	
	 Verification therefore answers:
	
	 > "What evidence do we have that the implementation satisfies the
	 > specification?"
	
	 rather than:
	
	 > "Does the agent say that it is finished?"
	
	 ### 5. Deploy
	
	 Deployment moves a verified change from the development state into its
	 intended environment.
	
	 Human review and governance remain explicit control points where required.
	 The purpose of the automated lifecycle is not to remove human judgment,
	 but to ensure that humans are reviewing a well-defined artifact with
	 reproducible evidence.
	
	 #### Outcome
	
	 - Verified implementation is deployed.
	 - Deployment metadata is recorded.
	 - `./log/sessions.json` is updated with the session outcome.
	 - A `./doc/.changeset` entry is created when the project requires one.
	 - The deployed commit/version/artifact is associated with the session.
	
	 Administrative bookkeeping should be generated from the session rather
	 than reconstructed manually after deployment.
	
	 ### 6. Maintain
	
	 Maintenance closes the lifecycle loop.
	
	 Production is observed for:
	
	 - Bugs
	 - Regressions
	 - Operational failures
	 - Security issues
	 - New requirements
	 - User feedback
	 - Opportunities for improvement
	
	 A production signal does not merely create a bug ticket. It can become
	 the input to a new SDLC session.
	
	 ```text
	              Deploy
	                 │
	                 ▼
	              Maintain
	                 │
	        ┌────────┴────────┐
	        │                 │
	     healthy          new signal
	        │                 │
	        │                 ▼
	        │             new intent
	        │                 │
	        └─────────────────┘
	                  │
	                  ▼
	               Plan
	 ```
	
	 #### Outcome
	
	 - Production state is observed.
	 - Issues and opportunities are captured.
	 - Significant new work produces a new `intent.md`.
	 - The new intent begins another session.
	
	 Git commits are part of preserving and identifying the resulting state,
	 but committing code is not the purpose of Maintain. Maintain exists to
	 close the feedback loop between the deployed system and future intent.
	
	 ## Control model
	
	 The AI-native SDLC separates three different forms of authority:
	
	 ```text
	 Human judgment
	      │
	      │ defines intent / approves decisions
	      ▼
	 Agent execution
	      │
	      │ implements and iterates
	      ▼
	 Verification system
	      │
	      │ produces evidence
	      ▼
	 Deployment gate
	 ```
	
	 These should not be conflated.
	
	 - **Human** decides what should be built and where judgment is required.
	 - **Agent** performs the implementation work.
	 - **Deterministic tooling** establishes facts about the implementation.
	 - **AI evaluation** provides structured semantic evidence where deterministic
	   checks are insufficient.
	 - **Verification** determines whether the required evidence has been
	   satisfied.
	 - **Deployment** occurs only after the required gates pass.
	
	 The central design principle is therefore:
	
	 > AI performs the work, artifacts preserve the reasoning, verification
	 > establishes evidence, and humans retain authority over intent and
	 > consequential decisions.
	
	 ## Reference
	
	 - https://claude.com/blog/the-ai-native-sdlc-playbook
	let foo = 1;
	//  ┌─────────────── SESSION ────────────────┐
	//  │                                        │
	//  │ intent → spec → plan → implementation  │
	//  │                         │               │
	//  │                         ▼               │
	//  │              deterministic checks      │
	//  │                         │               │
	//  │                         ▼               │
	//  │                    JEV eval            │
	//  │                         │               │
	//  │                 ┌───────┴───────┐      │
	//  │                 │               │      │
	//  │               FAIL            PASS     │
	//  │                 │               │      │
	//  │                 └──→ BUILD      │      │
	//  │                                 ▼      │
	//  │                              DEPLOY    │
	//  │                                 │      │
	//  └─────────────────────────────────┼──────┘
	//                                    │
	//                                    ▼
	//                                MAINTAIN
	//                                    │
	//                                    ▼
	//                                 INTENT
