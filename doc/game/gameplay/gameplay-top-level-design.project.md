# 游戏可玩性顶层设计（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-top-level-design.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-top-level-design.prd.md`
审计轮次: 9

## ROUND-002 主从口径
- 本文件为 gameplay 项目主入口，其余 gameplay project 为增量计划。

## 任务拆解

### T0 文档与结构对齐
- [x] 将顶层设计文档迁移到 `doc/game/`：`doc/game/gameplay/gameplay-top-level-design.prd.md`
- [x] 将工程设计分册迁移并重命名为语义化文件：`doc/game/gameplay/gameplay-engineering-architecture.md`
- [x] 修复工程设计分册 Markdown 围栏问题，确保文档可正常渲染

### T1 顶层设计字段补齐
- [x] 在顶层设计文档中补齐必备字段：目标、范围、接口/数据、里程碑、风险
- [x] 在工程设计分册中补齐范围、接口/数据、里程碑、风险

### T2 设计评审准备
- [x] 组织一次可玩性评审，确认微/中/长循环是否可验证
- [x] 将“爽点曲线”映射为可量化指标（留存、冲突频次、联盟活跃度）
- [x] 对战争与政治机制补充最小可行数值基线（成本/收益/冷却约束）

### T3 工程落地拆解（下阶段）
- [x] 落地 Gameplay Runtime 治理闭环首个生产切片（`doc/game/gameplay/gameplay-runtime-governance-closure.prd.md`）：ABI gameplay 元数据、Runtime 校验、mode+kind 槽位冲突检测、就绪度报告与测试
- [x] 拆解 WASM Gameplay Kernel API 的实现任务（读取/提案/事件总线），并落地生命周期规则切片（`doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.prd.md`）
- [x] 拆解 War/Governance/Crisis/Economic/Meta 模块 MVP 任务，并完成协议与模块生产实现（`doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.prd.md`、`doc/game/gameplay/gameplay-module-driven-production-closure.prd.md`）
- [x] 为每个模块定义 `test_tier_required` 与 `test_tier_full` 测试矩阵（见下文“Gameplay 模块测试矩阵引用”）

### T4 前期工业引导闭环（2026-03-15）
- [x] 冻结“首个制成品 -> 首条稳定生产链 -> 首座工厂单元 -> 可交易工业品 -> 受保护工业节点”作为新手前期主引导链。
- [x] 将前 30 天体验路径改写为“工业成长优先，联盟/治理/战争后接”，并同步评审与指标口径。
- [x] `runtime_engineer`：补齐工业里程碑所需的生产完成、停机、恢复状态与审计事件，确保结果可由状态与事件历史解释。
- [x] `viewer_engineer`：把 `已接受 / 执行中 / 已产出 / 停机原因` 做成主界面显式反馈，优先覆盖首个制成品与工厂开工场景。
- [x] `qa_engineer`：新增“首个制成品 / 停机恢复 / 首座工厂单元”playability 卡片与 `test_tier_required` 手动回归链路。

### T5 PostOnboarding 阶段目标链（2026-03-18）
- [x] 冻结 `FirstSessionLoop -> PostOnboarding -> MidLoop` 的阶段承接口径，并新增专题 PRD / design / project。
- [x] `viewer_engineer` / `runtime_engineer`：对齐 `PostOnboarding` 阶段机、主目标来源、阻塞分类与恢复逻辑。
- [x] `viewer_engineer`：落地阶段切换卡、主目标卡、阶段完成卡，关闭当前 `#46` 的产品承接缺口。
- [x] `qa_engineer`：新增 `#46` required-tier / Web 闭环与 playability 卡片证据，形成通过或阻断结论。

### T6 纯 API 客户端等价（2026-03-19）
- [x] 冻结“纯 API 客户端在信息粒度、动作能力和持续游玩上与 UI 等价”专题 PRD / design / project。
- [x] `viewer_engineer` / `runtime_engineer`：将关键玩家语义从 UI 私有组装下沉到协议级 canonical snapshot。
- [x] `runtime_engineer` / `agent_engineer` / `viewer_engineer`：补齐纯 API 正式玩家动作面与恢复逻辑，避免降级为 observer-only。
- [x] `qa_engineer`：建立 UI/API parity matrix 与纯 API 长玩 required/full 验收。

