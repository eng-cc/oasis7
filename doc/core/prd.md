# core PRD

审计轮次: 6

## 目标
- 作为项目级总 PRD，提供 oasis7 的全局设计全貌入口。
- 统一跨模块边界、关键链路、术语口径与验收基线。
- 确保各模块改动可追溯到 PRD-ID、任务和测试证据。

## 范围
- 覆盖全项目模块地图、端到端链路、关键分册导航与治理基线。
- 覆盖 PRD-ID 到 `doc/core/project.md` 的任务映射。
- 不覆盖各模块实现细节正文（由模块 PRD 与专题分册承载）。

## 接口 / 数据
- 项目级 PRD 入口: `doc/core/prd.md`
- 项目管理入口: `doc/core/project.md`
- 文件级索引: `doc/core/prd.index.md`
- 追踪主键: `PRD-CORE-xxx`
- 模块入口总览: `doc/README.md`
- 测试与发布参考: `testing-manual.md`
- 活跃 cross-module 专题:
  - `doc/core/player-access-mode-contract-2026-03-19.prd.md`（PRD-CORE-009）

## 里程碑
- M1 (2026-03-03): 完成模块 PRD 体系重构并建立项目级总览入口。
- M2: 固化跨模块变更影响检查清单（设计/代码/测试/发布）。
- M3: 建立 PRD-ID -> Task -> Test 追踪报表。
- M4 (2026-03-10): 建立阶段收口优先级与跨角色执行口径，统一玩法 / runtime / testing / playability / headless 的发布前闭环目标。
- M5 (2026-03-19): 冻结 `software_safe / pure_api` 双模式总契约，并明确其与 `execution lane` 的分层边界。

## 风险
- 模块并行演进过快时，全局总览可能滞后于真实实现。
- 模块间术语不统一会造成评审误判与接口漂移。

## 1. Executive Summary
- Problem Statement: 项目已拆分为多个模块 PRD，但缺少一个“只读一份文档即可掌握整体设计”的全局总入口。
- Proposed Solution: 在 core PRD 中固化项目全局模块地图、关键端到端链路、关键分册导航和统一治理口径，使其成为仓库级设计总览。
- Success Criteria:
  - SC-1: `doc/README.md` 将 `doc/core/prd.md` 作为推荐阅读第一入口。
  - SC-2: core PRD 明确列出全部模块职责、关键链路与关键分册。
  - SC-3: 跨模块改动评审可基于 core PRD 完成影响面识别。
  - SC-4: 新增模块级需求可映射到对应模块 PRD 与 core 基线。
  - SC-5: 当前阶段优先级（P0/P1/P2）在 core PRD 中有唯一口径，并能映射到对应模块任务与角色 owner。
  - SC-6: 发布前必须先完成玩法微循环、runtime 验收、testing 证据、playability 反馈四条 P0 闭环，缺任一项不得给出 go 结论。
  - SC-7: headless-runtime、自动化稳定性、文档一致性收口具备明确 P1 责任划分与交付标准。
  - SC-8: `software_safe / pure_api` 双模式在 core 中具备唯一 taxonomy、claim envelope、mode/provider/lane 分层约束，其中 `software_safe` 是主要正式 Web 入口、`pure_api` 是一等公民 no-UI mode，且 `pure_api` formal gameplay 继续要求 active LLM access；`agent_direct_connect/provider_loopback_http` 仅保留为兼容 alias，agent provider 的当前正式配置模型必须拆成 `agent_decision_source + agent_provider_backend/contract/transport/url/auth/connect_timeout_ms/profile + agent_execution_lane`，`non-3D / 2D 优先` 只允许作为交付优先级或交互范围描述。
  - SC-9: core 活跃专题标题、Viewer 活跃手册与实际窗口/Web 标题对齐 `oasis7` 品牌；内部旧品牌兼容命名仅以实现说明形式保留，不得继续冒充公开标题。
  - SC-10: `engineering`、`scripts`、`world-runtime` 的历史专题标题在不改动内部实现标识的前提下完成 `oasis7` 品牌收口，减少 active/historical 入口里的旧品牌混用。

