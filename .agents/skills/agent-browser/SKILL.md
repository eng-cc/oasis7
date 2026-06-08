---
name: agent-browser
description: Use when automating browser interaction with the local agent-browser CLI, including navigation, forms, screenshots, extraction, authentication, responsive checks, or visual diff workflows.
allowed-tools: Bash(npx agent-browser:*), Bash(agent-browser:*)
---

# Browser Automation

## When to Use

Use this skill when:

- a task needs scripted browser navigation, clicking, typing, screenshots, extraction, or visual comparison
- a local or external web app needs repeatable UI verification outside the in-app Browser plugin path
- authentication, session persistence, viewport emulation, or downloaded artifacts are part of the browser task

Do not use this skill when:

- the Codex in-app Browser plugin is explicitly requested or is the better local target surface
- a direct API, fixture, or unit test can verify the behavior more cheaply than browser automation

## Core Workflow

1. Start with the minimal open/snapshot/action loop and capture refs before interacting.
2. Read `references/full-guidance.md` only for the command family you need: auth, session persistence, screenshots, diffing, locators, native mode, or templates.
3. Prefer bounded outputs and domain/action policies when pages contain untrusted content.
4. Record browser commands and observable results in the task execution log when the browser check is task evidence.

## Supporting Files

- `references/full-guidance.md`: detailed original guidance, examples, patterns, and command/reference material.

## Oasis7-Specific Surfaces

- agent-browser CLI commands documented in `references/full-guidance.md`
- `AGENTS.md` UI Web closed-loop constraints
- `testing-manual.md` S6 when working on oasis7 UI flows

## Known Failure Modes

- Refs from a snapshot are not durable after navigation or DOM changes; re-snapshot before using stale refs.
- Saved auth or browser state can hide first-run bugs; use a clean session when validating onboarding behavior.
- Screenshots alone are weak evidence for data/state changes; pair them with text extraction, diffing, or app-visible state.

## Guardrails

- Keep this entrypoint concise; move heavy examples or catalog material to supporting files.
- Do not bypass oasis7 task/worktree truth or professional role ownership when the workflow requires it.
- Do not present reference material as verified project behavior without checking the current repo state.

## Verification

- Run the exact browser command used for the claim and inspect the returned snapshot/screenshot/diff.
- For skill edits, run `./scripts/lint-skills.sh`.
