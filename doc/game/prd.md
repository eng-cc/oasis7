# game PRD

审计轮次: 10

## 目标
- 建立 game 模块设计主文档，统一需求边界、技术方案与验收标准。
- 确保 game 模块后续改动可追溯到 PRD-ID、任务和测试。

## 范围
- 覆盖 game 模块当前能力设计、接口边界、测试口径与演进路线。
- 覆盖 PRD-ID 到 `doc/game/project.md` 的任务映射。
- 不覆盖实现代码逐行说明与历史过程记录。

## 接口 / 数据
- PRD 主入口: `doc/game/prd.md`
- 项目管理入口: `doc/game/project.md`
- 文件级索引: `doc/game/prd.index.md`
- 追踪主键: `PRD-GAME-xxx`
- 测试与发布参考: `testing-manual.md`
- 跨模块模式 taxonomy: `doc/core/player-access-mode-contract-2026-03-19.prd.md`

## 里程碑
- M1 (2026-03-03): 完成模块设计 PRD 主体重写与任务改造。
- M2: 补齐模块设计验收清单与关键指标。
- M3: 建立 PRD-ID -> Task -> Test 的长期追踪闭环。

## 风险
- 模块边界演进快，文档同步可能滞后。
- 指标口径不稳定会降低验收一致性。
## 1. Executive Summary
- Problem Statement: 玩法规则、经济系统、战争治理和发行可玩性要求分布在多份专题文档，缺少统一入口来描述游戏模块的产品目标与验收指标。
- Proposed Solution: 以 game PRD 作为 gameplay 设计总入口，统一定义核心循环、规则层边界、数值治理和发行质量门槛。
- Success Criteria:
  - SC-1: 新增 gameplay 功能均能映射到 PRD-GAME-ID。
  - SC-2: 核心玩法场景（新手/经济/战争）在测试矩阵中具备对应用例。
  - SC-3: 每次版本发布前至少完成一轮可玩性卡片收集并回填闭环。
  - SC-4: 关键玩法规则变更同步更新 game PRD 与 project 文档。
  - SC-5: 微循环关键动作具备可见反馈与计时提示，发布前可玩性卡片评分显著提升。
  - SC-6: 长期在线场景下，治理改动与世界状态具备 tick 级可验证证书和可重放一致性证明。
  - SC-7: 长期在线 P0 能力（状态权威分层、确定性回放/回滚、反作弊与反女巫、经济闭环、可运维性）具备独立 PRD 与分任务门禁。
  - SC-8: 首次行动闭环完成后，玩家在同一会话内获得显式 `PostOnboarding` 阶段目标，并能看到进度、阻塞与下一步建议。
  - SC-9: 纯 API 客户端与 UI 客户端在信息粒度、可执行动作和持续游玩能力上具备可审计的等价性，不再只是探针式协议入口。
  - SC-10: 当前阶段与下一阶段准入门禁具备统一专题 PRD；在 headed Web/UI、pure API、no-UI、longrun/recovery 和 liveops 口径全部收口前，不得把项目对外升级为 `closed beta`。

## 2. User Experience & Functionality
- User Personas:
  - 玩法设计者：需要统一管理玩法目标与平衡约束。
  - 玩法开发者：需要规则层与实现层的映射边界。
  - 发行评审者：需要可度量的可玩性验收标准。
  - 运行值守/SRE：需要持续监控、告警与故障恢复手册，保障长期在线稳定性。
  - 纯 API 玩家/自动化代理：需要不依赖浏览器的正式玩家入口，且不损失玩法语义与持续游玩能力。
- User Scenarios & Frequency:
  - 玩法规则迭代：每个玩法改动周期至少 1 次规则审阅。
  - 核心循环回归：每周执行，覆盖新手/经济/战争路径。
  - 发布前可玩性评估：每个候选版本至少 1 次。
  - 缺陷复盘与再平衡：高优先级问题关闭前必须复测。