## 2. User Experience & Functionality
- User Personas:
  - 架构负责人：需要一份文档快速把握全局设计与边界。
  - 模块维护者：需要明确自己模块在全局链路中的位置与依赖。
  - 发布负责人：需要统一口径判定跨模块风险与放行条件。
  - `producer_system_designer`：需要统一当前阶段优先级、owner 分工与完成定义，避免团队继续平均发力。
- User Scenarios & Frequency:
  - 架构评审：每次跨模块需求评审前至少 1 次，核对影响边界与依赖。
  - 模块联调：每周多次，按链路检查上游/下游耦合点是否一致。
  - 发布评估：每个版本候选至少 1 次，基于统一门禁做 go/no-go 判定。
  - 新成员入项：入项首日使用，快速建立项目全局认知。
  - 阶段收口评审：每轮版本收口前至少 1 次，明确 P0/P1/P2 优先级、owner、依赖与阻断项。
- User Stories:
  - PRD-CORE-001: As an 架构负责人, I want a project-wide blueprint, so that I can reason about cross-module impact quickly.
  - PRD-CORE-002: As a 模块维护者, I want one place to see end-to-end chains, so that I can design compatible changes.
  - PRD-CORE-003: As a 发布负责人, I want unified release/test governance, so that go/no-go decisions are auditable.
  - PRD-CORE-004: As a `producer_system_designer`, I want a stage-closure source of truth, so that the team aligns on what must ship first, who owns it, and what evidence is required before release.
  - PRD-CORE-005: As a `producer_system_designer`, I want a ranked next-round priority slate, so that the team starts the new cycle from one agreed execution path instead of diffusing effort.
- PRD-CORE-006: As a `producer_system_designer`, I want a formal version-candidate go/no-go entry after readiness reaches `ready`, so that release approval, residual risks, and role handoff are explicit and auditable.
- PRD-CORE-007: As a 新协作者, I want `doc/README.md` to include the current public-preview reading path, so that I start from the right entry points.
- PRD-CORE-008: As a `producer_system_designer`, I want the global docs hub synced with repo/site posture, so that navigation stays consistent.
- PRD-CORE-009: As a `producer_system_designer`, I want one cross-module contract for `software_safe / pure_api`, so that release, QA, playability, and provider-backed lane terminology stay aligned.
- Critical User Flows:
  1. Flow-CORE-001: `读取模块地图 -> 识别改动所属模块 -> 定位上下游依赖 -> 形成影响面清单`
  2. Flow-CORE-002: `读取关键链路 -> 映射到模块 PRD-ID -> 对照测试分层 -> 输出发布风险判断`
  3. Flow-CORE-003: `发现口径冲突 -> 回溯分册来源 -> 在 core 基线中统一术语与边界 -> 回写模块文档`
  4. Flow-CORE-004: `评估当前项目状态 -> 划分 P0/P1/P2 -> 指定跨角色 owner / 输入 / 输出 / Done -> 回写对应模块 project`
  5. Flow-CORE-005: `收集玩法 / runtime / testing / playability / headless 证据 -> 对照阶段收口门禁 -> 形成 go / no-go 结论`
  6. Flow-CORE-006: `确认本轮 completed -> 汇总候选缺口 -> 划分 P0/P1/P2 -> 选定下一条执行主路径`
  7. Flow-CORE-007: `判断结论属于 software_safe / pure_api 哪一模式 -> 若原文只写 non-3D/2D 优先则先回补真实 mode_id -> 绑定 execution lane 与 evidence -> 输出不越界的 claim`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 模块地图导航 | 模块名、职责、关键载体、入口路径 | 进入模块 PRD 与 project 文档 | `draft -> reviewed -> published` | 默认按模块分层顺序展示 | 所有贡献者可读，维护者可改 |
