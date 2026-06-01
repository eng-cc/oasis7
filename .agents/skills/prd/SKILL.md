---
name: prd
description: 'Generate high-quality Product Requirements Documents (PRDs) for software systems and AI-powered features. Includes executive summaries, user stories, technical specifications, and risk analysis.'
license: MIT
---

# Product Requirements Document (PRD)

## Overview

Design comprehensive, production-grade Product Requirements Documents (PRDs) that bridge the gap between business vision and technical execution. This skill works for modern software systems, ensuring that requirements are clearly defined.

## Oasis7 Workflow Binding

In oasis7, this skill is a specialist planning surface, not a standalone workflow. TPM must bind PRD work to the same owner role, `.pm` task, canonical worktree, and PR chain before repository writeback, but TPM only coordinates that binding; the planning/domain conclusion must be owned by the appropriate professional slice.

- Write PRD outputs to repo-owned docs such as `doc/<module>/prd.md` or `doc/**/**.prd.md`.
- Record the PRD route, TODOs, and downstream handoff in `.pm/tasks/<TASK-UID>.execution.md`.
- Do not treat PRD-only output as implementation-ready unless the workflow router has also confirmed project/task execution truth and verification entry.

## When to Use

Use this skill when:

- Starting a new product or feature development cycle
- Translating a vague idea into a concrete technical specification
- Defining requirements for AI-powered features
- Stakeholders need a unified "source of truth" for project scope
- User asks to "write a PRD", "document requirements", or "plan a feature"

---

## Operational Workflow

### Phase 1: Discovery (The Interview)

Before writing a single line of the PRD, you **MUST** interrogate the user to fill knowledge gaps. Do not assume context.

**Ask about:**

- **The Core Problem**: Why are we building this now?
- **Success Metrics**: How do we know it worked?
- **Constraints**: Budget, tech stack, or deadline?

### Phase 2: Analysis & Scoping

Synthesize the user's input. Identify dependencies and hidden complexities.

- Map out the **User Flow**.
- Define **Non-Goals** to protect the timeline.

### Phase 3: Technical Drafting

Generate the document using the **Strict PRD Schema** below.

---

## PRD Quality Standards

### Requirements Quality

Use concrete, measurable criteria. Avoid "fast", "easy", or "intuitive".

```diff
# Vague (BAD)
- The search should be fast and return relevant results.
- The UI must look modern and be easy to use.

# Concrete (GOOD)
+ The search must return results within 200ms for a 10k record dataset.
+ The search algorithm must achieve >= 85% Precision@10 in benchmark evals.
+ The UI must follow the 'Vercel/Next.js' design system and achieve 100% Lighthouse Accessibility score.
```

### Developer-Ready Completeness (Mandatory)

A PRD is not "ready for implementation" unless it is detailed enough that engineering and QA can execute without oral补充.

You MUST explicitly cover:

- **Functional Spec Depth**: interaction flow, field definitions, button behavior, state transitions, sorting/computation rules, permission logic.
- **Edge Cases**: network failure, empty data, permission denied, timeout, concurrency conflict, data corruption fallback.
- **NFR with Targets**: performance, compatibility, security, expected data scale, extensibility constraints.
- **Testability**: acceptance criteria, done definition, validation method, regression impact scope, and PRD-ID -> Task -> Test traceability.
- **Decision Record**: chosen approach, rejected alternatives, and evidence/rationale.

### Document Boundary (PRD vs Project vs Devlog)

Use the following split and avoid duplicate definitions:

| Doc | Focus | Must Not Include |
| --- | --- | --- |
| PRD (`doc/<module>/prd.md`, `doc/**/**.prd.md`) | `Why/What/Done` (scope, behavior, acceptance, NFR, risks, decisions) | task breakdown, daily progress |
| Project (`doc/**/**.project.md`) | `How/When/Who` (tasks, dependency, owner, status, execution plan) | rewriting target-state requirements |
| Devlog (`doc/devlog/YYYY-MM-DD.md`) | immutable daily record (timestamp, done, pending, blockers) | requirement definitions |

Guardrails:
- `Critical User Flows` describe system/user/operator runtime behavior, not "rewrite/migrate docs" steps.
- `Acceptance Criteria` describe verifiable outcomes (behavior/invariants/metrics/tests), not file existence.
- `Roadmap/Milestones` describe shipped capability phases (MVP/v1.1/v2.0), not daily to-do updates.

