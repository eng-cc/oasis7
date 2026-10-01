# DistFS 生产化硬化设计

## 1. 问题、目标与非目标

本地文件索引/CAS、manifest及reward probe保持可诊断完整性；严格区分权威状态恢复与可丢弃scheduler cursor连续性。

Owner role：runtime_engineer；canonical repository：eng-cc/oasis7；固定审读基线 f9d5a552d9af04c1b1398262808198a58e560230，2026-09-26。本设计消费下表有限关系；旧章节保留作兼容详细条款，新增DES细化原义务而不改算法/schema/默认值/产品承诺。任务实际source/integration/tested tree/config/environment/window/结果由GitHub Issue/Project evidence维护，本文不伪造候选/通过结果。

非目标：不以文档active/合并、local fixture、历史MIG或CLI可编译宣称运行、BFT、部署、SLA或release。剩余215-object required set及root SC1..10/full组合不变。

## 2. 上游约束与相关角色

runtime/P2P拥有机制；ops拥有同窗口部署事实；QA拥有组合证据判定；producer拥有产品含义；消费者和WASM拥有各自schema/manifest/行为。professional_acceptance与product_requirement是关系类型，本文不新增机器trace schema或activation。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [professional_acceptance: dh-local-index](distfs-production-hardening.prd.md#dh-local-index) | path拒绝/CAS先于atomic index/local FileStore功能 | [des-dh-index](#des-dh-index) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-local-index](distfs-production-hardening.prd.md#dh-local-index) | expected hash同进程匹配后操作，None兼容、冲突无分布式lock | [des-dh-precondition](#des-dh-precondition) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-local-index](distfs-production-hardening.prd.md#dh-local-index) | audit missing/dangling/orphan；delete只unreferenced且unpinned回收 | [des-dh-audit-gc](#des-dh-audit-gc) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-manifest](distfs-production-hardening.prd.md#dh-manifest) | sorted canonicalCBOR/CAS export，完整validate后atomic import | [des-dh-manifest](#des-dh-manifest) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-local-probe](distfs-production-hardening.prd.md#dh-local-probe) | local challenge integrity/cursor rotation，不是remote attestation | [des-dh-probe](#des-dh-probe) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-local-probe](distfs-production-hardening.prd.md#dh-local-probe) | atomic scheduler persist；missing默认，坏state warning/default不阻main settlement/tick | [des-dh-probe-state](#des-dh-probe-state) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-backoff-report](distfs-production-hardening.prd.md#dh-backoff-report) | budget/backoff/reason multiplier/old defaults兼容、不改算法/公式 | [des-dh-backoff](#des-dh-backoff) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-backoff-report](distfs-production-hardening.prd.md#dh-backoff-report) | chain-runtime CLI严格范围；epoch aggregate-only、详细state local | [des-dh-report](#des-dh-report) | runtime/node/net/distfs与QA；相关外部authority | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |

## 3. 当前状态、目标状态与差距

| 对象 | 当前专业合同（冻结source） | 目标 / 差距 | 实现 / 执行证据 |
| --- | --- | --- | --- |
| 本专题现行规则 | 配对PRD与保留旧章节的有界接口/失败合同 | manifest import可覆盖映射；probe坏state默认丢连续性但非权威恢复；shared refs/pins和failed-import nonmutation需独立验证。无remote challenge/PoRep/PoSt/ACL/lease/encryption/global恢复承诺。 | 本次source阅读，未运行behavior tests；具体测试定义存在不等于覆盖或通过 |
| 跨节点组合能力 | local documented contract，历史完成是provenance | 依赖root/runtime/ops/consumer与full同candidate验证 | 运行/部署/发行未验证；不能由文档推导 |

## 4. 边界与结构

DistFS拥有local index/CAS/audit/manifest/probe算法；chain-runtime拥有CLI/config/load/persist/worker/report；调用方拥有冲突/覆盖许可。probe本地self-test不验证remote provider；resilience拥有provider/maintenance，S9A/runbook拥有真实环境。

### des-dh-index

path拒绝/CAS先于atomic index/local FileStore功能。

path校验→CAS写→atomic index；if-match查当前hash再write/delete，None旧兼容；delete仅reclaim无索引ref且unpinned。manifest按canonical排序CBOR导出CAS，导入完整validation→atomic替换；fail保留原映射。probe load missing默认、unreadable/malformed warning默认→cursor轮转/预算/退避→aggregate→atomic scheduler state，不阻settlement主链。

### des-dh-precondition

expected hash同进程匹配后操作，None兼容、冲突无分布式lock。

path校验→CAS写→atomic index；if-match查当前hash再write/delete，None旧兼容；delete仅reclaim无索引ref且unpinned。manifest按canonical排序CBOR导出CAS，导入完整validation→atomic替换；fail保留原映射。probe load missing默认、unreadable/malformed warning默认→cursor轮转/预算/退避→aggregate→atomic scheduler state，不阻settlement主链。

### des-dh-audit-gc

audit missing/dangling/orphan；delete只unreferenced且unpinned回收。

path校验→CAS写→atomic index；if-match查当前hash再write/delete，None旧兼容；delete仅reclaim无索引ref且unpinned。manifest按canonical排序CBOR导出CAS，导入完整validation→atomic替换；fail保留原映射。probe load missing默认、unreadable/malformed warning默认→cursor轮转/预算/退避→aggregate→atomic scheduler state，不阻settlement主链。

### des-dh-manifest

sorted canonicalCBOR/CAS export，完整validate后atomic import。

path校验→CAS写→atomic index；if-match查当前hash再write/delete，None旧兼容；delete仅reclaim无索引ref且unpinned。manifest按canonical排序CBOR导出CAS，导入完整validation→atomic替换；fail保留原映射。probe load missing默认、unreadable/malformed warning默认→cursor轮转/预算/退避→aggregate→atomic scheduler state，不阻settlement主链。

### des-dh-probe

local challenge integrity/cursor rotation，不是remote attestation。

path校验→CAS写→atomic index；if-match查当前hash再write/delete，None旧兼容；delete仅reclaim无索引ref且unpinned。manifest按canonical排序CBOR导出CAS，导入完整validation→atomic替换；fail保留原映射。probe load missing默认、unreadable/malformed warning默认→cursor轮转/预算/退避→aggregate→atomic scheduler state，不阻settlement主链。

### des-dh-probe-state

atomic scheduler persist；missing默认，坏state warning/default不阻main settlement/tick。

path校验→CAS写→atomic index；if-match查当前hash再write/delete，None旧兼容；delete仅reclaim无索引ref且unpinned。manifest按canonical排序CBOR导出CAS，导入完整validation→atomic替换；fail保留原映射。probe load missing默认、unreadable/malformed warning默认→cursor轮转/预算/退避→aggregate→atomic scheduler state，不阻settlement主链。

### des-dh-backoff

budget/backoff/reason multiplier/old defaults兼容、不改算法/公式。

path校验→CAS写→atomic index；if-match查当前hash再write/delete，None旧兼容；delete仅reclaim无索引ref且unpinned。manifest按canonical排序CBOR导出CAS，导入完整validation→atomic替换；fail保留原映射。probe load missing默认、unreadable/malformed warning默认→cursor轮转/预算/退避→aggregate→atomic scheduler state，不阻settlement主链。

### des-dh-report

chain-runtime CLI严格范围；epoch aggregate-only、详细state local。

path校验→CAS写→atomic index；if-match查当前hash再write/delete，None旧兼容；delete仅reclaim无索引ref且unpinned。manifest按canonical排序CBOR导出CAS，导入完整validation→atomic替换；fail保留原映射。probe load missing默认、unreadable/malformed warning默认→cursor轮转/预算/退避→aggregate→atomic scheduler state，不阻settlement主链。

## 5. 关键运行流程

path校验→CAS写→atomic index；if-match查当前hash再write/delete，None旧兼容；delete仅reclaim无索引ref且unpinned。manifest按canonical排序CBOR导出CAS，导入完整validation→atomic替换；fail保留原映射。probe load missing默认、unreadable/malformed warning默认→cursor轮转/预算/退避→aggregate→atomic scheduler state，不阻settlement主链。

拒绝保留可定位原因；timeout/retry受原策略有界限制。并发冲突遵循原guard/precondition/共识authority，不能因接线或重连获得新权限。取消/替代只有外部专业合同明确支持时成立，本设计不新建取消或补偿schema。

## 6. 接口与数据合同

FileMetadata path/hash/size/updated_at；write/read/stat/list/delete是local抽象；write_file_if_match/delete_file_if_match expected hash同进程逻辑precondition不是锁。FileIndexAuditReport missing/dangling/orphan；FileIndexManifest/Ref canonicalCBOR/CAS。DistfsProbeRuntimeConfig和reward-distfs-probe/adaptive-multiplier CLI属chain-runtime；旧state defaults兼容。

| producer → consumer | 输入身份/版本 | 顺序与幂等 | success / error | 兼容与资源边界 |
| --- | --- | --- | --- | --- |
| local store/probe → chain-runtime | 原PRD列明identity/hash/version，不复制schema | 上述flow及DES验证顺序；local幂等不推导global exactly-once | 完成相应validation/apply/persist/publish后才声明该结果；失败不伪进展 | 原policy/defaults及旧data兼容；不加未知deadline/阈值 |

## 7. 状态、事务与持久化

CAS先于index atomic；delete不会承诺底blob永不处理。manifest import是受控替换不是merge，完整校验前不apply。probe文件reward-runtime-distfs-probe-state.json只scheduler state；warning/default不能用于PoS/世界recovery。backoff记录连续失败、deadline/skip/时长/reason/multiplier本地，epoch只aggregate checks/failures/ratio。

accepted、applied、persisted、published必须分别具备各自证据；workflow done/文档active不属于运行状态。外部receipt/journal/state root生命周期和recovery由runtime authority；本地观察不能提供更强保证。

## 8. 部署、安全与运行约束

路径禁止empty/absolute/..；unreferenced AND unpinned才GC；precondition冲突调用方retry节流和所有权，不跨进程/节点线性化。budget/base/max/reason multiplier控I/O，不改变challenge验证/reward公式/network协议。secrets不入证据，保state/log/error纠config/code/deployment根因。

设计只定义受控输入和失败条件；操作留在现行runbook。当前governed live网络raw-copy禁令和signed-V2边界只适用于该runbook环境，不否定历史/offline local事实。任何真实fleet/health/restore claim需固定inventory/manifest/package/config与同窗口证据，不从triad标签或本地成功推导。

## 9. 质量与容量

| 环境/刺激 | 预期响应 | 判定指标/阈值来源 | 验证入口 | 当前证明范围 |
| --- | --- | --- | --- | --- |
| 同冻结候选的local unit/fixture，逐DES负例与旧数据兼容 | 精确拒绝/no false mutation；正确路径满足原顺序 | 配对PRD常量/default/policy；未定义容量/latency不新增数字 | §11.1逐场景的source/手册 | source定义/计划，SN1未执行 |
| 多节点/恢复/服务角色组合，固定world/version/config/window | 同history/root或明确隔离/只读，不产生无证世界效果 | root与testing-manual S9A；root历史lag≤50、DistFS ratio≤0.1、mismatch0及insufficient_data阻断口径保留 | [testing-manual](../../../testing-manual.md) S9A/S9/S10；按原场景选择tier | 无真实规模、窗口或容量/SLA执行证明 |

## 10. 兼容、迁移与回滚

MIG067..075、MIG080仅历史；旧probe state字段default兼容。Phase5细report superseded、Phase8 CLI chain-runtime、Phase9backoff非externaltelemetry；MIG059/060 builtin WASM pipeline外部，feedback ledger不混权限。manifest错误覆盖无法靠重启撤销；known localbackup只按授权offline恢复，不泛化live raw copy。

本文保留原path/旧章节anchors与历史ID，不替换task truth；添加的接受/DES只细化旧合同。新输入、消费者或语义变更需重新绑定/专业审读。package/config回退不能复原已删chain state或撤回committed效果；破坏性操作依原runbook与runtime恢复合同。

## 11. 验证设计与可追溯性

下表是可复用计划：existing test source、planned场景与executed evidence分开。每条只消费准确upstream/DES；执行时须同可比较candidate/config/入口/环境，local fixture不升级为真实网络。未定位完整测试写planned，owner为runtime/QA；缺证阻断该组合结论，不能从required局部降低root full。

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [professional_acceptance: dh-local-index](distfs-production-hardening.prd.md#dh-local-index) | [des-dh-index](#des-dh-index) | path拒绝/CAS先于atomic index/local FileStore功能 | [testing-manual S4/S9A](../../../testing-manual.md)；exact local test source `crates/oasis7_distfs/src/lib_tests.rs`；existing file_store_rejects_invalid_paths、file_store_write_read_roundtrip、file_store_overwrite_updates_hash_and_metadata；index-write故障完整oracle planned。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-local-index](distfs-production-hardening.prd.md#dh-local-index) | [des-dh-precondition](#des-dh-precondition) | expected hash同进程匹配后操作，None兼容、冲突无分布式lock | [testing-manual S4/S9A](../../../testing-manual.md)；exact local test source `crates/oasis7_distfs/src/lib_tests.rs`；existing file_store_write_if_match_enforces_hash_precondition、file_store_delete_if_match_enforces_hash_precondition、file_store_if_match_rejects_invalid_expected_hash；只同进程。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-local-index](distfs-production-hardening.prd.md#dh-local-index) | [des-dh-audit-gc](#des-dh-audit-gc) | audit missing/dangling/orphan；delete只unreferenced且unpinned回收 | [testing-manual S4/S9A](../../../testing-manual.md)；exact local test source `crates/oasis7_distfs/src/lib_tests.rs`；existing file_index_audit_reports_missing_dangling_and_orphan_blobs、file_store_delete_preserves_blob_referenced_by_another_file；pin/shared-ref/GC完整负例需审读，非pruning世界证明。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-manifest](distfs-production-hardening.prd.md#dh-manifest) | [des-dh-manifest](#des-dh-manifest) | sorted canonicalCBOR/CAS export，完整validate后atomic import | [testing-manual S4/S9A](../../../testing-manual.md)；exact local test source `crates/oasis7_distfs/src/manifest.rs`；existing file_index_manifest_export_import_restores_index、file_index_manifest_import_rejects_missing_blob；failed import非mutation/canonical order场景planned；仅local mapping。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-local-probe](distfs-production-hardening.prd.md#dh-local-probe) | [des-dh-probe](#des-dh-probe) | local challenge integrity/cursor rotation，不是remote attestation | [testing-manual S4/S9A](../../../testing-manual.md)；exact local test source `crates/oasis7_distfs/src/challenge_scheduler.rs`；existing probe_with_cursor_records_hash_mismatch_failure、probe_with_cursor_advances_and_accumulates_state；localCAS。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-local-probe](distfs-production-hardening.prd.md#dh-local-probe) | [des-dh-probe-state](#des-dh-probe-state) | atomic scheduler persist；missing默认，坏state warning/default不阻main settlement/tick | [testing-manual.md](../../../testing-manual.md)；S4 required planned chain-runtime load/persist：missing/unreadable/malformed/writefailure与worker进展；distfs_probe_runtime.rs仅implementation locator，尚未定位完整测试。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-backoff-report](distfs-production-hardening.prd.md#dh-backoff-report) | [des-dh-backoff](#des-dh-backoff) | budget/backoff/reason multiplier/old defaults兼容、不改算法/公式 | [testing-manual S4/S9A](../../../testing-manual.md)；exact local test source `crates/oasis7_distfs/src/challenge_scheduler.rs`；existing probe_with_policy_limits_checks_per_round、probe_with_policy_applies_backoff_after_failure_round、probe_cursor_state_deserializes_from_legacy_snapshot、probe_with_policy_rejects_zero_reason_multiplier；local。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |
| [professional_acceptance: dh-backoff-report](distfs-production-hardening.prd.md#dh-backoff-report) | [des-dh-report](#des-dh-report) | chain-runtime CLI严格范围；epoch aggregate-only、详细state local | [testing-manual.md](../../../testing-manual.md)；S4 required planned CLI拒非法值、legacy state/readback与epoch fields oracle；CLI/worker/report完整测试未定位，本地lib suite不足。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | local test source≠执行通过；全拓扑/恢复/finality/release未证明 |

## 12. 决策、长期风险与未决问题

选择保留现有专业合同并精确分配，拒绝以导航/历史标签免除真实设计，也拒绝将local证明升级为full。manifest import可覆盖映射；probe坏state默认丢连续性但非权威恢复；shared refs/pins和failed-import nonmutation需独立验证。无remote challenge/PoRep/PoSt/ACL/lease/encryption/global恢复承诺。

runtime/P2P负责缺失技术/实现；ops负责环境事实；QA负责计划覆盖和verdict；producer审读产品含义。新算法/schema、权限/默认值变化、额外消费者或claim扩展触发重新评审；未解除项保留在GitHub正式evidence及剩余required set，不能靠未来Issue占位关闭。下列原章节的里程碑/审计轮次是保留provenance，不维护可变进度。



- 对应需求文档：`doc/p2p/distfs/distfs-production-hardening.prd.md`
- 对应项目记录：GitHub Issue / GitHub Project

## 设计定位

本设计将 MIG-067..075 的九个完成阶段收敛为一个历史可追溯、以当前代码为行为真值的稳定入口。它覆盖本地文件索引保护、local-CAS challenge 自探测、reward-runtime 配置/状态接线和 adaptive backoff；不扩张为分布式一致性、远程证明或生产恢复设计。

## 结构与边界

- 文件索引层：规范化相对路径映射 `FileMetadata`；写入先落 CAS，再原子更新本地 `files_index.json`。CAS precondition、审计、孤儿判定和 manifest 导入导出保护本地索引；删除会回收未被索引引用且未 pin 的 blob。并发冲突由调用者处理，不能由接口名推导跨进程线性化。
- 本地 probe 层：cursor 和 policy 从本地 CAS 选择 blob，生成本地自检报告；它不验证远程 provider 也不构成多节点 attestation。
- reward-runtime 层：`oasis7_chain_runtime` 解析 `--reward-distfs-*` 参数，加载/原子写入 probe state，并把错误降为 warning + default cursor，保持主链路可用。
- adaptive 层：每轮预算、原因分类 multiplier 和最大 backoff 控制局部 I/O 压力；状态字段默认化确保旧 probe state 可读。
- reporting 层：当前对外 epoch report 只承载 aggregate checks/failures/ratio。详细 cursor/config/backoff 是本地状态而非外部 metrics contract。

## 运行面约束

- probe-state 缺失可默认初始化；不可读或 malformed 状态会警告后默认化。这是 best-effort scheduler continuity，不是数据、checkpoint 或 state-sync 的恢复路径。
- 不以重启掩盖 state、配置或 blob 失败。运维应保存状态文件、日志和稳定错误，再依现行 runbook 处理环境根因。
- distributed provider、replica maintenance、拓扑和 NodeRuntime 最佳努力轮询以 `distfs-distributed-resilience` 为权威；其失败语义不得被本地自探测文档覆盖。
- 当前代码/测试是行为锚点；Phase 5 的详细 epoch-report 描述已被 aggregate-report 现状替代，Phase 8 的 CLI 归属为 chain runtime，Phase 9 的 backoff 不是外部 telemetry。

## 验证入口

- `crates/oasis7_distfs/src/{lib.rs,manifest.rs,challenge_scheduler.rs}`。
- `crates/oasis7/src/bin/oasis7_chain_runtime/{cli.rs,distfs_probe_runtime.rs,reward_runtime_worker.rs}`。
- 本地回归：`env -u RUSTC_WRAPPER cargo test -p oasis7_distfs --lib`。
- 涉及节点、真实数据可用性或恢复声明时，使用 `testing-manual.md` S9A 的 state-sync/blob closure、triad、real-env 和 release 分层验证；本设计不把本地测试提升为这些证据。
