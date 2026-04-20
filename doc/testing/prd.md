# testing PRD

审计轮次: 7

## 目标
- 建立 testing 模块设计主文档，统一需求边界、技术方案与验收标准。
- 确保 testing 模块后续改动可追溯到 PRD-ID、任务和测试。

## 范围
- 覆盖 testing 模块当前能力设计、接口边界、测试口径与演进路线。
- 覆盖 PRD-ID 到 `doc/testing/project.md` 的任务映射。
- 不覆盖实现代码逐行说明与历史过程记录。

## 接口 / 数据
- PRD 主入口: `doc/testing/prd.md`
- 项目管理入口: `doc/testing/project.md`
- 文件级索引: `doc/testing/prd.index.md`
- 追踪主键: `PRD-TESTING-xxx`
- 测试与发布参考: `testing-manual.md`

## 里程碑
- M1 (2026-03-03): 完成模块设计 PRD 主体重写与任务改造。
- M2: 补齐模块设计验收清单与关键指标。
- M3: 建立 PRD-ID -> Task -> Test 的长期追踪闭环。

## 风险
- 模块边界演进快，文档同步可能滞后。
- 指标口径不稳定会降低验收一致性。
## 1. Executive Summary
- Problem Statement: 测试套件覆盖范围广（required/full、Web 闭环、长跑、分布式），但目标与触发矩阵若不集中维护，容易出现“通过 CI 但缺少真实风险覆盖”。
- Proposed Solution: 以 testing PRD 统一定义分层测试体系、触发条件、证据标准与发布门禁，并对齐 `testing-manual.md`。
- Success Criteria:
  - SC-1: 关键改动路径均可映射到明确测试层级（S0~S10）。
  - SC-2: required/full 门禁持续可用且与手册口径一致；其中 PR `required-gate` 允许在保持稳定 check context 的前提下按 changed paths 剪裁无关重型组件。
- SC-3: Web UI 闭环与分布式长跑在发布流程中有可追溯证据，且明确区分 `Viewer(agent-browser)` 与 `launcher(GUI Agent first)` 两条驱动链路。
  - SC-3A: `release-gate-web` 在 `renderMode=software_safe` 的主 Web 入口上，必须接受 `play/pause` 先返回 `queued` 的 live-control 契约，并以后续 `step` 收到 `completed_advanced` 且产出正向 world delta 作为 formal progress 判据，不再要求 `play` 立刻推进 tick 或强制选中 Agent。
  - SC-4: 测试任务 100% 映射 PRD-TESTING-ID。
  - SC-5: 活跃 testing 专题文档按批次完成人工迁移到 strict schema，并统一 `*.prd.md` / `*.project.md` 命名。
- SC-6: builtin wasm（m1/m4/m5）hash 发布链路具备 changed-path scope planner、跨 runner 对账、required check 保护与本地只读校验策略。
  - SC-7: 主链 Token 创世前具备一份 QA 审计清单，覆盖分配比例、custody/treasury 语义、个人上限、创世流通与首年释放上限，避免带着错误经济配置进入执行。

## 2. User Experience & Functionality
- User Personas:
  - 测试维护者：需要统一分层模型与执行标准。
  - 功能开发者：需要明确改动后最小必跑集合。
  - 发布负责人：需要审计级测试证据判断放行。
  - 制作人与经济配置维护者：需要一份可审计的创世配置检查表，避免只靠聊天结论发币。
- User Scenarios & Frequency:
  - 开发分支回归：每次核心改动后触发一次 required 路径。
  - 发布候选验证：每个候选版本执行 required + full 组合。
  - 专项长跑：高风险链路按周执行并沉淀趋势结果。
  - 失效复盘：出现逃逸缺陷后补齐回归与触发矩阵。
  - 前期工业体验回归：影响 `首个制成品 / 停机恢复 / 首座工厂单元` 时，补跑 required-tier 手动卡组。
  - 创世配置冻结前审计：每次准备冻结 Token 分配表时执行一次 required-tier 配置审计。
