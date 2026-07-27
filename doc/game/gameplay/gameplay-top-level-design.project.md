# 游戏可玩性顶层设计（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-top-level-design.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-top-level-design.prd.md`
审计轮次: 9

## ROUND-002 历史记录与当前范围
- ROUND-002 曾以本文件集中记录顶层设计计划；该历史记录不构成当前 project 的主从关系。
- 本文件现仅保存核心玩法骨架与 `PRD-GAME-012` 的 topic project/evidence；模块当前状态回 `doc/game/project.md`，其他 topic project 在各自声明范围内维护计划与证据。

## 任务拆解

### T0 文档与结构对齐
- [x] 将顶层设计文档迁移到 `doc/game/`：`doc/game/gameplay/gameplay-top-level-design.prd.md`
- 历史完成记录：gameplay 工程架构语义已吸收到 gameplay top-level 与 world-runtime/WASM 专业权威，过程从 Git history 追溯。
- [x] 修复工程设计分册 Markdown 围栏问题，确保文档可正常渲染

### T1 顶层设计字段补齐
- [x] 在顶层设计文档中补齐必备字段：目标、范围、接口/数据、里程碑、风险
- [x] 在工程设计分册中补齐范围、接口/数据、里程碑、风险

### T2 设计评审准备
- [x] 组织一次可玩性评审，确认微/中/长循环是否可验证
- [x] 将“爽点曲线”映射为可量化指标（留存、冲突频次、联盟活跃度）
- [x] 对战争与政治机制补充最小可行数值基线（成本/收益/冷却约束）

### T3 工程落地拆解（下阶段）
- 历史完成记录：Gameplay Runtime 治理闭环首个生产切片已落地；当前 ABI、manifest、slot conflict 与 readiness 边界见 `doc/world-runtime/prd.md#gameplay-生命周期协议边界` 和 `doc/world-runtime/wasm/wasm-interface.md`，历史 closure 从 Git history 追溯。
- 历史完成：WASM Gameplay Kernel API 与生命周期规则切片（读取、提案、事件总线和 tick 推进）已落地；当前 runtime 协议边界见 `doc/world-runtime/prd.md#gameplay-生命周期协议边界`，实现过程从 Git history 与 GitHub task evidence 追溯。
- 历史完成：War/Governance/Crisis/Economic/Meta 模块 MVP 已完成协议与模块生产实现；玩家侧长期合同由 gameplay 顶层设计与战争/政治数值基线承接，已退役的 layer/module closure 增量专题从 Git history 与 GitHub task evidence 追溯。
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

### T8 稳定 early-retention 与持续能力合同
原 2026-04-09 dated retention triplet 的 durable 产品承诺已由 `doc/product/world-rules-core-gameplay/first-session-and-continuation.prd.md` 承接；PRD-GAME-012 的 gate 隔离、0~30 分钟 beat、threshold、provenance、anti-script、branch/rollback/opportunity 与 quote/preview 专业合同已收敛到 `gameplay-top-level-design.prd.md`，当前 task/verdict/live 边界由 `doc/game/project.md` 与 testing evidence 承接。
- 历史实现记录：`viewer_engineer` / `runtime_engineer` 已收口首次进入与最小控制地板的前台控制门控与 ack 语义，让 headed Web/UI 与 `software_safe` 不再把明确 `blocked` / `no_progress` 压扁成伪 timeout；`TASK-GAME-065` 的 `hold` 只保留为 2026-04-15 历史 baseline，当前 fresh formal truth 已由 `issue-160-first-capability-closeout` 刷新为 `pass`。
- [x] `runtime_engineer` / `viewer_engineer`：已把 `PostOnboarding` 后 10 分钟工业中循环加厚为“韧性生产 -> 第一次扩产取舍 -> 通用 mid-loop”的目标包。
- [x] `viewer_engineer` / `agent_engineer`：已收口首屏噪音、玩家身份与后果可见化，把当前主目标、阻塞、代价与奖励节奏做成首屏主语义。
- `qa_engineer`：已建立 active-LLM 10 分钟留存 gate，并明确 `--no-llm` 仅保留 debug/probe lane，不再作为正式留存结论；2026-04-15 baseline 曾收口为 `10-minute trust gate = hold`、`first capability gate = not_run`，而 2026-05-17 fresh formal truth 已由 `issue-160-first-capability-closeout` 刷新为 `trust gate = pass`、`first capability gate = pass`。

