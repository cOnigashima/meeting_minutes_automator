---
description: Interactive implementation-task quality review and validation
allowed-tools: Read, Glob, Grep
argument-hint: <feature-name>
---

# Implementation Tasks Validation

Run an implementation task quality review for feature: **$1**

## Command Workflow

### 1. Prerequisite Checks
- Ensure `.kiro/specs/$1/tasks.md` exists. If missing, abort with: `Tasks not generated yet. Run /kiro:spec-tasks first.`
- Load `.kiro/specs/$1/spec.json`; if `approvals.tasks.generated != true`, abort with: `Tasks not marked as generated. Complete /kiro:spec-tasks before validation.`
- Load supporting context:
  - Tasks: `@.kiro/specs/$1/tasks.md`
  - Design: `@.kiro/specs/$1/design.md`
  - Requirements: `@.kiro/specs/$1/requirements.md`
  - Spec metadata: `@.kiro/specs/$1/spec.json`
  - Steering (Always): `@.kiro/steering/product.md`, `@.kiro/steering/structure.md`, `@.kiro/steering/tech.md`, `@.kiro/steering/principles.md`
  - Additional steering with `mode: Always` or applicable inclusion rules
- Optional: if `.kiro/specs/$1/tasks.md` is an umbrella spec, load referenced sub-spec metadata for traceability checks

### 2. Review Focus Areas

#### A. Numbering & Structure Compliance
- Major tasks must use sequential integers (`1.`, `2.`, `3.` …)
- Subtasks must use one decimal level (`1.1`, `1.2` …) with max depth of two levels
- Each task/subtask line uses Markdown checkboxes `- [ ]`
- Subtasks must include at least two detail bullets with actionable statements
- Flag duplicate numbers, gaps (e.g., missing 2.1) or nested levels beyond two

#### B. Requirements Coverage & Traceability
- Every subtask must end with `_Requirements: ..._`
- Validate each ID exists in `requirements.md`
- Detect unmapped requirements (IDs present in requirements but not referenced)
- Flag deprecated or mismatched IDs (e.g., case differences, typos)
- For umbrella specs, ensure cross-links to child spec IDs follow traceability matrix

#### C. Sequencing & Dependency Integrity
- Sequence should progress foundation → feature implementation → integration → verification, aligning with `design.md`
- Later tasks must not depend on outcomes of unfinished earlier tasks without explicit ordering
- Raise issues where testing tasks precede prerequisite implementation or where dependencies contradict design assumptions

#### D. Testing Strategy Alignment
- Confirm tasks include test planning/implementation covering required levels from `docs/dev/coding-standards.md` (Unit / Integration / E2E as applicable)
- Highlight missing regression coverage, mock/fake preparation, or observability verification steps
- Check that security/offline scenarios required by steering principles are addressed

#### E. Principle Compliance Spot-Check
- Ensure tasks reference steering principles when relevant (Process Boundary, Offline-First, Security Boundary, Resource Management, Vendor Lock-in)
- Identify tasks that might violate boundaries (e.g., Chrome extension storing tokens)

### 3. Output Template
Return structured review with the following sections:

```
## Findings

### Blocking Issues (max 3)
1. <Issue description, referencing task number and requirement IDs>

### Major Concerns (optional)
- <Non-blocking but serious observations>

### Minor Observations (optional)
- <Nit-level or informational notes>

### Strengths
- <Positive aspects worth keeping>

### Overall Decision
- GO | NO-GO
- Rationale summarizing task readiness

### Recommended Next Actions
1. <Concrete action items grouped by priority>
```

Guidelines:
- If zero blocking issues, leave section empty (but keep header)
- Decision must be `GO` only when no blocking issues remain and coverage is complete
- Limit blocking issues to top three critical problems; aggregate duplicates
- Encourage actionable remediations referencing file paths and lines when possible

### 4. Post-Validation Notes
- Do **not** mutate any files; this command is read-only
- Success criteria: identifying inconsistencies early so `/kiro:spec-tasks` output can be corrected prior to approval
- Future integration: Keep logic modular to enable reuse by guardian agents or CI validations

