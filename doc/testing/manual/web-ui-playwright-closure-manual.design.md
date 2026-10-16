# oasis7：Web UI Playwright 闭环测试手册（2026-02-28）设计

- 历史/共享设计 companion: `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
- 历史任务追溯: GitHub task issue evidence comments 与 git history
- 当前 Playwright 实跑系列入口: `doc/testing/manual/web-ui-playwright-closure-manual.manual.md`

<a id="1-设计定位"></a>
## 1. 设计定位：目标与范围
定义测试手册专题设计，统一系统性测试、Web UI Playwright 闭环与工程化维护方式。

## 2. 设计结构
- 手册结构层：明确入口、章节组织与适用范围。
- 执行方法层：沉淀测试步骤、命令模板与证据要求。
- 工具链对齐层：把 Playwright、系统测试与门禁口径统一到手册。
- 维护治理层：建立版本更新、引用互链与长期维护约定。

<a id="playwright-source-capture-method"></a>
跨层场景采用可审计的 capture→action→oracle→artifact join：先固定 QA receiver 指定的 source clause、candidate/tree、world/branch/window、profile、真实依赖和 viewport；再通过真实可见 UI 控件完成玩家动作；`__AW_TEST__` 仅读取 readiness/state/history，不执行动作或代替权威 oracle；之后按 before/action/pending-or-outage/fresh-recovery/final 顺序采样并对齐同候选 API/journal/Agent authority refs。语义由 [QA scene contract](../prd.md#qa-infrastructure-scene-families)决定，本设计只定义页面采证的可重复步骤。

### 2.1 需求承接与分配表

本次有限分配只展开原QA采证方法与四个planned接收场景，不改companion的Viewer/launcher driver分流、renderer/S6、F1–F4、named session/owned close、prompt/chat/echo、时限、兼容产物目录与脱敏义务；这些仍由原companion、通用manual及各专业authority独立承接。下表Trace refs仅引用既有唯一声明，不改变产品覆盖/实现状态。


| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [professional_acceptance: 采证计划与记录](../prd.md#qa-infrastructure-receiver) | UI evidence joins selected QA scene, candidate, viewport, oracle, archive and tier; visible actions remain distinct from read-only state probes. | [doc/testing/manual/web-ui-playwright-closure-manual.design.md#playwright-source-capture-method](web-ui-playwright-closure-manual.design.md#playwright-source-capture-method) | QA owns verdict/scene; Viewer and Agent own their surfaces; Runtime/P2P/gameplay own authoritative oracles. | Design defines capture only; no browser/provider run, runtime implementation, release decision, or playability claim.  产品层 N/A；reason: 本关系仅定义QA如何绑定source、scene与可审计产物，不独立定义玩家世界结果；scope: 仅原采证方法、可见动作与只读探针区分；下列产品场景及其全部权威oracle仍分别适用；仅免新增独立产品叶分配，不免专业验收。 |
| [professional_acceptance: 采证环境与层级判定](../prd.md#qa-infrastructure-tier-boundaries) | Keep required deterministic/API evidence separate from full real-provider, Agent parity, and external headed desktop+narrow evidence. | [doc/testing/manual/web-ui-playwright-closure-manual.design.md#playwright-full-tier-boundary](web-ui-playwright-closure-manual.design.md#playwright-full-tier-boundary) | QA selects tier; Ops/provider owners supply real dependency provenance; Agent/Viewer owners verify actual consumer paths. | Mock, screenshot-only, local contract, or absent viewport cannot satisfy the full tier.  产品层 N/A；reason: 本关系只定义QA采证环境的选择和证据判定，不能独立授予发布或玩法结论；scope: 仅required/full采证边界；下列SC-9环境子义务和原full完整要求均保持；仅免新增独立产品叶分配，不免专业验收。 |
| Trace refs: O7-REQ-05c84021-ee48-41ef-b94a-8aa9ceb8048c；[product_requirement: SC-8](../../product/world-infrastructure/prd.md#sc-infra-008) | PWT-003 / V-INDUSTRIAL、V-CONSUMER：可见推荐动作的ACK与canonical receipt、同世界state delta、数量和lineage分别采集。 | [采证条款](#playwright-source-capture-method) | Gameplay/Runtime拥有工业oracle；Viewer/Agent拥有消费表面；QA按所选scene判定。 | 仅原可见动作的消费采证；不证明完整工业root、hold、背压或整条流水线。 |
| Trace refs: O7-REQ-4fcbad4f-f576-4bcf-b4fc-b027967b0405；[product_requirement: REQ-DWE-002](../../product/world-infrastructure/deterministic-world-execution.prd.md#req-dwe-002) | PWT-005 / V-EXECUTION、V-CONSUMER：在选中无效果待决/推进反馈场景时，区分queued、completed_advanced、blocker与正向world delta，不以按钮响应推断世界推进。 | [采证条款](#playwright-source-capture-method) | Runtime/P2P拥有pending/committed权威；Viewer呈现；QA对账。 | 仅待决无效果与真实progression的消费判读；不定义pause权限、pending持久化或恢复协议。 |
| Trace refs: O7-REQ-ee563883-fe1d-4192-b58b-53251d32920b；[product_requirement: REQ-DCS-005](../../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-005) | PWT-006 / V-RECOVERY、V-CONSUMER：仅当选中服务状态转换场景时，采集当前状态、受影响操作、最后可信边界、blocker与真实下一步。 | [采证条款](#playwright-source-capture-method) | P2P/Runtime拥有service gate；Ops分类实际故障依赖；Viewer/Agent投影；QA判定。 | 普通provider断线不证明service gate；完整DCS-005的receipt 0/1、正向新请求/模块行动续跑与合法新配方仍独立验收。 |
| Trace refs: O7-REQ-2b7cfef9-d917-46ea-a3a9-e3a4fb96d730；[product_requirement: SC-10](../../product/world-infrastructure/prd.md#sc-infra-010) | PWT-007 / V-WORLD-SCOPE、V-CONSUMER：锁定同world、branch、candidate和observation window，采集target scope与receipt/world identity；local证据不呈现为global effect。 | [采证条款](#playwright-source-capture-method) | Runtime拥有world identity；Viewer与pure API拥有消费；QA核对同候选字段。 | 只接收消费者identity和语义对账；不证明全部跨世界拒绝实现，不把no-LLM observer/debug或不同候选当parity。 |
| Trace refs: O7-REQ-4792541c-aaa4-4ac1-9a2d-374df44eba6a；[product_requirement: SC-9](../../product/world-infrastructure/prd.md#sc-infra-009) | 仅SC-9所选full采证环境子义务：同候选真实provider、本地栈、外部headed desktop+narrow截图/console与provider-backed Agent parity。 | [采证条款](#playwright-full-tier-boundary) | QA选择tier与verdict；Ops/provider确认依赖provenance；Agent/Viewer确认实际路径。 | 不把四PWT计划替代八格authority/read×四边界矩阵，不新增第九格；required deterministic/API与full完整义务独立保留。 |
| [professional_acceptance: source-to-scene record](../prd.md#qa-infrastructure-source-scene-method) | 逐记录固定case/action、candidate/world/window、lane、provider provenance、raw artifact locator/byte length/digest、失败signature与exact unmet assertion；保留planned与实际执行状态的区别。 | [证据记录合同](#playwright-evidence-record-contract) | QA拥有记录完整性；domain owner拥有语义oracle，Ops/provider拥有环境事实。 | 产品层 N/A；reason: 本项仅是QA材料身份、字节完整性和失败记录的内部证据合同，不独立定义玩家结果；scope: 仅原记录字段与artifact join；artifact digest只证明字节完整性，不是domain receipt/finality；不免上述产品场景oracle和full要求。 仅免新增产品叶分配，不免专业验收。 |

## 3. 关键接口 / 入口
- 测试手册入口
- 步骤/命令模板
- Playwright / 系统测试链接点
- 手册维护约定

<a id="playwright-evidence-record-contract"></a>
每条 UI 记录绑定 `caseId`、可见 action、候选/world/window 身份、访问模式和执行 lane、关键输入/输出、反馈阶段、provider kind/model/endpoint route/preflight provenance（如适用）、raw screenshot/state/console/request-error/API/journal refs、每份 artifact 的 byte length/digest、mock 禁用/检测结果、失败 signature 与 exact unmet assertion。秘密只由受控运行时持有，不复制进报告或 artifact。PWT-003/005/006/007 按手册矩阵保持 planned，直到相同候选实际执行并保存可读产物。

<a id="4-约束与边界"></a>
## 4. 约束与边界：风险限制
- 手册必须服务真实测试流程而非重复 PRD。
- 命令与步骤需可直接执行或映射。
- 不在本专题扩展新的测试框架。
- 同候选 UI 语义应与 API/journal/Agent 权威 observation 逐字段对齐；本地化文本可不同。按钮 ACK、截图或 debug snapshot 单独不能证明 committed effect、finality 或 world parity。
- `test_tier_full` 的 headed UI 证据要求外部真实 browser、真实 provider-backed path、desktop+narrow 两类 viewport 的截图/console，以及适用的 provider-backed Agent parity。`provider_local_mock` 只作 plumbing 诊断；缺失 full 环境要标 `blocked`/`unverified`。
- 按 [QA tier boundary](../prd.md#qa-infrastructure-tier-boundaries) 区分 shape-only、deterministic contract 与 full environment；此 capture design 不改变 PWT-004 日期状态或把规划用例写成执行结果。
- <a id="playwright-full-tier-boundary"></a>外部 headed desktop+narrow 的 browser/runtime/provider 和 Agent-parity proof 必须来自同一候选及对应窗口；本篇不控制 provider availability，也不将规划矩阵或 stale artifacts 升格为已验证。

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [professional_acceptance: 采证计划与记录](../prd.md#qa-infrastructure-receiver) | [doc/testing/manual/web-ui-playwright-closure-manual.design.md#playwright-source-capture-method](web-ui-playwright-closure-manual.design.md#playwright-source-capture-method) | PWT-003/005/006/007 capture must bind visible action to the QA scene and same-candidate authoritative API/journal/Agent observation; only applicable when a player-facing browser flow is selected. | Manual contract only, not execution: [PWT planned matrix](web-ui-playwright-closure-manual.manual.md#pwt-planned-matrix) defines PWT-003, PWT-005, PWT-006, PWT-007. On actual future run, freeze candidate/tree/world/window before UI action; browser selection follows S6. | Future same-candidate action/state/API-or-journal refs, each raw screenshot/state/console/request-error artifact with locator, byte length, and digest; current authoring evidence is the planned method only. | These four cases remain planned and unexecuted here; no actual provider, browser, runtime, or gameplay result is claimed. |
| [professional_acceptance: 采证环境与层级判定](../prd.md#qa-infrastructure-tier-boundaries) | [doc/testing/manual/web-ui-playwright-closure-manual.design.md#playwright-full-tier-boundary](web-ui-playwright-closure-manual.design.md#playwright-full-tier-boundary) | The same-candidate real provider, external headed desktop+narrow browser, and provider-backed Agent parity requirements apply only to `test_tier_full`; required deterministic/API evidence does not satisfy them. | [testing-manual.md S6](../../../testing-manual.md#testing-s6-qa-receiver) is the execution selector; PWT manual matrix is the scenario source. No browser/provider command was run in this authoring slice. | If run later, preserve provider kind/model/endpoint route/preflight provenance, actual browser/platform/viewport, same-candidate consumer refs and both viewport capture sets. | Provider availability, active-LLM API parity, external headed environment and Agent parity remain unverified; this design does not establish required/full, release, or playability pass. |
| Trace refs: O7-REQ-05c84021-ee48-41ef-b94a-8aa9ceb8048c；[product_requirement: SC-8](../../product/world-infrastructure/prd.md#sc-infra-008) | [采证条款](#playwright-source-capture-method) | PWT-003 / V-INDUSTRIAL、V-CONSUMER：可见推荐动作的ACK与canonical receipt、同世界state delta、数量和lineage分别采集。 | [PWT planned matrix](web-ui-playwright-closure-manual.manual.md#pwt-planned-matrix)；PWT-003 planned：固定同candidate/tree/world/window和目标，实际可见提交一次，分别保存ACK、receipt、state delta与数量/lineage。 | 未来同候选source/action/state/API或journal refs及raw screenshot/console/request-error，逐artifact保存locator、byte length、digest和exact unmet assertion；当前仅方法定义。 | 仅原可见动作的消费采证；不证明完整工业root、hold、背压或整条流水线。 本次未执行，不构成browser/provider/runtime/recovery或playability通过。 |
| Trace refs: O7-REQ-4fcbad4f-f576-4bcf-b4fc-b027967b0405；[product_requirement: REQ-DWE-002](../../product/world-infrastructure/deterministic-world-execution.prd.md#req-dwe-002) | [采证条款](#playwright-source-capture-method) | PWT-005 / V-EXECUTION、V-CONSUMER：在选中无效果待决/推进反馈场景时，区分queued、completed_advanced、blocker与正向world delta，不以按钮响应推断世界推进。 | [PWT planned matrix](web-ui-playwright-closure-manual.manual.md#pwt-planned-matrix)；PWT-005 planned：固定formal viewer profile及runtime/provider lane，经可见控件操作，对照前后state、feedback stage及同候选事件/receipt；没有权威world delta不能认推进。 | 未来同候选source/action/state/API或journal refs及raw screenshot/console/request-error，逐artifact保存locator、byte length、digest和exact unmet assertion；当前仅方法定义。 | 仅待决无效果与真实progression的消费判读；不定义pause权限、pending持久化或恢复协议。 本次未执行，不构成browser/provider/runtime/recovery或playability通过。 |
| Trace refs: O7-REQ-ee563883-fe1d-4192-b58b-53251d32920b；[product_requirement: REQ-DCS-005](../../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-005) | [采证条款](#playwright-source-capture-method) | PWT-006 / V-RECOVERY、V-CONSUMER：仅当选中服务状态转换场景时，采集当前状态、受影响操作、最后可信边界、blocker与真实下一步。 | [PWT planned matrix](web-ui-playwright-closure-manual.manual.md#pwt-planned-matrix)；PWT-006 planned：固定实际依赖与恢复窗口，经授权注入或观察故障，按before/outage/fresh-recovery/final对账状态、旧历史及receipt；不重放非幂等动作。无真实依赖标blocked/unverified。 | 未来同候选source/action/state/API或journal refs及raw screenshot/console/request-error，逐artifact保存locator、byte length、digest和exact unmet assertion；当前仅方法定义。 | 普通provider断线不证明service gate；完整DCS-005的receipt 0/1、正向新请求/模块行动续跑与合法新配方仍独立验收。 本次未执行，不构成browser/provider/runtime/recovery或playability通过。 |
| Trace refs: O7-REQ-2b7cfef9-d917-46ea-a3a9-e3a4fb96d730；[product_requirement: SC-10](../../product/world-infrastructure/prd.md#sc-infra-010) | [采证条款](#playwright-source-capture-method) | PWT-007 / V-WORLD-SCOPE、V-CONSUMER：锁定同world、branch、candidate和observation window，采集target scope与receipt/world identity；local证据不呈现为global effect。 | [PWT planned matrix](web-ui-playwright-closure-manual.manual.md#pwt-planned-matrix)；PWT-007 planned：先执行一项可见动作，再以active-provider pure API读取同一canonical状态，核对status/blocker、receipt/root和next action；保留两侧原始字段及provider身份。 | 未来同候选source/action/state/API或journal refs及raw screenshot/console/request-error，逐artifact保存locator、byte length、digest和exact unmet assertion；当前仅方法定义。 | 只接收消费者identity和语义对账；不证明全部跨世界拒绝实现，不把no-LLM observer/debug或不同候选当parity。 本次未执行，不构成browser/provider/runtime/recovery或playability通过。 |
| Trace refs: O7-REQ-4792541c-aaa4-4ac1-9a2d-374df44eba6a；[product_requirement: SC-9](../../product/world-infrastructure/prd.md#sc-infra-009) | [采证条款](#playwright-full-tier-boundary) | 仅SC-9所选full采证环境子义务：同候选真实provider、本地栈、外部headed desktop+narrow截图/console与provider-backed Agent parity。 | [S6执行选择](../../../testing-manual.md#testing-s6-qa-receiver)；SC-9 full planned：依QA tier与主手册S6选择环境，记录provider kind/model/endpoint route/preflight provenance、实际browser/platform与两viewport capture；任何必需环境缺失保持blocked/unverified。 | 未来同候选source/action/state/API或journal refs及raw screenshot/console/request-error，逐artifact保存locator、byte length、digest和exact unmet assertion；当前仅方法定义。 | 不把四PWT计划替代八格authority/read×四边界矩阵，不新增第九格；required deterministic/API与full完整义务独立保留。 本次未执行，不构成browser/provider/runtime/recovery或playability通过。 |
| [professional_acceptance: source-to-scene record](../prd.md#qa-infrastructure-source-scene-method) | [证据记录合同](#playwright-evidence-record-contract) | 逐记录固定case/action、candidate/world/window、lane、provider provenance、raw artifact locator/byte length/digest、失败signature与exact unmet assertion；保留planned与实际执行状态的区别。 | [PWT planned matrix](web-ui-playwright-closure-manual.manual.md#pwt-planned-matrix)的每次记录逐项核对字段、原始产物可读性和byte length/digest，并将actual action与同候选权威oracle对齐；planned，无实际执行。 | 原始artifact及逐字段record、缺项/冲突原因、准确未满足断言；没有可读产物不能升级verdict。 | artifact digest只证明字节完整性，不是domain receipt/finality；不免上述产品场景oracle和full要求。 未运行，不声称schema/fixture或浏览器通过。 |

## 5. 设计演进计划
- 先固化手册目录与范围。
- 再补步骤模板和工具链对齐。
- 最后沉淀维护约定。
