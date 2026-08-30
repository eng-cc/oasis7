# Production Supervisor Runtime — Target Design

Version: **v0.1.0**
Last Updated: **2026-08-30**
Status: **non-normative design companion**

> This document describes a target runtime design. It is not a workflow
> specification, task ledger, implementation claim, release decision, or
> permission to bypass a gate. The normative authority is the [Engineering
> Workflow Source of Truth](./source-of-truth.md#capability-and-ownership),
> including [lifecycle ownership](./source-of-truth.md#lifecycle-ownership),
> the [canonical state machine](./source-of-truth.md#canonical-state-machine),
> [workflow states](./source-of-truth.md#workflow-states),
> [ready and done gates](./source-of-truth.md#ready-and-done), and the
> [pre-PR review packet](./source-of-truth.md#pre-pr-review-packet).

## 1. Purpose and authority boundary

The target is a durable executor for the existing lifecycle, not a replacement
for the lifecycle. It should allow a task to resume from durable evidence after
process, session, network, model, or CI interruption while preserving the
existing task, worktree, branch, review, merge, and cleanup authorities.

TPM remains the accountable workflow coordinator, integrator, and continuation
owner. The supervisor is a target runtime executor, not an accountability owner.
Professional roles retain domain conclusions, and `qa_engineer` retains release
blocking judgment. The supervisor may adjudicate mechanical identity, ordering,
freshness, and receipt validity, but it must route semantic disagreement to the
matching professional role through TPM.

GitHub Issues, Project fields, and task-issue evidence comments remain the
task-truth and formal evidence surfaces defined by the [GitHub Project-backed PM
contract](./source-of-truth.md#123-github-project-backed-pm-contract). Runtime
state may cache execution progress and wake delivery, but it must project to
those authorities rather than create a second mutable task database.

## 2. Current capability and target

The current production surface is intentionally blocked. The supervisor can
create and identity-bind a checkpoint, but it does not advance through a
trusted bootstrap, action, validator, collaboration, or wake connector. The
production action and validator adapters fail closed; fixture reducers and
fake-GitHub paths are test-only. See the [capability status](./source-of-truth.md#capability-status)
and the operational [supervisor skill](../../../.agents/skills/tpm-production-supervisor/SKILL.md).

The target is still subject to the same closed state enum and lifecycle order.
Target behavior must not be described as implemented until the promotion
criteria in section 11 are independently evidenced.

## 3. Exactly four trusted producer classes

The target has exactly four trusted producer classes. A class is a trust
boundary, not a workflow state or a role title.

| Producer class | Target responsibility | Independent proof boundary |
| --- | --- | --- |
| Trusted mechanical/bootstrap action | Execute a fixed, task-bound allowlisted action for bootstrap, freeze, verification, PR, merge, sync, or cleanup. | Repository-owned executable identity, task/repository/state/lease/head binding, durable action receipt, and live effect readback. |
| Independent live validator/readback | Resolve current Git, task-truth, GitHub, PR-gate, or filesystem facts and validate an action or receipt. | A validator authority distinct from the caller's assertion; fixtures and self-signed receipts are never production proof. |
| Trusted collaboration/attestation | Dispatch a professional slice and return attested identity, attempt, bounded artifact, and artifact digest. | Runtime-issued dispatch/return evidence bound to the task, slice, epoch, dispatch acknowledgement, actual agent, and artifact. |
| Wake/scheduler runtime | Persist a resume condition and deliver an event or bounded timer wake across turns and process restarts. | Runtime-owned lease, delivery identity, condition, deadline, and readback; a JSON mutation by a caller is not delivery. |

The four classes are jointly necessary. A collaboration runtime cannot unblock
unattended work without mechanical execution, independent validation, and a
real wake owner. A live evaluation is acceptance evidence for the runtime, not
a fifth producer class.

## 4. Durable execution model

### 4.1 Checkpoint, CAS, lease, and fencing

The supervisor owns one canonical checkpoint for the task and active bootstrap
epoch. Creation is exclusive and identity-bound to the task UID, repository,
canonical worktree, branch, state path, and current lifecycle phase. It never
overwrites an existing checkpoint or accepts caller-selected foreign state.

Every transition uses compare-and-swap over the expected revision and lease.
The active lease supplies a fencing token; a takeover advances the fencing
identity so a stale process, delayed wake, or zombie agent cannot commit a
result. A transition that cannot prove its expected revision, lease, epoch,
task identity, or current head fails closed and leaves the prior durable state
intact.

Checkpoint recovery is readback-driven. A crash or uncertain mutation first
reconciles the durable journal and live authority; it does not create a second
checkpoint, silently reset state, or infer completion from a process exit code.

#### Durable checkpoint and journal contract

The target checkpoint is a versioned `tpm-supervisor-checkpoint/v1` envelope.
Its required identity fields are `task_uid`, `repository`,
`canonical_worktree`, `task_branch`, `default_branch`, `checkpoint_path`,
`bootstrap_epoch`, and `evidence_epoch` (the latter may be `null` only during
bootstrap before the first acceptance, plan, producer, or review authority
exists).  Its controller fields are the closed
`workflow_state`, implementation `phase`, `next_result`, `revision`, the
`lease` record, `head_oid` when a head is bound, `event_head_digest`,
`transition_head_digest`, `state_digest`, and `journal_generation`.  The
checkpoint also carries `rejection_head_digest`, which chains classified
input rejections without treating them as lifecycle progress.  The
checkpoint is authoritative only when its schema, identity, revision, and
digests validate together; a cache or a caller-selected path is not a
checkpoint.

The journal uses `tpm-supervisor-journal/v1` records with
`journal_id`, `logical_action_id`, `idempotency_key`, `operation`,
`state`, `intent_digest`, `effect_digest`, `readback_digest`,
`revision_before`, `revision_after`, `bootstrap_epoch`, `evidence_epoch`,
`lease_id`, `fencing_token`, and producer/observation timestamps.  The only
effect states are `intent`, `acted`, `readback_validated`, `committed`, and
`conflict`; `conflict` is terminal for that logical action until an
authorized new epoch.  A durable intent is written before an external call;
an acted response is written before attempting readback; a committed journal
record and checkpoint transition are written only after the independent
validator proof is accepted for an effect-bearing action.  A pre-action
capability block or classified input rejection is a non-effect transition: it
has no action, producer receipt, validator proof, or external effect and
records only the blocker/rejection.  An uncertain call is therefore recovered
by the same idempotency key and live readback, never by guessing from process
exit status.

Every local mutation is one lock-held transaction: acquire the persistent
task lock, read and validate the current checkpoint, compare
`revision`/`lease_id`/`fencing_token`, append the journal record, write a
temporary file, flush and `fsync` it, atomically replace the destination, and
`fsync` the containing directory.  The journal and checkpoint head digests
must be mutually cross-bound.  A failed CAS returns the current revision and
fencing identity without changing the prior bytes.  The external call is
bounded by the lease; if the lease expires before its effect can be fenced or
read back, the result is `conflict` and requires reconciliation.

The lease record contains `lease_id`, runtime `holder_id`, strictly
monotonic `fencing_token`, `acquired_at`, `renewed_at`, `expires_at`, and the
trusted clock source.  Acquire is allowed only when no lease exists or the
current lease is proven expired under that clock; renew is allowed only for
the current lease before expiry; takeover is a CAS that advances the fencing
token and records the superseded lease.  Every action, wake delivery, and
external mutation carries the lease/fencing pair.  The corresponding live
authority must reject a stale token before applying or accepting an effect;
checking the token only when committing the local checkpoint is insufficient.

Checkpoint corruption is recovered by validating the newest complete
generation and its journal digest chain.  Exactly one valid candidate may be
selected; ambiguity, a missing generation, or an invalid chain is
`capability_blocked` with the bytes preserved for audit.  A schema migration
is a repository-owned, journaled transaction that writes a new version while
retaining the old bytes/digests as historical evidence; if the migration
changes task/worktree/branch or authority identity, it also starts the
required new bootstrap epoch.  It never edits a receipt or silently
interprets an old schema as current.  Compaction is permitted only after all
retained actions are
`committed`, `conflict`, or explicitly terminal: it keeps the latest
checkpoint, event/transition head digests, invalidation records, terminal
tombstones, and unresolved-effect records, and atomically publishes the new
generation.  It is forbidden during an active effect, lease takeover, or
uncertain readback.

### 4.2 Typed events, actions, receipts, and transitions

The target reducer consumes validated typed events and emits exactly one typed
next action or terminal outcome. Conceptually, the closed workflow state maps
to one of `Execute(action)`, `Dispatch(slice)`, `Wait(condition)`,
`Complete(receipt)`, `CapabilityBlocked(reason)`, or `Fail(reason)`; these are
runtime result types, not additions to the canonical workflow-state enum.
`Complete(receipt)` is reserved for a canonical terminal outcome such as
`post_merge_done` or `closed_without_merge`; an intermediate merge, task-done,
main-sync, or cleanup receipt cannot complete the task by itself.

An action is produced by the repository-owned action builder and contains a
stable action identity, phase, task UID, checkpoint revision, lease/fencing
identity, current head, executable identity, and required inputs. Its receipt
records the producer, command/result artifacts, exit status, timestamps, and
artifact digests. The supervisor does not treat an exit code or caller-authored
JSON as proof.

An independent validator reads the live effect and validates the receipt's
identity and schema. Only then may the reducer append a transition containing
the validated observation and advance the checkpoint. External mutations use
idempotency keys plus live readback; duplicate suppression alone is insufficient.

#### Versioned envelopes

The following are target wire/storage contracts. Producers may add fields only
under a new schema version; unknown required fields, missing identity, or
schema/version mismatch fail closed. All use canonical UTF-8 JSON, UTC RFC
3339 timestamps, and lowercase SHA-256 digests.

`evidence_epoch` is nullable only on the bootstrap envelope before the first
acceptance, plan, producer, or review authority is bound; it is concrete on
all later route/action/receipt/proof/collaboration/wake/transition envelopes.
`supersedes` is nullable only on attempt 1 (including initial bootstrap);
every replacement names the immediately superseded attempt. Null never means
unknown. A pre-action capability-block transition has no action, receipt, proof, or external effect and carries only its typed blocker/rejection reason.

#### Digest preimages and exclusions

Every digest is `SHA-256(utf8("oasis7/tpm/<schema>\n" + canonical_json(preimage)))`;
canonical JSON sorts object keys by Unicode code point, removes whitespace,
normalizes numbers, and preserves specified array order. The digest being
computed, framing, signatures/authentication headers, secrets, raw payloads,
local paths, and mutable arrival order are excluded; signatures authenticate the resulting digest and are never hashed into the bytes they sign.

Event preimages contain identity/epoch/sequence/causation, parent event head, and
typed payload; `payload_digest` hashes payload-only JSON. Action preimages
contain logical action/attempt/epoch/operation, target, expected revision/head,
key, retry policy, required-input, and producer/executable digests. Receipt
preimages contain action/attempt, effect ID/digest/target, request/result
digests, status, exit status, and observed identity, excluding proof fields.
Effect preimages contain action/attempt/key, operation/target, request/
requested-effect digests, status/result, and producer identity, excluding
receipt/proof/readback/case references. Readback preimages contain action/
effect/query/authority identity, target, expected/observed digests, fencing,
status, and result, excluding receipt/proof/case references. Proof preimages
contain receipt/effect/readback digests plus live observation, expected/observed
effect, fencing, result, and authority. Transition preimages contain parent
heads, event/action identity, receipt/proof digests, from/to state/phase,
revisions, outcome, and state digest. Receipt is independent; proof binds it;
transition binds both without a cycle.

Wall-clock audit fields and transport delivery times are excluded from semantic
preimages; deadlines use logical clock ticks. Referenced digests
(`payload_digest`, `required_inputs_digest`, `effect_digest`,
`readback_digest`, `receipt_digest`, `validator_proof_digest`) are included
when named, but the digest currently computed is never included. At generation
one, all parent heads use the fixed 64-zero hexadecimal genesis digest; heads
are never omitted or `null` and later envelopes carry the preceding digest.

| Envelope | Required fields and invariants |
| --- | --- |
| `tpm-supervisor-event/v1` | `event_id`, `event_type`, `task_uid`, `repository`, `canonical_worktree`, `bootstrap_epoch`, `evidence_epoch`, `source_class`, `source_id`, `sequence`, `parent_event_digest`, `checkpoint_revision`, `lease_id`, `fencing_token`, `occurred_at`, `ingested_at`, `causation_id`, `payload_digest`, and typed `payload`; `sequence` is monotonic within the task/epoch and duplicate `event_id` is ignored only when its full digest matches. |
| `tpm-supervisor-action/v1` | `action_id` (logical identity), `attempt_id`, `attempt`, `supersedes`, `phase`, `operation`, `producer_class`, `task_uid`, `repository`, `canonical_worktree`, `task_branch`, `bootstrap_epoch`, `evidence_epoch`, `expected_revision`, `lease_id`, `fencing_token`, `expected_head_oid` when bound, `expected_base_oid`/`comparison_oid` when applicable, `comparison_ref` when applicable, `required_inputs_digest`, `idempotency_key`, `created_at`, `deadline`, `retry_budget`, and executable/producer digest; retries preserve `action_id` and key while replacing only attempt fields. |
| `tpm-supervisor-receipt/v1` | `receipt_id`, `action_id`, `attempt_id`, `attempt`, `supersedes`, `phase`, `task_uid`, `repository`, `canonical_worktree`, `task_branch`, `bootstrap_epoch`, `evidence_epoch`, `head_oid`/`base_oid` when bound, `lease_id`, `fencing_token`, `idempotency_key`, `effect_id`, `effect_digest`, `effect_target`, `request_digest`, `effect_status`, `exit_status`, `result_artifact_digests`, `observed_effect_digest` when an effect was observed, `producer_id`, `producer_class`, executable/protocol digest, and `observed_at`; its independent `receipt_digest` is an envelope-header digest computed outside the body preimage, and the receipt contains no validator-proof field. It is a locator for proof, never proof merely because it has exit status zero. |
| `tpm-validator-proof/v1` | `proof_id`, `validator_id`, `validator_class`, `authority_id`, `key_id`, authenticated signature/transport evidence, `receipt_digest`, `effect_digest` when an effect exists, `readback_digest`, `live_query_id`, `live_observation_digest`, `observed_identity`, `expected_effect_digest`, `observed_effect`, `observed_fencing_token`, `result` (`accepted`, `rejected`, or `unknown`), and `observed_at`; `accepted` requires an authority distinct from the action caller and a live readback matching the requested effect. |
| `tpm-supervisor-transition/v1` | `transition_id`, `event_id`, `action_id` when applicable, `receipt_id` when applicable, `receipt_digest` when a receipt exists, `parent_event_digest`, `parent_transition_digest`, `parent_rejection_digest`, `from_state`, `to_state`, `from_phase`, `to_phase`, `expected_revision`, `new_revision`, lease/fencing identity, `validator_proof_digest` when a proof exists, typed `outcome_code`, `state_digest`, and `committed_at`; one CAS commit appends at most one transition for one logical event/action. A pre-action capability-block or classified rejection has no receipt/proof and uses its typed outcome code. |
| `tpm-supervisor-effect/v1` | `effect_id`, `action_id`, `attempt_id`, `attempt`, `supersedes`, `task_uid`, `repository`, `canonical_worktree`, `task_branch`, `bootstrap_epoch`, `evidence_epoch`, `lease_id`, `fencing_token`, `idempotency_key`, `operation`, canonical `effect_target`, `request_digest`, `requested_effect_digest`, closed `effect_status` (`intent`, `acted`, `absent`, `unknown`, `conflict`, `rejected`, or `readback_validated`), closed `result` (`pending`, `accepted`, `rejected`, `absent`, `unknown`, or `conflict`), `external_effect_id` when assigned, `observed_effect_digest` when observed, `producer_id`, `producer_class`, executable/protocol digest, and audit timestamps; the independent `effect_digest` is an envelope-header digest, and this envelope has no receipt, validator-proof, or case digest. `accepted` describes an external authority response only; it is not independent proof. |
| `tpm-supervisor-readback/v1` | `readback_id`, `authority_id`, `validator_id`, `validator_class`, `task_uid`, `repository`, `canonical_worktree`, `task_branch`, `bootstrap_epoch`, `evidence_epoch`, `action_id`, `attempt_id`, `attempt`, `idempotency_key`, `lease_id`, `expected_fencing_token`, `effect_id` when applicable, `query_id`, canonical `effect_target`, `expected_effect_digest`, `observed_effect_digest` when observed, `observed_identity`, `observed_fencing_token`, closed `readback_status` (`complete`, `unavailable`, `rejected`, or `stale`), closed `result` (`match`, `absent`, `mismatch`, `unknown`, `rejected`, or `stale`), authenticated transport/signature evidence, and `observed_at`; `validator_id` must name an authority distinct from the action producer. The independent `readback_digest` is an envelope-header digest, and this envelope has no receipt, proof, or case digest. `match` means the live observation equals the expected effect; it is not a lifecycle transition until a validator proof is accepted. |
| `tpm-collaboration-action/v1` | `dispatch_id`, `slice_id`, `role`, `task_uid`, both epochs, packet/scope/integration digests, `action_id`, `attempt_id`, `attempt`, `supersedes`, `deadline`, `idempotency_key`, and the supervisor checkpoint/lease/fencing identity; it is a derived action envelope whose dispatch acknowledgement must be runtime-issued. |
| `tpm-collaboration-return/v1` | `dispatch_id`, `slice_id`, `role`, `task_uid`, both epochs, `action_id`, `attempt_id`, `attempt`, `started_at`, `returned_at`, bounded artifact path/digest, disposition/findings, runtime identity, runtime attestation/key metadata, and `supersedes`; it is accepted only after live attestation and artifact readback. |

An effect ID is runtime-derived from logical action/attempt/key/target. Its
record progresses monotonically from `intent` to `acted`, `absent`, `unknown`,
`conflict`, `rejected`, or `readback_validated`; readback may refine but cannot
rewrite request, target, key, or prior observation. A readback retry has a new
ID linked by query/attempt fields. Neither envelope advances lifecycle; only a
transition with independent proof does.

Receipt `effect_status` is `not_attempted|acted|readback_validated|unknown|
conflict|rejected`; `committed` is journal/transition state, not producer assertion.
Valid effect pairs are `intent/pending`, `acted/pending`,
`acted/accepted`, `absent/absent`, `unknown/unknown`, `conflict/conflict`,
`rejected/rejected`, or `readback_validated/accepted`; others reject. Valid
readback pairs are `complete` with `match|absent|mismatch`,
`unavailable/unknown`, `rejected/rejected`, or `stale/stale`; `match` still
needs independent proof.
Transition `outcome_code` uses the repository-owned catalog (including
`accepted`, `producer_missing`, `retry_exhausted`, `return_exhausted`,
`stale_return`, `stale_timeout`, `stale_fencing`, `identity_mismatch`,
`receipt_mismatch`, `stale_receipt`, `rejected(category)`, `replay_mismatch`, `wake_expired`, `wake_cancelled`, and
`unresolved_effect`); additions require a schema/validator update.

Cross-bindings are one-way to avoid cycles: action binds effect by
`action_id`/attempt/key/target/request digest; effect binds only action/key;
receipt references `effect_digest`; readback references action/effect and
expected digest; proof references receipt/effect/readback digests; transition
references receipt/proof/state digests; and case references transition/effect/
readback digests. Effect/readback never reference downstream digests, and
proof/transition digests never enter the state-hash preimage.

`workflow_state` is always one of the closed states in the canonical source;
`phase`, `event_type`, `operation`, and blocker reason are separate typed
fields.  `source_class`, `producer_class`, `validator_class`, `authority_id`,
and key metadata are proof inputs, not caller-provided labels.  The runtime
must verify the executable/protocol digest and the validator's authenticated
identity against a repository-owned allowlist before accepting any envelope.

#### Replay envelope and canonical state hash

The read-only `tpm-supervisor-replay/v1` envelope consumes only validated
events/proofs, migration records, and the initial checkpoint; it never repeats
an external action, wake, or cleanup. Required fields are `replay_id`,
task/repository/worktree identity, both epochs, `checkpoint_schema`, ordered
`input_event_digests`, `input_proof_digests`, revision-ordered
`input_rejection_digests`, `migration_digests`, `deterministic_clock`,
`random_seed`, `runtime_id_map`, initial/final state digests,
`transition_count`, revision-keyed `state_digests`, zero
`external_effect_calls`, and `observed_at`. Input order is event sequence,
causal dependency, then `event_id`; indeterminate order blocks replay. The
clock is logical, the seed explicit, and runtime IDs use the recorded map.

`state_digest` uses the exact algorithm
`SHA-256(utf8("oasis7/tpm/tpm-supervisor-state-hash/v1\\n" +
canonical_json(preimage)))`.  The preimage is this ordered semantic object;
all keys are present, with `null` only where the envelope contract permits it:

```json
{
  "schema": "tpm-supervisor-state-hash/v1",
  "task_uid": "...", "repository": "...", "canonical_worktree": "...",
  "task_branch": "...", "bootstrap_epoch": "...", "evidence_epoch": null,
  "workflow_state": "...", "phase": "...", "next_result": {},
  "revision": 0, "head_oid": null, "base_oid": null, "comparison_oid": null,
  "event_parent_digest": "...", "transition_parent_digest": "...",
  "active_action": null, "wait": null, "effect_states": [],
  "rejection_parent_digest": "...", "terminal_outcome": null
}
```

`active_action` contains logical action/key/attempt/effect status; `wait`
contains typed condition, logical deadline tick, and delivery state;
`effect_states` is sorted by logical action/attempt/target; and the rejection
parent covers rejections committed before this event. Event/transition/
rejection fields use parent-chain, never soon-to-be-published head, digests.
Envelope digests, signatures, transport metadata, leases/fencing, journal
generation, paths, wall clocks, and runtime IDs normalized by `runtime_id_map`
are excluded but remain independently checked safety/audit data.

Replay compares state digest after every transition and at final state. The
first unsupported schema/migration or expected-vs-actual preimage mismatch
emits `capability_blocked(replay_mismatch)` with revision/event and expected/
actual digests plus the retained artifact; it advances no lifecycle state and
invokes no external effect. Differences only in excluded metadata are
equivalent; semantic array order or omitted fields are not.

Commit order is normative: validate event/receipt, compute `state_digest` from
pre-transition parent heads, compute transition digest from it plus independent
receipt/proof digests, then publish new heads in one CAS. No preimage may refer
to the head digest it is about to publish.

#### State and phase transition matrix

The matrix below is a complete reducer contract.  Phase names are
implementation detail; they do not add workflow states.  Every row consumes
one validated event and emits one result, and every transition is subject to
the CAS/lease/fencing rules above.

| Current state | Phase/event guard | Single reducer result | Next phase or terminal outcome |
| --- | --- | --- | --- |
| `action_required` | Authorized consumer starts the recorded action with matching action/revision/lease | `Execute(action)` or `Dispatch(slice)` | `running` in the same phase; no caller may substitute a new action |
| `action_required` | Required producer or validator is unavailable before the recorded action is issued | `CapabilityBlocked(producer_missing)` | `capability_blocked`; no action, receipt, proof, wake, or external effect |
| `running` | Validated action receipt and validator proof | `Execute(next)` or `Dispatch(next slice)` | Next lifecycle phase in the phase matrix below |
| `running` | Trusted temporary external condition with a durable deadline | `Wait(condition)` | `external_wait`; wake metadata is persisted before returning |
| `running` | Required producer/validator/attestation is unavailable | `CapabilityBlocked(reason)` | `capability_blocked`; no synthetic receipt or wake subtype |
| `running` | A validated action outcome is non-retryable (not an input-envelope rejection) | `Fail(reason)` | `failed`; resume requires authorized new evidence epoch or fresh bootstrap |
| `external_wait` | Authenticated wake/event whose delivery and checkpoint CAS match | `Execute(next)` or `Dispatch(next slice)` | `running`; stale/duplicate delivery does not mutate state |
| `capability_blocked` | Newly available trusted capability proves the same epoch and identity | `Execute(action)` or `Dispatch(slice)` | `action_required` or `running`, according to the recorded consumer boundary |
| `failed` | No ordinary resume event | `Fail(reason)` | Remains terminal for the epoch; only a new epoch/fresh bootstrap can proceed |
| `completed` | No valid lifecycle event is accepted | `Complete(receipt)` | Remains terminal; `post_merge_done` or `closed_without_merge` only |
| Any state, including terminal states | Input envelope fails schema, identity, epoch, revision, lease, fencing, scope, ordering, or authority validation | `Reject(code)` | Same state/phase; rejection is recorded, with no lifecycle progress, external effect, or proof |

| Phase family | Required validated evidence | Result and next phase |
| --- | --- | --- |
| `bootstrap` | Task-truth, owner, mapping, repository, worktree, branch, and checkpoint readback | `Execute(bootstrap)` then `Dispatch(route)` |
| `route` | Task-bound route and slice contracts | `Dispatch(slice)` for `dispatch` |
| `dispatch` / `execute` | Runtime dispatch acknowledgement and bounded slice return/attempt state | `Wait(condition)` until returns are due; then `Dispatch` retry/replace or advance to `integrate` |
| `integrate` | All required returns are attested, digest-valid, and scope-compatible | `Execute(freeze)`; incomplete slices wait, while exhausted/invalid slices become the classified retry or failure outcome, never a merge bypass |
| `freeze` | Frozen implementation head and comparison/base identity | `Execute(verify)` at `draft_candidate` |
| `verify` | Trusted exact-head CI receipt and planner authority | `Dispatch(review)` / `review` |
| `review` | All required role returns and dispositions for the same frozen head/epoch | `Execute(closeout)` / `pre_pr_ready` |
| `closeout` | Repository helper receipt plus task-truth/readback of the review packet | `Execute(promote_draft)` / `pre_pr_ready` |
| `create_pr` / `record_pr` / `comment` | Repository helper receipt plus independent task/PR and issue-comment readback | `Wait` for the live PR gate / `pr_watch` |
| `watch` | Current-head required checks, mergeability, reviews, comments, threads, and holds | `Wait` for a temporary condition, `Dispatch(fix)` for actionable findings, or `Execute(merge)` only on the live gate receipt |
| `fix` / `reverify` / `push` | Current-head fix artifact and fresh verification/readback | Return to `review`/`watch`; any head change creates a new evidence epoch |
| `merge` / `merge_receipt` | Live merged PR receipt bound to the reviewed head/epoch | `Execute(task_done)`; an intermediate merge receipt is not terminal |
| `task_done` / `main_sync` / `safe_cleanup` | Ordered task truth, main-sync, cleanup journal, and finalizer readbacks | `Complete(post_merge_done)` only after finalization |
| Any permitted early non-merge entry | Classified reason, bounded evidence, verified task completion, and terminal tombstone | `Complete(closed_without_merge)` through the canonical non-merge finalizer |

The final two rows preserve the canonical terminal order and the early
non-merge route; no `merge`, `task_done`, `main_sync`, or `safe_cleanup`
receipt alone can produce `completed`.

For a merged task, `post_merge_done` is valid only after the canonical
`merge receipt -> task done -> main sync -> safe cleanup receipt ->
post-merge finalize` readbacks.  The merged path retains the task issue open
until finalization.  For a classified non-merge task, `closed_without_merge`
requires the evidence-bound non-merge receipt/ledger and the terminal
`checkout_recreation_forbidden: true` tombstone.  These are the same terminal
authorities and resume rules defined by the [canonical terminal
runbook](./source-of-truth.md#terminal-runbook); this design does not add a
second finalizer.

#### Deterministic timeout, rejection, and reconciliation outcomes

`Reject(code)` is a reducer result for an input that is syntactically
representable but fails identity, schema, epoch, revision, lease, fencing,
scope, ordering, or authority validation.  The runtime appends a classified
rejection transition with `from_state == to_state` and
`from_phase == to_phase`, increments only the durable revision needed to
record that transition, and performs no external effect or lifecycle
progress.  An unparseable input is wrapped by the trusted ingress with its
raw-byte digest and the same classified rejection transition; it is never
silently dropped.  Rejection transitions have no action, receipt, or
validator proof.

The reducer resolves races under the task lock by comparing revision, logical
action/attempt, epoch, and fencing token.  The first valid CAS wins; a later
event for the superseded identity is a rejection, not a second transition.
The outcomes are deterministic:

| Trigger and guard | Durable result | Lifecycle/effect outcome |
| --- | --- | --- |
| Producer is unavailable before an action is issued | `CapabilityBlocked(producer_missing)` transition with typed blocker | `capability_blocked`; no action, receipt, validator proof, wake, or external effect |
| Action deadline expires with no trusted effect result | `Wait(reconcile)` plus durable timeout/readback intent | `external_wait`; query the canonical target by the same key before retry or classification; no phase advance |
| Timeout readback finds the requested effect | `acted` then `readback_validated`; issue an independent validator proof | Commit the normal one-CAS transition only after proof; never issue a second effect |
| Timeout readback proves the effect absent and retry budget/deadline remains | `Wait(retry)` with a new attempt linked by `supersedes` | `external_wait`; same logical action/key, no lifecycle advance |
| Timeout readback is contradictory or unqueryable | `Fail(unresolved_effect)` and journal `conflict` | `failed`; no second mutation, promotion, or cleanup effect |
| Retry budget or logical deadline is exhausted after absent/temporary results | `Fail(retry_exhausted)` with last observation and escalation authority | `failed`; no phase advance and all later returns are stale |
| Collaboration return deadline expires while attempts remain | `Wait(return_retry)` and a replacement dispatch attempt | `external_wait`; same logical dispatch/action identity, no integration |
| Collaboration return deadline expires with no attempts left | `Fail(return_exhausted)` | `failed`; a later return cannot advance the phase |
| Late return arrives after timeout replacement/failure | `Reject(stale_return)` transition bound to the late digest | No integration, state progress, or external effect; retain artifact for reconciliation |
| Return and timeout race before either CAS | The event whose expected revision/fencing CAS succeeds is canonical | The loser is `Reject(stale_return)` or `Reject(stale_timeout)`; exactly one transition/effect path |
| One delivery attempt expires while logical wake budget remains | Expire that delivery record and schedule the next attempt by the same wake/key | `external_wait` unchanged; no consumer invocation or lifecycle progress |
| Logical wake deadline expires, or an authorized owner cancels with no replacement event | `Fail(wake_expired)` or `Fail(wake_cancelled)` and terminal delivery record | `failed`; no consumer invocation or external effect; a replacement event must win in the same CAS to avoid failure |
| Wake cancellation races a trusted replacement event | Persist cancellation and replacement under one CAS | Replacement event alone advances the lifecycle; the cancelled wake cannot consume or invoke a consumer |
| Reconciliation after lease takeover finds committed matching digest | Return the existing committed transition and receipt | No new mutation or transition; higher fencing token only records the takeover/readback |
| Reconciliation after takeover finds an acted matching effect | Record readback and obtain independent proof, then commit once if current revision permits | One normal transition; stale holder result is rejected |
| Reconciliation after takeover finds no effect with budget remaining | Create the next attempt with the same key | `external_wait`; no duplicate logical effect |
| Reconciliation finds a contradictory effect or stale fencing token | `Fail(unresolved_effect)` or `Reject(stale_fencing)` as classified | No acceptance, lifecycle progress, or new external effect |

## 5. Evidence epochs and invalidation

An evidence epoch binds the task/bootstrap identity and, where applicable, the
comparison base, implementation head, acceptance/plan digest, producer
authority, and review/CI authority. Receipts, slice returns, review ledgers,
claims, and gate decisions are valid only inside their bound epoch.

Any task identity, mapping, owner, acceptance, active authority, scope, role,
packet, plan, base, head, producer authority, or conclusion change creates a
new epoch and invalidates downstream evidence. An ordinary Project lifecycle
status observation does not create an epoch unless it changes a bound authority.
Invalidation records why the prior artifact is stale while retaining its
historical bytes. A changed head must loop through fresh verification and
review; an epoch must never be refreshed across semantic or authority drift.

Envelope identity follows two levels of epoch.  `bootstrap_epoch` identifies
the task/worktree/branch/checkpoint identity and changes only through the
repository-owned bootstrap or same-UID identity-migration path.  The
`evidence_epoch` identifies the acceptance, base/head, producer, and
review/CI authorities for a lifecycle attempt.  An action or event that is
valid before an epoch change is retained as historical evidence but is not
eligible for integration after it.  Invalidation is itself a
`tpm-supervisor-transition/v1` record with the stale artifact digest, reason,
superseding epoch, and authority that made the decision.

## 6. Retry, idempotency, and recovery boundaries

Transport retry preserves the logical slice or action identity, packet, scope,
and epoch while incrementing an attempt. Each delivery or receipt attempt may
have a new identity linked to the prior attempt with `supersedes`. It is
permitted for delivery loss, timeout, or transient transport failure. CAS
permits only one canonical state transition for the logical identity.

Semantic change is not a transport retry. A changed task truth, head/base,
acceptance, role, write scope, packet, or requested operation requires a new
slice/action identity and evidence epoch. Results from the old identity cannot
be integrated into the new one.

Every external mutation must be resumable by an idempotency key and independent
readback. The runtime must distinguish an uncommitted intent, an acted effect
awaiting readback, a committed effect, and an unresolvable conflict. It must
not promise exactly-once execution of an external system; the target is
at-least-once delivery with exactly-once durable state transition.

The idempotency key is runtime-derived, never accepted as a caller-selected
trust root: `task_uid / bootstrap_epoch / evidence_epoch-or-none /
logical_action_id / canonical-effect-target`.  It is stable for every
transport retry of that logical action and changes for a semantic operation,
new target, or new epoch.  `attempt_id` is unique per delivery, `attempt` is
monotonic from one, and `supersedes` links a replacement attempt to the
previous attempt.  A receipt is accepted only when its action, key, epoch,
expected revision, and current fencing pair match; a duplicate with the same
receipt digest returns the already-committed transition, while a stale or
out-of-order attempt returns `stale_receipt` without a state mutation.

Each operation has an explicit deadline, retry budget, backoff policy, and
transient/permanent error classifier in its action.  A transient failure or
`Retry-After` response remains `external_wait` while budget and deadline
remain; a missing trusted producer remains `capability_blocked`; a proven
non-retryable contract violation becomes `failed`.  Budget exhaustion never
advances the phase and must retain the last live observation and escalation
authority.  When an external response is lost, readback first queries the
canonical effect target by the same key.  Found effect -> `acted`/validated;
absent effect -> retry the same key; contradictory or unqueryable results ->
`conflict` and fail closed.  A successful local transition is not evidence
that an external mutation happened.

#### Crash and restart recovery matrix

| Crash boundary | Recovery action | Forbidden shortcut |
| --- | --- | --- |
| Before durable intent | Rebuild the pending action from the checkpoint and write the same intent/key | Calling a new key or advancing the phase |
| After intent, before external response | Query the effect target by key, then retry the same key only if absent | Assuming no effect because the process exited |
| After effect/response, before readback | Persist/locate the `acted` record and independently validate the live effect | Treating the caller response as proof |
| After validated readback, before checkpoint commit | Revalidate lease/revision and commit one CAS transition, or reconcile after takeover | Appending a second transition |
| After checkpoint commit, before response | Read the committed receipt/transition and return it idempotently | Repeating the external mutation |
| Corrupt or torn checkpoint | Select exactly one valid generation from the digest chain; otherwise remain blocked with bytes preserved | Resetting, deleting, or creating a second checkpoint |
| Lease expiry during an action or wake | Take over with a higher fencing token, reconcile by key, and reject the stale result | Accepting a late result from the old holder |

Recovery is readback-driven for both local and external effects.  Every
recovery result records the selected journal generation, prior/new revision,
lease/fencing identity, live observation digest, and whether the logical
action was committed, retried, or left in conflict.

#### Migration, replay, compaction, and CAS fault operations

A schema migration runs under the task lock and a current lease with no active
effect or unresolved readback.  It validates the old envelope and digest
chain, applies a pure repository-owned transform, and appends a
`migration/v1` journal record containing old/new schema versions, old/new
checkpoint and journal digests, the transform executable/protocol digest,
and the migration authority.  The new generation is written, flushed,
`fsync`ed, atomically published, and directory-`fsync`ed before a final
readback and CAS.  The old generation and bytes remain immutable history.
Unsupported, ambiguous, identity-changing, or partially published migrations
remain `capability_blocked`; an identity change starts a new bootstrap epoch
and cannot reuse old actions or proofs.

Compaction is an atomic, journaled operation, never an in-place rewrite.  It
requires no active `intent`, `acted`, `readback_validated`, takeover, or
uncertain delivery; every omitted action is `committed`, `conflict`, or
explicitly terminal.  The compaction manifest records the omitted journal-ID
range, retained head digests, state digest before/after, invalidation records,
terminal tombstones, and unresolved-effect records.  Compaction is accepted
only when it changes the physical generation without incrementing lifecycle
revision or appending a lifecycle transition and the semantic state digest is
identical before and after; replay of
the pre- and post-compaction generations must therefore produce the same
final digest.  A crash selects exactly one complete generation; two valid
generations with different heads, or no valid generation, remain blocked with
all bytes retained.

| Fault boundary | Required durable outcome | Forbidden shortcut |
| --- | --- | --- |
| CAS expected revision or lease/fencing mismatch | Return current revision/identity and append a classified rejection record | Overwriting the current checkpoint or retrying with a new logical action |
| Crash before journal write/flush | Existing checkpoint remains authoritative; rebuild the same action/key | Inferring an external effect from process exit |
| Crash after journal flush before checkpoint publish | Recover journal intent/acted state by key, then either validate once or retry the same key | Publishing a second checkpoint generation without reconciliation |
| Crash after checkpoint replace before directory `fsync` | Validate complete generation and cross-digest chain; choose exactly one valid generation | Choosing by file timestamp or deleting the older bytes |
| Journal/checkpoint cross-digest mismatch | Preserve both generations and enter `capability_blocked(corrupt_chain)` | Repairing a digest or silently dropping a record |
| Migration transform or final readback fails | Keep old generation authoritative and record blocked migration evidence | Treating a partially written new schema as current |
| Compaction manifest or before/after state hash mismatch | Keep the pre-compaction generation and retain the attempted manifest | Dropping records to make compaction appear complete |
| Replay input, migration, or state-hash mismatch | Emit `capability_blocked(replay_mismatch)` with first mismatch details | Replaying an external effect or accepting the final state anyway |

## 7. Collaboration and wake runtime

`action_required` means that a task-bound authorized consumer must execute the
recorded typed action. It is distinct from `external_wait`, where an available
trusted authority has a temporary condition and durable resume path, and from
`capability_blocked`, where required machinery or attestation does not exist.

The collaboration boundary consumes `tpm-collaboration-action/v1` and returns
only `tpm-collaboration-return/v1` evidence bound to dispatch acknowledgement,
actual runtime identity, attempt, task/slice/epoch identity, and artifact
digest. A plan, echoed payload, static adapter pin, or caller-provided
`agent_id` is not runtime attestation. Missing collaboration or attestation
machinery remains `capability_blocked`.

After a trusted dispatch exists, a temporary absent return may be represented
by the single canonical `external_wait` state with wait metadata: `wait_class`,
authority, task/bootstrap epoch, checkpoint revision/fencing identity, resume
condition, wake policy, deadline/retry budget, and delivery identity. A missing
producer is not an `external_wait` subtype and must not add states such as
`external_wait.system` or `external_wait.policy`.

The wake owner persists task UID, checkpoint/revision, lease/fencing identity,
condition, deadline, and delivery ID. It can resume on a trusted event or
bounded timer, revalidates the lease and current checkpoint, and rejects stale
or duplicate delivery. The wake owner remains installed for a wait; active
turn polling is not a substitute for scheduler delivery. These rules refine,
but do not replace, the [canonical state contract](./source-of-truth.md#workflow-states).

#### Phase-to-producer and validator trust matrix

The target has exactly one executor producer class and one independent
validator boundary for each phase family. A phase may use more than one
operation, but it may not invent a fifth trust class or accept its executor's
assertion as its own validation.

| Phase or operation | Executor producer class | Independent validator/readback authority |
| --- | --- | --- |
| `bootstrap` | Trusted mechanical/bootstrap action | Task-truth, Project mapping, repository/worktree/branch readback |
| `route` | Trusted collaboration/attestation | Task-bound route and dispatch acknowledgement readback |
| `dispatch`, `execute`, `integrate` | Trusted collaboration/attestation | Runtime dispatch/return attestation, artifact digest, scope and integration-barrier readback |
| `freeze` | Trusted mechanical/bootstrap action | Git identity, frozen tree, branch and base/head readback |
| `verify` | Trusted mechanical/bootstrap action | Independent CI/check planner receipt bound to the frozen head |
| `review` | Trusted collaboration/attestation | Task-issue review ledger, role/slice identity, artifact digest, and head/epoch readback |
| `closeout`, `create_pr`, `record_pr`, `comment` | Trusted mechanical/bootstrap action | GitHub task/PR identity and issue-comment readback |
| `watch` | Trusted mechanical/bootstrap action | Independent live PR-gate readback of checks, mergeability, reviews, threads, comments, and holds |
| `fix` | Trusted collaboration/attestation | Runtime return attestation plus current-head artifact and scope readback |
| `reverify`, `push` | Trusted mechanical/bootstrap action | Repository-owned verification/Git remote readback at the same head/epoch |
| `merge`, `merge_receipt` | Trusted mechanical/bootstrap action | Live GitHub merge/readback receipt bound to PR, head, and gate epoch |
| `task_done` | Trusted mechanical/bootstrap action | Task-issue and Project task-truth readback |
| `main_sync` | Trusted mechanical/bootstrap action | Default-worktree Git remote/ancestry or patch-equivalence readback |
| `safe_cleanup` | Trusted mechanical/bootstrap action | Filesystem/worktree/common-dir and cleanup-journal readback |
| `external_wait` delivery, takeover, or cancellation | Wake/scheduler runtime | Runtime-owned delivery identity, checkpoint CAS, lease/fencing, and event readback |

The executor receipt and validator proof are separate artifacts. The proof
lifecycle is: the reducer issues an action only after a checkpoint CAS; the
producer returns a versioned receipt; the independent validator resolves the
live effect and issues `tpm-validator-proof/v1`; the reducer commits one
transition containing that proof digest; and any authority/head/epoch change
invalidates both receipt and proof. A validator identity is registered by a
repository-owned authority record with an authenticated key/transport,
rotation and revocation metadata, and an executable/protocol digest. A static
adapter pin, local self-signature, caller-selected executable, or echoed
`agent_id` is not a proof lifecycle.

#### Wake event and delivery contract

The wake owner stores a `tpm-supervisor-wake/v1` record with `wake_id`,
`delivery_id`, task/checkpoint identity, bootstrap/evidence epoch,
`expected_revision`, `lease_id`, `fencing_token`, `condition_digest`,
`not_before`, `deadline`, `attempt`, `supersedes`, `delivery_state`, and
authenticated scheduler-owner metadata. Delivery records use
`scheduled -> due -> delivering -> delivered -> consumed`; `expired` and
`cancelled` are terminal delivery-record outcomes, not workflow states.

Only the runtime scheduler can install or cancel a wake after a checkpoint
CAS. It derives `not_before` and deadlines from a trusted monotonic clock,
retaining UTC timestamps only for audit; a caller-supplied clock or JSON
mutation is not a wake. A trusted event or due timer is delivered with the
same task/epoch/revision/fencing identity. The consumer acknowledgement must
bind `wake_id`, `delivery_id`, action/slice identity, and receipt digest; the
owner persists that acknowledgement before one CAS-bound consume transition.

Delivery loss or process exit before consume reuses the logical delivery and
idempotency key, increments `attempt`, and may issue a new delivery identity
linked by `supersedes`. A duplicate delivery with the same digest returns the
existing consume result. A stale revision, lease, epoch, deadline, or fencing
token is rejected without invoking the consumer. If a takeover races with
delivery, the higher fencing token wins; the old owner may neither
acknowledge nor consume. If the effect of a delivery is uncertain, the owner
first reads the delivery/transition journal and live checkpoint before
redelivery. The owner remains installed until consume, explicit cancel, or
terminal reconciliation; a Codex heartbeat may continue the human-operated
workflow but is not this production wake owner.

## 8. Plan DAG and evidence graph boundaries

The existing Plan-Gap Evidence remains the mutable plan truth in the GitHub
task issue. A machine-readable plan or dependency DAG may be generated as a
derived, digest-bound artifact for scheduling and validation; it must not become
a second task ledger or runtime authority. Each parallel slice must have
disjoint write scope and explicit dependencies, and integration remains
serialized through TPM's canonical worktree and task chain.

An evidence graph may provide a derived view linking bootstrap, plan, frozen
head, action receipts, validator observations, slice returns, reviews, PR gate,
merge, sync, and cleanup artifacts. Each node retains its producer, authority,
epoch, digest, and dependencies. The graph does not replace the individual
receipt validators, the GitHub task-issue evidence sink, or the [pre-PR review
packet](./source-of-truth.md#pre-pr-review-packet).

## 9. M1–M4 dependency path

These are implementation milestones, not workflow states, gates, or current
status claims. The dependency order is strict for unattended promotion:

| Milestone | Target outcome | Depends on |
| --- | --- | --- |
| M1 — durable supervisor core | Promote/re-prove the rich test-only reducer behind a production trust boundary; add checkpoint CAS, lease, fencing, journal, epoch, and recovery semantics while keeping fixtures outside production imports. | Existing canonical lifecycle, bound GitHub task truth, canonical worktree/branch, and bootstrap snapshot. |
| M2 — trusted mechanical path | Add fixed mechanical/bootstrap action execution and independent live validator/readback for non-LLM phases, with idempotent external mutation recovery. | M1. |
| M3 — trusted collaboration | Add durable dispatch, runtime-issued attestation, bounded return artifacts, retry/timeout/death recovery, and at-least-once delivery with idempotent, readback-backed duplicate rejection for professional slices. | M1 and M2. |
| M4 — wake/event runtime | Add persisted event/timer delivery, lease-aware takeover, and cross-process/session resume without model-turn polling. | M1–M3. |

### 9.1 Milestone entry and exit evidence

Milestone evidence is an implementation-planning contract, not a replacement
for GitHub task truth or a new gate. Each entry/exit packet is immutable,
records the task/bootstrap/evidence epoch and artifact digests, and contains
the verification command, expected result, actual result, and independent
readback authority. A fixture result, caller-authored JSON, or static adapter
configuration may be a test input but never an exit proof.

| Milestone | Entry evidence | Exit evidence required before the next milestone |
| --- | --- | --- |
| M1 — durable supervisor core | Canonical lifecycle/state contract; bound task truth and bootstrap snapshot; canonical worktree/branch; versioned envelope manifest; fixture reducer baseline explicitly marked test-only; current capability remains `blocked` | Production reducer/checkpoint implementation; CAS and lease/fencing conformance; atomic journal/checkpoint and corruption recovery report; schema migration/compaction report; crash-point matrix; `tpm-supervisor-replay/v1` with zero external-effect calls and matching canonical state hashes; bounded long-run/compaction result; production import audit proving fixtures are absent |
| M2 — trusted mechanical path | M1 exit packet and current task/head/epoch readback; phase-to-producer/validator matrix; executable and protocol allowlist; external mutation key/readback contract | Every mechanical phase has a fixed action builder, authenticated executable identity, independent live validator, and stage-bound receipt; transient/ambiguous external effects recover by key; process-kill/restart and stale-fencing tests pass against independently observable effects |
| M3 — trusted collaboration | M1 and M2 exit packets; runtime connector authority and attestation key lifecycle; disjoint slice scopes and integration dependencies | Dispatch acknowledgement, actual runtime identity, attempt/supersession, bounded artifact digest, and return attestation are runtime-issued; timeout/death/retry/duplicate/out-of-order returns are covered; integration is CAS-bound and rejects forged or stale returns |
| M4 — wake/event runtime | M1–M3 exit packets; scheduler owner, trusted clock, delivery transport, and wake schema/proof registry | Install/deliver/takeover/cancel survives process/session restart; duplicate wake and lease-race tests prove one consume transition; stale delivery is rejected; acknowledgement/readback is durable; no active-turn polling is needed for unattended resume |

The M1 replay result must replay only validated event/proof inputs: it must
not repeat external mutations. The replay packet records the event sequence,
schema versions, migration path, deterministic clock inputs, random-seed or
runtime-ID inputs, and state digest at each transition. Long-run evidence
records event count/time budget, peak and retained storage, compaction points,
recovery latency, and any unresolved journal records. QA owns the final
budget and release-blocking judgment; runtime supplies the measurements.

No milestone by itself changes the source-of-truth capability status. In
particular, M3 cannot unblock unattended operation while M2 or M4 is absent,
and M4 cannot be claimed from a local JSON mutation.

## 10. Targeted QA staging and live evaluation

After M1–M4, `qa_engineer` should own an isolated staging evaluation of the
supervisor runtime. This is a targeted runtime promotion criterion, not a
universal required gate for every workflow or documentation change. The
evaluation is a design target only: it does not claim that the production
adapter, transport, or runtime attestation exists today. The current
capability remains `blocked` until the evidence below is available. A green
fixture reducer, static TOML adapter check, local self-signed receipt, or
production script exit code is not live-evaluation evidence.

### 10.1 Staging producer manifest and trust authority

The initial staging target uses a separate repository/project/credentials
namespace and an independently queryable staging transport. Staging must be
unable to mutate the production repository, Project, task ledger, or external
effect target. The staging control plane, rather than the supervisor caller,
issues and stores one signed `tpm-supervisor-staging-manifest/v1`. Its
canonical JSON digest is bound into every milestone and case artifact.

The manifest has the following required fields. Omission, duplication,
unknown producer class, expired attestation, digest mismatch, or a readback
that does not match the manifest is `capability_blocked`, not a passing case.

| Manifest field | Staging acceptance condition |
| --- | --- |
| `schema`, `manifest_digest`, `run_id`, `task_uid`, `bootstrap_epoch`, `evidence_epoch` | The schema is supported; the digest is over canonical JSON; identifiers and both epochs match the task and all case artifacts. |
| `staging_repo`, `staging_project`, `namespace`, `base_oid`, `head_oid`, `issued_at`, `expires_at` | Repository, Project, credentials, and effect targets are staging-only; the frozen OIDs and validity window are independently read back. |
| `producer_classes` | Exactly one independently addressable entry exists for each canonical class: `Trusted mechanical/bootstrap action`, `Independent live validator/readback`, `Trusted collaboration/attestation`, and `Wake/scheduler runtime`; live evaluation is evidence, not a fifth producer class. |
| Per-producer `producer_id`, `authority_id`, `key_id`, `protocol_digest`, `executable_digest`, `receipt_schema`, `dispatch_endpoint`, `readback_endpoint`, `attestation` | The staging control plane authenticates the producer and readback endpoint; the executable/protocol/receipt digests are allowlisted; the producer cannot choose its own trust-root identity. |
| `trust_roots` and `allowlist_digest` | Action, validator, collaboration, and wake authorities are separately named; the validator/readback authority is distinct from the action caller; key and executable rotation changes the evidence epoch. |
| `signature`, `control_plane_id`, `control_plane_readback` | A staging authority signs the manifest and an independent query returns the same digest, task, epochs, OIDs, producer set, and expiry. Caller-authored JSON and local self-signatures do not satisfy this field. |

The manifest is not a production credential and cannot promote the canonical
capability by itself. It is a test authority for one immutable staging run.
The run is invalid if a producer is selected from an environment variable,
fixture, static adapter pin, or unverified local process rather than from the
manifest and its independent readback.

### 10.2 M1–M4 QA packet and artifact expectations

The M1–M4 table in §9.1 is the dependency contract. QA accepts a milestone
only when its entry and exit artifacts are immutable, digest-addressed, and
bound to the same `task_uid`, canonical worktree, frozen base/head, bootstrap
epoch, evidence epoch, and staging-manifest digest. Every artifact records
the exact command or transport query, expected result, observed result,
independent readback, role/authority, timestamp, and artifact digests. A
human assertion without the command output and readback is incomplete.

| Milestone | Required entry artifact | Required exit artifact |
| --- | --- | --- |
| M1 durable local state | Source-of-truth and task-truth snapshot; canonical worktree/branch and base/head readback; checkpoint/journal schema and migration versions; fixture-only baseline explicitly marked non-production. | Reducer/checkpoint/CAS/lease/fencing transcripts; atomic journal and corruption-recovery result; migration/compaction result; crash/restart result; `tpm-supervisor-replay/v1` with zero external-effect calls and canonical state-hash result; bounded long-run storage/latency result; import audit. |
| M2 mechanical production loop | M1 exit packet plus phase-to-producer trust matrix, producer allowlist, action/receipt/validator schemas, idempotency-key derivation, and the staging manifest. | For every mechanical phase: runtime-issued action, independent validator proof, live effect readback, stage receipt, ambiguous-effect reconciliation, kill/restart recovery, stale-fencing rejection, and exact-head/task/epoch binding. |
| M3 runtime collaboration | M1/M2 exit packets plus connector authority and attestation lifecycle, disjoint slice scopes, dispatch/return schemas, attempt/supersession rules, and bounded budgets. | Runtime-issued dispatch identity and attested return; timeout/death/retry/duplicate/out-of-order cases; artifact and digest readback; CAS rejection of forged/stale returns; no integration when required returns are missing or partial. |
| M4 wake and scheduler | M1–M3 exit packets plus scheduler owner, trusted monotonic clock, wake schema, delivery identity, lease/fencing rules, and restart boundary. | Process/session restart recovery; duplicate wake race with one consume; stale/out-of-order wake rejection; durable ack and readback; bounded wake delivery; no active-turn polling; final state/effect digest. |

The packet has `milestone-entry/v1` and `milestone-exit/v1` records even when a
milestone is blocked. A blocked record contains the missing artifact,
failure signature, resume condition, and retained-byte/artifact digest. A
partial packet cannot be upgraded to `pass` by rerunning only the final
command; the affected milestone and all dependent exits must be rerun under a
new evidence epoch.

### 10.3 Initial staging budgets and deterministic repetitions

The following are explicit initial-staging design targets, not current
capability claims, production SLOs, or evidence that the adapter is available.
The values are fixed for a run; changing one requires a new run ID, manifest
digest, and evidence epoch.

| Budget or repetition | Initial staging target |
| --- | --- |
| Clean full production-adapter runs | Exactly 3, seeds `7001`, `7002`, `7003`; each must use the manifest-selected adapter and all four producer classes. |
| Adversarial repetitions | Exactly 3 per matrix case, one each with seeds `7001`, `7002`, `7003`; no random or wall-clock-derived case selection. |
| Per-case and whole-run wall clock | `120 s` per case; `1,800 s` per clean/adversarial run; a timeout is a failed or blocked case, never an implicit retry. |
| Action/transport timeout and retries | `30 s` action deadline; at most `3` attempts for one logical action; retry backoff capped at `10 s`; attempts preserve the action ID and idempotency key. |
| Wake delivery | Durable deadline `120 s`; at most `3` delivery attempts per wake identity; duplicate/stale deliveries do not consume budget as new logical work. |
| Recovery | `60 s` from process/session restart or lease takeover to a durable readback and one accepted/rejected outcome; otherwise the case is blocked/failed with retained evidence. |
| Cost and external effects | `0.00` charged production-side API cost, `0` production mutations, and staging-only credentials/effect targets; an unmeasured or non-zero production-side effect fails the run. |
| Cross-process soak | Exactly 3 runs with the same seed set; each runs `900 s`, delivers `100` wake events, and performs `3` scheduled kill/restart/takeover points. |

Budgets apply to observed runtime behavior, not just configuration. The
report records elapsed monotonic time, attempt and delivery counts, charged
cost, restart/takeover count, and the reason for every budget stop.

### 10.4 Stage × fault × expected outcome matrix

The matrix is a closed initial-staging catalog: `B01`, `B02`, `A01`–`A05`,
`V01`–`V02`, `C01`–`C03`, `W01`–`W03`, `R01`, `M01`, `S01`, and `K01` (19
IDs). Case IDs are never inferred from prose. The matrix has 57 records and
the operational catalog has 24 (8 IDs x 3 seeds); with 3 positive and 3
soak records, the base packet has exactly 87 records. Missing, duplicate, or
unknown ID/seed blocks it.
`case_kind` is `adversarial|positive|soak`; adversarial records carry exactly
one `matrix_case_id` or `operational_case_id`, with `case_id` equal to
`adversarial-<catalog-id>-<seed>`; positive/soak IDs are `positive-<seed>` /
`soak-<seed>`.

`input_workflow_state` is one of the six canonical states and `input_phase` is
from the phase matrix. `expected_status`/`expected_phase` are typed
`canonical:<value>` or `same_as_input`; a phase is never a status. The closed
outcome catalog is `accepted`, `producer_missing`, `retry_exhausted`,
`return_exhausted`, `stale_return`, `stale_timeout`, `stale_fencing`,
`stale_receipt`, `identity_mismatch`, `receipt_mismatch`, `replay_mismatch`,
`wake_expired`, `wake_cancelled`, `unresolved_effect`, and `rejected`; adapters may not add codes. Generic `rejected` requires one category: `schema|identity|epoch|revision|lease|fencing|scope|ordering|authority|malformed`.
Mapping is `schema|malformed -> rejected(category)`, `identity|epoch|scope|authority -> rejected(category)`,
`revision|lease|ordering -> stale_receipt`, and `fencing -> stale_fencing`; same-digest duplicates return the existing result, while different-digest duplicates map to `stale_receipt`.
Phase-specific aliases remain closed and must carry their category in evidence.

Cardinality is six integer predicates `A/P/S/R/B/C`: accepted external effects,
phase advances, workflow-state changes, classified rejections, capability
blocks, and cleanup mutations. Predicates are `=n`/`<=n` (positive/soak also
allow `>=n`, encoded as `{op:eq|lte|gte,value:n}`); `A<=1K`/`A<=2K` binds stable
logical keys. Recovery tokens use `rule(attempts<=n,elapsed_s<=n,new_epoch=<bool>,same_key=<bool>)`;
`reject_same_epoch`/`capability_unchanged` require `new_epoch=false`, while
only `authorized_reset` may set `new_epoch=true` and must name its authority.

The following matrix is the minimum fault suite; status/phase are observed at the fault boundary, cardinality covers recovery, and each injection records its point and proves every predicate by readback.
| Case ID | Stage | Input workflow state | Input phase | Fault injection | Expected status rule | Expected phase rule | Expected outcome code | Effect cardinality | Recovery rule |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `B01` | Bootstrap | `running` | `bootstrap` | Missing/partial manifest, producer, trust root, or attestation | `canonical:capability_blocked` | `same_as_input` | `producer_missing` | `A=0/P=0/S=1/R=0/B=1/C=0` | `capability_unchanged(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=false)` |
| `B02` | Bootstrap | `running` | `bootstrap` | Wrong task UID, worktree, branch, base/head, or epoch in an input packet | `same_as_input` | `same_as_input` | `identity_mismatch` | `A=0/P=0/S=0/R=1/B=0/C=0` | `reject_same_epoch(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=false)` |
| `A01` | Action | `action_required` | `execute` | Kill before durable intent | `canonical:action_required` | `same_as_input` | `accepted` | `A<=1K/P=0/S=0/R=0/B=0/C=0` | `same_key(attempts<=3,elapsed_s<=60,new_epoch=false,same_key=true)` |
| `A02` | Action | `running` | `execute` | Kill after intent but before response/readback | `canonical:external_wait` | `same_as_input` | `accepted` | `A<=1K/P=0/S=1/R=0/B=0/C=0` | `same_key(attempts<=3,elapsed_s<=60,new_epoch=false,same_key=true)` |
| `A03` | Action | `running` | `execute` | Transient external failure or `Retry-After` | `canonical:external_wait` | `same_as_input` | `accepted` | `A<=1K/P=0/S=1/R=0/B=0/C=0` | `same_key(attempts<=3,elapsed_s<=120,new_epoch=false,same_key=true)` |
| `A04` | Action | `running` | `execute` | Unqueryable or contradictory effect after an ambiguous response | `canonical:failed` | `same_as_input` | `unresolved_effect` | `A=0/P=0/S=1/R=0/B=0/C=0` | `authorized_reset(attempts<=1,elapsed_s<=60,new_epoch=true,same_key=false,reset_authority=escalation)` |
| `A05` | Action/receipt | `running` | `execute` | Forged, stale, duplicate-with-different-digest, wrong task/head, or wrong lease/fencing receipt | `same_as_input` | `same_as_input` | `stale_receipt` | `A=0/P=0/S=0/R=1/B=0/C=0` | `reject_same_epoch(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=false)` |
| `V01` | Validator | `running` | `verify` | Validator unavailable before proof can be issued | `canonical:capability_blocked` | `same_as_input` | `producer_missing` | `A=0/P=0/S=1/R=0/B=1/C=0` | `capability_unchanged(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=false)` |
| `V02` | Validator | `running` | `verify` | Live readback disagrees with requested effect or receipt digest | `canonical:failed` | `same_as_input` | `unresolved_effect` | `A=0/P=0/S=1/R=0/B=0/C=0` | `authorized_reset(attempts<=1,elapsed_s<=60,new_epoch=true,same_key=false,reset_authority=escalation)` |
| `C01` | Collaboration dispatch | `running` | `dispatch` | Required producer absent before dispatch | `canonical:capability_blocked` | `same_as_input` | `producer_missing` | `A=0/P=0/S=1/R=0/B=1/C=0` | `capability_unchanged(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=false)` |
| `C02` | Collaboration return | `external_wait` | `execute` | Trusted dispatch exists but return is temporarily absent/late | `canonical:external_wait` | `same_as_input` | `accepted` | `A=0/P=0/S=0/R=0/B=0/C=0` | `same_key(attempts<=3,elapsed_s<=120,new_epoch=false,same_key=true)` |
| `C03` | Collaboration return | `external_wait` | `execute` | Forged, stale, replayed, out-of-order, partial, or wrong-scope return | `same_as_input` | `same_as_input` | `stale_return` | `A=0/P=0/S=0/R=1/B=0/C=0` | `reject_same_epoch(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=false)` |
| `W01` | Wake | `external_wait` | `external_wait` | Duplicate, out-of-order, or stale-lease wake | `same_as_input` | `same_as_input` | `stale_fencing` | `A=0/P=0/S=0/R=1/B=0/C=0` | `reject_unchanged(attempts<=1,elapsed_s<=120,new_epoch=false,same_key=false)` |
| `W02` | Wake | `external_wait` | `external_wait` | Valid wake races process restart/takeover | `canonical:running` | `same_as_input` | `accepted` | `A=0/P=0/S=1/R=0/B=0/C=0` | `wake_race(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=true)` |
| `W03` | Wake | `external_wait` | `external_wait` | Logical wake deadline expires | `canonical:failed` | `same_as_input` | `wake_expired` | `A=0/P=0/S=1/R=0/B=0/C=0` | `authorized_reset(attempts<=1,elapsed_s<=120,new_epoch=true,same_key=false,reset_authority=owner)` |
| `R01` | Review/fix | `running` | `review` | Exact-head artifact missing or review/CI receipt is stale | `canonical:running` | `same_as_input` | `stale_receipt` | `A=0/P=0/S=0/R=1/B=0/C=0` | `reject_same_epoch(attempts<=1,elapsed_s<=120,new_epoch=false,same_key=false)` |
| `M01` | Merge receipt | `running` | `watch` | Malformed merge receipt input | `same_as_input` | `same_as_input` | `receipt_mismatch` | `A=0/P=0/S=0/R=1/B=0/C=0` | `reject_same_epoch(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=false)` |
| `S01` | Main sync | `running` | `main_sync` | Process kill after merge effect but before sync/readback | `canonical:external_wait` | `same_as_input` | `accepted` | `A<=2K/P=0/S=1/R=0/B=0/C=0` | `same_key(attempts<=3,elapsed_s<=60,new_epoch=false,same_key=true)` |
| `K01` | Safe cleanup | `running` | `safe_cleanup` | Incomplete, contradictory, or missing cleanup evidence | `canonical:failed` | `same_as_input` | `unresolved_effect` | `A=0/P=0/S=1/R=0/B=0/C=0` | `no_delete(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=false)` |
The operational catalog is also closed and not inferred from matrix prose; its
slash-separated expected-status/phase cells are tagged `status_rule/phase_rule`.
Each ID is repeated for seeds `7001`, `7002`, `7003` (24 records). Migration,
compaction, and replay use the canonical
`replay_mismatch` outcome (the source contract explicitly groups those
faults); no new outcome code may be invented by an adapter.

| Operational ID | Fault family / injection | Input state/phase | Expected status / phase | Expected outcome | Effect cardinality | Recovery rule |
| --- | --- | --- | --- | --- | --- | --- |
| `O01` | retry / retry budget exhausted | `running` / `execute` | `canonical:failed` / `same_as_input` | `retry_exhausted` | `A=0/P=0/S=1/R=0/B=0/C=0` | `authorized_reset(attempts<=1,elapsed_s<=60,new_epoch=true,same_key=false,reset_authority=escalation)` |
| `O02` | return / collaboration deadline exhausted | `external_wait` / `execute` | `canonical:failed` / `same_as_input` | `return_exhausted` | `A=0/P=0/S=1/R=0/B=0/C=0` | `authorized_reset(attempts<=1,elapsed_s<=60,new_epoch=true,same_key=false,reset_authority=escalation)` |
| `O03` | timeout / stale timeout loses return race | `external_wait` / `execute` | `same_as_input` / `same_as_input` | `stale_timeout` | `A=0/P=0/S=0/R=1/B=0/C=0` | `reject_unchanged(attempts<=1,elapsed_s<=120,new_epoch=false,same_key=false)` |
| `O04` | wake / authorized cancellation without replacement | `external_wait` / `external_wait` | `canonical:failed` / `same_as_input` | `wake_cancelled` | `A=0/P=0/S=1/R=0/B=0/C=0` | `authorized_reset(attempts<=1,elapsed_s<=120,new_epoch=true,same_key=false,reset_authority=owner)` |
| `O05` | replay / input or state-hash mismatch | `running` / `execute` | `canonical:capability_blocked` / `same_as_input` | `replay_mismatch` | `A=0/P=0/S=1/R=0/B=1/C=0` | `capability_unchanged(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=false)` |
| `O06` | migration / unsupported or partially published transform | `running` / `bootstrap` | `canonical:capability_blocked` / `same_as_input` | `replay_mismatch` | `A=0/P=0/S=1/R=0/B=1/C=0` | `capability_unchanged(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=false)` |
| `O07` | compaction / manifest or before-after hash mismatch | `running` / `execute` | `canonical:capability_blocked` / `same_as_input` | `replay_mismatch` | `A=0/P=0/S=1/R=0/B=1/C=0` | `capability_unchanged(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=false)` |
| `O08` | replay / attempted external effect or forbidden final-state accept | `running` / `execute` | `canonical:capability_blocked` / `same_as_input` | `replay_mismatch` | `A=0/P=0/S=1/R=0/B=1/C=0` | `capability_unchanged(attempts<=1,elapsed_s<=60,new_epoch=false,same_key=false)` |
The closed positive catalog is `positive-7001`, `positive-7002`, and
`positive-7003` (one per fixed seed), each with typed assertions:
`case_kind=positive`, input `action_required/bootstrap`,
`final_status={kind:canonical,value:completed}`,
`final_phase={kind:canonical,value:post_merge_done}`,
`expected_outcome_code=accepted`, exactly four independently read-back
producer classes, and the exact phase sequence
`[bootstrap,route,dispatch,execute,integrate,freeze,draft_candidate,verify,review,closeout,promote_draft,create_pr,record_pr,comment,watch,merge,merge_receipt,task_done,main_sync,safe_cleanup,post_merge_finalize,post_merge_done]`,
`accepted_external_effects >= 1`, `phase_advances = 21`,
`classified_rejections = 0`, `capability_blocks = 0`,
`cleanup_mutations = 1`, and `all_effects_independently_readback = true`.
Fix/reverify loops are not silently inserted into this positive catalog; a
loop requires a separate catalog version and exact sequence.
The closed soak catalog is `soak-7001`, `soak-7002`, and `soak-7003`, each a
900-second cross-process run with
`input={workflow_state:external_wait,phase:external_wait}` and
typed assertions `wake_deliveries = 100`, `wake_consumes = 100`,
`duplicate_consumes = 0`, `stale_wake_accepts = 0`,
`kill_restart_points = 3`, `takeovers = 3`, `active_turn_poll_count = 0`,
`max_recovery_latency_s <= 60`, `accepted_external_effects = 0`,
`final_status={kind:same_as_input}`, `final_phase={kind:same_as_input}`, and
an independent readback. Every wake has its own
delivery ID, attempt, fencing proof, consume result, and readback; any
missing nested wake record blocks the whole soak case.
Every catalog row is repeated for all three fixed seeds. A row passes only
when the observed canonical status/phase rule, typed outcome code, effect
cardinality, and recovery rule match exactly; “eventually green” or a
successful process exit is not a pass. The catalog and count rule are part of
the acceptance contract, not report prose.
### 10.5 Machine-readable case summary and human report

The staging case's `observed.transitions`, `observed.effects`, and
`observed.readbacks` are typed arrays, not free-form JSON. Each item uses its
repository envelope identity/digest (`transition/v1`, `effect/v1`, or
`readback/v1`), carries state/phase or attempt/query, proof/fencing, expected
and observed digests, closed status/result, and staging-only `run_id`/`case_id`
binding outside its standalone preimage. Arrays have deterministic sort keys
(`new_revision`/ID, `action_id`/`attempt`/ID, `query_id`/ID); corresponding
digest lists must match exactly. Unknown fields, duplicate IDs, malformed
items, or case identity/digest mismatch reject without lifecycle progress.
`record_digest` is the complete case-object preimage (excluding itself) under
`oasis7/tpm/tpm-supervisor-staging-case/v1`; items never contain it, and
signatures/transport framing/local paths are excluded.

Each case emits one immutable `tpm-supervisor-staging-case/v1` JSON record.
The required shape includes the catalog identity, input state/phase, typed
status/phase rules, outcome-code enum, cardinality predicates, and bounded
recovery rule; numeric assertions use `{op:eq|lte|gte,value:n}`, while
booleans and exact arrays are typed. The shorthand `A/P/S/R/B/C` expands to the named integer
fields shown below (`P` is phase advances and `S` is workflow-state changes),
and `canonical`/`same_as_input` is a tagged object rather than a prose value.
```json
{
  "schema": "tpm-supervisor-staging-case/v1", "run_id": "...", "case_id": "adversarial-B01-7001", "seed": 7001,
  "case_kind": "adversarial", "catalog": "matrix", "matrix_case_id": "B01", "operational_case_id": null,
  "task_uid": "...", "bootstrap_epoch": "...", "evidence_epoch": "...", "base_oid": "...", "head_oid": "...", "manifest_digest": "...",
  "stage": "Bootstrap", "fault": "...", "injection_point": "...", "input_workflow_state": "running", "input_phase": "bootstrap",
  "budgets": {"case_timeout_s": 120, "action_timeout_s": 30, "max_attempts": 3, "wake_deadline_s": 120, "recovery_deadline_s": 60},
  "observed": {"fault_boundary": {"workflow_state": "capability_blocked", "phase": "bootstrap", "cardinality": {"accepted_external_effects": 0, "phase_advances": 0, "state_changes": 1, "classified_rejections": 0, "capability_blocks": 1, "cleanup_mutations": 0}},
    "final": {"workflow_state": "capability_blocked", "phase": "bootstrap", "cardinality": {"accepted_external_effects": 0, "phase_advances": 0, "state_changes": 1, "classified_rejections": 0, "capability_blocks": 1, "cleanup_mutations": 0}},
    "transitions": [], "effects": [], "readbacks": [], "attempts": 0, "wake_deliveries": 0, "elapsed_s": 0, "charged_cost_usd": 0.0},
  "recovery": {"restart_count": 0, "takeover_count": 0, "readback_queries": [], "signature": "...", "rule": "capability_unchanged", "attempts": 1, "elapsed_s": 0, "new_epoch": false, "same_idempotency_key": false, "final_status": {"kind": "canonical", "value": "capability_blocked"}},
  "expected": {"status_rule": {"kind": "canonical", "value": "capability_blocked"}, "phase_rule": {"kind": "same_as_input"}, "expected_outcome_code": "producer_missing",
    "effect_cardinality": {"accepted_external_effects": {"op": "eq", "value": 0}, "phase_advances": {"op": "eq", "value": 0}, "state_changes": {"op": "eq", "value": 1}, "classified_rejections": {"op": "eq", "value": 0}, "capability_blocks": {"op": "eq", "value": 1}, "cleanup_mutations": {"op": "eq", "value": 0}},
    "recovery": {"rule": "capability_unchanged", "max_attempts": 1, "max_elapsed_s": 60, "new_epoch": false, "same_idempotency_key": false}},
  "assertions": {"required_producer_classes": null, "required_phase_sequence": [], "wake_deliveries": null, "wake_consumes": null, "kill_restart_points": null, "takeovers": null, "active_turn_poll_count": null, "all_effects_independently_readback": true},
  "outcome": "pass|fail|blocked", "independent_readback": {"authority_id": "...", "query_id": "...", "observation_digest": "..."},
  "transition_digests": [], "effect_digests": [], "readback_digests": [], "artifact_digests": [], "failure_signature": null, "residual_risk": [], "record_digest": "..."
}
```

The run aggregate `tpm-supervisor-staging-summary/v1` includes manifest,
task/head/epoch, fixed seeds, per-catalog results, budgets, artifact digests,
missing/partial/stale evidence, residual risks, signatures, and one
`pass|fail|blocked` outcome. Exact counts are
`matrix_case_count=19`, `operational_case_count=8`,
`adversarial_case_count=(19+8)*3=81`, `positive_case_count=3`, `soak_case_count=3`, `total_case_count=87`; every catalog ID/seed pair occurs exactly once and both
ID arrays equal their closed catalogs. Supervisor-only output is not authority.

The human `staging-report.md` mirrors `tpm-supervisor-staging-report/v1` with identity/authority, namespace, commands/results, M1–M4 links, timelines, budgets, reproduction, missing evidence, residual risk, and QA recommendation.
Front matter carries epochs, frozen OIDs, manifest/catalog and aggregate digests, exact counts, outcome, and report digest. Its explicit partitions have one typed row per matrix/operational ID/seed (81 adversarial rows), one per positive seed (3), and one per soak seed (3): exactly 87 case/seed rows; omissions or human/aggregate mismatch are incomplete.
### 10.6 Promotion predicates

The target acceptance set requires all of the following:

- one complete production-adapter path is exercised end to end (initial
  staging target: the three fixed-seed runs above), from bootstrap and route
  through action, validator, collaboration, wake, freeze, verify, review/fix,
  merge receipt, task done, main sync, safe cleanup, and final terminal
  receipt; each producer identity and external effect is independently read
  back;
- every matrix and operational row passes for all three repetitions/seeds,
  with no accepted effect, phase advance, accepted state change, integration,
  or cleanup on a rejected/forged/stale/partial input;
- the catalog set equals the 19 matrix plus 8 operational IDs above and the
  aggregate contains exactly 81 adversarial, 3 positive, 3 soak, and 87 total
  records; each positive has the exact 21-advance phase sequence ending in
  `post_merge_done`, four
  producer readbacks, and zero rejection/block records;
- every soak repetition has exactly 100 wake deliveries/consumes and 3 kill/
  restart/takeover points, with zero duplicate consumes or stale wake accepts,
  no active-turn polling, and recovery/readback within the fixed budgets; and
- the M1–M4 packet, manifest, machine records, human report, signatures,
  digests, and residual-risk disposition are complete and mutually
  consistent.

Any missing, partial, stale, mismatched, unqueryable, or self-signed evidence
blocks promotion. A blocked case is not converted to pass by omitting it from
the aggregate, increasing a timeout, reducing repetitions, or relying on a
local exit code. Passing this isolated target is necessary evidence for a
future capability change, not a claim that the canonical capability is
currently unblocked; the source-of-truth status and human-operated lifecycle
remain authoritative until the promotion criteria in §11 are satisfied.

QA decides whether the packet is release-blocking; runtime owns adapter,
transport, attestation, and recovery facts; repository-health owns artifact
and environment integrity; TPM records the bounded findings in the canonical
task evidence. No role may substitute a local fixture, static role adapter,
or caller-authored JSON for the missing staging authority.

## 11. Non-goals and promotion criteria

Non-goals for this design are:

- changing the canonical lifecycle, closed workflow states, gate definitions,
  terminal order, or TPM/professional/QA ownership;
- removing bootstrap, GitHub task truth, evidence comments, frozen-head review,
  or the fail-closed human-operated path;
- treating this document, a static role adapter, fixture, or caller JSON as
  production authority;
- introducing a second mutable task database, a second plan truth, or a new
  `external_wait.*` state taxonomy;
- requiring universal live autonomous evaluation for documentation-only or
  ordinary human-operated workflow changes; or
- solving model tiering, cost optimization, or exactly-once external execution
  before trust boundaries and recovery are proven.

Promotion from `blocked` to an implemented unattended capability requires
repository-owned evidence that all four producer classes are available and
independently observable, M1–M4 invariants hold under crash/retry/takeover,
external effects reconcile by idempotency and live readback, collaboration
returns carry runtime-issued attestation; deterministic replay/state-hash
equivalence and long-run/compaction evidence are present; and the QA-owned
isolated staging evaluation passes with its fault, budget, and residual-risk
evidence. The canonical source must be updated first, then its thin
operational skill and contract tests synchronized. Until that evidence exists,
the [capability
status](./source-of-truth.md#capability-status) remains blocked and the
[current human-operated lifecycle](./source-of-truth.md#canonical-lifecycle)
continues to govern.