口径更新（2026-05-17）: T8 当前 producer verdict 继续保持两层。`10-minute trust gate` 只判断“是否已经值得继续玩”，`first capability gate` 再判断“首个持续能力是否已闭环”。2026-04-15 的 `hold/not_run` 保留为历史 baseline；当前 fresh active-LLM formal truth 已更新为 `trust gate = pass`、`capability gate = pass`，证据见 `doc/testing/evidence/issue-160-first-capability-closeout-2026-05-17.md`。

- [x] gameplay-early-retention-focus-order (PRD-GAME-012) [test_tier_required]: `producer_system_designer` 已把当前 gameplay scope freeze 正式改写为“`trust gate` 地板恢复 -> `PostOnboarding` capability closure -> 工业停机/修复可读 -> 间接控制因果与下一步”四级优先顺序，并补充 defer 规则：在这些 blocker 清空前，不扩大高风险对抗/治理/元进度在首局中的曝光，也不允许用 `--no-llm` / operator-only lane 充当正式放行依据。 Trace: .pm/tasks/task_886e2ef4878645a6a6ab69c588dce57e.yaml
- [x] issue-161-action-causality-blocker-taxonomy (PRD-GAME-012) [test_tier_required]: `viewer_engineer` 已把玩家目标反馈的统一执行状态机与小型 blocker taxonomy 下沉到 canonical `player_gameplay` snapshot，并在 `software_safe` 正式 Web 主入口显式区分 `world_constraint` 与 `agent_override`，让玩家可以直接判断“世界条件阻塞”还是“agent 改走了另一条已接受的执行路径”。 Trace: .pm/tasks/task_b3a14c16dbf04258865c10c80a9fa460.yaml

### T9 物理尺度与间接控制对齐（2026-05-07）
- [x] gameplay-physical-scale-contract-freeze (PRD-GAME-013) [test_tier_required]: `producer_system_designer` 已新增 `PRD-GAME-013` 专题 PRD / design / project，正式冻结“厘米真值 / coarse-grained 子系统 / 玩家动作粒度 / 表现层夸张”四层尺度合同，并完成 `game` 根入口、`gameplay` 主文档、索引与当前 task execution log 挂载。 Trace: .pm/tasks/task_5dfbbe7c8c0c4557bef2b49612da3081.yaml
- [x] runtime-native-resolution-declaration (PRD-GAME-013) [test_tier_required]: `runtime_engineer` 已把 `simulator` 中现存 coarse-grained 子系统补成显式声明表，并用定向单测锁住厘米真值、km bucket 与 location-site snapping 规则。 Trace: .pm/tasks/task_303dedfe38b04036a198c256cc858e29.yaml
- [x] viewer-scale-surface-truth-labeling (PRD-GAME-013) [test_tier_required]: `viewer_engineer` 已把 `software_safe` 正式 Web 主入口补成“物理真值 + 表现层解释”双轨表面，让玩家能直接读到 world bounds、地点半径和距离样本，并明确 marker/zoom 不等于真实几何尺寸。 Trace: .pm/tasks/task_103c448874b7494a8312418995889098.yaml
- [x] agent-action-contract-boundary-alignment (PRD-GAME-013) [test_tier_required]: `agent_engineer` 已把 dual-mode / action contract 的现行动作面收口为低频间接控制白名单，并显式把 `jump / attack / use_item / block_editing` 回收到 future embodied candidate gate。 Trace: .pm/tasks/task_15890765ee3b4188a1e2766973f392fc.yaml
- [x] qa-scale-consistency-matrix (PRD-GAME-013) [test_tier_required]: `qa_engineer` 已完成四层尺度合同一致性矩阵，确认 runtime/viewer/agent 口径一致，并把 blocker 签名归档到 `doc/testing/evidence/gameplay-scale-consistency-matrix-2026-05-07.md`。 Trace: .pm/tasks/task_8205baa6d2fb46388b11c1eed340fdf5.yaml

