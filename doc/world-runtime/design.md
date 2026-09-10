# world-runtime 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/world-runtime/prd.md`
- 当前任务状态与变更过程：对应 GitHub task issue / Project 与 Git history
- 对应文件级索引: `doc/world-runtime/prd.index.md`

## 1. 设计定位

`world-runtime` 是 world-infrastructure 的上层确定性执行层：它不决定产品规则或网络/共识部署，而是把已排序的世界动作确定性地执行为事件、状态根、receipt、checkpoint 与可验证回放。gameplay、Agent 与 Viewer 经版本化协议提交 intent 或消费已提交状态；它们不拥有世界推进权。

## 2. 阅读顺序
1. `doc/world-runtime/prd.md`
2. `doc/world-runtime/design.md`
3. `doc/world-runtime/prd.index.md`
4. 对应 GitHub task issue（仅在需要当前任务状态、阻断或验收过程时）
5. 下钻 `governance/`、`module/`、`runtime/`、`wasm/`、`testing/` 等专题目录

## 3. 目标设计结构

- **分布式基础层（`doc/p2p/` authority）**：governance registry、validator set、Tendermint/CometBFT-style BFT finality、P2P、DistFS、checkpoint/source selection 与 state sync。它决定何时一个 action batch 已有可验证的 finality certificate。
- **版本化 consensus-execution protocol**：请求绑定 `world_id`、protocol/runtime-manifest version、parent committed height/hash、ordered action envelope 与 `action_root`；结果绑定 execution block/hash、`state_root`、receipt/journal references 和结构化 reject/fault。in-process adapter 是当前部署选择，future IPC adapter 是同一协议的另一 transport；两者必须跑同一 conformance 与 replay fixtures。
- **确定性执行层（本模块 authority）**：从同一个 committed parent state 重放 action sequence，运行已治理激活的 runtime manifest，持久化结果与 checkpoint/replay anchors。proposer 的结果只是候选；每个 active validator 必须 independently re-execute，并只在 root/hash/result 相符时对 BFT vote/certificate 作出本地执行证明。
- **消费者层**：ordinary player game + light companion、operator full infrastructure node、dev/local game + embedded/full local node 都使用同一协议。消费者只投影 finalized/verified state；light companion 可提交 signed intent，但不模拟权威状态。
- **验证层**：determinism/replay, manifest/artifact compatibility, root mismatch, recovery and adapter conformance tests. 任何 root、artifact、certificate、continuity 或 replay mismatch 都 fail closed。

## 4. Runtime activation, deployment, and recovery target

- **Governed activation**：ordinary runtime upgrades are content-addressed manifests/version selected by governance and activated at a committed height. Validators prefetch and verify before activation; missing or mismatched artifacts block execution/voting. Node software delivery does not itself activate deterministic world semantics.
- **Four release lanes**：(1) rolling node-software patches, (2) governance-activated runtime manifests, (3) independent client applications, and (4) coordinated foundational protocol upgrades/forks. The last lane is required when consensus rules, the consensus-execution protocol, or host ABI become incompatible; it needs coordinated binaries and migration proof.
- **Fail-closed availability**：when finality is unavailable, player/light-companion profiles expose only the last verified state plus clearly pending intents with no world effect. Dev/local execution uses a separate `world_id` and is never reconciled into the global history.
- **Recovery trust chain**：immutable tier/genesis identity manifest -> quorum-finalized checkpoint/header bound to the active validator registry -> hash-bound snapshot -> canonical committed-log replay -> state-root verification -> serve/vote. Snapshot and DistFS are transport/cache material, not independent authority.

## 5. Current implementation boundary and target gap

Current code/doc contracts already bind ordered actions, roots, committed execution records, artifact hashes, ordered registry/module-lifecycle events, checkpoints, and canonical replay. Public direct module call/command, trusted capability command, and governed registry/module-lifecycle proposal apply now prepare typed projections against a borrowed immutable base and publish only after state/manifest/registry/artifact/schedule/cache-invalidation/journal/allocator/backpressure/consensus validation succeeds. The governed proposal seam preserves sorted lifecycle-event order followed by `ManifestUpdated` and `Governance::Applied`, then installs the prepared batch once; post-prepare failure injection proves that none of those projections become visible on failure. Native tick migration has also begun at the nested-reducer boundary: factory depreciation, native due-economy completion, and due material-transit completion now derive deterministically sorted event bodies from an immutable world view before canonical publication, with non-mutation regressions. Due-economy preparation preserves the build-before-recipe phase boundary and each phase's priority/ready-time/job-id ordering. Material-transit preparation additionally projects replay-deterministic SLA counters in the same due-job order and installs that projection only after every completion event has been appended. Agent-claim epoch processing uses a narrower prepared-decision seam: it prepares at most one event against the current state, publishes it, then re-reads claim and balance state before any follow-up so same-owner upkeep funding, grace, release, reclaim, and refund provenance retain their required sequential semantics. These are composable precursors, not a root batch commit; public step wrappers still provide their coarse cloned-`World` rollback boundary. Authority-drift and post-prepare failure injection also cover direct module state/output rollback; existing-format `ModuleCallFailed` audit events are recorded separately only after staged business mutations are discarded. The overall execution capability remains `partial`, because step/tick and remaining lifecycle/recovery entrypoints, persisted instance-state alignment, recovery/replay, durable external effects, and receipt/outbox publication are not one proven transaction boundary. The current contracts also do **not** yet prove the broader target end state: signed per-validator re-execution results; a persisted/verifiable >2/3-stake BFT commit certificate; prevote/precommit rounds, locks, timeout/view-change; protocol-versioned in-process/IPC conformance; governed runtime-manifest activation readiness/rollback; light-companion proof verification; or the complete checkpoint/disaster-recovery trust chain. These are implementation and verification gaps, not claims of present network readiness.

Restricted starter-grant expiry now follows the same immutable preparation pattern: eligibility, balance-derived expiry amounts, metadata, and BTreeMap account order are frozen before sequential canonical publication. This remains a nested precursor under the existing root rollback boundary.

