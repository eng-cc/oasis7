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

Current code/doc contracts already bind ordered actions, roots, committed execution records, artifact hashes, atomic module transitions, checkpoints, and canonical replay. They do **not** yet prove the target end state: signed per-validator re-execution results; a persisted/verifiable >2/3-stake BFT commit certificate; prevote/precommit rounds, locks, timeout/view-change; protocol-versioned in-process/IPC conformance; governed runtime-manifest activation readiness/rollback; light-companion proof verification; or the complete checkpoint/disaster-recovery trust chain. These are implementation and verification gaps, not claims of present network readiness.

## 6. 集成点
- `doc/p2p/prd.md`
- `doc/world-simulator/prd.md`
- `doc/testing/prd.md`

## 7. 专题导航
- 核心治理进入 `governance/`
- 模块发布与实体进入 `module/`
- 运行时行为进入 `runtime/`
- 执行器与 ABI 进入 `wasm/`

## 8. ModuleStore persistence / restore boundary

- The default world directory owns one module-store closure: registry, manifest metadata, and content-addressed wasm artifacts. `save_to_dir` / `load_from_dir` are the normal route; compatibility `*_with_modules` entrypoints do not establish a parallel format.
- Restore accepts legacy directories without a store, but an existing store must validate registry/meta/artifact hash consistency and return structured version, missing-artifact, or manifest-mismatch errors rather than silently repairing bytes.
- Module instance identity/version/hash, owner, install target, active state, and install time are persisted for replay routing. Upgrades validate compatibility before one atomic registry/state/event transition.
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