- User Stories:
  - PRD-GAME-001: As a 玩法设计者, I want a canonical gameplay blueprint, so that feature decisions are coherent.
  - PRD-GAME-002: As a 玩法开发者, I want clear rule-layer boundaries, so that runtime and gameplay modules evolve safely.
  - PRD-GAME-003: As a 发行评审者, I want measurable playability gates, so that release readiness is objective.
  - PRD-GAME-004: As a 玩家/评测者, I want micro-loop feedback visibility, so that control and pacing are reliable.
  - PRD-GAME-005: As a 运行治理者, I want deterministic distributed execution and governance guardrails, so that the world can run online for the long term.
  - PRD-GAME-006: As a 运行值守者, I want a P0 production hardening baseline for long-run online operation, so that adversarial and failure scenarios stay controllable.
  - PRD-GAME-007: As a 新玩家, I want a post-onboarding stage objective chain, so that I know what to pursue after the first guided action.
  - PRD-GAME-008: As a 纯 API 玩家, I want the same gameplay information and actions as the UI client, so that I can keep playing without a browser.
  - PRD-GAME-009: As a 制作人与阶段评审 owner, I want a unified closed-beta admission gate, so that stage upgrades and external claims are evidence-driven instead of topic-by-topic guesses.
  - PRD-GAME-010: As a 制作人与 limited preview owner, I want one controlled external execution loop, so that the new claim envelope is validated with real feedback instead of internal assumptions.
  - PRD-GAME-011: As a 中循环玩家与玩法 owner, I want agent claims to keep a non-zero main-token-denominated cost and require upkeep, while allowing a restricted starter claim balance for `slot-1`, so that agent control reflects sustained commitment without forcing limited preview users to hold transferable assets.
- 模式分层说明：按 `PRD-CORE-009`，`PRD-GAME-008` 所承接的是玩家访问模式 `pure_api`，而不是 OpenClaw `headless_agent` 一类 execution lane。
- Critical User Flows:
  1. Flow-GAME-001: `玩法需求提出 -> 规则层建模 -> 映射实现边界 -> 进入开发`
  2. Flow-GAME-002: `执行核心循环回归 -> 记录可玩性问题 -> 分级 -> 回填修复任务`
  3. Flow-GAME-003: `发布前汇总可玩性证据 -> 对照门禁 -> 输出放行结论`
  4. Flow-GAME-004: `治理提案 -> 投票 -> timelock -> epoch 生效 -> tick 证书审计回放`
  5. Flow-GAME-005: `P2P 状态传播 -> 权威裁决 -> 回放一致性核验 -> 告警/回滚 -> 事件复盘`
  6. Flow-GAME-006: `完成首次行动闭环 -> 进入 PostOnboarding 阶段 -> 达成首个持续能力里程碑 -> 进入中循环方向`
  7. Flow-GAME-007: `纯 API 客户端连接 -> 获取 canonical gameplay snapshot -> 执行动作/聊天/推进 -> 恢复阶段与下一步 -> 持续推进到中循环入口`
  8. Flow-GAME-008: `制作人冻结当前阶段 -> runtime/viewer/QA/liveops 汇总统一 release gate -> 若任一关键门禁失败则维持 internal_playable_alpha_late -> 全部通过后才允许升级 closed_beta_candidate 口径`
  9. Flow-GAME-009: `制作人冻结 limited preview 执行边界 -> liveops 发起受控 callout -> QA 持续守门并吸收真实信号 -> 制作人决定 continue / hold / reassess`
  10. Flow-GAME-010: `玩家查看未认领 agent 的 canonical 报价 -> 系统按 slot 计算可用的 restricted/liquid funding source -> 支付 activation fee 并锁定 bond -> 进入 upkeep 结算周期 -> 在 release / delinquent reclaim / idle reclaim 中结束 claim`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 核心玩法循环 | 场景、动作、资源、结果 | 执行循环并记录关键指标 | `designed -> implemented -> validated` | 先主循环后扩展循环 | 玩法负责人审核变更 |
