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

Current code/doc contracts already bind ordered actions, roots, committed execution records, artifact hashes, ordered registry/module-lifecycle events, checkpoints, and canonical replay. Governed registry/module-lifecycle proposal apply now has a coarse staged-publication boundary: it applies on a cloned `World`, publishes only after full success, and has authority-drift failure-injection coverage for register, upgrade, activate, and deactivate rollback. The overall lifecycle capability remains `partial`, because other entrypoints, persisted instance-state alignment, recovery/replay, durable external effects, and receipt/outbox publication are not one proven transaction boundary. The current contracts also do **not** yet prove the broader target end state: signed per-validator re-execution results; a persisted/verifiable >2/3-stake BFT commit certificate; prevote/precommit rounds, locks, timeout/view-change; protocol-versioned in-process/IPC conformance; governed runtime-manifest activation readiness/rollback; light-companion proof verification; or the complete checkpoint/disaster-recovery trust chain. These are implementation and verification gaps, not claims of present network readiness.

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
- Module instance identity/version/hash, owner, install target, active state, and install time are persisted for replay routing. Upgrades currently validate compatibility and append ordered lifecycle events; one atomic registry/state/event transition remains a target and requires lifecycle failure-injection proof.
- Runtime anchors are `crates/oasis7/src/runtime/module_store.rs`, `runtime/error.rs`, and `runtime/tests/persistence.rs`.

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

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