| 关键链路追踪 | 链路名称、上游、下游、测试门禁 | 依据链路定位依赖变更与测试范围 | `identified -> validated -> archived` | 高风险链路优先检查 | 发布负责人具备最终裁定权 |
| 术语与口径统一 | 术语名、定义、引用文档、更新时间 | 发现冲突后统一定义并回写引用 | `conflict -> resolved -> synced` | 以核心术语集为唯一优先级 | core 维护者审核后生效 |
| 阶段优先级台账 | 优先级层级、目标、owner、输入、输出、验收标准、阻断条件 | 评审后确认 `P0/P1/P2` 与 owner 映射，并回写模块 project | `candidate -> aligned -> executing -> gated -> released` | `P0 > P1 > P2`；P0 未完成时不得提升 P1/P2 为发布结论主路径 | `producer_system_designer` 拥有排序权；模块 owner 负责承接执行 |
| 跨角色交付矩阵 | 发起角色、接收角色、handoff 输入、产出物、回写位置、验证方式 | 发起方提交 handoff / 接收方确认 done / owner 回写 PRD、project、devlog | `requested -> accepted -> delivered -> verified` | 先按“最先落地代码/文档的 owner”排序，再按发布风险高低排序 | 仅标准角色名可出现在交付矩阵与 devlog |
| 发布收口门禁 | P0/P1/P2 状态、证据路径、阻断结论、例外说明、复审时间 | 汇总证据并输出 `go/no-go/conditional-go` | `not_ready -> conditionally_ready -> ready -> released` | 缺任一 P0 证据时强制 `not_ready` | 发布负责人给出结论，core owner 负责口径一致性 |
| 下一轮优先级清单 | 优先级、主题、owner、输入、输出、进入条件 | 收口后排序并选定下一条执行主路径 | `candidate -> ranked -> selected -> planned` | 先看发布影响，再看闭环依赖，再看 owner 就绪度 | `producer_system_designer` 排序，`qa_engineer` 复核 |
| 玩家访问模式契约 | `mode_id`、`claim_scope`、`fallback_target`、`execution_lane`、`forbidden_claims`、`gameplay_prerequisites` | 评审前先给结论绑定模式，再生成对外/QA 结论 | `unclassified -> bounded -> evidenced -> published` | 玩家访问模式只有 `software_safe / pure_api` 两项；`non-3D / 2D 优先` 只能作优先级或范围描述；lane 只作附加维度；`pure_api` formal gameplay 默认要求 active LLM access | `producer_system_designer` 冻结 taxonomy，模块 owner 联审 |
- Acceptance Criteria:
  - AC-1: core PRD 包含模块职责矩阵。
  - AC-2: core PRD 包含至少 4 条关键端到端链路描述。
  - AC-3: core PRD 给出关键分册导航并可从 `doc/README.md` 到达。
  - AC-4: core project 文档任务与 PRD-CORE-ID 可映射。
  - AC-5: 文档级 `审计轮次` 仅可对应已落档的正式 ROUND 台账；在 `ROUND-NNN` 正式启动文件落档前，不得保留脱离台账的 `审计轮次 > NNN` 标记。
  - AC-6: core PRD 明确列出当前阶段 `P0/P1/P2` 收口项、对应 owner、输入、输出、验收标准与阻断条件。
  - AC-7: `P0` 至少覆盖玩法微循环、runtime 验收、testing 证据、playability 反馈四条闭环，并明确它们是发布前必要条件。
  - AC-8: `P1` 至少覆盖 core 一致性审查、headless-runtime 长稳门禁、自动化稳定性收口，并定义角色交付边界。
  - AC-9: `P2` 仅包含不阻塞发布的体验 polish 与治理补完，不得与 P0/P1 混淆。
  - AC-10: `PRD-CORE-004` 可映射到 `doc/core/project.md` 中的任务与 `test_tier_required` 验证方法。
  - AC-11: `PRD-CORE-005` 必须明确下一轮第一优先级、对应 owner role、输入/输出与进入条件。
  - AC-12: `PRD-CORE-009` 必须把 `software_safe / pure_api` 与 `execution lane` 的边界写成唯一 taxonomy，并要求证据与 claim 显式绑定 mode；其中 `pure_api` formal gameplay 必须显式声明 active LLM prerequisite。
  - AC-13: core 活跃专题、Viewer 活跃手册与 Viewer 用户可见标题必须统一使用 `oasis7` 品牌；若为兼容保留旧内部实现名，必须明确标注为 internal compatibility naming。
  - AC-14: `engineering`、`scripts`、`world-runtime` 下仍可读的历史/治理/运行时专题标题必须改为 `oasis7` 品牌；仅实现标识、环境变量、脚本参数与历史证据正文可继续保留旧内部命名。