- User Stories:
  - PRD-TESTING-001: As a 测试维护者, I want one canonical testing strategy, so that suite evolution stays coherent.
  - PRD-TESTING-002: As a 开发者, I want clear trigger matrices, so that I can run the right tests efficiently.
  - PRD-TESTING-003: As a 发布负责人, I want auditable evidence bundles, so that release decisions are defensible.
  - PRD-TESTING-004: As a 文档维护者, I want each legacy testing topic doc manually migrated with content-preserving rewrite, so that historical intent remains accurate after format upgrade.
  - PRD-TESTING-005: As a 发布工程维护者, I want builtin wasm hash chain hardened end-to-end, so that hash drift can be blocked and traced before release.
  - PRD-TESTING-006: As a `qa_engineer`, I want a token genesis allocation audit checklist, so that producer/runtime can freeze mint configuration without hidden control or circulation risk.
- Critical User Flows:
  1. Flow-TST-001: `识别改动类型 -> 匹配 S0~S10 -> 日常提交先执行 commit baseline，再按风险升级到 required/full -> 输出结果`
  2. Flow-TST-002: `发布前执行 full 套件 -> 按 Viewer/launcher 选择正确驱动链路 -> 汇总命令/日志/截图 -> 生成证据包`
  3. Flow-TST-003: `线上问题复盘 -> 回填触发矩阵 -> 增加回归用例 -> 验证闭环`
  4. Flow-TST-004: `逐篇阅读 legacy 专题文档 -> 按 strict schema 人工重写 -> 改名为 .prd/.project -> 回归校验`
  5. Flow-TST-005: `触发 wasm hash 校验 -> 跨 runner 对账 -> required check 放行/阻断 -> 发布链路收口`
  6. Flow-TST-006: `识别工业引导体验改动 -> 运行自动化前置 -> 执行 playability 卡组 -> 回写 QA 阻断结论`
  7. Flow-TST-007: `读取 token 创世参数表 -> 逐项核对比例/recipient/vesting/流通边界 -> 输出 QA verdict -> 回流 producer 决策`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 分层测试触发 | 改动类型、测试层级、命令集合、changed-path scope | 依据矩阵选择最小必跑集合；PR `required-gate` 先规划 `minimal / targeted / full` 再执行命中的重型组件 | `planned -> scoped -> running -> passed/failed` | 默认先 `commit`，按风险升级到 `required`，发布加跑 `full`；docs-only / 无关元数据 PR 允许在 stable context 下退化为 governance/fmt-only | 开发者可执行，发布者可放行 |
