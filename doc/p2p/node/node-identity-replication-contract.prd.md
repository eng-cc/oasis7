# Node identity and replication contract

## Node 专业接受与设计分配

Owner role：runtime_engineer；canonical repository：eng-cc/oasis7；审读 source baseline：f9d5a552d9af04c1b1398262808198a58e560230（2026-09-26）。本文的 current 指冻结源所记专业合同；本次未独立验证实现、部署或测试通过。target 是规范目标，historical 是 MIG/CCG/TASK 与 dated evidence provenance。GitHub Issue/Project 维护实际任务与候选证据。新增稳定条款是原义务的细化入口；原章节/常量/命令/失败边界仍有效。

<a id="nir-bootstrap"></a>
### Local identity

local/dev missing key 仅启用节点启动前在当前 config path 创建；disabled 不创建；missing/malformed/conflict/unwritable 明确失败，不替换已有非法 identity。不是 custody/admission。

配对设计必须用该条款的独立条件建立承接/验证关系；本条不把文档合并解释为运行能力或组合通过。

<a id="nir-signer-binding"></a>
### Conditional signer map

配置时完整覆盖 validator set、无 unknown、normalized32-byte ed25519；proposal/attestation/commit 先正常验签再 binding，缺失或 mismatch 拒绝。

配对设计必须用该条款的独立条件建立承接/验证关系；本条不把文档合并解释为运行能力或组合通过。

<a id="nir-ingest-order"></a>
### Replication ingest

world/topic-isolated injection；验 signature/source→apply→persist guard→observe progress。stale/duplicate/invalid/失败不推进 peer/committed observation 或污染 guard；local/remote single-writer ordering 与持久 guard 保留。

配对设计必须用该条款的独立条件建立承接/验证关系；本条不把文档合并解释为运行能力或组合通过。

<a id="nir-recovery"></a>
### Authoritative recovery

corrupt/unreadable PoS state blocks start，不默认重置；保留诊断并纠正 state/config/deployment 根因。无自动 deploy/state-sync/restore/rollback 或 topology/readiness 承诺。

配对设计必须用该条款的独立条件建立承接/验证关系；本条不把文档合并解释为运行能力或组合通过。


- Design: `doc/p2p/node/node-identity-replication-contract.design.md`
- Project record: GitHub Issue / GitHub Project

## 目标与权威边界

This is the current `node/` authority for local node-identity bootstrap,
validator-to-signer binding, and signed replication ingestion. It absorbs the
completed `PRD-P2P-MIG-088`, `PRD-P2P-MIG-092`, `PRD-P2P-MIG-095`, and
`PRD-P2P-MIG-099` triplets. Historical implementation detail remains traceable
through Git history and GitHub task evidence, not through a second live
authority.

The contract covers the node-runtime configuration and replication boundary. It
does not define consensus or governance admission, reward settlement, production
custody, public-network deployment, topology health, or QA release readiness.

## 范围

This scope is limited to the node-side identity and replication contracts
listed below. It does not widen the authority to consensus, governance,
settlement, deployment, or release decisions.

## Current contract

### 接口 / 数据：identity and signer binding

- A local development node may ensure a missing node keypair in its current
  config path before it starts. Disabled-node flows do not bootstrap identity.
- Missing, malformed, conflicting, or unwritable configuration fails with an
  explicit diagnostic. It must not silently replace an existing invalid identity.
- When `validator_signer_public_keys` is configured, it binds every configured
  validator ID to one normalized 32-byte ed25519 public key. The map must cover
  the validator set, contain no unknown validator, and reject invalid keys.
- For proposal, attestation, and commit messages, normal signature verification
  precedes the enabled validator-to-signer binding check. A missing or mismatched
  public key is rejected.

The local config identity is a development/bootstrap convenience only. It is not
a production keystore, KMS/HSM custody mechanism, rotation or revocation
workflow, validator admission, governance signer, mainnet claim, or readiness
verdict. Private keys, seeds, mnemonics, and complete secret environments must
not enter documentation, inventory, monitor artifacts, or evidence.

### 接口 / 数据：replication and recovery

- `NodeRuntime` can receive a `NodeReplicationNetworkHandle`; its replication
  path supports network injection and world/topic isolation.
- Signed `FileReplicationRecord` ingestion validates the applicable identity and
  source boundary, applies/persists successfully, and only then advances
  replication observation such as peer heads or committed progress. Errors remain
  observable; a failed apply must not be represented as a progressed peer.
- Local and remote single-writer guards, record ordering, and persisted guard
  state protect against stale or duplicate remote application. Invalid records
  do not enter local state.
- Corrupt or unreadable PoS recovery state blocks node startup rather than
  selecting a default state. A restart is diagnostic or temporary recovery, not
  a substitute for correcting the state, configuration, or deployment root cause.

## Transport and operational boundary

The runtime injection abstraction is not evidence that a libp2p deployment,
NAT traversal, public reachability, peer inventory, or mainnet-grade network
health exists. Transport labels do not establish topology truth: health and role
must be read from the current runtime status/evidence window. Current real-triad
sampling and bounded operator evidence remain under
`node-triad-operations-observability.*`.

The historical UDP fallback and `aw.<world_id>.replication` topic spelling are
not current transport authority. They may be inspected in history only unless a
future code-backed contract explicitly restores them. This document does not add
PKI distribution, multi-writer CRDT, DHT/Kad indexing, automatic deployment,
state sync, restore, rollback, or release automation.

## 里程碑

- M1: Four completed historical triplets are absorbed into this stable node
  contract without carrying their obsolete transport wording forward.
- M2: Shared routes and the file index are repaired by the integration owner,
  then the retired basenames pass the frozen-head stale-reference gate.
- M3: Runtime and QA owners retain targeted verification of signer binding,
  ingest ordering, recovery, and duplicate/stale replication handling.

## 风险

- A local bootstrap key can be mistaken for a custody or governance signer; the
  non-custody boundary must remain explicit.
- Transport labels, UDP fallback, or historical `aw.*` topics can be mistaken
  for a current deployed topology; they are not current authority.
- Restarting after corruption can hide the real config/state/deployment defect;
  startup failure must remain observable and fail closed.

## Verification and failure posture

Targeted node verification must cover signer-map rejection, missing/mismatched
signer rejection, apply-before-observe ingest behavior, corrupt-state startup
failure, duplicate/stale record rejection, and persisted replication recovery.
The specific command set is selected by the runtime and QA owners; documentation
migration itself does not claim runtime, integration, or release success.

For this authority migration, run:

```sh
./scripts/doc-governance-check.sh
./scripts/readme-link-check.sh
git diff --check
```

## Explicit non-claims

- A valid local keypair or signer map is not custody, governance truth, or a
  production/mainnet readiness signal.
- Signed replication is not a public-chain, topology-health, settlement, or QA
  release verdict.
- This document does not supersede runtime consensus semantics, the formal
  network-tier contract, or the mainnet security/governance authority.