- Non-Goals:
  - 不在 core PRD 中替代模块详细技术分册。
  - 不在 core PRD 中维护逐版本实现变更流水（该信息在 devlog）。
  - 不在本 PRD 中重写各模块的实现细节或替代其 `project.md` 执行计划。
  - 不把 launcher / explorer 体验新增功能作为当前阶段的主发布驱动。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 文档治理检查脚本、`rg` 检索、测试手册与 CI 脚本用于核验跨模块一致性。
- Evaluation Strategy: 以跨模块评审返工率、口径冲突数、发布前补文档次数评估全局 PRD 有效性。
  对 `PRD-CORE-004`，额外以 P0 闭环完成率、证据包完整率、go/no-go 评审一次通过率评估阶段收口质量。

## 4. Technical Specifications
- Architecture Overview: core 作为“全局设计总览层”，不承载业务实现代码，而承载全局结构、统一约束和跨模块链路描述。

### 当前阶段收口优先级（Stage Closure Backlog）
| 优先级 | 目标 | 主要 owner | 输入 | 输出 | 完成定义 |
| --- | --- | --- | --- | --- | --- |
| P0 | 完成玩法微循环收口 | `producer_system_designer` / `viewer_engineer` / `qa_engineer` | `doc/game/project.md` 中 `TASK-GAME-018`、当前 runtime_live 体验证据、Viewer Web 闭环能力 | 视觉优化二期、截图/视频/结论证据包、`game` 状态回写 | 玩家可直观看到控制结果、默认模式负担下降、世界可读性提升，且至少一轮截图闭环通过 |
| P0 | 补齐 runtime 核心边界验收 | `runtime_engineer`（联审：`producer_system_designer`） | 确定性 / WASM / 治理边界、当前 runtime 测试与限制说明 | 验收清单、阻断条件、例外口径 | 每条关键边界都有测试映射，并能直接用于发布评审 |
| P0 | 建立 testing 触发矩阵与发布证据包 | `qa_engineer`（联审：`producer_system_designer`） | `testing-manual.md`、各模块 `test_tier_required/full`、现有脚本与证据产物 | S0~S10 触发矩阵、证据包模板、放行摘要格式 | 任一任务都能反推必跑测试，PRD-ID / 任务 / 测试结果可串联 |
| P0 | 补齐 playability 反馈闭环 V1 | `qa_engineer`（联审：`producer_system_designer`） | 现有 playability 输出、截图/视频、玩法目标 | 反馈卡字段、评分口径、高优问题追踪模板 | 每条体验问题都有固定记录格式，且可进入发布讨论 |
| P1 | 完成 core 一致性审查收口 | `producer_system_designer` | 各模块 PRD / project、已有 ROUND 台账 | 新一轮审查记录、整改项、复审状态 | 关键模块术语、轮次、追踪字段统一 |
| P1 | 建立 headless-runtime 长稳门禁骨架 | `runtime_engineer` / `qa_engineer` | headless-runtime 协议、生命周期、鉴权链路、长稳产物 | 生命周期与鉴权清单、归档模板、故障追溯模板 | 能覆盖启动/运行/停止/恢复，且故障后可回放关键证据 |
| P1 | 收口自动化残余不稳定点 | `viewer_engineer` / `qa_engineer` | `agent-browser` 闭环现状、A/B 首连与录屏问题 | 稳定复跑方案、限制说明 | 主闭环可复跑，不再影响版本级证据收集 |
| P2 | launcher / explorer 体验 polish | `viewer_engineer` | 当前 launcher / explorer 成果、体验问题清单 | 次级体验优化项 | 不新增核心复杂度，不挤占 P0/P1 资源 |
| P2 | README / site / scripts / engineering 治理补完 | 对应模块 owner | 未完成治理任务、入口与脚本使用痛点 | 一致性检查、链接检查、趋势统计等治理产物 | 提升维护性，但不阻塞当前阶段发布 |