| 可玩性问题分级 | 问题描述、严重级、复现步骤、责任人 | 提交后自动进入待修复队列 | `opened -> triaged -> fixed -> verified` | 高严重级优先 | 评审者可调整级别 |
| 发行门禁评审 | 证据包、风险等级、放行建议 | 审查后给出 go/no-go | `pending -> reviewed -> released/blocked` | 风险优先级驱动结论 | 发布负责人最终决策 |
| 分布式执行与治理 | `tick/block hash`、`state_root`、治理提案元数据、身份信誉/抵押 | 发起提案、投票、队列生效、紧急刹车/否决 | `draft -> voting -> queued -> applied/rejected` | tick 全序执行 + epoch 边界生效 | 治理角色+阈值双重校验 |
| 长期在线 P0 硬化 | 权威源标识、回放哈希、作弊风险分、经济源汇统计、SLO 指标 | 执行权威裁决、回放验证、惩罚/申诉、经济阈值调节、告警确认 | `observed -> validated -> enforced -> recovered` | 先一致性后可用性，异常按严重级优先处置 | 运行值守与治理角色联合审批 |
| PostOnboarding 目标链 | `stage_id`、`goal_id`、`goal_type`、`progress`、`blocker_primary`、`next_step_hint` | 完成首次行动闭环后生成主目标并持续更新 | `introduced -> active -> blocked -> completed -> branch_ready` | 默认工业持续能力优先，完成首个里程碑后再展开治理 / 冲突 / 扩张方向 | 玩家可见，系统生成，玩法负责人定义口径 |
| 纯 API 客户端等价 | `player_gameplay_snapshot`、`available_actions`、`recent_feedback`、`parity_level` | 客户端查看阶段/目标/阻塞、执行推进/聊天/命令、恢复会话 | `observer_only -> playable -> parity_verified` | UI/API 共用 canonical 语义，不允许各算一套 | 已连接客户端可读；写操作按玩家鉴权 |
| 阶段准入门禁 | `current_stage`、`candidate_stage`、`claim_envelope`、`trend_status`、`gate_lane_status` | 汇总 headed Web/UI、pure API、no-UI、longrun/recovery 与 liveops 口径，输出升阶或维持原阶段结论 | `internal_playable_alpha -> internal_playable_alpha_late -> closed_beta_candidate -> closed_beta` | 先统一 gate，再允许升级对外口径；任一关键 lane 阻断即整体阻断 | `producer_system_designer` 最终拍板；`qa_engineer` 可独立给阻断建议 |
| 受控 limited preview 执行 | `preview_round_status`、`callout_id`、`signal_quality`、`claim_drift_status`、`qa_recommendation` | 发起 controlled builder-facing 预览、归档真实信号、持续校验 gate、输出 continue/hold/reassess 结论 | `ready_to_run -> running -> reviewed -> continue/hold/reassess` | 先验证口径是否受控，再判断是否扩大节奏 | `producer_system_designer` 最终拍板；`liveops_community` 执行；`qa_engineer` 守门 |
| Agent 认领成本与维护 | `claim_owner_id`、`claim_slot_index`、`activation_fee_amount`、`claim_bond_amount`、`upkeep_per_epoch`、`restricted_starter_claim_balance`、`eligible_claim_balance`、`grace_deadline_epoch`、`release_cooldown_epochs` | 玩家确认认领、系统结算 upkeep、主动释放或强制回收 | `unclaimed -> claimed_active -> upkeep_grace -> released/forced_reclaimed -> unclaimed` | 首个 claim 也必须收费；`slot-1` 可优先消费 restricted starter bucket，`slot-2/3` 成本单调递增并受 `reputation_tier` cap 限制 | runtime 原子校验单 owner 与 funding provenance；viewer/pure API 只读取 canonical 字段 |
- 核心玩法循环验收矩阵（TASK-GAME-002）:
| 循环 | 验收场景（Given / When / Then） | 规则层边界（PRD-GAME-002） | 证据事件/状态 | `test_tier_required` 入口 | 通过阈值（Done） | 失败处置 |
| --- | --- | --- | --- | --- | --- | --- |
| 新手循环（前 1~3 天） | Given `llm_bootstrap` 场景可启动；When 玩家完成“选定目标 -> 首次指令 -> 收到反馈”；Then 在单次会话内形成 `观察 -> 决策 -> 反馈 -> 调整` 闭环。 | 新手引导只消费 runtime 已开放动作（不允许越权改写世界状态）；动作被拒绝时必须返回可解释原因。 | `DomainEvent::ActionAccepted`；viewer 任务循环快照与倒计时提示。 | `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::gameplay_protocol::gameplay_actions_emit_action_accepted_before_resolution_event -- --nocapture`；`env -u RUSTC_WRAPPER cargo test -p oasis7_viewer player_mission_tests:: -- --nocapture` | 必须出现“先接受后解析”的动作证据；玩家任务循环快照能稳定展示剩余提示与反馈计时。 | 阻断合入；补齐失败动作链路日志与 UI 快照，按 P1 建立修复任务并复测。 |
| 经济循环 | Given 双方具备可结算资源；When 执行 `Open -> Accept -> Settle` 经济合约；Then 合约状态与声誉/税费变化可回放。 | 经济规则不得绕过资源守恒；结算溢出/配额/黑名单冲突必须原子拒绝且不污染状态。 | `DomainEvent::EconomicContractOpened/Accepted/Settled/Expired`；`economic_contracts` 状态与声誉快照。 | `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::gameplay_protocol::economic_contract_ -- --nocapture` | 合约终态必须可解释（`Settled` 或 `Expired`）；税费、信誉奖励与策略上限一致；异常路径无半提交状态。 | 阻断合入；输出冲突合约 ID、策略参数与状态差异，回归通过前不得进入发布评审。 |
| 战争循环 | Given 至少两联盟且满足动员成本；When 发起宣战并推进 tick；Then 战争按时结算并写入胜负与参与者后果。 | 宣战必须校验联盟成员身份与动员资源；活动战争期间违反约束的动作（如违规加入）必须拒绝。 | `DomainEvent::WarDeclared/WarConcluded`；`wars` 状态（`active/winner/loser/concluded_at`）。 | `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::gameplay_protocol::war_ -- --nocapture` | 战争必须在设计时长内自动收敛；胜负、资源后果与事件链一致；拒绝路径具备明确规则原因。 | 阻断合入；保存冲突 tick 与战报证据，按 P0 进入规则修复并执行全链路复测。 |
- 矩阵基线一致性校验：`env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required scenario_specs_match_ids -- --nocapture`，用于确保场景入口与矩阵引用保持一致。
- 可玩性问题分级与修复闭环模板（TASK-GAME-003）:
| 等级 | 判定条件 | 典型影响 | 发布门禁动作 | 修复时限（SLO） |
| --- | --- | --- | --- | --- |
| P0 | 关键循环不可达、规则越权、同输入不同结果、核心反馈缺失导致“不可玩”。 | 新手/经济/战争任一主循环中断或产生不可恢复分叉。 | 直接 `blocked`，禁止发布；必须完成复测并由发布负责人确认。 | `<= 24h` 完成修复与复测结论。 |
| P1 | 主循环可运行但体验明显退化，存在稳定复现路径且影响关键指标。 | 玩家可继续游玩但决策反馈延迟、收益失真或冲突结局异常。 | 默认阻断；若需放行必须登记风险豁免 ID 与回滚预案。 | `<= 72h` 完成修复，或进入带风险放行审批。 |
| P2 | 体验瑕疵或低频异常，不破坏主循环可达性。 | 文案、引导节奏、次要可见性偏差。 | 可带缺陷放行，但必须进入下一版本回归清单。 | 下一个迭代周期前关闭。 |
| P3 | 观察项或优化建议，暂无稳定复现与用户影响证据。 | 研发/评测发现的潜在改进点。 | 不阻断发布，纳入趋势看板跟踪。 | 按周评审并决定是否升级优先级。 |
- 闭环执行模板（字段 + 流程）:
| 阶段 | 必填字段 | 执行动作 | 状态流转 | 验证与证据 | 权限/时限 |
| --- | --- | --- | --- | --- | --- |
| 问题提报 | `issue_id`、循环类型（新手/经济/战争）、复现步骤、证据路径、`PRD-GAME-ID` | 创建标准卡片并绑定对应循环与版本。 | `opened` | 至少 1 条可复现证据（事件/日志/UI 截图）。 | 评测者可创建；当日完成。 |
| 问题分级 | 严重级、影响范围、责任人、目标修复版本 | 按分级矩阵打标，确认是否触发发布阻断。 | `opened -> triaged` | 关联门禁条目与预计回归入口。 | 玩法负责人批准；`<= 24h`。 |
| 修复执行 | 根因、修复提交、测试计划、回滚方案 | 开发修复并同步 PRD/project 追踪关系。 | `triaged -> fixing` | 提交记录 + 定向测试计划。 | 责任开发执行；P0/P1 按 SLO。 |
| 修复验证 | 回归命令、结果、剩余风险、复测结论 | 执行定向回归与抽样联动回归。 | `fixing -> verified` | 测试日志 + 关键事件/状态对照。 | QA/评审者确认；未通过不得关闭。 |
| 发布结论 | 发布候选版本、豁免单（若有）、审计人 | 形成 go/no-go 结论并回填证据包。 | `verified -> closed` 或 `verified -> deferred` | 发布记录、豁免审批、回滚预案。 | 发布负责人；P0 不允许 `deferred`。 |
- 闭环强制约束:
  - 无复现证据的 P0/P1 不得降级。
  - P0/P1 在 `verified` 前不得进入发布评审。
  - `deferred` 必须带豁免 ID、风险说明、下一次复测日期。
