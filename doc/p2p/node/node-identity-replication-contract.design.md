# Node identity and replication contract design

## 1. 问题、目标与非目标

把local identity、conditional signer map、signed replication的验证/持久/观察顺序固定，避免假进展和corrupt authoritative recovery默认化。

Owner role：runtime_engineer；canonical repository：eng-cc/oasis7；固定审读基线 f9d5a552d9af04c1b1398262808198a58e560230，2026-09-26。本设计消费下表有限关系；旧章节保留作兼容详细条款，新增DES细化原义务而不改算法/schema/默认值/产品承诺。任务实际source/integration/tested tree/config/environment/window/结果由GitHub Issue/Project evidence维护，本文不伪造候选/通过结果。

非目标：不以文档active/合并、local fixture、历史MIG或CLI可编译宣称运行、BFT、部署、SLA或release。剩余215-object required set及root SC1..10/full组合不变。

## 2. 上游约束与相关角色

runtime/P2P拥有机制；ops拥有同窗口部署事实；QA拥有组合证据判定；producer拥有产品含义；消费者和WASM拥有各自schema/manifest/行为。professional_acceptance与product_requirement是关系类型，本文不新增机器trace schema或activation。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [professional_acceptance: nir-bootstrap](node-identity-replication-contract.prd.md#nir-bootstrap) | disabled不建key；missing可localdev ensure；malformed/conflict/unwritable失败无invalid overwrite | [des-nir-bootstrap](#des-nir-bootstrap) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: nir-signer-binding](node-identity-replication-contract.prd.md#nir-signer-binding) | 配置map完整normalized32-byte，无unknown，signature先于binding，缺失/mismatch拒绝 | [des-nir-signer](#des-nir-signer) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: nir-ingest-order](node-identity-replication-contract.prd.md#nir-ingest-order) | network injection只路由；world/topic隔离不混状态 | [des-nir-isolation](#des-nir-isolation) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: nir-ingest-order](node-identity-replication-contract.prd.md#nir-ingest-order) | apply/persist成功才peer/committed observation；failure不假推进 | [des-nir-ingest](#des-nir-ingest) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: nir-ingest-order](node-identity-replication-contract.prd.md#nir-ingest-order) | single-writer/order/duplicate/stale持久guard；hash/write失败不污染 | [des-nir-guard](#des-nir-guard) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: nir-recovery](node-identity-replication-contract.prd.md#nir-recovery) | corrupt/unreadable PoS authoritative state blocks start；保留诊断纠根因 | [des-nir-recovery](#des-nir-recovery) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |

## 3. 当前状态、目标状态与差距

| 对象 | 当前专业合同（冻结source） | 目标 / 差距 | 实现 / 执行证据 |
| --- | --- | --- | --- |
| 本专题现行规则 | 配对PRD与保留旧章节的有界接口/失败合同 | 可选map未启用部署不能被说全节点binding；persist-failure/crash atomicity尚需node oracle；ops/root全链恢复外部。 | 本次source阅读，未运行behavior tests；具体测试定义存在不等于覆盖或通过 |
| 跨节点组合能力 | local documented contract，历史完成是provenance | 依赖root/runtime/ops/consumer与full同candidate验证 | 运行/部署/发行未验证；不能由文档推导 |

## 4. 边界与结构

config bootstrap只本地dev；consensus/governance/custody各自authority。NodeRuntime持有network handle和guard/state，network仅运输，ops读取runtime status不重定义truth。FileReplicationRecord必须world/topic/source/signature/ordering适用。

### des-nir-bootstrap

disabled不建key；missing可localdev ensure；malformed/conflict/unwritable失败无invalid overwrite。

enabled节点读当前config→缺key可创建→严格map验证→normal signature→binding→source/world/order→apply→persist guards→publish observation。任何拒绝停止该记录不记成功；startup required recovery load失败blocked。disabled不创建身份，重复/stale不应用。

### des-nir-signer

配置map完整normalized32-byte，无unknown，signature先于binding，缺失/mismatch拒绝。

enabled节点读当前config→缺key可创建→严格map验证→normal signature→binding→source/world/order→apply→persist guards→publish observation。任何拒绝停止该记录不记成功；startup required recovery load失败blocked。disabled不创建身份，重复/stale不应用。

### des-nir-isolation

network injection只路由；world/topic隔离不混状态。

enabled节点读当前config→缺key可创建→严格map验证→normal signature→binding→source/world/order→apply→persist guards→publish observation。任何拒绝停止该记录不记成功；startup required recovery load失败blocked。disabled不创建身份，重复/stale不应用。

### des-nir-ingest

apply/persist成功才peer/committed observation；failure不假推进。

enabled节点读当前config→缺key可创建→严格map验证→normal signature→binding→source/world/order→apply→persist guards→publish observation。任何拒绝停止该记录不记成功；startup required recovery load失败blocked。disabled不创建身份，重复/stale不应用。

### des-nir-guard

single-writer/order/duplicate/stale持久guard；hash/write失败不污染。

enabled节点读当前config→缺key可创建→严格map验证→normal signature→binding→source/world/order→apply→persist guards→publish observation。任何拒绝停止该记录不记成功；startup required recovery load失败blocked。disabled不创建身份，重复/stale不应用。

### des-nir-recovery

corrupt/unreadable PoS authoritative state blocks start；保留诊断纠根因。

enabled节点读当前config→缺key可创建→严格map验证→normal signature→binding→source/world/order→apply→persist guards→publish observation。任何拒绝停止该记录不记成功；startup required recovery load失败blocked。disabled不创建身份，重复/stale不应用。

## 5. 关键运行流程

enabled节点读当前config→缺key可创建→严格map验证→normal signature→binding→source/world/order→apply→persist guards→publish observation。任何拒绝停止该记录不记成功；startup required recovery load失败blocked。disabled不创建身份，重复/stale不应用。

拒绝保留可定位原因；timeout/retry受原策略有界限制。并发冲突遵循原guard/precondition/共识authority，不能因接线或重连获得新权限。取消/替代只有外部专业合同明确支持时成立，本设计不新建取消或补偿schema。

## 6. 接口与数据合同

config path/keypair和optional validator_signer_public_keys由本地config提供；每validator normalized32-byte ed25519无遗漏/额外。proposal/attestation/commit同验签-before-binding。NodeReplicationNetworkHandle可注入world/topic route，FileReplicationRecord包含原版本ordering identity，不能增新schema或恢复旧aw topic/UDP authority。

| producer → consumer | 输入身份/版本 | 顺序与幂等 | success / error | 兼容与资源边界 |
| --- | --- | --- | --- | --- |
| config/network/DistFS → NodeRuntime | 原PRD列明identity/hash/version，不复制schema | 上述flow及DES验证顺序；local幂等不推导global exactly-once | 完成相应validation/apply/persist/publish后才声明该结果；失败不伪进展 | 原policy/defaults及旧data兼容；不加未知deadline/阈值 |

## 7. 状态、事务与持久化

identity missing/malformed/conflicting/unwritable可区分；非法已有key不replace。record validated不等于applied；applied不等于persisted；persist成功才observation。local/remote writer guard持久保护stale/duplicate。crash durability及node apply+persist原子边界须实际测试，不能借DistFS guard单测代签。

accepted、applied、persisted、published必须分别具备各自证据；workflow done/文档active不属于运行状态。外部receipt/journal/state root生命周期和recovery由runtime authority；本地观察不能提供更强保证。

## 8. 部署、安全与运行约束

secret不进入docs/inventory/monitor/artifacts；node/session/consensus/governance identity分开。可用local key不提供KMS/HSM/rotation/revoke/admission。注入不证明libp2p/NAT/公网或当前triad health；role从current runtime status/evidence窗口读取。

设计只定义受控输入和失败条件；操作留在现行runbook。当前governed live网络raw-copy禁令和signed-V2边界只适用于该runbook环境，不否定历史/offline local事实。任何真实fleet/health/restore claim需固定inventory/manifest/package/config与同窗口证据，不从triad标签或本地成功推导。

## 9. 质量与容量

| 环境/刺激 | 预期响应 | 判定指标/阈值来源 | 验证入口 | 当前证明范围 |
| --- | --- | --- | --- | --- |
| 同冻结候选的local unit/fixture，逐DES负例与旧数据兼容 | 精确拒绝/no false mutation；正确路径满足原顺序 | 配对PRD常量/default/policy；未定义容量/latency不新增数字 | §11.1逐场景的source/手册 | source定义/计划，SN1未执行 |
| 多节点/恢复/服务角色组合，固定world/version/config/window | 同history/root或明确隔离/只读，不产生无证世界效果 | root与testing-manual S9A；root历史lag≤50、DistFS ratio≤0.1、mismatch0及insufficient_data阻断口径保留 | [testing-manual](../../../testing-manual.md) S9A/S9/S10；按原场景选择tier | 无真实规模、窗口或容量/SLA执行证明 |

## 10. 兼容、迁移与回滚

吸收MIG088/092/095/099历史只保留provenance。旧transport/topic不复活；未来接线更改必须runtime/network证据。已有guard/recovery数据按原合同读，错误blocked不能静默default；无自动deploy/restore/rollback，restart只诊断或暂时恢复且须根因修复。

本文保留原path/旧章节anchors与历史ID，不替换task truth；添加的接受/DES只细化旧合同。新输入、消费者或语义变更需重新绑定/专业审读。package/config回退不能复原已删chain state或撤回committed效果；破坏性操作依原runbook与runtime恢复合同。

## 11. 验证设计与可追溯性

下表是可复用计划：existing test source、planned场景与executed evidence分开。每条只消费准确upstream/DES；执行时须同可比较candidate/config/入口/环境，local fixture不升级为真实网络。未定位完整测试写planned，owner为runtime/QA；缺证阻断该组合结论，不能从required局部降低root full。

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [professional_acceptance: nir-bootstrap](node-identity-replication-contract.prd.md#nir-bootstrap) | [des-nir-bootstrap](#des-nir-bootstrap) | disabled不建key；missing可localdev ensure；malformed/conflict/unwritable失败无invalid overwrite | [testing-manual.md](../../../testing-manual.md)；S4 required配置场景：enabled/disabled/missing/invalid/path冲突/read-only；检查文件前后与明确diagnostic。精确bootstrap全病例测试尚未定位，planned。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: nir-signer-binding](node-identity-replication-contract.prd.md#nir-signer-binding) | [des-nir-signer](#des-nir-signer) | 配置map完整normalized32-byte，无unknown，signature先于binding，缺失/mismatch拒绝 | [testing-manual S4/S9A](../../../testing-manual.md)；exact local test source `crates/oasis7_node/src/tests_hardening.rs`；existing config_rejects_duplicate_validator_signer_bindings、pos_engine_rejects_signed_mode_without_complete_validator_signer_bindings、pos_engine_rejects_signed_proposal_when_signer_binding_mismatches_validator；per-message完整矩阵planned。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: nir-ingest-order](node-identity-replication-contract.prd.md#nir-ingest-order) | [des-nir-isolation](#des-nir-isolation) | network injection只路由；world/topic隔离不混状态 | [testing-manual S4/S9A](../../../testing-manual.md)；exact local test source `crates/oasis7_node/src/tests/non_sequencer_followers.rs`；existing runtime_network_replication_respects_topic_isolation、replication_network_handle_rejects_empty_topic；local fixture，多world/source鉴权完整负例planned。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: nir-ingest-order](node-identity-replication-contract.prd.md#nir-ingest-order) | [des-nir-ingest](#des-nir-ingest) | apply/persist成功才peer/committed observation；failure不假推进 | [testing-manual S4/S9A](../../../testing-manual.md)；exact local test source `crates/oasis7_node/src/tests_network_gap_sync_execution_failures.rs`；existing successor_probe_does_not_advance_replication_cursor_when_execution_fails、gap_sync_does_not_advance_replication_cursor_when_execution_fails、out_of_order_replication_ingest_does_not_advance_contiguous_persisted_cursor；persist/crash oracle仍planned。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: nir-ingest-order](node-identity-replication-contract.prd.md#nir-ingest-order) | [des-nir-guard](#des-nir-guard) | single-writer/order/duplicate/stale持久guard；hash/write失败不污染 | [testing-manual S4/S9A](../../../testing-manual.md)；exact local test source `crates/oasis7_distfs/src/replication.rs`；existing apply_replication_record_hash_mismatch_does_not_mutate_guard、apply_replication_record_write_failure_does_not_mutate_guard、replay_replication_records_restores_files_in_order；DistFS局部不能证明node atomicity。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: nir-recovery](node-identity-replication-contract.prd.md#nir-recovery) | [des-nir-recovery](#des-nir-recovery) | corrupt/unreadable PoS authoritative state blocks start；保留诊断纠根因 | [testing-manual S4/S9A](../../../testing-manual.md)；exact local test source `crates/oasis7_node/src/tests_hardening.rs`；existing runtime_start_fails_when_pos_state_snapshot_is_corrupted；同config/state source重启拒绝，unreadable/persist故障完整矩阵planned；不是restore/full恢复。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |

## 12. 决策、长期风险与未决问题

选择保留现有专业合同并精确分配，拒绝以导航/历史标签免除真实设计，也拒绝将local证明升级为full。可选map未启用部署不能被说全节点binding；persist-failure/crash atomicity尚需node oracle；ops/root全链恢复外部。

runtime/P2P负责缺失技术/实现；ops负责环境事实；QA负责计划覆盖和verdict；producer审读产品含义。新算法/schema、权限/默认值变化、额外消费者或claim扩展触发重新评审；未解除项保留在GitHub正式evidence及剩余required set，不能靠未来Issue占位关闭。下列原章节的里程碑/审计轮次是保留provenance，不维护可变进度。



- PRD: `doc/p2p/node/node-identity-replication-contract.prd.md`
- Project record: GitHub Issue / GitHub Project

## Design position

The design joins only the node-side contracts that must agree: local identity
bootstrap, optional validator signer binding, signed replication, persistence,
and fail-closed recovery. It consumes runtime-published status rather than
creating a second consensus, reachability, or topology truth.

## Flow

```text
config validation/bootstrap -> node identity + optional validator binding
  -> signature/source validation -> apply + persist guard state
  -> publish observed replication progress/status
```

Each arrow is ordered. In particular, unsuccessful validation or persistence
does not advance observed peer or committed progress. Startup load failure is a
blocked start, not a default-state recovery.

## Components and constraints

| Component | Current responsibility | Constraint |
| --- | --- | --- |
| Config bootstrap | Load a usable local node keypair on the verified current path | Explicit error for malformed/unwritable config; no silent replacement of invalid identity. |
| Signer binding | Validate normalized validator-to-ed25519 public-key mapping when configured | Missing/mismatched signing key rejects the message; governance/admission remains external. |
| Replication handle | Inject the node replication network and isolate its world/topic route | Injection is not a deployed libp2p or public-reachability claim. |
| Replication ingest | Verify source/signature, apply record, persist ordering guards, then observe progress | Invalid, stale, duplicate, or failed records cannot update status as success. |
| Recovery | Load required node/PoS and replication state | Corruption is surfaced and blocks the applicable start; restarting does not repair root cause. |
| Observability | Expose failure/progress to the runtime status and operations evidence path | The triad monitor only samples/projections; it does not redefine runtime truth. |

## Security and operating boundary

Node identity, transport/session identity, consensus signer, and governance
signer remain distinct. Local config generation supplies none of the custody,
rotation, revocation, registry, or ceremony guarantees required for production.
No private signer material is emitted to documentation or operational artifacts.

Historical UDP and `aw.*` transport wording is deliberately excluded. Likewise,
this design adds no automatic deploy/restart/rollback/restore/state-sync path;
such operations require separately authorized environment evidence.

## Evolution

Future transport or topology work may update this contract only with current
runtime evidence and the network authority. It must preserve apply-before-
observe ordering, explicit recovery failure, and the separation between a local
bootstrap key and production signer/governance truth.
