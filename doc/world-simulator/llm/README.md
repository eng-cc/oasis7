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
- `continuous-agent-harness.prd.md` and `.design.md` define the Agent-owned
  cognition lifecycle above that provider boundary: session/turn/request
  identity, provider-neutral context and feedback isolation, memory-write
  policy, bounded continuation, and one Harness shared by Builtin and
  ProviderBacked implementations. Runtime scheduling, action MVCC, durable
  cognition journal, recovery, and receipts remain owned by the paired
  `doc/world-runtime/runtime/agent-cognition-lifecycle.*` contract.
- The Continuous Harness target uses a non-recursive V1 `request_digest`
  (canonical outer context without its output digest or `transport_attempt`) and a global per-agent
  active-session/turn mutex; session IDs partition state but do not permit
  concurrent turns for one Agent. `committed/rejected/failed/pending` are the
  only feedback statuses; cancellation and scheduler backpressure map through
  explicit terminal/pending reasons and late-response cleanup.
- Loopback target decision traffic uses the `ContinuousAgentRequestContextV1` /
  `ContinuousAgentResponseContextV1` outer wrappers; old DecisionRequest/Response and feedback
  DTO bodies are accepted only in an explicitly marked `compatibility_lane=legacy_v1`.
- The target response wrapper preserves tagged `wait/wait_ticks/act/query/module_command/
  module_command_response` variants; Query is read-only, and typed module responses require host
  validation before Runtime handling.
- Target memory writes use `MemoryWriteIntentV1`; omitted summary is explicit `present=false`
  (present summaries are non-empty after normalization); tags may be an empty list, but empty tag
  elements are invalid. The legacy inner DTO is compatibility-lane only.
- P0 memory write scope is limited to `turn_private` and `session_private`; the reserved
  `agent_private_long_term` scope is disabled by default and deferred to P2 authority.
- Continuation proposals remain Harness policy inputs; Runtime projections carry the shared
  `FinalityBindingV1`/world bindings, and proposal/context digests use the paired `H_v1` registry.
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
- Legacy DTOs and heuristic fallback are compatibility-only lanes: they require
  explicit lane markers and `legacy_no_cognition_proof`/`legacy_heuristic_used`,
  and cannot satisfy production async, target parity, or automatic
  memory/continuation proof. Target traces and feedback use fixed byte/count
  bounds, deterministic redaction, and negative overflow fixtures.
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