- 发布前可玩性门禁与回归节奏（TASK-GAME-004）:
| 阶段 | 触发频率 / 时点 | 必跑入口 | 通过标准 | 输出证据 | 失败处置 |
| --- | --- | --- | --- | --- | --- |
| 日常回归（D） | 有 gameplay/viewer 改动的工作日 | `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required scenario_specs_match_ids -- --nocapture`；`env -u RUSTC_WRAPPER cargo test -p oasis7_viewer player_mission_tests:: -- --nocapture` | 两条命令全绿；新增问题中 `P0=0`。 | 当日测试日志 + 问题清单更新。 | 阻断当日合入，转入 `TASK-GAME-003` 闭环。 |
| 候选版本回归（RC） | 每个候选版本至少 1 轮 | `./scripts/ci-tests.sh required`；`env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::gameplay_protocol:: -- --nocapture` | required 套件通过；新手/经济/战争协议回归无回退；`P0/P1=0` 或具备豁免。 | RC 回归报告 + 命令与结论。 | 标记 `blocked`，禁止进入发布评审。 |
| Web 闭环门禁（D-1） | 发布前 1 天 | `./scripts/run-game-test-ab.sh --headed --no-llm`（S6）；按 `doc/playability_test_result/game-test.prd.md` 填写卡片 | A/B 流程 `PASS`；`console error = 0`；有效控制命中率 `>= 80%`；无未豁免 `P0/P1`。 | `output/playwright/playability/<run_id>/` + `doc/playability_test_result/card_*.md`。 | 阻断发布，进入修复并复跑 S6。 |
| 发布评审（D0） | 发布会 | 汇总 D/RC/D-1 证据包并执行 go/no-go 评审 | 证据链完整、结论一致、风险闭环清晰。 | 发布结论（`go`/`no-go`）、豁免单、回滚预案。 | 结论为 `no-go` 时冻结版本并触发应急回归。 |
- 发布证据包最小字段:
  - 命令清单（含执行时间、执行人、结果摘要）。
  - 日志与产物路径（测试日志、agent-browser 录屏/截图、可玩性卡片）。
  - 问题闭环状态（`P0~P3` 分级、豁免 ID、下次复测时间）。
  - 最终决策记录（go/no-go、风险、回滚策略）。