| Web UI 驱动分流 | `surface_type`、`driver`、`evidence_mode` | Viewer 页面默认走 `agent-browser`；`oasis7_web_launcher` 产品动作默认走 GUI Agent，页面仅做状态/字段校验 | `selected -> driven -> verified` | 先按 surface 分流，再决定是否补充 Canvas/页面采样 | QA/发布与产品 owner 共同遵循 |
| software_safe release 语义门禁 | `renderMode`、`lastControlFeedback.stage`、`deltaLogicalTime`、`deltaEventSeq` | `software_safe` 下允许 `play/pause` 先返回 `queued`；formal progress 以后续 `step` 的 `completed_advanced` + 正向 world delta 判定 | `queued -> completed_advanced` 或 `queued -> blocked` | 主 Web 入口不再要求 `play` 立刻涨 tick，也不再强制 `selectedKind=agent`；若 `llm_required` 显式阻断则按 blocker 合约留痕 | QA/发布维护者维护 |
| 证据包归档 | 命令、日志、截图、结论、责任人 | 执行后归档并建立索引 | `collecting -> archived -> reviewed` | 按版本与模块分层索引 | 测试维护者负责最终校验 |
| 缺陷回归闭环 | 缺陷ID、触发条件、修复提交、复测结论 | 缺陷关闭前必须绑定回归记录 | `opened -> fixed -> regressed -> closed` | 高风险缺陷优先回归 | QA/维护者可更新状态 |
| 文档格式迁移 | 旧文档路径、约束点清单、目标命名 | 人工重写并更名，补全映射与验证证据 | `inventory -> migrated -> validated` | 先迁移活跃文档、后迁移归档文档 | 维护者审批迁移质量，贡献者执行 |
| Builtin wasm hash 治理 | 模块集、canonical token、runner 摘要、required check context、release evidence、scope planner | 执行 Docker canonical `sync --check`、按 changed paths 规划 scope、摘要导出与证据对账、分支保护同步 | `check-only -> planned -> reconciled -> protected` | 发布清单仅允许 `linux-x86_64` canonical token，identity 输入使用 receipt + 白名单；无关 PR 保持 stable required-context no-op | 本地默认只读校验，写路径限定非 CI 的显式授权 |
| Release 资产预构建复用 | web dist artifact、cargo cache key、bundle build command set | 同一 release workflow 先产出 viewer/launcher 静态包并复用 warm cache；后续打包不得重复 bootstrap 相同 Web 产物 | `bootstrapped -> reused -> packaged` | 先复用同轮 artifact / cache，再允许脚本 fallback；原生 bundle 构建优先单次 cargo 调用 | QA / 发布维护者维护 release 时延口径 |
| Windows 路径兼容校验 | tracked path、invalid segment、gate command、release runner | 在 required gate 早期扫描 git tracked paths，阻断 `windows-2022` 无法 checkout 的文件名进入 release/package-native | `scanned -> pass/block` | 默认按 git tracked path 全量扫描；发现 Windows 非法字符、保留名、尾随空格/点即直接 fail | QA / 发布维护者维护跨平台 release 可达性 |
| Runtime gate 分片执行 | full-suite shard、sync check、runner capability、日志 artifact | 将 release runtime gate 拆成 core/support/sync 并行 job；聚合 gate 统一裁决是否放行 | `planned -> sharded -> aggregated` | 重型 `oasis7` full-tier 优先单独成 shard，其余 support / sync 独立并行；最终必须全部成功 | QA / 发布维护者维护 runtime 关键路径 |
| Token 创世配置审计 | `bucket_id`、`ratio_bps`、`recipient`、`cliff_epochs`、`linear_unlock_epochs`、`genesis_liquid`、`founder_cap_bps`、`year1_external_release_cap_bps` | 逐项核对参数表与经济口径，输出 `pass/block` 审计结论 | `draft -> audited -> pass/block` | `sum=10000 bps`；项目战略控制 `5000 bps`；协议长期储备 `3500 bps`；`genesis_liquid=0`；个人上限 `<=1500 bps` | `qa_engineer` 独立出具结论，producer 决定是否冻结 |
- Acceptance Criteria:
  - AC-1: testing PRD 覆盖分层模型、触发矩阵、证据规范。
  - AC-2: testing project 文档维护分层测试演进任务。
  - AC-3: 与 `testing-manual.md` 保持一致且互相引用。
  - AC-4: 新增测试流程需标注 `test_tier_required` 或 `test_tier_full`。
  - AC-5: 每个迁移批次必须提供“原文约束点 -> 新章节映射”并通过文档治理检查。
  - AC-6: builtin wasm 发布链路治理（Docker canonical build + single canonical token + wasm-determinism-gate + required check + identity/release evidence 输入收敛）具备独立专题与任务追踪。
  - AC-7: `oasis7_web_launcher` / launcher Web 控制面必须显式标注 GUI Agent 优先，`agent-browser` 仅作为状态、字段与页面加载校验补充。
  - AC-8: 对前期工业引导体验的改动，必须能从 `testing-manual.md` 直接跳转到对应 required-tier 手动卡组。
  - AC-9: 同一 release workflow 内，Web release gate 与 `build-web-dist` 必须复用同一组 wasm/cargo cache，bundle 原生二进制构建默认收敛为单次 cargo 调用，避免重复 bootstrap。
  - AC-9B: `release-gate-web` 的 `software_safe` 分支必须按当前 live-control 契约验收：`play/pause` 允许 `queued`，后续 `step` 必须收口为 `completed_advanced` 且 `deltaLogicalTime > 0` 或 `deltaEventSeq > 0`；缺失 world delta 时要给出显式失败签名，而不是把“未即时涨 tick / 未选中 Agent”误判成回归。
  - AC-9A: release/package-native 触发前必须由 Linux required gate 显式扫描 tracked paths 的 Windows 兼容性，阻断会让 `windows-2022` checkout 直接失败的路径名。
  - AC-10: `release-gate-runtime` 必须允许将 `ci-tests.sh full` 拆为至少两个并行 shard，并与 builtin wasm sync 检查独立聚合，保证放行语义不变。
  - AC-11: runtime shard 划分必须按关键路径持续重平衡；`oasis7 --lib --bins` 等中重量级套件不应长期挤占最重 shard。
  - AC-12: `doc/testing/**` 仍可读历史专题的首行标题必须统一使用 `oasis7` / `oasis7 Runtime` 品牌；旧 `oasis7*` 标题仅允许保留在正文历史上下文与证据原文中。
  - AC-13: `token-genesis-allocation-audit-checklist-2026-03-22` 专题文档与执行模板落盘并映射 `TASK-TESTING-062`，明确创世参数审计项、阻断条件、证据字段与 verdict 口径。