Threat-heatmap preparation follows the same composable boundary: `prepare_threat_heatmap(&self)` derives a fresh `BTreeMap` from the immutable world view, while `refresh_threat_heatmap()` installs that prepared map once. The pure preparation is covered for snapshot, journal, and prior-heatmap nonmutation; this exposes composability only and does not claim new root transaction atomicity or alter War/Crisis semantics.

Emergency-brake activation and release now use a typed prepared sidecar delta: guardian validation and the existing activation-max/release-`None` reducer semantics are preserved, while event id/era, journal, backpressure, and tick-consensus preparation complete before one installation of the World brake field. A post-prepare failure leaves the snapshot, brake, journal, allocator, and consensus unchanged. The brake sidecar remains outside the canonical `WorldState` root schema, and this bounded publication seam does not claim a whole-World root transaction.

Finality epoch snapshot set/remove now uses a dedicated prepared governance sidecar delta. Predecessor, normalization, and removal-drift checks are shared with replay reduction; post-prepare failure leaves the persisted snapshot map, event allocator, journal, and tick consensus unchanged. The map remains outside the canonical `WorldState` root projection, so this closes one direct publication boundary without claiming a whole-World transaction.

Emergency-veto proposal publication now prepares the exact rejected proposal sidecar before the canonical event envelope, journal, allocator, and tick-consensus install. The existing guardian, approved-status, queued-state, reason, and replay checks remain authoritative; a post-prepare failure leaves the proposal map and all publication observables unchanged. Proposal sidecar data remains outside the `WorldState` root projection, so this is a bounded direct-mutator seam rather than a whole-World transaction.

Identity-penalty appeals now prepare an immutable replacement record before canonical event publication, and replay uses the same replacement helper. Existing appellant/reason, Applied-status, deadline, legacy detection-field backfill, stage-evidence hash, and chain-extension semantics remain unchanged; a post-prepare failure leaves the penalty map and publication observables unchanged. The penalty sidecar remains outside the `WorldState` root projection, so this is a bounded direct-mutator seam rather than a whole-World transaction.

Identity-penalty appeal resolution now prepares both the replacement penalty record and target identity profile before publication; the canonical borrowed state projection overlays the profile map entry for the next tick-consensus root. Accepted resolution retains slash refund/status restoration, rejected resolution retains timestamp-only profile updates, and both paths reuse the exact replay helper and evidence-chain hashes. Missing profiles and post-prepare failures leave all observables unchanged; this remains a bounded seam, not a whole-World transaction.

Identity-penalty application now previews the next penalty id and prepares the penalty record, profile mutation, and allocator successor before publication. The borrowed profile-map projection preserves sorted insertion for an absent profile, while retry/replay use the same staged result and consensus root. Invalid stake, duplicate, signer, and post-prepare failures do not consume the allocator or publish partial state; this remains a bounded seam, not a whole-World transaction.

Governance proposal creation and shadowing now use the same prepared publication seam: proposal creation previews the proposal id and id era, while both the proposal sidecar replacement and event publication are installed only after journal, allocator, and tick-consensus preparation succeeds. Existing manifest/patch, module-change, status, and manifest-hash validation remains in the public callers; replay retains the raw Proposed/ShadowReport payloads and proposal-id rollover semantics. The proposal sidecar remains outside the canonical WorldState root, so this closes two direct public mutator boundaries without claiming a whole-World transaction.

Approval and queueing now prepare the `Approved` and `Queued` event pair against an immutable proposal view and install the final proposal replacement, retained journal suffix, event allocator, backpressure count, and tick-consensus candidate together. The pair remains ordered and retains the existing bounded-journal eviction semantics; reject decisions retain their single `Approved` event shape. This is a bounded direct-mutator publication seam, not a whole-World transaction.

Standalone public action-module routing now uses the same borrowed
`TrustedCommandStage` for the complete sorted invocation set. Module state,
effects, emits, runtime charges, cache-miss additions, allocators, journal, and
the single tick-consensus candidate are prepared against the staged context and
installed once. A durable grouped base-head (state, journal, authorization,
policy, artifacts, scheduling, consensus, and governance) is checked before
any install write; a stale head returns `DistributedValidationFailed`. A module fault discards staged business output before one existing
`ModuleCallFailed` audit; an infrastructure failure after preparation installs
nothing and emits no audit. This closes only the direct action-route seam and
does not claim a root `ExecutionTransaction` for `step()` or other nested
event/tick paths.

Standalone public event-module routing now uses that same borrowed stage for
the sorted subscribed invocation set. The input event bytes and post-event
context are prepared together with module state, effects, emits, runtime
charges, cache-miss additions, allocators, journal, and tick consensus before
one install. The same durable base-head check runs before publication, while
live process-local cache entries and wall-clock telemetry are preserved.
Module faults discard staged business output before one existing
`ModuleCallFailed` audit; post-prepare infrastructure failures install
nothing and emit no audit. Tick schedule and routing metrics remain a separate
lifecycle boundary and are not implied by this event-route guarantee.

Standalone public tick routing now stages due schedule removals, wake
directives, deterministic routing metrics, module business output, cache-miss
additions, allocators, journal, and backpressure together. Its prepared envelope carries
no consensus candidate: `run_modules_for_current_tick()` remains the owner of
the single final tick record. A snapshot taken after direct routing but before
that finalization is intentionally not a replayable canonical checkpoint; only
a finalized-tick snapshot is. A pre-route snapshot plus the later journal
cannot reconstruct schedule/metric sidecars that were absent from that
snapshot. Due-record preflight errors retain the compatibility exception of
recording metrics directly while leaving the schedule unchanged.