- Acceptance Criteria:
  - AC-1: game PRD 覆盖核心玩法循环、治理机制、测试口径。
  - AC-2: game project 文档任务项可映射到 PRD-GAME-001/002/003。
  - AC-3: 与 `doc/game/gameplay/gameplay-top-level-design.prd.md`、`doc/game/gameplay/gameplay-engineering-architecture.md` 口径一致。
  - AC-4: 发行前可玩性回归必须在 testing 手册与测试结果中可追溯。
  - AC-5: 微循环反馈优化 PRD 定义可见反馈与计时规则，并形成可验证的评分提升目标。
  - AC-6: 新增长期在线分布式专题 PRD，明确 RSM、治理时延生效、身份与惩罚的验收约束。
  - AC-7: 新手/经济/战争三循环均具备 Given/When/Then、规则层边界、证据事件、`test_tier_required` 命令与失败处置，且可直接用于周回归。
  - AC-8: 可玩性问题分级模板覆盖 `P0~P3` 判定、发布阻断规则、责任人和时限，并能直接驱动 `opened -> triaged -> fixing -> verified -> closed/deferred` 闭环。
  - AC-9: 发布前门禁明确 D/RC/D-1/D0 节奏、必跑命令、通过阈值与证据包字段，能够直接产出 go/no-go 决策。
  - AC-10: 新增长期在线 P0 专题 PRD，覆盖状态权威分层、确定性回放/回滚、反作弊与反女巫、经济闭环、可运维性五项能力，并提供 PRD-ID 到任务与测试映射。
  - AC-11: 新增 `PostOnboarding` 专题 PRD，明确首次行动闭环后的阶段目标、阻塞分类、阶段完成与中循环承接，并可映射到 `#46` 的 required-tier 验收。
  - AC-12: 新增纯 API 等价专题 PRD，明确 canonical 玩家语义、动作集合、恢复逻辑与 UI/API parity matrix，并可映射到纯 API 长玩 required/full 验收。
  - AC-13: 新增 `PRD-GAME-009` 封闭 Beta 准入专题，明确当前阶段、统一 release gate、趋势阈值和对外口径边界，并能直接驱动跨角色 handoff。
  - AC-14: 新增 `PRD-GAME-010` 受控 limited preview 执行专题，明确 controlled builder-facing 外放、claim drift 纠偏、QA 持续守门与制作人 continue/hold/reassess 决策。
  - AC-15: 新增 `PRD-GAME-011` agent 认领成本专题，明确“首个也不免费”、claim bond/upkeep、`restricted starter claim balance`、闲置/欠费回收、tier cap 与 UI/API canonical 字段。
