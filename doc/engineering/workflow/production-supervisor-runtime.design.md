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
criteria in section 10 are independently evidenced.

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

No milestone by itself changes the source-of-truth capability status. In
particular, M3 cannot unblock unattended operation while M2 or M4 is absent,
and M4 cannot be claimed from a local JSON mutation.

## 10. Targeted QA staging and live evaluation

After M1–M4, `qa_engineer` should own an isolated staging evaluation of the
supervisor runtime. This is a targeted runtime promotion criterion, not a
universal required gate for every workflow or documentation change. The
evaluation should use an independently queryable staging transport, runtime
attestation, bounded time/cost budgets, and fault injection including process
kill/restart, duplicate wake, stale lease/return, transient external failure,
forged receipt, wrong task/head, and incomplete cleanup evidence.

The evaluation must prove durable recovery and fail-closed behavior across
bootstrap, action, validator, collaboration, wake, review/fix, merge receipt,
main sync, and safe cleanup boundaries. A green fixture reducer, static TOML
adapter check, local self-signed receipt, or production script exit code is
not live-evaluation evidence. QA decides whether the evidence is release-
blocking; runtime and repository-health roles provide their bounded technical
findings to TPM.

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
returns carry runtime-issued attestation, and the QA-owned isolated staging
evaluation passes with its fault, budget, and residual-risk evidence. The
canonical source must be updated first, then its thin operational skill and
contract tests synchronized. Until that evidence exists, the [capability
status](./source-of-truth.md#capability-status) remains blocked and the
[current human-operated lifecycle](./source-of-truth.md#canonical-lifecycle)
continues to govern.