Gameplay-cycle migration is stage-scoped: economic-contract expiry now freezes the due atomic-contract set, contract-id order, status-derived reputation deltas, and event bodies from an immutable view before publication. Governance proposal finalization is prepared only after those economic events apply; it freezes the sorted due proposal keys and the vote-derived winner, quorum, threshold, and final event payloads from that immutable post-economic state before publication. Crisis lifecycle then prepares at most one deterministic auto-spawn event from the post-governance state and publishes it before freezing the crisis-id-sorted timeout vector from the resulting state. War conclusion deliberately uses a narrower one-event decision: it prepares the lexicographically first due active war from the current post-crisis state, publishes that outcome, then re-reads resources and reputation before preparing the next due war. This preserves both contract-expiry effects and earlier war participant outcomes in later scoring and settlement. Active gameplay-module lifecycle directives follow the same one-directive boundary after their envelope is decoded: each directive is translated against the current immutable state into zero or one domain event, published canonically, and only then may the next directive resolve state-derived fields such as the war loser fallback; zero-point meta grants remain no-ops and malformed envelopes retain their existing module-failure path. These stage-local prepared decisions and vectors remain composable precursors under the outer cloned-`World` rollback boundary, not a whole-cycle or root transaction commit.

## 6. Architecture status, migration proof, and execution boundary

The runtime uses four explicit status labels so that a protocol surface is not
mistaken for a completed end-to-end capability:

| Status | Meaning | Required evidence |
| --- | --- | --- |
| `current` | Present in the current production or compatibility path. | Code, persisted shape, or an existing regression identifies the path. |
| `partial` | Only some entrypoints, lifecycle stages, or identity scopes satisfy the target. | The missing command path, consumer, or identity scope is named. |
| `target` | Normative destination, not a current availability or release claim. | Invariants, ordering, and failure semantics are explicit. |
| `proven` | Target behavior is demonstrated by repeatable execution, rejection, snapshot, recovery, and replay evidence. | A rerunnable fixture/test and its receipt/state-root or migration evidence are available. |

The present architecture is therefore `current` for the Kernel boundary,
ordered action/replay, and persisted module identity; `partial` for
Institution migration, unified transaction coverage, and command-path instance
authorization. A local staged command path or a compatibility record does not
upgrade those surfaces to `proven`.

### 6.1 Institution Migration Test

The first migration proof follows product SC-32: a governed activation
boundary selects exactly one Alliance/EconomicContract pilot. Producer,
runtime, and WASM roles supply evidence; they do not replace governance
approval. The test is `proven` only when all of the following hold:

1. A governed manifest binds artifact hash, schema/version, stable
   `instance_id`, activation, owner/subject, and capability limits.
2. The command traverses the same permission, budget, quote/resolve,
   affordability, staged Kernel apply, receipt, and journal path as native
   actions. The module cannot write canonical `WorldState`, the canonical
   journal, or an external effect directly.
3. Accepted state, event, receipt, and checkpoint commitments are consistent;
   an invariant, budget, persistence, or artifact failure produces one stable
   rejected/fault disposition with no partial business effect.
4. Restart, snapshot restore, canonical journal replay, and adapter
   conformance reproduce the same state root/receipt for the same accepted
   input, while legacy shapes remain readable only through an explicit
   compatibility adapter.
5. Two instances of the same artifact, with different `instance_id` values,
   can execute concurrently without state, event, receipt, or descriptor
   cross-write. A command target may never fall back to another instance with
   the same `module_id`.
6. The `institution-migration-v1` evidence bundle binds manifest, inputs,
   roots, receipts, snapshot/replay/recovery, and legacy-conformance reports.
   Preview/stale/deny/no-effect outcomes debit nothing; an accepted effect has
   exactly one debit and receipt; retries and replay neither double-charge nor
   create replay credit.
7. Post-commit external effects first enter a durable outbox/effect ledger.
   `effect_id` is deterministically derived in the receipt domain from effect kind,
   canonical target/payload, and an effect ordinal; the ordinal only distinguishes
   intentionally repeated equal effects in one receipt. `(world_id,
   execution_receipt_id, effect_id)` is the idempotency key. Same-key/same-payload
   redelivery is deduplicated; same-key/different descriptor, intent, or payload
   fails closed as `effect_id_conflict` without overwrite or dispatch. Crash recovery
   may redeliver an unacknowledged record, while canonical replay never performs the
   external business effect again.

Until this test is proven, Alliance/EconomicContract remain compatibility
surfaces, War remains deferred, Governance remains a Kernel guardrail, and
open Institution extensibility remains `target`.

### 6.2 Unified ExecutionTransaction (architecture P0)

`ExecutionTransaction` (or an equivalent staged boundary) is an architecture
P0 because it establishes one state-transition meaning before further module
expansion. P0 here describes dependency ordering, not an assertion of an
active production incident and not a requirement for a one-shot `WorldState`
rewrite. The boundary covers every world-effecting `step()`/
`step_with_modules()` action, native compatibility action, module command,
tick directive, direct/trusted command, install/upgrade, and recovery or
migration write.

The transaction stages parent state, logical time, resource reservation/debit,
module instance state, pending effects, tick schedule, journal/event data, and
sequence counters. Quote/resolve may run as a read-only preflight, but it must
bind parent, manifest, input root, and freshness and cannot mutate world state.
Module calls and Kernel/schema/capability validation operate only on the staged
view. A successful transition publishes state, events, receipt, and execution
commitment at one commit point. Any failed invariant, budget, artifact,
serialization, or persistence check discards the staged view and publishes
only a stable rejected/fault disposition. External non-rollbackable effects
are receipt-driven after commit through a durable outbox/effect ledger. Each
record binds the canonical effect descriptor, execution receipt, roots, intent
hash, dispatch/ack state, idempotency key, and observed external receipt. The
fixture covers commit-before-dispatch crash, dispatch-before-ack crash,
same-key/same-payload duplicate, same-key/different-payload conflict, duplicate
ack, restart redelivery, and replay with no external adapter call. Recovery may
redeliver an unacknowledged record; canonical replay rebuilds ledger state without
repeating the external business effect.

The migration is incremental: first route one existing command family and its
tick/replay fixtures through the boundary, then widen coverage while keeping
the legacy readers and deterministic ordering. This design does not promise a
big-bang ECS conversion, independent shard finality, cross-shard commit, or a
dynamic World Database.

#### 6.2.1 Explicit transaction and typed-delta model