- Non-Goals:
  - 不在本 PRD 中给出逐条数值参数表。
  - 不替代 runtime/p2p 的底层实现设计。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: LLM 行为测试套件、场景驱动回归、可玩性卡片采集流程。
- Evaluation Strategy: 以场景可达成率、关键动作成功率、可玩性反馈缺陷收敛时长作为评估指标。

## 4. Technical Specifications
- Architecture Overview: game 模块定义玩法层抽象，依赖 world-runtime 提供规则执行与资源约束，依赖 world-simulator 与 testing 模块提供可观测与验收。
- Integration Points:
  - `doc/game/gameplay/gameplay-top-level-design.prd.md`
  - `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
  - `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
  - `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
  - `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06.prd.md`
  - `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
  - `doc/game/gameplay/gameplay-engineering-architecture.md`
  - `doc/playability_test_result/prd.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 空场景配置：缺少关键玩法配置时禁止进入验收并给出缺失项。
  - 数据异常：数值配置越界时阻断合入并输出规则冲突说明。
  - 权限不足：非玩法负责人不得直接修改核心门禁阈值。
  - 并发冲突：同一玩法规则并行修改时需合并评审再落库。
  - 反馈缺失：无可玩性证据时不得进入发布评审。
  - 回归超时：关键循环回归超时需保留中间产物并重试。
  - 状态分叉：出现同 tick 不同 `state_root` 时阻断提交并触发恢复流程。
  - 提前生效：治理提案在 `timelock/epoch` 约束前申请生效必须拒绝。
  - 女巫攻击：疑似多号协同投票触发权重冻结与人工复核。
  - 权威漂移：同高度出现多个权威裁决来源时，必须拒绝非权威写入并触发仲裁告警。
  - 经济失衡：单位时间净增发/净流出超过阈值时触发自动降载策略与治理升级提案。
  - API 语义缺口：若玩家继续游玩所需字段仅存在于 UI 组装层，则 pure API 入口必须被判定为 `observer_only` 并阻断“等价”口径。
  - 口径漂移：若专题证据已通过但统一 stage gate 未建立，则必须维持 `internal_playable_alpha_late`，不得提前对外升级为 `closed beta`。
- Non-Functional Requirements:
  - NFR-GAME-1: 关键玩法回归覆盖率 100%（新手/经济/战争）。
  - NFR-GAME-2: 高优先级可玩性问题发布前闭环率 >= 95%。
  - NFR-GAME-3: 玩法门禁结论具备完整证据链（命令/日志/结论）。
  - NFR-GAME-4: 玩法规则口径在模块文档中 1 个工作日内同步。
  - NFR-GAME-5: 玩法改动必须可追溯到 PRD-ID。
  - NFR-GAME-6: RSM 回放一致性偏差率为 0（同输入同版本）。
  - NFR-GAME-7: 治理规则变更 100% 走提案链路并满足 `timelock + epoch` 生效。
  - NFR-GAME-8: 紧急权限触发事件 100% 具备可验签证据和审计记录。
  - NFR-GAME-9: 权威裁决冲突检测误放过率为 0，冲突告警在 60 秒内可见。
  - NFR-GAME-10: 回放漂移发现到回滚执行的 P95 时间 <= 10 分钟。
  - NFR-GAME-11: 经济源汇审计日批成功率 100%，关键异常 5 分钟内触发告警。
  - NFR-GAME-12: P0 故障（一致性/作弊/经济/可用性）均具备可执行 runbook 与演练记录。
  - NFR-GAME-13: 正式纯 API 玩家入口 100% 具备 canonical `stage/goal/progress/blocker/next_step/available_actions` 字段，不允许依赖 UI 私有拼装。
  - NFR-GAME-14: 纯 API required-tier 长玩回归必须在 fresh bundle 本地可复跑，并至少推进到首个持续能力里程碑。
  - NFR-GAME-15: 在 `PRD-GAME-009` 的统一 release gate 未通过前，公开渠道 100% 维持 `limited playable technical preview` 口径，不允许出现 `closed beta` / `play now` / `live now`。
  - NFR-GAME-16: `PRD-GAME-010` 的每一轮受控 limited preview 执行都必须在同日回写信号归档、owner 与 next action，并允许 QA 因真实反馈把 unified gate 从 `pass` 回退为 `block`。
  - NFR-GAME-17: `PRD-GAME-011` 的首个 claim 免费路径命中次数必须为 `0`，并且 claim / upkeep / refund / slash / restricted grant 事件必须 100% 进入 token 审计链路。
- Security & Privacy: gameplay 不直接处理密钥；涉及玩家反馈与行为数据时遵循最小化采集与脱敏记录。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03-03): 建立 gameplay 统一设计基线与验收指标。
  - v1.1: 对齐战争/治理/经济三条主循环的跨模块测试门禁。
  - v2.0: 形成玩法改动到可玩性结果的量化闭环报表。
- Technical Risks:
  - 风险-1: 玩法复杂度上升导致规则冲突。
  - 风险-2: 只看技术测试通过而忽略真实可玩性退化。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-GAME-001 | TASK-GAME-001/002/005 | `test_tier_required` | 核心循环验收矩阵检查 | 玩法主循环一致性 |
| PRD-GAME-002 | TASK-GAME-002/003/005 | `test_tier_required` + `test_tier_full` | 规则层边界回归、跨模块联动抽样 | gameplay/runtime 协同稳定性 |
| PRD-GAME-003 | TASK-GAME-003/004/005 | `test_tier_required` | 问题分级模板抽样、修复闭环记录核验、发布门禁对账 | 发布质量与玩家体验风险 |
| PRD-GAME-004 | TASK-GAME-006 + TASK-GAMEPLAY-MLF-001/002/003/004 | `test_tier_required` | 微循环反馈可见性回归 + 可玩性卡片评分复核 | 玩家控制感与节奏体验 |
| PRD-GAME-005 | TASK-GAME-008 + TASK-GAME-DCG-001/002/003/004/005/006/007/008/009/010 | `test_tier_required` + `test_tier_full` | Tick 证书、治理时序、身份惩罚闭环验证 | 长期在线一致性与治理安全 |
| PRD-GAME-006 | TASK-GAME-012/013/014/015/016/017 | `test_tier_required` + `test_tier_full` | 权威分层裁决回归、回放与回滚演练、反作弊/反女巫对抗用例、经济源汇审计、SRE runbook 演练 | 长期在线 P0 稳定性与运维可信度 |
| PRD-GAME-007 | TASK-GAME-021 + TASK-GAMEPLAY-POD-001/002/003/004 | `test_tier_required` | 文档治理检查、Viewer / Web required-tier 回归、playability 卡片复核 | 新手阶段承接、`#46` 回归、目标链表达稳定性 |
| PRD-GAME-008 | TASK-GAME-023 + TASK-GAMEPLAY-API-001/002/003/004 | `test_tier_required` + `test_tier_full` | 文档治理检查、协议字段对账、纯 API 长玩回归、UI/API parity matrix、full-tier 长稳抽样 | 纯 API 正式入口、阶段承接、持续游玩等价性 |
| PRD-GAME-009 | TASK-GAME-028/029/030/031/032/033 | `test_tier_required` + `test_tier_full` | 文档治理检查、统一 release gate、趋势基线对账、longrun/recovery 证据、runbook 口径检查 | 当前阶段判断、封闭 Beta 准入、对外口径一致性 |
| PRD-GAME-010 | TASK-GAME-035/036/037/038 | `test_tier_required` | 文档治理检查、limited preview callout 与回流模板核验、QA 守门结论、producer 复盘记录 | 受控预览执行、claim drift、继续/暂停决策 |
| PRD-GAME-011 | TASK-GAME-039/040/041/042/043/044/045/046/047/048/049/050/051 | `test_tier_required` + `test_tier_full` | 文档治理检查、claim/upkeep/reclaim 状态机回归、restricted bucket 与 provenance 回归、grant lifecycle 与 issuer runbook、Viewer/API parity、经济审计与 abuse suite | agent 占有边界、token sink、受限启动余额、回收与可观测性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-GAME-001 | 以玩法循环为需求主轴组织验收 | 以功能列表平铺验收 | 循环视角更贴近真实体验链路。 |
| DEC-GAME-002 | 引入问题分级与闭环模板 | 缺陷统一平级处理 | 可优化修复优先级与发布节奏。 |
| DEC-GAME-003 | 发布评审绑定可玩性证据 | 仅依赖技术测试 | 能降低“可运行但不好玩”的发布风险。 |
| DEC-GAME-004 | 以“新手/经济/战争”分循环验收矩阵驱动 `TASK-GAME-002` | 仅保留统一 required/full 命令清单 | 分循环矩阵更易映射规则边界、失败处置与责任归属。 |
| DEC-GAME-005 | 采用 `P0~P3 + 闭环模板 + deferred 豁免` 的分级机制 | 仅保留缺陷列表，不定义状态与门禁 | 可保证问题优先级、修复责任与发布决策可审计。 |
| DEC-GAME-006 | 采用 `D/RC/D-1/D0` 四阶段可玩性门禁节奏 | 仅在发布前一次性回归 | 分阶段门禁可提前暴露风险，降低临门一脚失败概率。 |
| DEC-GAME-007 | 新增独立 `PRD-GAME-006` 作为长期在线 P0 基线 | 将 P0 细节继续堆叠在 `PRD-GAME-005` 现有章节中 | 独立基线更利于跨角色（玩法/运行/安全/经济）协同验收与长期维护。 |
| DEC-GAME-008 | 新增独立 `PRD-GAME-007` 作为 `FirstSessionLoop` 之后的阶段承接专题 | 继续把 post-4/4 行为留在静态提示或零散 Viewer 文案里 | 独立专题更利于把 `#46` 从 UI 缺陷提升为正式玩法阶段设计问题。 |
| DEC-GAME-009 | 新增独立 `PRD-GAME-008` 作为纯 API 正式玩家入口专题 | 继续把无 UI 路径视为探针/调试能力，不定义玩法等价门禁 | 用户目标是长期以 API 玩游戏，必须把“协议可用”升级为“玩法可玩且等价”。 |
| DEC-GAME-010 | 新增独立 `PRD-GAME-009` 作为封闭 Beta 准入专题 | 继续把阶段判断留在零散 evidence/devlog 中 | 当前阶段已跨过原型，但还未达到 Beta；独立专题更利于统一 gate、趋势与对外口径。 |
| DEC-GAME-011 | 新增独立 `PRD-GAME-010` 作为受控 limited preview 执行专题 | 在没有真实外部样本的情况下直接扩大节奏或继续只做内部判断 | 现在缺的不是新的技术门，而是“limited playable technical preview”在真实执行中是否稳定成立的治理闭环。 |
| DEC-GAME-012 | 新增独立 `PRD-GAME-011` 作为 agent 认领成本专题，并明确首个 claim 也不免费 | 继续允许零成本首槽或只对后续槽位收费 | agent 控制权需要体现持续承诺；首槽免费会成为囤位与多号利用的最短路径。 |
| DEC-GAME-013 | 对 `PRD-GAME-011` 追加 `restricted starter claim balance`，作为 `slot-1` 专用受限资金来源 | 继续要求所有首个 claim 都必须持有可转账 liquid；或直接空投可转账 main token | 受控测试需要启动资金，但不能把启动补贴变成可转账资产，也不能推翻“首个 claim 非免费”的主规则。 |
