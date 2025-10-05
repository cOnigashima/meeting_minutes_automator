---
description: Interactive requirements quality review and validation
allowed-tools: Read, Glob, Grep
argument-hint: <feature-name>
---

# Requirements Validation

Interactive requirements review for feature: **$1**

## Context Loading

### Prerequisites Validation
- Requirements document must exist: `.kiro/specs/$1/requirements.md`
- Spec metadata must exist: `.kiro/specs/$1/spec.json`
- If either is missing, stop with the message: "Run `/kiro:spec-requirements $1` to generate the requirements first."

### Review Context
- Spec metadata: @.kiro/specs/$1/spec.json
- Requirements document: @.kiro/specs/$1/requirements.md
- Steering references (Always load):
  - Product strategy: @.kiro/steering/product.md
  - Architecture overview: @.kiro/steering/structure.md
  - Technology constraints: @.kiro/steering/tech.md
  - Core principles: @.kiro/steering/principles.md
- Custom steering: Load every `.md` in `.kiro/steering/` whose inclusion mode is `Always`.
- **Parent/Umbrella spec alignment**: If `$1` is a sub-spec (check `notes` in `spec.json` or naming convention such as `meeting-minutes-*`), also load:
  - Parent requirements: `.kiro/specs/<parent-feature>/requirements.md`
  - Parent design (if available): `.kiro/specs/<parent-feature>/design.md`
  Example: `meeting-minutes-core`→parent `meeting-minutes-automator`.

Keep track of referenced requirement IDs from the parent spec so gaps or overlaps can be called out.

## Task: Interactive Requirements Quality Review

### Review Focus
Highlight only the issues that would block downstream design or implementation. Limit to **maximum 3 blocking issues**.

### Core Review Criteria

#### 1. Scope & Coverage Alignment (Critical)
- Do the requirements cover every major user goal/business outcome described in the brief and parent spec?
- Are non-goals respected (avoid expanding beyond agreed scope)?
- Are upstream dependencies/assumptions called out (e.g., other sub-spec deliverables, external APIs)?

#### 2. Requirement Structure & Quality
- Introduction summarises problem, stakeholders, and value.
- Requirement sections are grouped coherently (persona, workflow stage, capability, etc.).
- Acceptance criteria use testable EARS patterns (`WHEN/IF/WHILE/WHERE ... SHALL ...`).
  - *EARS recap*: “WHEN [event] THEN [system] SHALL [response]”, “IF [condition] THEN …”, etc.
- Requirements stay solution-agnostic (no design/implementation details embedded).

#### 3. Constraints, Non-Functional & Compliance
- Performance, security/privacy, offline behaviour, accessibility and resource constraints are captured when required by steering.
- Platform or regulatory constraints from steering docs are acknowledged.
- Conflicts, open questions, or TBDs are explicitly tracked.

#### 4. Consistency, Traceability & Metadata
- Terminology matches the glossary and steering docs.
- Each major requirement links back to a parent objective/ID when a parent spec exists.
- `spec.json` reflects the current approval state (`approvals.requirements.generated = true`, etc.).
- Any cross-spec dependencies are listed with their IDs or references.

### Review Process
1. Analyse the requirements document with the above criteria and compare against the loaded steering/parent context.
2. Identify up to **3 blocking issues**. Reference section numbers, IDs or headings where possible.
3. Capture **1–2 strengths** that should be preserved.
4. Decide whether requirements are ready for design (GO) or need revisions (NO-GO).
5. Recommend concrete next steps (e.g., sections to revise, stakeholders to consult, additional requirements to add).

### Output Format
Follow the language defined in `spec.json` (`language` field).

#### Requirements Review Summary
Concise overview of readiness and key observations.

#### Critical Issues (max 3)
For each issue provide:
- **Issue** – short title with reference
- **Impact** – why it blocks progress
- **Recommendation** – actionable fix, including which doc/section to update

#### Strengths
1–2 bullet points highlighting what is working well.

#### Final Assessment
- **Decision**: GO / NO-GO
- **Rationale**: bring together the main reasons
- **Next Steps**: explicit actions required (e.g., “Update Section 3.2 to cover offline queueing”, “Sync with parent spec ID UC-003”).

### Guidance
- Ask clarifying questions instead of assuming when context seems missing.
- Tie every critique back to steering, parent requirements, or business goals.
- Keep feedback specific and testable; reference IDs/sections to make revisions straightforward.
- Reinforce adherence to the design principles (offline-first, security boundary, etc.) as part of the evaluation.

---

## Follow-up Commands
- If the requirements receive a **GO** decision: `/kiro:spec-design $1`
- After revisions: rerun `/kiro:validate-requirements $1` to confirm readiness