Module-state updates and runtime-charge events use the prepared event-publication
boundary. Charge preparation owns one replacement payer cell and only touched
treasury entries, applies compute then electricity debits locally, and preserves
negative-fee/missing-payer/error ordering, saturating credits, zero-fee activity
timestamps, and shared-resource aggregation. Replay invokes the same pure
preparation followed by infallible installation. Publication hashes the prepared
entries through the existing borrowed `CommandStateOverlay`; it does not clone
`World` or `WorldState`. Consensus validation and the post-prepare failpoint precede
state, journal retention/backpressure, event id/era, and consensus installation.
`ModuleEmitted` remains a no-state event in the existing output order. This closes
these event seams only, not all nested operations or the root transaction.

`ModuleInstalled` and `ModuleUpgraded` prepare sparse instance/payer/treasury
replacements plus the module-keyed target and install-counter successor. Fee
validation retains priority over upgrade instance/owner/module/version checks.
The direct state reducer prepares legacy world-ledger/cache normalization only
after validation, so rejected events cannot leak compatibility migration writes.
The world reducer also prepares registry-backed schedule lookup before installing
anything; replay shares those preparations. Install state keys keep blank-to-module
and trimmed-nonblank compatibility, while nonblank schedule keys and upgrade lookup
remain raw. Inactive events remove schedules without requiring registry records;
active events require a record even when it has no tick subscription. A borrowed
projection merges the touched instance, target, material ledger/cache and fee
entries, including exactly one routed mailbox event, into the consensus root.
Publication installs only after consensus checks and the existing failpoint.
Rollback uses the same prepared state reducer and exact rollback validation
messages, preserving the historical absence of instance-key schedule updates.

Governed install/upgrade/rollback extend `PreparedGovernanceProposalApply` with
the final lifecycle event before installation. The tail checks its proposal id
and applied-manifest hash, prepares the owned instance delta against unchanged
state, and resolves scheduling against the prepared registry. Its borrowed state
projection uses the prepared manifest hash; event id/era, retained journal,
eviction accounting, and final consensus are extended locally. Both logical
post-prepare failpoints (governance and final lifecycle) precede any installation.
Only then are governance sidecars, cache invalidations, instance state and its
single mailbox event installed. Governance errors retain the caller's existing
`ActionRejected` conversion; lifecycle-tail errors propagate without installing
the governance batch. Local certificate policy/build ordering and explicit
certificate validation are unchanged. Proposal/shadow/approval prelude remains
outside this bounded commit; it is not full action/root atomicity.

Release completion supplies an owned typed context to the install path instead
of inferring its result from the last live journal entry. Both governed and
already-registered installs extend their unpublished envelopes with sorted
product, recipe, factory events and `ModuleReleaseApplied`. Each stage validates
its root/consensus candidate and logical failpoint before any canonical install.
The sparse release delta owns only changed profile/request/mapping entries,
affected agents and legacy world-material normalization, never a `World` or
`WorldState` clone. Its agent overlay begins with the fee-debited install cell
and install mailbox, so a shared installer/operator cannot overwrite the debit
or lose/duplicate events. Borrowed projections merge both instance and release
entries under the prepared manifest hash. Final request/status/mapping checks
retain their order without partial request mutation. Raw profile/final-status
reducers and publication reuse those preparations, preserving replay and direct
event semantics. Input-order prevalidation, profile overwrite rejection,
proposal-id-zero rejection for nonempty profiles, and empty-profile success are
unchanged. This adds release business completion, not prelude deduplication or
global action/root atomicity.

The three release precursors (`ModuleReleaseRequested`,
`ModuleReleaseAttested`, and `ModuleReleaseRolesBound`) also use the sparse
release projector for raw, live-action, and replay publication. It additionally
projects the request allocator and role-binding updates, while touched agent
cells carry activity and the routed projection supplies exactly the
operator/requester mailbox. Attestation mapping lookup remains late in reducer
error order but now fails only against the candidate. Role binding updates both
operator and a distinct target's activity while routing only to the operator.
Legacy world-material normalization and the final root are projected without a
`WorldState` clone before the publication failpoint.

Release review events use that same sparse request/mapping/agent preparation.
Shadowing validates request and status, then requires and clones the mapping
before changing either copy. Role approval preserves normalized-role,
required-role and existing-approver priority; rejection preserves terminal
status and nonblank-reason priority. Approval and rejection continue to update
a mapping only when one exists, while raw events still tolerate a missing actor
and therefore omit activity/mailbox delivery. The prepared request, optional
mapping, actor replacement and legacy material normalization drive the borrowed
root projection before journal, allocator, retention, consensus and failpoint
installation. Action handlers retain their stricter actor/policy validation and
continue to propagate a valid Shadow event's required-mapping reducer error.
This review-event boundary does not include request submission, attestation,
role binding or outer-action idempotency.

Marketplace listing and bid actions prepare their order and optional immediate
sale as one publication batch. The event-bound delta owns only affected agents,
one artifact's owner/listing/bid entries, fee-resource replacements, market
allocators and legacy world-material normalization. An absent staged listing or
bid entry means deletion; the borrowed serializer excludes it while preserving
unaffected entries and canonical key order. Raw listed/bid/sale reducers use the
same pure preparation before legacy normalization, so failed buyer lookup,
debit, seller credit or bid-reference validation cannot remove agents or change
ownership. Matching reads the staged fee-debited cells and updated order book;
listing price, highest-bid/lower-order-id selection and exact-tie vector order
remain unchanged. Every logical event stages its routed mailbox, journal
retention/allocator and consensus root and failpoint before one infallible
install. Artifact ownership does not transfer module instances or schedules.
This is a marketplace business boundary, not whole-World cloning or general
action idempotency; delist, cancellation, destruction and deployment are not
newly composed into it.

Artifact deployment extends the same one-hash sparse state preparation with
publisher fee settlement, owner replacement and listing/bid deletion. Source
and binary action validation still finish first; a validated owned registration
then holds the hash and immutable bytes beside the prepared deployed event.
Journal retention, cursor, consensus root and logical failpoint validate before
the event state and artifact set/byte map install. Standalone artifact
registration retains immediate validate-then-install behavior by using the same
owned registration. Same-hash redeployment therefore continues to charge the
fee, overwrite ownership and clear market orders while leaving the module cache
untouched. External source compilation is outside rollback. Because the
deployed event records only hash and byte length, event-only replay reconstructs
fee/owner/market state but not new byte content; durable bytes remain the
artifact persistence sidecar contract.

