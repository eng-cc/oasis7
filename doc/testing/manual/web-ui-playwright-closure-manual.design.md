# oasis7：Web UI Playwright 闭环测试手册（2026-02-28）设计

- 历史/共享设计 companion: `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
- 历史任务追溯: GitHub task issue evidence comments 与 git history
- 当前 Playwright 实跑系列入口: `doc/testing/manual/web-ui-playwright-closure-manual.manual.md`

## 1. 设计定位
定义测试手册专题设计，统一系统性测试、Web UI Playwright 闭环与工程化维护方式。

## 2. 设计结构
- 手册结构层：明确入口、章节组织与适用范围。
- 执行方法层：沉淀测试步骤、命令模板与证据要求。
- 工具链对齐层：把 Playwright、系统测试与门禁口径统一到手册。
- 维护治理层：建立版本更新、引用互链与长期维护约定。

<a id="playwright-source-capture-method"></a>
跨层场景采用可审计的 capture→action→oracle→artifact join：先固定 QA receiver 指定的 source clause、candidate/tree、world/branch/window、profile、真实依赖和 viewport；再通过真实可见 UI 控件完成玩家动作；`__AW_TEST__` 仅读取 readiness/state/history，不执行动作或代替权威 oracle；之后按 before/action/pending-or-outage/fresh-recovery/final 顺序采样并对齐同候选 API/journal/Agent authority refs。语义由 [QA scene contract](../prd.md#qa-infrastructure-scene-families)决定，本设计只定义页面采证的可重复步骤。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [doc/testing/prd.md#qa-infrastructure-receiver](../prd.md#qa-infrastructure-receiver) | UI evidence joins selected QA scene, candidate, viewport, oracle, archive and tier; visible actions remain distinct from read-only state probes. | [doc/testing/manual/web-ui-playwright-closure-manual.design.md#playwright-source-capture-method](web-ui-playwright-closure-manual.design.md#playwright-source-capture-method) | QA owns verdict/scene; Viewer and Agent own their surfaces; Runtime/P2P/gameplay own authoritative oracles. | Design defines capture only; no browser/provider run, runtime implementation, release decision, or playability claim. |
| [doc/testing/prd.md#qa-infrastructure-tier-boundaries](../prd.md#qa-infrastructure-tier-boundaries) | Keep required deterministic/API evidence separate from full real-provider, Agent parity, and external headed desktop+narrow evidence. | [doc/testing/manual/web-ui-playwright-closure-manual.design.md#playwright-full-tier-boundary](web-ui-playwright-closure-manual.design.md#playwright-full-tier-boundary) | QA selects tier; Ops/provider owners supply real dependency provenance; Agent/Viewer owners verify actual consumer paths. | Mock, screenshot-only, local contract, or absent viewport cannot satisfy the full tier. |

## 3. 关键接口 / 入口
- 测试手册入口
- 步骤/命令模板
- Playwright / 系统测试链接点
- 手册维护约定

<a id="playwright-evidence-record-contract"></a>
每条 UI 记录绑定 `caseId`、可见 action、候选/world/window 身份、访问模式和执行 lane、关键输入/输出、反馈阶段、provider kind/model/endpoint route/preflight provenance（如适用）、raw screenshot/state/console/request-error/API/journal refs、每份 artifact 的 byte length/digest、mock 禁用/检测结果、失败 signature 与 exact unmet assertion。秘密只由受控运行时持有，不复制进报告或 artifact。PWT-003/005/006/007 按手册矩阵保持 planned，直到相同候选实际执行并保存可读产物。

## 4. 约束与边界
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
| [doc/testing/prd.md#qa-infrastructure-receiver](../prd.md#qa-infrastructure-receiver) | [doc/testing/manual/web-ui-playwright-closure-manual.design.md#playwright-source-capture-method](web-ui-playwright-closure-manual.design.md#playwright-source-capture-method) | PWT-003/005/006/007 capture must bind visible action to the QA scene and same-candidate authoritative API/journal/Agent observation; only applicable when a player-facing browser flow is selected. | Manual contract only, not execution: [PWT planned matrix](web-ui-playwright-closure-manual.manual.md#pwt-planned-matrix) defines PWT-003, PWT-005, PWT-006, PWT-007. On actual future run, freeze candidate/tree/world/window before UI action; browser selection follows S6. | Future same-candidate action/state/API-or-journal refs, each raw screenshot/state/console/request-error artifact with locator, byte length, and digest; current authoring evidence is the planned method only. | These four cases remain planned and unexecuted here; no actual provider, browser, runtime, or gameplay result is claimed. |
| [doc/testing/prd.md#qa-infrastructure-tier-boundaries](../prd.md#qa-infrastructure-tier-boundaries) | [doc/testing/manual/web-ui-playwright-closure-manual.design.md#playwright-full-tier-boundary](web-ui-playwright-closure-manual.design.md#playwright-full-tier-boundary) | The same-candidate real provider, external headed desktop+narrow browser, and provider-backed Agent parity requirements apply only to `test_tier_full`; required deterministic/API evidence does not satisfy them. | [testing-manual.md S6](../../../testing-manual.md#testing-s6-qa-receiver) is the execution selector; PWT manual matrix is the scenario source. No browser/provider command was run in this authoring slice. | If run later, preserve provider kind/model/endpoint route/preflight provenance, actual browser/platform/viewport, same-candidate consumer refs and both viewport capture sets. | Provider availability, active-LLM API parity, external headed environment and Agent parity remain unverified; this design does not establish required/full, release, or playability pass. |

## 5. 设计演进计划
- 先固化手册目录与范围。
- 再补步骤模板和工具链对齐。
- 最后沉淀维护约定。