### T10 间接控制 control-feeling 合同（2026-05-14）
- [x] indirect-control-feeling-contract-freeze (PRD-GAME-014) [test_tier_required]: `producer_system_designer` 已新增 `PRD-GAME-014` 专题 PRD / design / project，并完成 `game` 根入口、`gameplay` 主文档、索引与当前 task execution log 挂载，正式冻结 accepted intent、主因果、打断/重排与续玩恢复四项 guarantees。 Trace: .pm/tasks/task_89828a4d2c1b4e73987103699c10fa7d.yaml
- [x] runtime-control-feeling-canonical-contract (PRD-GAME-014) [test_tier_required]: `runtime_engineer` 已把 `player_gameplay` canonical snapshot 与 recent-feedback 真值对齐到 control-feeling 合同，正式补齐 accepted intent、intent scope/target、status reason、last world change、resume anchor、primary blocker 与 resume-next-step 字段，并让 `prompt_control` / `agent_chat` / `gameplay_action` / world-control 共享同一 runtime 语义面。 Trace: .pm/tasks/task_f3c25dd6688f40fbbcf05df9036a83ec.yaml
- 后续待建任务统一收口在 `doc/game/gameplay/gameplay-indirect-control-agency-contract.project.md`，避免在 gameplay 主入口重复展开未绑定 Trace 的计划行。

### T11 小玩家成长线与成熟世界承接（2026-05-17）
- [x] small-player-progression-contract-freeze (PRD-GAME-015) [test_tier_required]: `producer_system_designer` 已新增 `PRD-GAME-015` 专题 PRD / design / project，并完成根入口、`gameplay` 主文档、索引与 execution log 挂载；正式冻结 mature-world 小玩家默认主线 `local operator -> regional specialist -> limited-scope regional influence`，明确 `protected first industrial win` 指低爆炸半径、可恢复和 leverage 可见，而不是新手无敌豁免。 Trace: .pm/tasks/task_d97dfa29208444a9b6a652f2a12fb65d.yaml
- 产品承诺已收敛到 `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md`，专业玩法合同保留在 `gameplay-top-level-design.prd.md`。runtime truth、viewer surface 与 Agent 文档合同已有历史任务证据；当前剩余验证是 `qa-mature-world-small-player-fresh-sample`，只有 fresh sample 能把 `grind_only` / `forced_major_power_dependency` 从 watch 升级为当前 verdict。

### T12 Future：玩法 mode 的参与与准入可读性（未实现）
- [ ] mode-participation-admission-contract (PRD-GAME-001) [test_tier_required]: 在独立 runtime / viewer 任务中实现顶层 PRD 的 mode participation / prospective admission 合同；当前无 `GameplayModeReadiness`、runtime 或 UI 实现，且不包含匹配、排队、邀请或自动补人。当前 GitHub #2609 / 此 Task UID 仅记录已完成的设计目标文档决策，不是实现真值；尚无实现 task UID。仅当获准的 runtime / viewer scope 明确新增 mode-entry surface 时，才创建该独立实现任务。 Trace: https://github.com/eng-cc/oasis7/issues/2609 (task_ed7d98168cba42a187015bc53cb7afe7)

## 依赖

- 运行时与模块治理基线：`doc/world-runtime/prd.md`
- 测试流程与分层矩阵：`testing-manual.md`
- 世界规则与边界约束：`doc/product/world-rules-core-gameplay/prd.md`
- 战争与政治数值基线：`doc/product/world-rules-core-gameplay/war-politics-baseline.prd.md`

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
- 已完成：文档归位、命名语义化、必备字段补齐、工程分册格式修复、Gameplay Runtime/模块化/协议扩展任务拆解与落地、Gameplay 模块测试矩阵引用固化、设计评审准备与战争/政治数值基线补齐、前期工业引导闭环文档冻结（首个制成品/工厂主链）、T4 的 runtime 工业状态/事件与 viewer 主反馈闭环、T5 的 `PostOnboarding` 阶段目标链闭环、T6 的纯 API 客户端等价闭环、T7 的封闭 Beta 准入专题冻结与根入口挂载、T11 的 mature-world 小玩家成长线合同冻结。
- 未完成：当前无 `T7` 技术阻塞；后续保留统一 gate、trend baseline 与 liveops 节奏的持续监控，以及 T12 mode participation / prospective admission 的独立实现任务。
- 阻塞项：无统一 gate 技术阻塞；当前继续保持 `internal_playable_alpha_late` 属于 producer claim 决策，不得据此宣称 `closed beta approved`。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