Marketplace teardown completes the one-hash sparse transition for delist,
bid-cancel and destroy events. Their validation, fee settlement, agent
activity, owner/listing/bid deletions, legacy world-material normalization,
journal retention, allocator and consensus root are prepared before one
infallible install. Raw destroy remains a canonical-state-only event and does
not remove artifact membership, bytes or cache entries.

The `DestroyModuleArtifact` action additionally carries a prepared retirement
sidecar bound to the identical destroyed-event hash. Only after publication
preparation succeeds does installation remove artifact membership and bytes
and reset the complete module cache while retaining its configured capacity.
Registry, instances and tick schedules are outside the retirement delta. A
failed tail installs neither canonical teardown nor sidecars, so retry charges
and publishes once. This does not change action validation priority, raw-event
sidecar compatibility, persistence, or general action idempotency.

The standalone `apply_module_changes` compatibility API now reuses the governed
lifecycle projector but owns a separate batch publication boundary. It keeps
the stable category order register, upgrade, activate, deactivate and the
stable module-id sort within each category, while projecting registry records,
artifact membership, tick schedules and targeted prepared-subscription cache
invalidations before allocating any durable event. Every `ModuleEvent` retains
the caller's proposal id and actor fields with no `caused_by` value. Journal
retention, event-id era and the final tick-consensus record install once after
all events validate; an empty changeset is a strict no-op. This does not alter
the raw single-`ModuleEvent` path or governed proposal application.

The target production boundary is an explicit `ExecutionTransaction` holding
a read-only canonical `World` base, a `TransitionBuffer`, and a `Live` or
`Replay` mode. `TransitionBuffer` is a typed overlay rather than a cloned
`World` or reflection/JSON patch. It groups deltas for world state, non-state
runtime authorities, rolling sequences, journal batches, pending/inflight
queues, schedules, replay-deterministic metrics, consensus, effects, and the
persistence plan. Scalar changes use typed value replacement and collection
changes use typed map/set operations. Wall-clock observability and process-local
caches never participate in deterministic commitments.

Every public world-effecting entrypoint creates exactly one root transaction.
Nested module routing, reducers, and `append_event` share that root buffer;
`append_event` is transaction-internal and never mutates canonical `World`
directly. A nested operation may create a lightweight savepoint containing
overlay mutation marks, event-batch length, sequence cursors, queue/schedule
operation marks, and deterministic-metric marks, but it cannot independently
commit. A child fault aborts the root. Only an existing domain rule that
explicitly permits a stable business rejection may restore a savepoint and
append that rejection inside the same root transaction.

The transaction base head binds at least the state root, manifest and module
registry roots, journal length and commitment, event id/era, logical time,
queue roots, tick-consensus head, and capability-authorization root. Prepare
performs every fallible validation, allocation, capacity check, commitment,
serialization, and persistence-staging operation against the base plus overlay.
It yields a `PreparedCommit`; installing that commit is a deterministic,
non-fallible typed write. A changed base head rejects the commit as stale and
discards the buffer. Public readers observe only the previous canonical
generation until publication.

#### 6.2.2 Disposition and idempotency contract

Execution returns an accepted, rejected, or faulted disposition when it can
durably form a stable outcome. Accepted atomically publishes all business
state, journal, sequence, queue/schedule, receipt, commitment, deterministic
metric, idempotency, and outbox records. Rejected represents a deterministic
input, authority, budget, resource, or freshness refusal and contains no
accepted business effect. Faulted represents traps, schema/artifact mismatch,
invariant failure, serialization/persistence failure, or commit uncertainty;
it cannot be converted into a recoverable domain rejection. If no stable
disposition can be committed, the API returns an infrastructure error and the
canonical projection remains byte/semantically unchanged. An audit record for
a rejection or fault, when required, is part of that same root commit and is
never appended by a second best-effort transaction.

The first stateful Phase 1 queue slice fixes the public effect-emission
disposition more narrowly. Capability missing, expiry, or kind mismatch is a
preflight rejection with no intent/event allocation or audit. A deterministic
policy deny advances the intent allocator and atomically publishes exactly one
`PolicyDecisionRecorded(Deny)` disposition, without `EffectQueued`. After an
allow decision, queue admission, both journal events, rolling event/intent
sequences and eras, eviction metrics, and tick consensus are one prepared
batch: hard queue-full or a post-prepare infrastructure failure installs none
of them. A full queue may still deterministically evict an existing unlinked
intent, preserving the established bounded-queue rule; authorization-linked
intents are not evictable. Raw `EffectQueued` and `ReceiptAppended` now also use
a typed sidecar replacement for
pending/inflight queues and pending-effect eviction accounting before canonical
event publication. The raw seam preserves queue-full, FIFO eviction,
unknown-intent, duplicate-removal, journal, allocator, retention, and consensus
ordering, and installs only after the post-prepare failure boundary. These
slices do not make durable outbox or replay allocation part of the unified root
buffer yet.

The next Phase 1 stateful slice migrates public receipt ingestion away from a
cloned `World`. A typed prepared receipt delta validates the known intent,
previews the optional authorization-closure event, computes the authorization
audit/link/root overlay, signs or verifies against the virtual closure journal,
removes the pending or inflight intent, and prepares the final receipt event,
journal limit, event allocator, deterministic metrics, and one consensus
candidate. The install step is infallible. A signature, authorization,
consensus, or post-prepare failure therefore closes no link, consumes no queue
entry, and publishes no event or sequence. Existing event order, `CausedBy`,
receipt DTO, and replay-only reducer behavior remain compatible.

This receipt slice does not establish durable duplicate-receipt idempotency or
outbox acknowledgement. The current DTO lacks the approved
`(world_id, execution_receipt_id, effect_id)` identity and canonical descriptor
hash ledger, so same-receipt retry and conflicting-receipt rejection remain a
Phase 3 authority/schema migration rather than an inferred `intent_id` rule.

