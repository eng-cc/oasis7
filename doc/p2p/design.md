# p2p 模块设计总览

## 1. 问题、目标与非目标

同一 world 的共识顺序、可验证材料与执行/恢复共同形成权威底座；解决 receipt、transport 或历史恢复被误当最终性/可服务的问题。

Owner role：runtime_engineer；canonical repository：eng-cc/oasis7；固定审读基线 f9d5a552d9af04c1b1398262808198a58e560230，2026-09-26。本设计消费下表有限关系；旧章节保留作兼容详细条款，新增DES细化原义务而不改算法/schema/默认值/产品承诺。任务实际source/integration/tested tree/config/environment/window/结果由GitHub Issue/Project evidence维护，本文不伪造候选/通过结果。

非目标：不以文档active/合并、local fixture、历史MIG或CLI可编译宣称运行、BFT、部署、SLA或release。剩余215-object required set及root SC1..10/full组合不变。

## 2. 上游约束与相关角色

runtime/P2P拥有机制；ops拥有同窗口部署事实；QA拥有组合证据判定；producer拥有产品含义；消费者和WASM拥有各自schema/manifest/行为。professional_acceptance与product_requirement是关系类型，本文不新增机器trace schema或activation。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [professional_acceptance: p2p-ordered-execution](prd.md#p2p-ordered-execution) | 下一committed height、有序序列/root/decode及journal绑定；错误不推进 | [des-p2p-ordered-commit](#des-p2p-ordered-commit) | runtime/WASM/消费者/ops/QA各自authority；同candidate证据 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [professional_acceptance: p2p-receipt-finality-boundary](prd.md#p2p-receipt-finality-boundary) | receipt≠QC；同parent/manifest/actions独立重执行，缺artifact/fault/root拒绝vote/commit | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | runtime/WASM/消费者/ops/QA各自authority；同candidate证据 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [professional_acceptance: p2p-target-bft](prd.md#p2p-target-bft) | Propose/Prevote/Precommit仅verified >2/3 active stake cert生效；world/height/round/phase/roots/set和dedup签名绑定 | [des-p2p-target-bft](#des-p2p-target-bft) | runtime/WASM/消费者/ops/QA各自authority；同candidate证据 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [professional_acceptance: p2p-target-recovery-trust-chain](prd.md#p2p-target-recovery-trust-chain) | manifest/genesis→cert/header+set transition→hash snapshot→canonical replay→root→serve/vote；每环缺失/冲突/回退/异world停止 | [des-p2p-target-recovery](#des-p2p-target-recovery) | runtime/WASM/消费者/ops/QA各自authority；同candidate证据 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [professional_acceptance: p2p-target-role-isolation](prd.md#p2p-target-role-isolation) | 治理validator与permissionless服务、light/archive角色隔离；验证材料而非operator身份；prune重建+archive条件 | [des-p2p-layer-authority](#des-p2p-layer-authority) | runtime/WASM/消费者/ops/QA各自authority；同candidate证据 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [professional_acceptance: p2p-service-gate](prd.md#p2p-service-gate) | verified-serving→stale/catching-up→readonly→serving；proof冲突→isolated；非serving新intent receipt0、历史不改 | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | runtime/WASM/消费者/ops/QA各自authority；同candidate证据 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [professional_acceptance: p2p-replication-recovery](prd.md#p2p-replication-recovery) | writer/epoch从seq1及guard不污染；权威恢复错误与qualified storage-challenge fallback区分 | [des-p2p-replication-scope](#des-p2p-replication-scope) | runtime/WASM/消费者/ops/QA各自authority；同candidate证据 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [product_requirement: req-dcs-001](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-001) | 验证certificate及hash-bound材料，拒绝服务/cache/peer自授authority；runtime state apply仍外部 | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | P2P/runtime、ops、consumer与QA；product owner审读含义 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: ac-dcs-001](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#ac-dcs-001) | 验证certificate及hash-bound材料，拒绝服务/cache/peer自授authority；runtime state apply仍外部 | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | P2P/runtime、ops、consumer与QA；product owner审读含义 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: req-dcs-002](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-002) | sameworld恢复链每环验证；历史只读与重新服务闸门分开 | [des-p2p-target-recovery](#des-p2p-target-recovery) | P2P/runtime、ops、consumer与QA；product owner审读含义 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: ac-dcs-002](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#ac-dcs-002) | sameworld恢复链每环验证；历史只读与重新服务闸门分开 | [des-p2p-target-recovery](#des-p2p-target-recovery) | P2P/runtime、ops、consumer与QA；product owner审读含义 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: req-dcs-003](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-003) | governed validator与permissionless服务隔离；公网IP非通用前提 | [des-p2p-layer-authority](#des-p2p-layer-authority) | P2P/runtime、ops、consumer与QA；product owner审读含义 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: ac-dcs-003](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#ac-dcs-003) | governed validator与permissionless服务隔离；公网IP非通用前提 | [des-p2p-layer-authority](#des-p2p-layer-authority) | P2P/runtime、ops、consumer与QA；product owner审读含义 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: req-dcs-004](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-004) | verified >2/3 active stake证书才推进；wrongset/equivocation/round缺证拒绝 | [des-p2p-target-bft](#des-p2p-target-bft) | P2P/runtime、ops、consumer与QA；product owner审读含义 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: ac-dcs-004](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#ac-dcs-004) | verified >2/3 active stake证书才推进；wrongset/equivocation/round缺证拒绝 | [des-p2p-target-bft](#des-p2p-target-bft) | P2P/runtime、ops、consumer与QA；product owner审读含义 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: req-dcs-005](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-005) | 服务转换及manifest/head/finality conjunction；非serving intent receipt0 | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | P2P/runtime、ops、consumer与QA；product owner审读含义 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: ac-dcs-005](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#ac-dcs-005) | 服务转换及manifest/head/finality conjunction；非serving intent receipt0 | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | P2P/runtime、ops、consumer与QA；product owner审读含义 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: req-dwe-001](../product/world-infrastructure/deterministic-world-execution.prd.md#req-dwe-001) | 同parent/version/order重执行及root绑定；runtime原子apply外部 | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | runtime/WASM/Agent/Viewer专业owner与QA；不定义其schema | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: ac-dwe-001](../product/world-infrastructure/deterministic-world-execution.prd.md#ac-dwe-001) | 同parent/version/order重执行及root绑定；runtime原子apply外部 | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | runtime/WASM/Agent/Viewer专业owner与QA；不定义其schema | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: req-dwe-002](../product/world-infrastructure/deterministic-world-execution.prd.md#req-dwe-002) | finality不可用保持无效果pending；恢复按当前条件重新裁决，consumer状态外部 | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | runtime/WASM/Agent/Viewer专业owner与QA；不定义其schema | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: ac-dwe-002](../product/world-infrastructure/deterministic-world-execution.prd.md#ac-dwe-002) | finality不可用保持无效果pending；恢复按当前条件重新裁决，consumer状态外部 | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | runtime/WASM/Agent/Viewer专业owner与QA；不定义其schema | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: req-dwe-003](../product/world-infrastructure/deterministic-world-execution.prd.md#req-dwe-003) | receipt无第二效果为交接条件；互斥lineage首有效receipt唯一胜者/终止其余由runtime拥有，拒绝过期只终止自身，独立intent并发 | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | runtime/WASM/Agent/Viewer专业owner与QA；不定义其schema | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: ac-dwe-003](../product/world-infrastructure/deterministic-world-execution.prd.md#ac-dwe-003) | receipt无第二效果为交接条件；互斥lineage首有效receipt唯一胜者/终止其余由runtime拥有，拒绝过期只终止自身，独立intent并发 | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | runtime/WASM/Agent/Viewer专业owner与QA；不定义其schema | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: req-dwe-004](../product/world-infrastructure/deterministic-world-execution.prd.md#req-dwe-004) | 首次committed/finality-verified canonical block与activation选version；compat declaration不选规则，历史receipt原manifest replay，缺证failclosed；runtime/WASM详细设计外部 | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | runtime/WASM/Agent/Viewer专业owner与QA；不定义其schema | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: ac-dwe-004](../product/world-infrastructure/deterministic-world-execution.prd.md#ac-dwe-004) | 首次committed/finality-verified canonical block与activation选version；compat declaration不选规则，历史receipt原manifest replay，缺证failclosed；runtime/WASM详细设计外部 | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | runtime/WASM/Agent/Viewer专业owner与QA；不定义其schema | external durability/lineage/activation/消费者设计及证据未闭合 |

## 3. 当前状态、目标状态与差距

| 对象 | 当前专业合同（冻结source） | 目标 / 差距 | 实现 / 执行证据 |
| --- | --- | --- | --- |
| 本专题现行规则 | 配对PRD与保留旧章节的有界接口/失败合同 | 目标锁定/解锁、anti-double-sign、timeout/new-round、set transition与partition/heal/restart需独立对抗性证明；本文给拒绝条件而不发明新lock算法/schema。pending/lineage/manifest/consumer字段及DC5 structured attachment仍外部未闭合。 | 本次source阅读，未运行behavior tests；具体测试定义存在不等于覆盖或通过 |
| BFT/QC/治理恢复 | root记录stake-threshold prototype；无持久QC/round/view-change/replication QC复验 | 目标证书/round/transition/recovery有明确拒绝条件，实现仍缺 | 运行/部署/发行未验证；不能由文档推导 |

## 4. 边界与结构

net 拥有 transport/peer/DHT；consensus 拥有 proposer/attestation/action-root；distfs 拥有CAS/replication/challenge；node/chain-runtime拥有commit后执行/恢复接线；proto仅共享wire；wasm_abi是manifest/runtime ABI单一来源。公开服务只能供应hash/proof材料，不能投票或写canonical。

### des-p2p-ordered-commit

下一committed height、有序序列/root/decode及journal绑定；错误不推进。

net 拥有 transport/peer/DHT；consensus 拥有 proposer/attestation/action-root；distfs 拥有CAS/replication/challenge；node/chain-runtime拥有commit后执行/恢复接线；proto仅共享wire；wasm_abi是manifest/runtime ABI单一来源。公开服务只能供应hash/proof材料，不能投票或写canonical。

### des-p2p-receipt-finality

receipt≠QC；同parent/manifest/actions独立重执行，缺artifact/fault/root拒绝vote/commit。

hash-bound material仍须与verified历史/roots相符；服务operator身份、RPC可达、缓存/local receipt不能代签。active validators在投票前从同committed parent、runtime/manifest、有序actions独立重执行，artifact/fault/root错误拒vote/commit。版本由首次committed/finalityverified canonical execution block与activation边界决定；client compatibility只是声明；历史按原manifest重放，跨version replacement服从外部lineage单次效果。

### des-p2p-target-bft

Propose/Prevote/Precommit仅verified >2/3 active stake cert生效；world/height/round/phase/roots/set和dedup签名绑定。

目标证书验证先确定同world/height/round/phase的活动治理set及stake，再逐签名验真并按validator去重，核对block/action/execution roots和严格大于三分之二stake；缺失、冲突、double-sign或错误set/round/phase/root拒绝且无权威进展。slot仅pacing/proposer，timeout进入独立round；lock/unlock、anti-double-sign及transition/restart必须保留同一安全约束，不能用本地threshold或重启跳过。具体lock/timeout证据及算法接口未由现prototype实现，本文不发明新算法。

### des-p2p-target-recovery

manifest/genesis→cert/header+set transition→hash snapshot→canonical replay→root→serve/vote；每环缺失/冲突/回退/异world停止。

按信任链逐环确认，snapshot blob缺失/hash错或replay/root不一致停止，不能用任意peer/latest backup/otherworld/覆盖替代。pruning必须先证明checkpoint+replay重建、hash/root验证和冗余archive可用。既有历史/offline local recovery事实不抹除；当前governed live部署禁止raw-copy路径，signed-V2只在原runbook限定observer/drill scope，observer高head不等于execution-required full restore。

运维接受入口是 [node-triad inventory/采样合同](node/node-triad-operations-observability.prd.md#inventory-与采样合同) 与 [权威边界](node/node-triad-operations-observability.prd.md#权威边界)；governed live 约束见 [bootstrap truth model](blockchain/public-testnet-governed-bootstrap.runbook.md#3-truth-model)、[hard rules](blockchain/public-testnet-governed-bootstrap.runbook.md#4-hard-rules) 和 [failure/rollback](blockchain/public-testnet-governed-bootstrap.runbook.md#14-phase-h---failure-handling-and-rollback)。这些具体环境的恢复 admission 不覆盖历史/offline local 恢复。

### des-p2p-layer-authority

治理validator与permissionless服务、light/archive角色隔离；验证材料而非operator身份；prune重建+archive条件。

net 拥有 transport/peer/DHT；consensus 拥有 proposer/attestation/action-root；distfs 拥有CAS/replication/challenge；node/chain-runtime拥有commit后执行/恢复接线；proto仅共享wire；wasm_abi是manifest/runtime ABI单一来源。公开服务只能供应hash/proof材料，不能投票或写canonical。

### des-p2p-target-service-gate

verified-serving→stale/catching-up→readonly→serving；proof冲突→isolated；非serving新intent receipt0、历史不改。

只读恢复只读verified历史与无效果pending；服务闸门必须同时满足identity/checkpoint/head/hash材料、当前append/finality、governing manifest compatibility和单调head。不满足时新intent receipt0；满足后pending当前permission/resources/version/conditions重审，无outage期限/priority继承，accepted新intent≤1 receipt/no第二效果。退回blocked/isolated不撤销/重放已confirmed receipt。consumer可读semantic/blocker/next-step与linked attachment由外部owner供给。

组合执行 lane 见 [Game World State Sync and Commit Closure](../testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md)；evidence envelope 见 [topology/node truth](../testing/templates/state-sync-closure-evidence-packet-template.md#topology-and-node-truth)、[peer-head/gap sync](../testing/templates/state-sync-closure-evidence-packet-template.md#peer-heads-and-gap-sync) 与 [bundle/blob closure](../testing/templates/state-sync-closure-evidence-packet-template.md#state-sync-bundle-and-blob-closure)。DC-5 还需要真实 linked structured attachment/schema，逐项含状态转换、manifest/head 负例、每 intent receipt0/1、consumer blocker/next-step；现有模板本身不闭合它。Ops同窗口事实、runtime根/receipt和消费者反馈由QA组合，缺任何适用字段不通过。

### des-p2p-replication-scope

writer/epoch从seq1及guard不污染；权威恢复错误与qualified storage-challenge fallback区分。

net 拥有 transport/peer/DHT；consensus 拥有 proposer/attestation/action-root；distfs 拥有CAS/replication/challenge；node/chain-runtime拥有commit后执行/恢复接线；proto仅共享wire；wasm_abi是manifest/runtime ABI单一来源。公开服务只能供应hash/proof材料，不能投票或写canonical。

## 5. 关键运行流程

先校验提交身份/大小/队列→固定有序actions/root→同parent/manifest重执行→匹配receipt commitments→适用finality验证→提交/复制→hash-bound持久记录→消费者committed读取。当前prototype推进与目标QC闸门不是同一实现证明。恢复先证明原world历史链再判只读；当前追加/finality/manifest/head全满足才重新serve/vote，失效回只读/隔离。

拒绝保留可定位原因；timeout/retry受原策略有界限制。并发冲突遵循原guard/precondition/共识authority，不能因接线或重连获得新权限。取消/替代只有外部专业合同明确支持时成立，本设计不新建取消或补偿schema。

## 6. 接口与数据合同

consensus→node交付有序payload/root与commit上下文；runtime→validator提供receipt/parent/manifest执行语义；DistFS/服务→恢复者供应hash-bound材料，不能自授authority。身份/版本/顺序不匹配拒绝，不接受latest任意backup。wire/schema仍属原专业authority，不增字段。

| producer → consumer | 输入身份/版本 | 顺序与幂等 | success / error | 兼容与资源边界 |
| --- | --- | --- | --- | --- |
| consensus/runtime/DistFS → node/consumer | 原PRD列明identity/hash/version，不复制schema | 上述flow及DES验证顺序；local幂等不推导global exactly-once | 完成相应validation/apply/persist/publish后才声明该结果；失败不伪进展 | 原policy/defaults及旧data兼容；不加未知deadline/阈值 |

## 7. 状态、事务与持久化

pending/candidate无世界效果；verified commit才推进权威高度。node block/action/execution roots、journal、snapshot作为同一绑定。目标lock/round/validator-set proof、certificate持久恢复必须一同成立；当前缺这些，不以旧TickCertificate补缺。恢复只读≠恢复可服务；闸门回退不改历史receipt。

accepted、applied、persisted、published必须分别具备各自证据；workflow done/文档active不属于运行状态。外部receipt/journal/state root生命周期和recovery由runtime authority；本地观察不能提供更强保证。

## 8. 部署、安全与运行约束

target protected validators + public services；governed registry有效epoch/stake/signer唯一投票真值。公网IP非通用前提。独立local/dev world永不并入global。prune前先证明重建/hash/root与冗余archives；service compromise只能隔离/缺证。

设计只定义受控输入和失败条件；操作留在现行runbook。当前governed live网络raw-copy禁令和signed-V2边界只适用于该runbook环境，不否定历史/offline local事实。任何真实fleet/health/restore claim需固定inventory/manifest/package/config与同窗口证据，不从triad标签或本地成功推导。

## 9. 质量与容量

| 环境/刺激 | 预期响应 | 判定指标/阈值来源 | 验证入口 | 当前证明范围 |
| --- | --- | --- | --- | --- |
| 同冻结候选的local unit/fixture，逐DES负例与旧数据兼容 | 精确拒绝/no false mutation；正确路径满足原顺序 | 配对PRD常量/default/policy；未定义容量/latency不新增数字 | §11.1逐场景的source/手册 | source定义/计划，SN1未执行 |
| 多节点/恢复/服务角色组合，固定world/version/config/window | 同history/root或明确隔离/只读，不产生无证世界效果 | root与testing-manual S9A；root历史lag≤50、DistFS ratio≤0.1、mismatch0及insufficient_data阻断口径保留 | [testing-manual](../../testing-manual.md) S9A/S9/S10；按原场景选择tier | 无真实规模、窗口或容量/SLA执行证明 |

## 10. 兼容、迁移与回滚

保留旧tick记录仅snapshot/replay/diagnostic兼容；不能推进finality。dormant path-index/empty runtime_bridge不复活；production WASM只external canonical binary+receipt，source compile仅显式dev/test。协议/manifest升级和历史replay由runtime authority管辖，不能用当前manifest改历史。

本文保留原path/旧章节anchors与历史ID，不替换task truth；添加的接受/DES只细化旧合同。新输入、消费者或语义变更需重新绑定/专业审读。package/config回退不能复原已删chain state或撤回committed效果；破坏性操作依原runbook与runtime恢复合同。

## 11. 验证设计与可追溯性

下表是可复用计划：existing test source、planned场景与executed evidence分开。每条只消费准确upstream/DES；执行时须同可比较candidate/config/入口/环境，local fixture不升级为真实网络。未定位完整测试写planned，owner为runtime/QA；缺证阻断该组合结论，不能从required局部降低root full。

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [professional_acceptance: p2p-ordered-execution](prd.md#p2p-ordered-execution) | [des-p2p-ordered-commit](#des-p2p-ordered-commit) | 下一committed height、有序序列/root/decode及journal绑定；错误不推进 | [testing-manual.md](../../testing-manual.md)；S9A phase2/3；同candidate重排、tamper、decode/root/journal负例；执行receipt/状态变化oracle。现有手册入口，完整组合未运行。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [professional_acceptance: p2p-receipt-finality-boundary](prd.md#p2p-receipt-finality-boundary) | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | receipt≠QC；同parent/manifest/actions独立重执行，缺artifact/fault/root拒绝vote/commit | [testing-manual.md](../../testing-manual.md)；S9A；全部活动validator+runtime相同输入/父状态/版本，逐一根不匹配及旧threshold1证据负例。target完整矩阵planned。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [professional_acceptance: p2p-target-bft](prd.md#p2p-target-bft) | [des-p2p-target-bft](#des-p2p-target-bft) | Propose/Prevote/Precommit仅verified >2/3 active stake cert生效；world/height/round/phase/roots/set和dedup签名绑定 | [testing-manual.md](../../testing-manual.md)；S9A full target场景：错误签名/阈值/集合/round、缺证/equivocation拒绝；锁定/解锁、timeout/new-round、transition、partition/heal/restart；prototype不能证明。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [professional_acceptance: p2p-target-recovery-trust-chain](prd.md#p2p-target-recovery-trust-chain) | [des-p2p-target-recovery](#des-p2p-target-recovery) | manifest/genesis→cert/header+set transition→hash snapshot→canonical replay→root→serve/vote；每环缺失/冲突/回退/异world停止 | [testing-manual.md](../../testing-manual.md)；S9A state-sync/full restore：逐环负例、snapshot/replay/state-sync/pruning/drill；同world+samecandidate+同窗口材料，readonly历史与writable闸门分判。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [professional_acceptance: p2p-target-role-isolation](prd.md#p2p-target-role-isolation) | [des-p2p-layer-authority](#des-p2p-layer-authority) | 治理validator与permissionless服务、light/archive角色隔离；验证材料而非operator身份；prune重建+archive条件 | [testing-manual.md](../../testing-manual.md)；S9A full：compromised服务/错误角色签名不能推进，archive失联禁止未证明prune；Ops同窗口inventory/role与runtime验证oracle。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [professional_acceptance: p2p-service-gate](prd.md#p2p-service-gate) | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | verified-serving→stale/catching-up→readonly→serving；proof冲突→isolated；非serving新intent receipt0、历史不改 | [testing-manual.md](../../testing-manual.md)；S9A + GWSC full同candidate：当前manifest/head/finality逐项负例、三恢复状态+gate rollback；receipt0/1与consumer blocker/next-step必须实际linked附件，现模板不足。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [professional_acceptance: p2p-replication-recovery](prd.md#p2p-replication-recovery) | [des-p2p-replication-scope](#des-p2p-replication-scope) | writer/epoch从seq1及guard不污染；权威恢复错误与qualified storage-challenge fallback区分 | [testing-manual S4/S9A](../../testing-manual.md)；exact local test source `crates/oasis7_node/src/tests_fetch_blob_chunking.rs`；existing fetch_blob_storage_challenge_empty_provider_routes_probe_bounded_generic_route、fetch_blob_storage_challenge_without_provider_lookup_does_not_probe_generic_route；node局部fixture，不是DHT fetch fallback或QC证明。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | target实现与full组合未证明；外部pending/lineage/manifest/消费者字段未闭合 |
| [product_requirement: req-dcs-001](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-001) | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | 验证certificate及hash-bound材料，拒绝服务/cache/peer自授authority；runtime state apply仍外部 | [testing-manual S9A](../../testing-manual.md)；target full同candidate场景按对应DES；DCS001/004的required局部不降低根full组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: ac-dcs-001](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#ac-dcs-001) | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | 验证certificate及hash-bound材料，拒绝服务/cache/peer自授authority；runtime state apply仍外部 | [testing-manual S9A](../../testing-manual.md)；target full同candidate场景按对应DES；DCS001/004的required局部不降低根full组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: req-dcs-002](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-002) | [des-p2p-target-recovery](#des-p2p-target-recovery) | sameworld恢复链每环验证；历史只读与重新服务闸门分开 | [testing-manual S9A](../../testing-manual.md)；target full同candidate场景按对应DES；DCS001/004的required局部不降低根full组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: ac-dcs-002](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#ac-dcs-002) | [des-p2p-target-recovery](#des-p2p-target-recovery) | sameworld恢复链每环验证；历史只读与重新服务闸门分开 | [testing-manual S9A](../../testing-manual.md)；target full同candidate场景按对应DES；DCS001/004的required局部不降低根full组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: req-dcs-003](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-003) | [des-p2p-layer-authority](#des-p2p-layer-authority) | governed validator与permissionless服务隔离；公网IP非通用前提 | [testing-manual S9A](../../testing-manual.md)；target full同candidate场景按对应DES；DCS001/004的required局部不降低根full组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: ac-dcs-003](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#ac-dcs-003) | [des-p2p-layer-authority](#des-p2p-layer-authority) | governed validator与permissionless服务隔离；公网IP非通用前提 | [testing-manual S9A](../../testing-manual.md)；target full同candidate场景按对应DES；DCS001/004的required局部不降低根full组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: req-dcs-004](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-004) | [des-p2p-target-bft](#des-p2p-target-bft) | verified >2/3 active stake证书才推进；wrongset/equivocation/round缺证拒绝 | [testing-manual S9A](../../testing-manual.md)；target full同candidate场景按对应DES；DCS001/004的required局部不降低根full组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: ac-dcs-004](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#ac-dcs-004) | [des-p2p-target-bft](#des-p2p-target-bft) | verified >2/3 active stake证书才推进；wrongset/equivocation/round缺证拒绝 | [testing-manual S9A](../../testing-manual.md)；target full同candidate场景按对应DES；DCS001/004的required局部不降低根full组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: req-dcs-005](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-005) | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | 服务转换及manifest/head/finality conjunction；非serving intent receipt0 | [testing-manual S9A](../../testing-manual.md)；target full同candidate场景按对应DES；DCS001/004的required局部不降低根full组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: ac-dcs-005](../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#ac-dcs-005) | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | 服务转换及manifest/head/finality conjunction；非serving intent receipt0 | [testing-manual S9A](../../testing-manual.md)；target full同candidate场景按对应DES；DCS001/004的required局部不降低根full组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | 只接受该P2P portion；target实现/执行/完整组合未证明 |
| [product_requirement: req-dwe-001](../product/world-infrastructure/deterministic-world-execution.prd.md#req-dwe-001) | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | 同parent/version/order重执行及root绑定；runtime原子apply外部 | [testing-manual S9A](../../testing-manual.md)；对应DWE同candidate跨runtime/P2P/消费者replay/outage/lineage/activation负例计划；DWE004 full，其余局部required不代根组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: ac-dwe-001](../product/world-infrastructure/deterministic-world-execution.prd.md#ac-dwe-001) | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | 同parent/version/order重执行及root绑定；runtime原子apply外部 | [testing-manual S9A](../../testing-manual.md)；对应DWE同candidate跨runtime/P2P/消费者replay/outage/lineage/activation负例计划；DWE004 full，其余局部required不代根组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: req-dwe-002](../product/world-infrastructure/deterministic-world-execution.prd.md#req-dwe-002) | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | finality不可用保持无效果pending；恢复按当前条件重新裁决，consumer状态外部 | [testing-manual S9A](../../testing-manual.md)；对应DWE同candidate跨runtime/P2P/消费者replay/outage/lineage/activation负例计划；DWE004 full，其余局部required不代根组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: ac-dwe-002](../product/world-infrastructure/deterministic-world-execution.prd.md#ac-dwe-002) | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | finality不可用保持无效果pending；恢复按当前条件重新裁决，consumer状态外部 | [testing-manual S9A](../../testing-manual.md)；对应DWE同candidate跨runtime/P2P/消费者replay/outage/lineage/activation负例计划；DWE004 full，其余局部required不代根组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: req-dwe-003](../product/world-infrastructure/deterministic-world-execution.prd.md#req-dwe-003) | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | receipt无第二效果为交接条件；互斥lineage首有效receipt唯一胜者/终止其余由runtime拥有，拒绝过期只终止自身，独立intent并发 | [testing-manual S9A](../../testing-manual.md)；对应DWE同candidate跨runtime/P2P/消费者replay/outage/lineage/activation负例计划；DWE004 full，其余局部required不代根组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: ac-dwe-003](../product/world-infrastructure/deterministic-world-execution.prd.md#ac-dwe-003) | [des-p2p-target-service-gate](#des-p2p-target-service-gate) | receipt无第二效果为交接条件；互斥lineage首有效receipt唯一胜者/终止其余由runtime拥有，拒绝过期只终止自身，独立intent并发 | [testing-manual S9A](../../testing-manual.md)；对应DWE同candidate跨runtime/P2P/消费者replay/outage/lineage/activation负例计划；DWE004 full，其余局部required不代根组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: req-dwe-004](../product/world-infrastructure/deterministic-world-execution.prd.md#req-dwe-004) | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | 首次committed/finality-verified canonical block与activation选version；compat declaration不选规则，历史receipt原manifest replay，缺证failclosed；runtime/WASM详细设计外部 | [testing-manual S9A](../../testing-manual.md)；对应DWE同candidate跨runtime/P2P/消费者replay/outage/lineage/activation负例计划；DWE004 full，其余局部required不代根组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | external durability/lineage/activation/消费者设计及证据未闭合 |
| [product_requirement: ac-dwe-004](../product/world-infrastructure/deterministic-world-execution.prd.md#ac-dwe-004) | [des-p2p-receipt-finality](#des-p2p-receipt-finality) | 首次committed/finality-verified canonical block与activation选version；compat declaration不选规则，历史receipt原manifest replay，缺证failclosed；runtime/WASM详细设计外部 | [testing-manual S9A](../../testing-manual.md)；对应DWE同candidate跨runtime/P2P/消费者replay/outage/lineage/activation负例计划；DWE004 full，其余局部required不代根组合。 | 当次GitHub task Issue evidence、实际命令/exit code/log/artifact；实际source/integration/tested tree/config/environment/window在执行时绑定 | external durability/lineage/activation/消费者设计及证据未闭合 |

## 12. 决策、长期风险与未决问题

选择保留现有专业合同并精确分配，拒绝以导航/历史标签免除真实设计，也拒绝将local证明升级为full。目标锁定/解锁、anti-double-sign、timeout/new-round、set transition与partition/heal/restart需独立对抗性证明；本文给拒绝条件而不发明新lock算法/schema。pending/lineage/manifest/consumer字段及DC5 structured attachment仍外部未闭合。

runtime/P2P负责缺失技术/实现；ops负责环境事实；QA负责计划覆盖和verdict；producer审读产品含义。新算法/schema、权限/默认值变化、额外消费者或claim扩展触发重新评审；未解除项保留在GitHub正式evidence及剩余required set，不能靠未来Issue占位关闭。下列原章节的里程碑/审计轮次是保留provenance，不维护可变进度。



审计轮次: 7

- 对应需求文档: `doc/p2p/prd.md`
- 对应任务管理: GitHub Issue / GitHub Project
- 对应文件级索引: `doc/p2p/prd.index.md`

## 1. 设计定位
`p2p` 模块的 `design.md` 负责描述链上大世界状态底座的总体设计入口。该底座由网络传输、共识、分布式存储、状态同步、执行记录、observer/ops 与 projection/API 等子层共同组成；P2P transport 是其中的网络层证据，不单独代表大世界状态闭环。

## 2. 阅读顺序
1. `doc/p2p/prd.md`
2. `doc/p2p/design.md`
3. GitHub Issue / GitHub Project（任务状态与历史）
4. `doc/p2p/prd.index.md`
5. 需要验证模块级闭环时进入 `testing-manual.md#s9a链上大世界状态底座自闭环`
6. 下钻 `blockchain/`、`consensus/`、`distfs/`、`network/`、`node/` 等专题目录

## 3. 设计结构
- 网络层：节点发现、连接、同步、传输与非全公网 reachability 边界。
- 共识层：身份、投票、finality、状态传播与一致性策略。
- 存储层：DistFS、blob closure、路径索引、复制与恢复。
- 状态同步层：replication、gap sync、state sync、checkpoint 与 peer-head freshness。
- 节点执行层：execution record/receipt、奖励、执行校验与治理对接。
- 观测与投影层：observer/ops 证据、API/viewer projection 与 claim boundary。

历史 `distributed-*` 专题的稳定设计已归入上述分层：`oasis7_net` 拥有 transport/peer/DHT，`oasis7_consensus` 拥有 proposer/attestation/action-root，`oasis7_distfs` 拥有 CAS/replication/challenge，node/chain-runtime 拥有 commit 后执行与恢复接线，`oasis7_proto` 只拥有共享 wire 类型。`oasis7_wasm_abi` 是 `ModuleManifest` 与 runtime ABI 类型的单一来源，不得在 net/proto/viewer 再定义副本。

当前设计不承诺历史 `/aw/rr/*`、固定 bitswap/graphsync、每次 zone commit 的 lease 接管、viewer 默认执行 bridge 或跨节点 DistFS challenge topic/envelope。production WASM 只消费外部 canonical binary + receipt；本地 source compile fallback 仅可作为显式 dev/test 兼容，不是节点生产路径。

早期 `execution_storage` path-index、observer/bootstrap path-index 与 `runtime_bridge` compile closure 只保留历史 provenance：相关 net 源文件未由当前 crate facade 暴露，`runtime_bridge` 只是空兼容 feature。设计不得把 dormant source 或 feature 可编译提升为当前恢复、同步或 bridge 能力。

## 4. 集成点
- `doc/world-runtime/prd.md`
- `doc/headless-runtime/prd.md`
- `doc/testing/prd.md`

## 5. 专题导航
- 基础链路进入 `network/`、`consensus/`
- mixed-topology / 非全公网主链级覆盖网络进入 `network/mainnet-private-reachability-architecture.*`
- 数据与复制进入 `distfs/`
- 节点执行与奖励进入 `node/`
- 区块链和生产化扩展进入 `blockchain/`；分布式运行时总体边界由本页与根 PRD 承载，`distributed/README.md` 仅作 successor 导航
- 模块级自闭环、claim level 与测试层级进入 `testing-manual.md#s9a链上大世界状态底座自闭环`

## 设计目标
- 提供链上大世界状态底座的总体设计入口，并把历史 `P2P 基础设施` 口径收束为底座内网络/存储/同步等组件证据。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。
- 不把单一 transport、storage、observer 或 node 专题的 green 结果提升为 S9A `module_full`、`integration_required` 或 `release_full`。

## 关键接口 / 入口
- 需求入口：`doc/p2p/prd.md`
- 执行入口：GitHub Issue / GitHub Project
- 索引入口：`doc/p2p/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