- Non-Goals:
  - 不在本 PRD 中替代业务模块的功能设计。
  - 不承诺所有测试都进入 CI 默认路径。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: `scripts/ci-tests.sh`、agent-browser 闭环工具、`oasis7_web_launcher` GUI Agent 接口、长跑脚本、结果汇总工具。
- Evaluation Strategy: 通过门禁通过率、缺陷逃逸率、回归定位时长、证据完整度衡量测试体系质量。

## 4. Technical Specifications
- Architecture Overview: testing 模块是仓库级验证层，负责连接代码改动、测试触发、证据产物与发布门禁。
- Integration Points:
  - `testing-manual.md`
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
  - `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.prd.md`
  - `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md`
  - `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
  - `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.prd.md`
  - `scripts/check-windows-paths.sh`
  - `scripts/ci-tests.sh`
  - `scripts/sync-m1-builtin-wasm-artifacts.sh`
  - `scripts/ci-m1-wasm-summary.sh`
  - `scripts/ci-verify-m1-wasm-summaries.py`
  - `.github/workflows/*`
- Edge Cases & Error Handling:
  - 网络波动：外部依赖失败时记录失败签名并支持重试，不静默跳过。
  - 空产物：测试通过但缺证据产物视为不通过。
  - 权限不足：CI 环境权限不足时标记阻塞并输出最小修复建议。
  - 超时：长跑套件超时需产出中间状态，防止误判为无结果。
  - 并发冲突：同一产物路径并发写入时强制分目录隔离。
  - 数据异常：日志格式破损时保留原始文件并标记解析失败。
  - Windows 非法路径：若 tracked path 含 Windows 非法字符、保留名或尾随空格/点，必须在 Linux required gate 先阻断，不允许等到 Windows runner checkout 才暴露。
  - 迁移断链：文档改名后若引用未同步，需在同批次修复并复测。
  - 创世语义误读：若把 `protocol:*` custody account 误当成已初始化 treasury bucket，QA 必须直接阻断。
  - 流通口径漂移：若创世参数表未显式声明 `genesis_liquid=0` 或首年外部释放上限，视为配置不完整。
- Non-Functional Requirements:
  - NFR-TST-1: required 套件变更前后执行时间波动 <= 20%。
  - NFR-TST-2: 发布证据包字段完整率 100%。
  - NFR-TST-3: 关键链路缺陷逃逸率持续下降（按月跟踪）。
  - NFR-TST-4: 测试手册与脚本口径冲突数为 0。
  - NFR-TST-5: 测试执行结果可在 30 分钟内完成追溯定位。
  - NFR-TST-6: 文档迁移批次在不降低治理质量的前提下保持可审阅粒度（每任务对应单文档或单专题）。
  - NFR-TST-7: builtin wasm hash 校验在多 runner 下可复现且差异可定位到模块与平台维度。
  - NFR-TST-8: Token 创世 QA 审计模板字段完整率必须为 `100%`，缺任何一项关键字段都不能给 `pass`。
- Security & Privacy: 测试日志与产物需避免泄露凭据；外部 API 测试使用最小化数据并执行脱敏。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03-03): 固化 testing 分层模型与证据标准。
  - v1.1: 补齐高风险路径的触发矩阵自动检查。
  - v2.0: 建立跨版本测试质量趋势分析与发布建议。
- Technical Risks:
  - 风险-1: 套件增长导致执行成本上升。
  - 风险-2: 手册与脚本不一致导致执行偏差。
  - 风险-3: hash 校验策略分散会导致 m4/m5 漂移长期难以收敛。
  - 风险-4: release 工作流若让 Web 构建缓存与 bundle 编译链路碎片化，会把耗时重新堆回关键路径。
  - 风险-5: runtime gate 若继续串行堆叠 full-tier 与 sync 检查，会长期锁死发布关键路径。
  - 风险-6: runtime shard 若长期失衡，即使已并行化，也会因为最长 shard 过重而回吐大部分收益。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-001 | TASK-TESTING-001/002/005/006 | `test_tier_required` | S0~S10 触发矩阵核验、手册一致性检查 | 分层测试入口与执行标准 |
| PRD-TESTING-002 | TASK-TESTING-002/003/006/053/054/055/056/release-windows-invalid-path-blocker/rust-required-gate-ondemand-scope | `test_tier_required` + `test_tier_full` | 证据模板抽样、发布前必填字段检查、release workflow 复用链路核验、runtime gate shard 聚合验证、required-gate changed-path planner 回归，以及 Windows checkout 兼容路径扫描 | 发布链路可信性与可复现性 |
| PRD-TESTING-003 | TASK-TESTING-003/004/006/053/054/055/056/release-windows-invalid-path-blocker/rust-required-gate-ondemand-scope | `test_tier_full` | 趋势指标回顾、缺陷逃逸复盘、release 关键路径对比，以及 required-gate scope 剪裁后的长期时延观察与 Windows checkout 失败签名回归 | 长期质量治理与发布风险控制 |
| PRD-TESTING-004 | TASK-TESTING-007/008/009/010/011/012/013/014/015/016/017/018/019/020/021/022/023/024/025/026/027/028/029/030/031/032/033/034/035/036/059/060/061 | `test_tier_required` | 原文约束点映射审查、命名与引用回归检查、历史专题标题零残留校验、活跃专题当前真值命名回归检查 | 专题文档可维护性与追溯一致性 |
| PRD-TESTING-005 | TASK-TESTING-037/038/039/040/wasm-determinism-gate-ondemand-scope | `test_tier_required` | keyed manifest/strict policy/changed-path scope planner/多 runner required checks/identity 输入收敛回归 | builtin wasm 发布链路稳定性 |
| PRD-TESTING-006 | TASK-TESTING-062 | `test_tier_required` | token 创世参数表审计清单、执行模板、p2p/testing 模块追踪回写 | 主链 Token 创世冻结与经济配置门禁 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-TST-001 | 采用 required/full 分层验证 | 全量套件每次必跑 | 保持效率与覆盖平衡。 |
| DEC-TST-002 | 证据包作为发布必备输入 | 只记录口头结论 | 审计与追溯能力不足风险更高。 |
| DEC-TST-003 | 以手册驱动触发矩阵统一口径 | 各模块自行定义测试口径 | 可减少跨模块冲突和遗漏。 |
| DEC-TST-004 | legacy 专题文档采用逐篇人工迁移并统一 `.prd` 命名 | 自动脚本批量改写 | 可确保内容语义与约束不丢失。 |
| DEC-TST-005 | Token 创世前增加 QA 审计清单与阻断 verdict | 仅由 producer/runtime 自审 | 经济配置错误一旦进入创世，后续修复成本极高。 |