The first bounded CapabilityAuthorization slices cover public grant
registration, invocation-context installation, budget-account installation,
agent-identity installation, and proof-bearing authority/revocation
administration. For a System subject, the
optional `SystemIdentityInstalled` and required `InvocationContextInstalled`
events are prepared as one typed batch; `BudgetAccountInstalled` uses the same
batch with a typed budget-map projection, while `GrantRegistered` stages the
canonical grant JSON map. `AuthorityInstalledWithProof` reuses the canonical
reducer transition validator while staging the authority record, proof,
revocation, supersession, and finalized receipt projections; revoke,
supersede, and trust-root rotation delegate to this seam. Preparation projects
only the affected authorization maps, previews event ids and eras, retained
journal/backpressure accounting, the final authorization root, and one
tick-consensus candidate; installation then publishes them without a fallible
step. A post-prepare failure therefore publishes no system/agent identity,
context, grant, budget account, or authority transition and consumes no
allocator, journal, consensus, root, or deterministic metric state. Identical
installation remains a no-op, and committed events retain the existing replay
reducers and event order.

Raw agent-intent lifecycle publication uses a full-event-bound sparse
projection shared with replay. It clones only the affected agent cell and at
most one intent-ledger entry; completed transitions first bind their receipt to
the prospective event id and existing journal witness. The projected root
combines the candidate intent/ledger state with exactly one routed mailbox
event before allocator, journal, retention, consensus, and the post-prepare
failpoint are installed. Provider-advisory and historical no-op reducer
semantics remain intact, while the public multi-event chat workflows retain
their existing outer sequencing boundary.

Raw `CommandCommitted` replay-equivalent publication uses a separate,
full-event-bound prepared delta. It validates against the current state,
manifest, journal head, invocation context, authority state, and durable effect
queues while staging only the five maps it can change: grants, nonces,
authorization receipts, budgets, and effect links. Revocation and invocation
context maps remain borrowed inputs to the full authorization-root projection.
All link checks finish before those staged maps and the root are installed with
the event envelope. This path does not execute a command or replace the
`TrustedCommandStage` live command boundary.

Raw `EffectReceiptCommitted` likewise has a replay-equivalent, full-event-bound
prepared delta, but owns only authorization receipts and effect links. It
preserves the reducer's blank-field, missing-link idempotency, link-binding,
and audit lookup order, accepts the event's arbitrary non-empty effect receipt
id, and derives the full authorization root with the other five maps borrowed.
It neither consumes effect queues nor signs or appends an external receipt;
those remain coupled exclusively in specialized `ingest_receipt` publication.

The trusted capability-command executor and public direct module call/command
now use the same architectural seam without cloning canonical `World`: a
borrowed-base typed stage owns only the
module-state, agent/resource, queue, allocator, journal/backpressure, and
process-local cache projections touched by the command. Authorization budget,
grant, nonce, receipt, and effect-link candidates are validated alongside that
stage; state and authorization roots plus the single tick-consensus candidate
are computed before publication. A post-prepare failpoint precedes one
infallible install sequence, so sandbox/output, budget, receipt, effect, event
ids, journal, and consensus cannot become observably half-published. Direct
module failures discard the business stage before the existing standalone
`ModuleCallFailed` audit is atomically appended. This is a bounded Phase 2
migration, not yet the shared root `ExecutionTransaction` for
all nested command/event paths. Durable receipt/outbox/idempotency, persistence
generation commit, remaining public mutation surfaces, and full replay/restore
closure are still outstanding, so CapabilityAuthorization and the overall
ExecutionTransaction capability remain `partial`.

Each retryable public root operation binds a stable operation id to world,
parent identity, canonical input hash, manifest/activation binding, and target.
The same identity and binding returns the original disposition without new
events, debits, queues, or outbox records. Reuse with any different binding is
an idempotency conflict and fails closed. Nested operations inherit the root
identity and use a deterministic child path; provisional event ids are not
external retry identities. External effects retain
`(world_id, execution_receipt_id, effect_id)` as their durable idempotency key:
same key/same descriptor is a retry, while same key/different descriptor is a
conflict.

#### 6.2.3 Replay, generation persistence, and durable outbox

Replay uses the same typed reducers through a root transaction but may only
consume existing canonical events. It validates event id/era, logical time,
parent, commitment, and order; it never creates an event or calls a sandbox,
LLM, effect adapter, or dispatcher. The complete journal suffix commits once,
and any suffix failure discards every reconstructed delta. Effect events only
rebuild ledger/outbox state.

Persistence publishes an immutable generation containing the snapshot,
journal, module store, sidecars, outbox, manifest, hashes, and completion
metadata. All files are staged, synced, and cross-validated before an atomic
latest-generation pointer switch. That pointer switch is the durable commit
point: before it, the old generation remains authoritative; after it, recovery
loads the complete new generation even if the process died before in-memory
installation. Installation after the switch contains no fallible work, and
generation garbage collection is outside the business commit seam. Missing,
mixed, or hash-inconsistent generations fail closed rather than being spliced.

External dispatch begins only after the durable generation commit. The outbox
uses at-least-once delivery with stable idempotency and independently atomic
lease/ack updates; it does not claim physical exactly-once behavior in an
external system. Receipt ingestion is a new root transaction that atomically
validates identity/signature/authorization linkage, updates outbox and
pending/inflight state, applies deterministic receipt effects, appends the
canonical receipt event, and advances commitments/sequences. Unknown or
conflicting receipts cannot consume a queue item before commit.

#### 6.2.4 Incremental delivery and proof levels

Implementation proceeds with Phase 0 plus five reviewable migration phases: (0) transaction/delta types,
savepoints, deterministic projection, failure injection, and a test-only
clone-backed oracle; (1) `append_event`, reducer, sequence, journal, schedule,
queue, commitment, consensus, and deterministic-metric staging; (2) step,
action, direct/trusted command, module lifecycle/routing/tick, observation, and
capability paths; (3) effects, receipts, durable outbox, and idempotency; (4)
atomic replay/restore; and (5) generation persistence and crash recovery.
Production clone-and-publish is removed only for a path whose replacement has
passed its failure-injection and replay gates.