### 项目模块地图（Design Map）
| 模块 | 主职责 | 关键实现载体 |
| --- | --- | --- |
| core | 全局设计总览、跨模块治理基线 | `doc/core/*` |
| engineering | 工程规范、文件约束、质量门禁 | `doc/engineering/*`, `scripts/*`, CI workflows |
| game | 玩法循环、治理/经济/战争规则设计 | `doc/game/*`, `crates/oasis7` (gameplay相关) |
| world-runtime | 世界内核、事件溯源、WASM执行与治理 | `doc/world-runtime/*`, `crates/oasis7`, `crates/oasis7_wasm_*` |
| world-simulator | 场景系统、Viewer/Launcher、LLM交互链路 | `doc/world-simulator/*`, `crates/oasis7_viewer`, `crates/oasis7_client_launcher` |
| p2p | 网络、共识、DistFS、多节点运行 | `doc/p2p/*`, `crates/oasis7_net`, `crates/oasis7_consensus`, `crates/oasis7_distfs`, `crates/oasis7_node` |
| headless-runtime | 无界面运行链路、鉴权、长稳运维能力 | `doc/headless-runtime/*`, `crates/oasis7/src/bin/*` |
| testing | 分层测试体系与发布门禁 | `doc/testing/*`, `testing-manual.md`, `scripts/ci-tests.sh` |
| scripts | 自动化脚本能力与执行约束 | `scripts/*`, `doc/scripts/*` |
| site | 站点信息架构、发布内容、SEO | `site/*`, `doc/site/*` |
| readme | 对外文档入口口径统一 | `README.md`, `doc/readme/*` |
| playability_test_result | 可玩性反馈证据与发布引用 | `doc/playability_test_result/*`, `doc/playability_test_result/game-test.prd.md` |

### 关键端到端链路（E2E Chains）
1. 玩家交互链路:
`Launcher/Viewer -> oasis7_viewer_live/oasis7_chain_runtime -> world-runtime -> event/journal -> UI反馈`
2. 世界执行链路:
`Action/Intent -> Rule Validation -> Resource/State Transition -> Event -> Snapshot/Replay`
3. 模块扩展链路:
`Rust Source -> WASM Artifact -> Register/Install -> Runtime Sandbox Execution -> Governance/Audit`
4. 分布式一致性链路:
`Node/Net -> Consensus Commit -> DistFS/State Replication -> Runtime Apply -> Viewer Observe`
5. 发布验证链路:
`PRD-ID Task -> core治理(test_tier_required) -> 模块专项(required/full) -> Web闭环/长跑 -> Evidence Bundle -> Release Decision`