Quick contrast:
- Bad: `Critical User Flows: read old doc -> rewrite -> update project doc -> verify commit`
- Good: `Critical User Flows: user triggers X -> system validates -> state transitions -> user sees Y`

For review gate items and veto conditions, use `check.md` as the canonical checklist.

---

## Strict PRD Schema

You **MUST** follow this exact structure for the output:

### 1. Executive Summary

- **Problem Statement**: 1-2 sentences on the pain point.
- **Proposed Solution**: 1-2 sentences on the fix.
- **Success Criteria**: 3-5 measurable KPIs.

### 2. User Experience & Functionality

- **User Personas**: Who is this for?
- **User Scenarios & Frequency**: When and how often each persona uses this feature.
- **User Stories**: `As a [user], I want to [action] so that [benefit].`
- **Critical User Flows**: Step-by-step flow for key paths.
- **Functional Specification Matrix**: fields, button actions, state transitions, sorting/computation rules, and permission behavior.
- **Acceptance Criteria**: Bulleted list of "Done" definitions for each story.
- **Non-Goals**: What are we NOT building?

### 3. AI System Requirements (If Applicable)

- **Tool Requirements**: What tools and APIs are needed?
- **Evaluation Strategy**: How to measure output quality and accuracy.

### 4. Technical Specifications

- **Architecture Overview**: Data flow and component interaction.
- **Integration Points**: APIs, DBs, and Auth.
- **Edge Cases & Error Handling**: network, timeout, empty/invalid data, concurrency, fallback behavior.
- **Non-Functional Requirements**: measurable performance, compatibility, security/privacy, scale, extensibility targets.
- **Security & Privacy**: Data handling and compliance.

### 5. Risks & Roadmap

- **Phased Rollout**: MVP -> v1.1 -> v2.0.
- **Technical Risks**: Latency, cost, or dependency failures.

### 6. Validation & Decision Record

- **Test Plan & Traceability**: PRD-ID -> Task -> `test_tier_required` / `test_tier_full` mapping.
- **Decision Log**: selected option, rejected options, and rationale/evidence.

---

## Implementation Guidelines

### DO (Always)

- **Define Testing**: For AI systems, specify how to test and validate output quality.
- **Iterate**: Present a draft and ask for feedback on specific sections.
- **Specify Edge Conditions**: Include failure and boundary handling, not only happy path.
- **Quantify NFRs**: Every non-functional constraint should be measurable.
- **Record Decisions**: Capture tradeoffs and why alternatives were rejected.

### DON'T (Avoid)

- **Skip Discovery**: Never write a PRD without asking at least 2 clarifying questions first.
- **Hallucinate Constraints**: If the user didn't specify a tech stack, ask or label it as `TBD`.
- **Ship Ambiguous Specs**: If implementation still needs oral clarification, PRD is incomplete.

---

## Example: Intelligent Search System

### 1. Executive Summary

**Problem**: Users struggle to find specific documentation snippets in massive repositories.
**Solution**: An intelligent search system that provides direct answers with source citations.
**Success**:

- Reduce search time by 50%.
- Citation accuracy >= 95%.

### 2. User Stories

- **Story**: As a developer, I want to ask natural language questions so I don't have to guess keywords.
- **AC**:
  - Supports multi-turn clarification.
  - Returns code blocks with "Copy" button.

### 3. AI System Architecture

- **Tools Required**: `codesearch`, `grep`, `webfetch`.

### 4. Evaluation

- **Benchmark**: Test with 50 common developer questions.
- **Pass Rate**: 90% must match expected citations.

# Check written PRD
Use the following checklist to evaluate the quality of a written PRD. 
check.md contains a detailed checklist template for PRD review, covering aspects such as business goals, user definition, scope control, and functional specifications. Each item should be marked with ✔ / ❌ / ⚠, and critical items must be fully checked to proceed with development.

## Review Gate (Mandatory)

After drafting, you MUST run a self-review against `check.md` and output:

1. Item-by-item marks (`✔ / ❌ / ⚠`) with evidence.
2. Overall gate result: `🟢 Ready` / `🟡 Risky` / `🔴 Not Ready`.
3. If not `🟢`, list concrete remediation actions before implementation.

Critical dimensions that cannot be `❌`:
- Scope definition
- Functional specification completeness
- Edge case coverage
- Testability