Capability is `design-ready` after this contract and its test oracle are
frozen, remains `partial` while any public mutation entrypoint uses a legacy
boundary, becomes `target-implemented` only after every entrypoint is migrated,
and becomes `proven` only after execution, rejection, fault, replay, recovery,
idempotency, outbox, persistence, serde, and WASM compatibility evidence all
pass. No single staged step test or in-memory swap proves general atomicity.

### 6.3 Command-path module-instance completeness

Instance identity is an authorization and addressing key, not merely a
persistence field. The stable logical key is `(world_id, module_id,
instance_id)`; `artifact_hash/schema_version/activation_epoch` is a separately
versioned binding at an execution height. Install, upgrade, tick, event routing,
and restore already preserve part of this identity; direct/trusted command lookup,
state updates, emitted events, receipt linkage, and machine descriptors remain
`partial` until every path carries the same target.

The target command path validates the logical key and active artifact/schema/
activation binding, owner/subject, capability, and budget before fixing the
instance target and binding digest into the staged command. Upgrades append a
version-lineage record keyed by execution height; restore/replay uses the
historical binding rather than today's artifact. State lookup, event, receipt,
snapshot, replay, and descriptor projection then use that fixed target. Missing,
conflicting, inactive, unauthorized, or state-mismatched targets fail closed
before the first effect; there is no global `module_id` fallback. Agent-facing
semantic interpretation and tool policy remain Agent/WASM authority; this
document only requires runtime to expose a verifiable instance-bound machine
descriptor input.

### 6.4 Gradual subsystem state encapsulation

State ownership is split by stable boundaries without changing the canonical
timeline in one step. The implementation sequence is:

1. Name a subsystem owner and its staged read/write surface while retaining the
   existing `WorldState` compatibility representation.
2. Move module instance state and its journal/replay anchors behind that
   surface, then apply the same pattern to jobs, schedules, and other bounded
   subsystems only after focused determinism and persistence fixtures pass.
3. Persist keys with world/instance/schema identity and keep legacy adapters
   explicit; no subsystem may silently reconstruct state from process-local
   cache.
4. Measure replay, recovery, footprint, and long-run behavior at each step;
   rollback means reverting the bounded adapter/migration, not rewriting
   historical receipts.

This gradual path preserves fixed execution order and snapshot compatibility.
ECS, sharding, and dynamic database work may be future implementation options,
but are deliberately deferred and are not runtime promises in the current
design.

## 7. 集成点
- `doc/p2p/prd.md`
- `doc/world-simulator/prd.md`
- `doc/testing/prd.md`

## 8. 专题导航
- 核心治理进入 `governance/`
- 模块发布与实体进入 `module/`
- 运行时行为进入 `runtime/`
- 执行器与 ABI 进入 `wasm/`

## 9. ModuleStore persistence / restore boundary