### 关键分册导航（只读总览后优先下钻）
- 运行时内核: `doc/world-runtime/runtime/runtime-integration.md`
- WASM 接口与执行: `doc/world-runtime/wasm/wasm-interface.md`, `doc/world-runtime/wasm/wasm-executor.prd.md`
- 场景矩阵: `doc/world-simulator/scenario/scenario-files.prd.md`
- Web 闭环测试策略: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- 玩家访问模式总契约: `doc/core/player-access-mode-contract-2026-03-19.prd.md`
- 分布式路线图: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md`
- 系统性测试手册: `testing-manual.md`

### 全局术语（Glossary）
- PRD-ID: 需求追踪主键，连接 PRD、任务、测试与发布证据。
- required/full: 分层测试的两级核心门禁。
- Web-first 闭环: 默认 UI 验证路径（agent-browser 优先）。
- Effect/Receipt: 运行时外部副作用与回执审计机制。
- Snapshot/Replay: 世界状态持久化与可重放能力。

- Integration Points:
  - `AGENTS.md`
  - `doc/README.md`
  - `testing-manual.md`
  - 各模块 `doc/<module>/prd.md` 与 `doc/<module>/project.md`
  - `doc/game/project.md`
  - `doc/world-runtime/project.md`
  - `doc/testing/project.md`
  - `doc/playability_test_result/project.md`
  - `doc/headless-runtime/project.md`
  - `doc/devlog/YYYY-MM-DD.md`
- Edge Cases & Error Handling:
  - 模块入口失效：若目标路径迁移，core 必须同步更新导航并保留可追溯说明。
  - 信息缺失：若模块 PRD 尚未更新，标记“口径待同步”并阻断发布结论。
  - 版本漂移：core 与分册冲突时，以最近审阅通过版本为准并触发修复任务。
  - 依赖冲突：同一链路被多个模块修改时，需合并影响面并重跑 required 级验证。
  - 测试证据缺口：无证据不得判定链路通过，必须补齐最小 required 证据。
  - 术语冲突：同术语多定义时优先使用 core 词典并登记决策记录。
  - owner 冲突：多个模块同时声称同一项为 `P0` 且 owner 不一致时，按“最先落地代码/文档的 owner”裁定，并回写 core / project / devlog。
  - project 缺承接：若 P0 项在 PRD 已定义但对应模块 `project.md` 未承接，状态只能记为 `candidate`，不得进入发布结论。
  - 证据格式未统一：若测试闭环可跑但证据包未统一格式，仅可记为 `conditionally_ready`，不得视作 fully ready。
  - 资源抢占：若 launcher / explorer 新需求与 P0 资源冲突，默认降级到 P2，除非能直接服务玩法闭环或发布门禁。
- Non-Functional Requirements:
  - NFR-CORE-1: 核心入口文档链接可用率 100%。
  - NFR-CORE-2: 跨模块评审时，影响面识别耗时 <= 30 分钟。
  - NFR-CORE-3: 发布评审前，PRD-ID 到测试证据映射完整率 100%。
  - NFR-CORE-4: 所有核心术语变更需在 1 个工作日内同步到相关入口文档。
  - NFR-CORE-5: core 主文档维持 <= 1000 行，超限必须拆分分册。
  - NFR-CORE-6: P0 项的 owner / 输入 / 输出 / 验收标准 / 阻断条件覆盖率 100%。
  - NFR-CORE-7: 发布评审时 P0 证据缺失数必须为 0；P1 可存在未完成项，但必须附带风险与缓解方案。
  - NFR-CORE-8: 跨角色 handoff 在 PRD / project / devlog 中的追溯链完整率 100%。
  - NFR-CORE-9: 一轮模块主项目全部收口后 1 个工作日内必须形成下一轮优先级清单。
- Security & Privacy: core 仅维护结构与治理口径；涉及密钥、签名、隐私数据的要求由对应模块 PRD 细化并执行。
  发布收口文档仅记录工程与玩法证据，不引入额外敏感数据；若引用线上/远程环境信息，需与对应模块 owner 联审后落档。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03-03): core PRD 成为项目级总览入口。
  - v1.1: 建立跨模块变更影响检查清单模板。
  - v2.0: 建立 PRD-ID 到测试证据的自动化追踪报表。
  - v2.1 (2026-03-06): 启动 ROUND-005，专项收敛文档状态时效字段、完成态字段、命名一致性与索引覆盖规则。
  - v2.2 (当前阶段): 建立阶段收口优先级、跨角色交付矩阵与发布前 P0 必备闭环。
  - v2.3 (下一阶段): 建立下一轮优先级清单，并把第一优先级固定到正式专题入口。
  - v2.4 (当前推进): 在版本级 readiness 达到 `ready` 后，建立正式 go/no-go 裁决入口与角色交接链。
- Technical Risks:
  - 风险-1: 模块新增能力未及时回填全局链路。
  - 风险-2: 总览与分册的口径同步依赖人工流程。
  - 风险-3: 若局部修订直接上调 `审计轮次` 而未建立正式 ROUND 台账，字段会失去可比性，并破坏 reviewed-files / progress-log 对账。
  - 风险-4: 若继续把 launcher / explorer 体验扩展排在玩法与发布治理前，项目会强化“能展示”而非“能稳定发布”的错配。
  - 风险-5: 若 runtime / testing / playability 的证据标准不同步，发布评审会退化为口头判断。
  - 风险-6: 若本轮收口后没有正式优先级入口，团队会重新回到平均发力与隐式尾注推进。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-CORE-001 | TASK-CORE-001/002/006/007 | `test_tier_required` | 入口完整性扫描、模块地图与导航可达检查 | 全项目入口与架构总览一致性 |
| PRD-CORE-002 | TASK-CORE-002/003/004/007 | `test_tier_required` | 关键链路映射核验、跨模块依赖抽样复核 | 跨模块设计兼容性与发布评审效率 |
| PRD-CORE-003 | TASK-CORE-004/005/007 | `test_tier_required` | 发布门禁证据映射校验、轮次一致性审查记录检查（含文档级审计轮次标记，缺省按0） | 发布决策可审计性与长期治理稳定性 |
| PRD-CORE-003 | TASK-CORE-008 | `test_tier_required` | `审计轮次 > 5` 漂移扫描、ROUND-005 基线回写、devlog 与 git 证据核对 | 审计标记口径恢复为正式台账语义 |
| PRD-CORE-003 | TASK-CORE-009 | `test_tier_required` | 全仓 `审计轮次 > 5` 扫描清零、缺失标记补齐为 5、devlog 与 git 证据核对 | 审计标记口径对齐到“全仓不高于 ROUND-005 基线” |
| PRD-CORE-004 | TASK-CORE-011/012/013/014 | `test_tier_required` | 阶段收口优先级、owner 分工、交付矩阵、go/no-go 模板与模块 project 映射抽样核验 | 当前阶段发布前闭环目标与责任边界一致性 |
| PRD-CORE-005 | TASK-CORE-016/017/018/019/020/021 | `test_tier_required` | 下一轮优先级清单、候选级入口、版本级扩展与 runtime 联合证据抽样核验 | 新一轮跨模块执行一致性 |
| PRD-CORE-006 | TASK-CORE-022 | `test_tier_required` | 正式版本候选 go/no-go 记录、风险附注与角色交接抽样核验 | 版本候选正式裁决一致性 |
| PRD-CORE-007 | TASK-CORE-023 | `test_tier_required` | `doc/README.md` 含根 README / site 阅读入口 | 全局导航准确性 |
| PRD-CORE-008 | TASK-CORE-023 | `test_tier_required` | 更新时间与新阅读顺序存在 | 公开口径同步性 |
| PRD-CORE-009 | TASK-CORE-028/049/050/051/052/053/055 | `test_tier_required` | 三模式总契约专题存在、core 主入口/索引互链、文档治理检查通过，并完成 `software_safe` 主 Web 入口定位、`pure_api` LLM-required/first-class no-UI 定位、provider-backed mode/lane 分层、结构化 provider taxonomy 收口与 `provider_loopback_http` / provider workspace 命名清理 | 项目级模式 taxonomy、claim 边界与 formal gameplay 分工一致性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-CORE-001 | 将 core 固化为项目全局唯一总览入口 | 各模块独立维护无全局入口 | 降低跨模块认知成本并提升评审效率。 |
| DEC-CORE-002 | 使用 PRD-ID 作为跨文档追踪主键 | 使用任务编号作为唯一主键 | PRD-ID 可跨任务周期稳定复用并支持审计。 |
| DEC-CORE-003 | core 文档治理任务默认绑定 `test_tier_required`；跨模块发布结论引用 testing 定义的 required/full 证据 | core 任务直接强制 required/full | 区分治理层与专项回归层，保持口径一致且可执行。 |
| DEC-CORE-004 | 在 ROUND-006 正式台账落档前，统一将脱离台账的高位 `审计轮次` 回写到 5 | 保留局部 `审计轮次: 6` 作为“专题修订痕迹” | `审计轮次` 的定义是“最近完成的正式审计轮次”，局部修订应通过 `最近更新` 和 devlog 追踪，而不是抬高正式轮次字段。 |
| DEC-CORE-005 | 将“阶段收口优先级”纳入 core 主 PRD 统一管理，而不是散落在多个模块 project 的状态说明里 | 仅在各模块 project 中维护各自优先级 | 阶段优先级本质是跨模块发布策略，需要由 `producer_system_designer` 在 core 层统一裁剪与仲裁。 |
| DEC-CORE-006 | 新一轮先冻结优先级清单，再启动第一优先级专题 | 主项目收口后直接随机挑模块继续推进 | 先统一排序，才能避免重新扩散资源。 |
| DEC-CORE-007 | readiness 达到 `ready` 后必须再落正式 go/no-go 记录 | 将 readiness board 直接作为最终放行记录 | readiness 与正式裁决是两个层级，必须分开留痕。 |
| DEC-CORE-008 | 将 `software_safe / pure_api` 定义为玩家访问模式，将 `player_parity / headless_agent / debug_viewer` 定义为 execution lane，并将 `non-3D / 2D 优先` 限定为优先级或交互范围描述 | 继续把这些话术混写在不同专题中 | 玩家入口、观战旁路、阶段优先级与无 UI 回归本质上属于不同抽象层，必须在 core 统一分层。 |
