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
- [x] issue-162-industrial-chain-legibility-closeout (PRD-GAME-012) [test_tier_required]: `producer_system_designer` 已将 `#162` 的 closeout trace 显式映射到 T4/T8 既有事实：工业状态、停机原因、恢复提示与首个工业里程碑已在 canonical player surface 上具备玩家可读反馈；该 closeout 不替代 active-LLM trust/capability gate 的独立结论。 Trace: .pm/tasks/task_4da3948c1c2c457c9529ee661e4af03d.yaml

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
- `qa_engineer`：已建立 active-LLM 10 分钟留存 gate，并明确 `--no-llm` 仅保留 debug/probe lane，不再作为正式留存结论；按当前拆分口径，active-LLM formal truth 为 `10-minute trust gate = hold`（因 `3` 条 active-LLM 10 分钟正式样本中有 `2` 条出现阶段回退并冻结世界时间，尚不足以证明“已经值得继续玩”）、`first capability gate = not_run`（当前 trust floor 已再次回退到更前置 blocker，因此 capability gate 未进入，不能再把旧样本表述成“当前已跑但尚未证明”）。

口径更新（2026-04-15）: T8 当前已将 producer verdict 拆成两层。`10-minute trust gate` 只判断“是否已经值得继续玩”，`first capability gate` 再判断“首个持续能力是否已闭环”。当前 active-LLM formal truth 仍是 `trust gate = hold`、`capability gate = not_run`；原因不是 capability 已独立判失败，而是 trust floor 再次回退后 capability gate 当前未进入。

- [x] gameplay-early-retention-focus-order (PRD-GAME-012) [test_tier_required]: `producer_system_designer` 已把当前 gameplay scope freeze 正式改写为“`trust gate` 地板恢复 -> `PostOnboarding` capability closure -> 工业停机/修复可读 -> 间接控制因果与下一步”四级优先顺序，并补充 defer 规则：在这些 blocker 清空前，不扩大战争/治理/元进度在首局中的曝光，也不允许用 `--no-llm` / operator-only lane 充当正式放行依据。 Trace: .pm/tasks/task_886e2ef4878645a6a6ab69c588dce57e.yaml
- [x] issue-161-action-causality-blocker-taxonomy (PRD-GAME-012) [test_tier_required]: `viewer_engineer` 已把玩家目标反馈的统一执行状态机与小型 blocker taxonomy 下沉到 canonical `player_gameplay` snapshot，并在 `software_safe` 正式 Web 主入口显式区分 `world_constraint` 与 `agent_override`，让玩家可以直接判断“世界条件阻塞”还是“agent 改走了另一条已接受的执行路径”。 Trace: .pm/tasks/task_b3a14c16dbf04258865c10c80a9fa460.yaml

### T9 物理尺度与间接控制对齐（2026-05-07）
- [x] gameplay-physical-scale-contract-freeze (PRD-GAME-013) [test_tier_required]: `producer_system_designer` 已新增 `PRD-GAME-013` 专题 PRD / design / project，正式冻结“厘米真值 / coarse-grained 子系统 / 玩家动作粒度 / 表现层夸张”四层尺度合同，并完成 `game` 根入口、`gameplay` 主文档、索引与当前 task execution log 挂载。 Trace: .pm/tasks/task_5dfbbe7c8c0c4557bef2b49612da3081.yaml
- [x] runtime-native-resolution-declaration (PRD-GAME-013) [test_tier_required]: `runtime_engineer` 已把 `simulator` 中现存 coarse-grained 子系统补成显式声明表，并用定向单测锁住厘米真值、km bucket 与 location-site snapping 规则。 Trace: .pm/tasks/task_303dedfe38b04036a198c256cc858e29.yaml
- [x] viewer-scale-surface-truth-labeling (PRD-GAME-013) [test_tier_required]: `viewer_engineer` 已把 `software_safe` 正式 Web 主入口补成“物理真值 + 表现层解释”双轨表面，让玩家能直接读到 world bounds、地点半径和距离样本，并明确 marker/zoom 不等于真实几何尺寸。 Trace: .pm/tasks/task_103c448874b7494a8312418995889098.yaml
- [x] agent-action-contract-boundary-alignment (PRD-GAME-013) [test_tier_required]: `agent_engineer` 已把 dual-mode / action contract 的现行动作面收口为低频间接控制白名单，并显式把 `jump / attack / use_item / block_editing` 回收到 future embodied candidate gate。 Trace: .pm/tasks/task_15890765ee3b4188a1e2766973f392fc.yaml
- [x] qa-scale-consistency-matrix (PRD-GAME-013) [test_tier_required]: `qa_engineer` 已完成四层尺度合同一致性矩阵，确认 runtime/viewer/agent 口径一致，并把 blocker 签名归档到 `doc/testing/evidence/gameplay-scale-consistency-matrix-2026-05-07.md`。 Trace: .pm/tasks/task_8205baa6d2fb46388b11c1eed340fdf5.yaml

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