- The default world directory owns one module-store closure: registry, manifest metadata, and content-addressed wasm artifacts. `save_to_dir` / `load_from_dir` are the normal route; compatibility `*_with_modules` entrypoints do not establish a parallel format.
- Restore accepts legacy directories without a store, but an existing store must validate registry/meta/artifact hash consistency and return structured version, missing-artifact, or manifest-mismatch errors rather than silently repairing bytes.
- `load_module_store_from_dir` uses a sparse prepared replacement: it loads and validates every sorted registry record into owned registry/set/byte values, then installs those three values once. No `World` clone is required, validation error priority is unchanged, failed hydration preserves the live cache and all world authority, and successful hydration preserves the existing cache policy.
- Raw logistics topology, direct transfer, and material-transit events use separate prepared transition modules. Start and completion preserve the reducer's validation order while staging only touched ledgers, route reservations, pending/settled identifiers, path authority, receipts, affected agents, and progress. The borrowed state projection includes the one generic actor mailbox route before hashing; installation is infallible and replay invokes the same transition helpers. A settled duplicate therefore remains a business-state no-op while its published root still commits the one routed mailbox event.
- Raw factory and recipe lifecycle events use separate sparse prepared transitions. The recipe transition stages only touched pending/settled jobs, factory state, material and power ledgers, logistics paths, actor activity, progress, and quote sinks. Replay shares the same pure preparation and infallible install path; idempotent completion and terminal validation-block events preserve business state while the prospective root still includes the single generic actor mailbox route.
- Raw core agent/body, observation, gameplay-policy, and material-profile events use a full-event-bound sparse transition over one touched agent, compatibility world materials, optional policy/progress, and one optional profile. Preparation owns all fallible body-item and profile validation; installation is infallible and replay shares the same transition. `ActionRejected` remains a no-state, no-route event, while every routed success is included once in the prospective agent mailbox root.
- The canonical append seam is fail closed: every world-event body must resolve to an explicit prepared classification before any reducer, event allocator, journal, retention, backpressure, or consensus work. Intentional no-state deltas retain and compare the complete body, and the former unbound generic route-only delta has been removed. This is an exhaustive publication-classification boundary, not a claim that every outer multi-event workflow is one transaction.
- Factory lifecycle publication uses a separate prepared transition with optional sparse factory upsert/deletion, build-job replacement/removal, settled and retired identity insertion, touched ledgers, affected actor, and projected progress. Build completion and retired recycle duplicates retain raw idempotency while prospective roots include their one declared mailbox route; maintenance/recycle validation and material preflight finish before any prepared state is installed.
- Raw single-event `ModuleEvent` publication reuses the governed lifecycle projector against owned registry/artifact/schedule values and records targeted subscription-cache invalidations; `ManifestUpdated` prepares the replacement manifest and its state-root projection. Both pass allocator, journal retention, consensus validation, and the post-prepare failure seam before an infallible install. Missing-record activation errors no longer leak an active mapping, while legacy informational fields and overwrite behavior remain unchanged.
- Restricted-admin registry updates and validator admission submit/approve/activate/revoke events use a full-body-bound prepared governance delta. Preparation preserves legacy validation priority and synthetic revocation behavior, and a borrowed state serializer overlays only the four touched registry/admission/identity fields when deriving the prospective consensus root; installation is infallible and does not clone `WorldState`.
- Raw proposal status events use one full-body-bound sidecar delta and shared pure status projectors. Because proposals remain outside canonical `WorldState`, their prospective consensus root is unchanged; the normal Approved-to-Queued pair and the governed module/manifest/Applied batch retain their dedicated multi-event publication paths.
- Raw resource/data/access events use a dedicated sparse projector. Validation reads canonical state, stages only touched agents plus one permission-owner or authenticated-nonce player entry, and projects compatibility material normalization without cloning `WorldState`. Root derivation overlays the routed AgentCells while installation remains unrouted, allowing the existing publication/replay router to deliver each mailbox event exactly once.
- Raw power redemption outcomes use a separate full-event-bound projector. A redeemed outcome stages one node balance, the reserve, one nonce and touched AgentCells; a rejected outcome stages only optional activity. The borrowed root overlays the target-only route and compatibility materials, while replay and publication share the same unrouted infallible install.
- Raw node-points settlement uses a dedicated full-event-bound compound projector. It preserves settlement, mint-policy, epoch-budget, bridge-distribution, treasury, account and supply validation order while staging sparse node/account/treasury/budget/bridge replacements plus the full append-only mint list and supply value. Publication computes the prospective root from borrowed untouched fields, crosses the failure seam, then installs infallibly; replay and action preview share preparation, and the event intentionally has no mailbox route.
- The five raw main-token monetary core events share a full-event-bound sparse projector while retaining separate reducer semantics. Genesis may replace the empty account/bucket maps; vesting and transfer touch only their accounts, bucket and nonce; epoch issuance touches supply, its issuance record and treasury entries, while fee settlement touches supply and its treasury entry without changing issuance records. Root serialization preserves canonical field order and projects only the existing beneficiary/source route, while installation stays unrouted and replay/action previews reuse the same preparation.
- Policy scheduling and treasury distribution use a separate full-event-bound governance-monetary projector. It stages scheduled-policy or treasury/account/supply/distribution-record updates with compatibility materials at their canonical root fields, installs without routing, and is reused by replay and action previews.
- Restricted starter-claim publication uses a dedicated full-event-bound projector for pool top-up and grant issue/expiry/revoke. It stages sparse treasury, beneficiary balance, supply, grant/top-up record and compatibility-material updates, projects the existing controller/issuer route exactly once, installs unrouted, and shares preparation with replay and action previews.
- Raw starter OC claim publication uses a standalone full-event-bound projector. It stages the claim, account, supply, optional treasury entry, target activity, and compatibility materials; the borrowed prospective root includes the existing raw-agent route exactly once, while publication and replay share the same unrouted infallible install.
- Claim release requests, grace entries, and idle warnings share a lightweight full-event-bound projector. It stages one claim plus release-only claimer activity and compatibility materials, projects the existing claimer route once, and leaves target activity, monetary state, and the outer epoch cursor untouched; release action preview reuses pure preparation.
- Agent claim creation and upkeep settlement share a full-event-bound economic projector. It stages the claim and processed-epoch cursor together with the claimer account, supply, sparse treasury, claimer activity, and compatibility materials; the borrowed root projects the existing claimer-only route once, while publication installs the unrouted delta infallibly and replay and claim-action preview reuse the same preparation.
- Claim release and reclaim terminal outcomes use a separate full-event-bound projector. It validates refund provenance and destination before staging claim deletion, sparse account/treasury/supply changes, the processed-epoch cursor, claimer activity, and compatibility materials. The prospective root includes the claimer route exactly once; tick event derivation remains sequential and raw reclaim reasons remain informational.
- Economic-contract open, accept, settle, and expire events share a full-event-bound sparse projector. It preserves raw overwrite and payload authority while staging the contract, overlapping participant AgentCells, world treasury resources, reputation, pair cooldown, reward windows, and compatibility materials. Action preflight and replay reuse preparation; expiry remains contract-id ordered and event-sequential, and the existing raw actor alone receives the routed event.
- The six simple capability-admission events reuse the batch projector through a standard full-body-bound publication delta containing grants, revocation/identity state, invocation contexts, budget accounts, and the derived authorization root. Grant projection shares the replay reducer's body, issuer, revocation, parent-chain, and immutability validation order. Compound command and effect-receipt events are deliberately excluded because their atomic boundaries include world state, journal-head, and effect queues.
- Module instance identity/version/hash, owner, install target, active state, and install time are persisted for replay routing. Upgrades currently validate compatibility and append ordered lifecycle events; one atomic registry/state/event transition remains a target and requires lifecycle failure-injection proof.
- Runtime anchors are `crates/oasis7/src/runtime/module_store.rs`, `runtime/error.rs`, `runtime/tests/persistence.rs`, and `runtime/world/module_store_load_transaction_regressions.rs`.

## 设计目标

- 提供 `world-runtime` 模块的总体设计入口，并明确其在 world-infrastructure 中的确定性执行责任、版本化协议与可恢复性边界。

## 设计范围

- 覆盖模块级结构、主链路、分层、target/current gap 与专题导航。
- 不替代 `doc/p2p/` 的 consensus/topology/finality authority、专题 `*.design.md` 的细化设计，或产品规则与客户端交互定义。

## 关键接口 / 入口
- 需求入口：`doc/world-runtime/prd.md`
- 当前任务入口：对应 GitHub task issue / Project
- 索引入口：`doc/world-runtime/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

Alliance/war raw publication uses a full-body-bound sparse projector. It prepares touched
alliances, wars, participants, reputation, and compatibility materials, derives the canonical
root through borrowed untouched state plus the existing route, and installs only after the
publication failure boundary. Replay and action preflight reuse the same reducer semantics.

Governance proposal, crisis, meta-progress, and product-validation events use four narrow
prepared projections behind one publication dispatch type. Each stages only its touched records
and AgentCells; borrowed canonical serialization adds the existing route before the failure seam,
while installation remains unrouted and replay shares the same raw reducer semantics.

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
