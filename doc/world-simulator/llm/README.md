# World-simulator LLM and provider authority

This directory contains durable requirements and contracts for in-world Agent
decision providers.  It is not a task board: mutable task status, assignments,
checklists, execution history, and review evidence belong to the GitHub Issue
and Project for the active task.

## Authority map

- `decision-provider-contract.prd.md` and `.design.md` define the provider-
  agnostic decision boundary: providers propose structured decisions, while the
  runtime validates and executes actions, owns world facts, and returns ordered
  feedback.
- `provider-loopback-http-contract.prd.md` and `.design.md` define the local
  HTTP adapter and its explicit failure/fallback contract.
- `provider-agent-experience-parity.prd.md` and `.design.md` define parity
  evaluation and the conditions for remaining experimental or becoming a
  default experience.
- `provider-agent-dual-mode.prd.md` and
  `provider-agent-dual-mode-contract.md` define the `player_parity` and
  `headless_agent` observation modes.  They share the same action schema and
  runtime validation; only their observation exposure differs.
- `llm-factory-strategy-optimization.*` and `llm-lmso29-stability.*` define
  prompt, typed-decision, recovery, and context-budget safeguards for the
  builtin Agent loop.

## Decision and evidence boundaries

- A provider is a candidate-decision source, never the authority for action
  semantics, resource effects, replay, memory truth, or player promises.
  Unknown, malformed, timed-out, or disallowed provider output must have a
  structured trace and follow the documented safe failure path.
- Prompt/module history, memory digests, raw provider output, token counts,
  latency, and retries are diagnostic or evaluation inputs.  They do not by
  themselves prove parity, cost, replay closure, release readiness, or default
  enablement.
- Evaluation evidence must retain a fixed scenario/fixture/profile/provider/
  adapter/protocol/timeout epoch, per-scenario artifacts, and repeated samples
  for nondeterministic providers.  Aggregate counts or historical samples do
  not replace the current PRD-specific gates.
- `player_parity` evidence must not include headless-only information; a
  headless success does not establish player-experience parity.  Dual-mode
  contract completion likewise does not grant default enablement; that remains
  subject to the parity PRD's behavior and latency gates.

Use the linked PRD/design/contract for durable technical or product claims.
Use GitHub task evidence for the mutable work history that previously appeared
in GitHub task issue evidence comments records.