### T7 封闭 Beta 准入门禁（2026-03-21）
- [x] 冻结“当前阶段为 internal_playable_alpha_late、下一阶段目标为 closed_beta_candidate”专题 PRD / design / project，并完成根入口挂载。
- [x] `runtime_engineer`：补齐 five-node no-LLM soak、replay/rollback drill 与 longrun release gate 的候选版本证据。
- [x] `viewer_engineer`：收口 `PostOnboarding` 首屏降噪、主目标优先级与玩家入口 full-coverage gate 的最小产品化包。
- [x] `qa_engineer`：建立统一 `closed_beta_candidate` release gate，串联 headed Web/UI、pure API、no-UI smoke、longrun/recovery 与 trend baseline。
- [x] `liveops_community`：收口封闭 Beta 候选 runbook、招募/反馈/事故回流模板与禁语清单。

### T8 10 分钟留存修复（2026-04-09）
- [x] 冻结“未来两周只优先做 5 条 retention lane”的专题 PRD / design / project，并完成根入口挂载。
- [x] `viewer_engineer` / `runtime_engineer`：已收口首次进入与最小控制地板的前台控制门控与 ack 语义，让 headed Web/UI 与 `software_safe` 不再把明确 `blocked` / `no_progress` 压扁成伪 timeout；fresh active-LLM formal lane 的 runtime floor 已恢复，但 retention gate 仍被 `TASK-GAME-065` 判定为 `hold`。
- [x] `runtime_engineer` / `viewer_engineer`：已把 `PostOnboarding` 后 10 分钟工业中循环加厚为“韧性生产 -> 第一次扩产取舍 -> 通用 mid-loop”的目标包。
- [x] `viewer_engineer` / `agent_engineer`：已收口首屏噪音、玩家身份与后果可见化，把当前主目标、阻塞、代价与奖励节奏做成首屏主语义。
- [x] `qa_engineer`：已建立 active-LLM 10 分钟留存 gate，并明确 `--no-llm` 仅保留 debug/probe lane，不再作为正式留存结论；当前 producer verdict 为 `hold`，因为 `3` 条 active-LLM 10 分钟正式样本均未形成“首个可持续能力”闭环，且其中 `2` 条样本出现阶段回退并冻结世界时间。

## 依赖

- 运行时与模块治理基线：`doc/world-runtime/prd.md`
- 测试流程与分层矩阵：`testing-manual.md`
- 世界规则与边界约束：`world-rule.md`
- 战争与政治数值基线：`doc/game/gameplay/gameplay-war-politics-mvp-baseline.md`

## Gameplay 模块测试矩阵引用

- `test_tier_required` 基线：`./scripts/ci-tests.sh required`（来源：`testing-manual.md` S1）
- `test_tier_full` 基线：`./scripts/ci-tests.sh full`（来源：`testing-manual.md` S2）
- Gameplay Runtime 协议定向：`env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::gameplay_protocol:: -- --nocapture`（来源：`testing-manual.md` S3）
- Gameplay LLM/Simulator 协议定向：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 simulator::llm_agent::tests:: -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 simulator::tests::submitter_access:: -- --nocapture`
- 场景回归入口：`env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required scenario_specs_match_ids -- --nocapture`（来源：`testing-manual.md` S7）

## 状态

- 当前状态：`进行中`
- 已完成：文档归位、命名语义化、必备字段补齐、工程分册格式修复、Gameplay Runtime/模块化/协议扩展任务拆解与落地、Gameplay 模块测试矩阵引用固化、设计评审准备与战争/政治数值基线补齐、前期工业引导闭环文档冻结（首个制成品/工厂主链）、T4 的 runtime 工业状态/事件与 viewer 主反馈闭环、T5 的 `PostOnboarding` 阶段目标链闭环、T6 的纯 API 客户端等价闭环、T7 的封闭 Beta 准入专题冻结与根入口挂载。
- 未完成：当前无 `T7` 技术阻塞；后续仅保留统一 gate、trend baseline 与 liveops 节奏的持续监控。
- 阻塞项：无统一 gate 技术阻塞；当前继续保持 `internal_playable_alpha_late` 属于 producer claim 决策，不得据此宣称 `closed beta approved`。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
