# Gameplay 10 分钟留存修复计划（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`

审计轮次: 2

## 任务拆解

- [x] TASK-GAMEPLAY-RR-001 (`PRD-GAME-012`) [test_tier_required]: `producer_system_designer` 已冻结未来两周只优先推进 5 条 retention lane，并完成 `game` 根入口、`gameplay` 主文档与当前 task execution log 挂载。
- [x] TASK-GAMEPLAY-RR-002 (`PRD-GAME-012`) [test_tier_required + test_tier_full]: `viewer_engineer` 已收口首次进入与最小控制地板的前台控制门控与 ack 语义，让 headed Web/UI 与 `software_safe` 不再把明确 `blocked` / `no_progress` 压扁成伪 timeout；fresh active-LLM formal lane 的 floor blocker 与恢复状态由 `TASK-GAMEPLAY-RR-005` 持续跟踪。
- [x] TASK-GAMEPLAY-RR-003 (`PRD-GAME-012`) [test_tier_required]: `runtime_engineer` 已将 `PostOnboarding` 后 10 分钟工业中循环加厚为“韧性生产 -> 第一次扩产取舍 -> 通用 mid-loop”的可复跑目标包，补齐首座工厂、首个制成品、停机恢复与扩产取舍的 canonical 语义。
- [x] TASK-GAMEPLAY-RR-004 (`PRD-GAME-012`) [test_tier_required]: `viewer_engineer` 已收口首屏噪音、玩家身份和后果可见化，把玩家身份、当前主目标、主阻塞、立即下一步以及代价/奖励反馈抬到前台主语义。
- Legacy `TASK-GAMEPLAY-RR-005` (PRD-GAME-012): `qa_engineer` 已区分 active-LLM formal lane 与 debug/probe lane。历史样本曾将 gate 从 `watch` 收口为 `hold`，卡在 `post_onboarding.establish_first_capability / 20%`；该结论现在只作为 historical baseline 保留。当前 fresh formal truth 已由 issue #160 closeout 更新为 `trust gate = pass`、`first capability gate = pass`，不再把旧 `hold/not_run` 当作当前 blocker。
- [x] first-10-30-minute-attraction-hardening (PRD-GAME-012) [test_tier_required]: `TASK-GAME-076` 已完成前 10/30 分钟吸引力诊断、deterministic-provider-backed attraction evidence、逐项 Playwright / actual UI-click / `__AW_TEST__` 完备性自动化与 content-volume gate；required tier 当前为 `motivation_density_pass`、`content_volume_pass`（`34/30` 分钟有效内容、`22/18` 次玩家操作）和 `attraction_pass`。真实玩家留存与生产 provider 放行仍需 live/provider playtest 另证。 Trace: .pm/tasks/task_e3e98d92b70f4168831f756a5872a4aa.yaml

## 任务建议标题（给后续 owner 直接开 task 用）

| 根任务 | owner role | 建议标题 |
| --- | --- | --- |
| `TASK-GAME-061` | `producer_system_designer` | Freeze gameplay 10-minute retention recovery scope and owner lanes |
| `TASK-GAME-062` | `viewer_engineer` | Stabilize first-session control floor across headed Web/UI and software_safe |
| `TASK-GAME-063` | `runtime_engineer` | Ship the first capability package after PostOnboarding |
| `TASK-GAME-064` | `viewer_engineer` | Reduce first-screen noise and surface player-facing consequences/rewards |
| `TASK-GAME-065` | `qa_engineer` | Establish active-LLM 10-minute trust gate and keep capability verdict separate |
| `TASK-GAME-076` | `gameplay_designer` | Maintain first-30-minute attraction gates and live/provider boundary |
| `viewer-economic-readability-first-capability-surface` | `viewer_engineer` | Make the first-capability economy and player value legible |

## Handoff Matrix

| 根任务 | 发起角色 | 接收角色 | 输入 | 期望输出 |
| --- | --- | --- | --- | --- |
| `TASK-GAME-062` | `producer_system_designer` | `viewer_engineer` | 最近 playability 卡片、`viewer` 阻断事实、首连/控制 floor 指标；`software_safe` 仅作为 compat alias 复核 | 正式入口稳定性收口与回归证据 |
| `TASK-GAME-063` | `producer_system_designer` | `runtime_engineer` | 工业引导卡组、`PostOnboarding` 阶段口径、M4 工业链目标 | 首个持续能力 canonical 状态、事件与恢复逻辑 |
| `TASK-GAME-064` | `producer_system_designer` | `viewer_engineer` | 首屏主目标优先级、噪音样本、当前奖励反馈缺口 | 主界面信息层级与反馈可见化收口 |
| `TASK-GAME-065` | `producer_system_designer` | `qa_engineer` | active-LLM 正式 lane 定义、debug lane 边界、阶段当前真值 | `10-minute trust gate` 的 `continue_playing / hold` 裁决，以及与 capability verdict 分开的归档 |
| `TASK-GAME-076` | `producer_system_designer` / `tpm` | `gameplay_designer` + `qa_engineer` + runtime/viewer/agent owners | 已完成的 attraction / motivation / automation evidence、`content_volume_pass=34/30`、内容量实现包 | 后续改动保持 required/live summary 可复跑，并继续保留 `progression_pass_but_attraction_weak` / `content_volume_weak` regression |
| `viewer-economic-readability-first-capability-surface` | `producer_system_designer` | `viewer_engineer` | 高风险修补后的经济可读性要求、first capability 样本、当前 player_gameplay surface | 玩家可见的 `投入/产出/价值/修复/下一步` surface 与回归证据 |

## 验收命令（草案）

- `TASK-GAME-061` / 文档挂载
  - `rg -n "PRD-GAME-012|TASK-GAME-061|TASK-GAME-062|TASK-GAME-063|TASK-GAME-064|TASK-GAME-065" doc/game/prd.md doc/game/project.md doc/game/gameplay/gameplay-top-level-design.prd.md doc/game/gameplay/gameplay-top-level-design.project.md doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.project.md`
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`
- `TASK-GAME-062` / 首次控制地板
  - `./scripts/run-game-test.sh`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 viewer::runtime_live::mapping -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer -- --nocapture`
  - headed Web/UI + `software_safe` 各 1 轮 `agent-browser` 主路径复跑并留证
- `TASK-GAME-063` / 首个持续能力门
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::economy:: -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer ui_text_industrial -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer feedback_tone_for_event_maps_warning_positive_and_info -- --nocapture`
  - `./scripts/run-game-test.sh`
  - 按 `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md` 复跑卡片 A/B/C，并补 `30` 分钟或 `1~3` 次会话 capability follow-up 样本
- `TASK-GAME-064` / 首屏降噪与后果可见化
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer push_feedback_toast_uses_runtime_industry_friendly_detail -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer sync_agent_chatter_bubbles_formats_runtime_industry_feedback -- --nocapture`
  - headed Web/UI 首屏截图对比与 Mission HUD/summary/toast/chatter 人工复核
- `viewer-economic-readability-first-capability-surface` / 经济可读性与 capability value surface
  - `rg -n "cost|output|value|repair|next step|first capability" crates/oasis7_viewer crates/oasis7/src/bin/oasis7_pure_api_client.rs`
  - headed Web/UI 与 pure API 人工复核 `投入 / 产出 / 新用途 / 修复动作 / 下一步价值`
  - `git diff --check`
- `TASK-GAME-065` / 10 分钟 trust gate
  - active-LLM 正式 lane：至少 3 轮 `./scripts/run-game-test.sh` + headed Web/UI 10 分钟 trust 样本
  - `viewer` floor：至少 1 轮正式入口复核；`software_safe` 仅作为 compat alias 复核
  - 回写 `doc/playability_test_result/card_*.md` 与 QA trust verdict，并单列 capability verdict 现状
- `TASK-GAME-076` / 前 10/30 分钟吸引力
  - 设计拆解: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.design.md#task-game-076-0-30-分钟吸引力玩法脚本`
  - 自动化覆盖矩阵: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.design.md#自动化覆盖矩阵`
  - Scenario driver / mock 边界: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.design.md#scenario-driver--mock-边界`
  - Scenario driver 代码源: `crates/oasis7_viewer/software_safe_src/gameplay_attraction_scenario.js`
  - Scenario summary writer: `crates/oasis7_viewer/scripts/write-gameplay-attraction-automation-summary.mjs`
  - Summary writer contract: `crates/oasis7_viewer/scripts/gameplay-attraction-summary-writer.test.mjs`
  - 逐项 Playwright runner: `scripts/viewer-gameplay-attraction-playthrough.sh`
  - 实际 UI 点击 runner: `scripts/viewer-gameplay-attraction-ui-click-playthrough.sh`
  - `__AW_TEST__` 完备性 runner: `scripts/viewer-aw-test-completeness-playthrough.sh`
  - 每个 beat 都必须有 deterministic 自动化断言或显式缺口状态；Playwright / agent-browser 覆盖真实浏览器玩家路径，viewer semantic contract（现有 npm 脚本名仍为 `test:feedback-contract`）/ Vitest 覆盖派生语义，pure API / Rust runtime harness 覆盖 canonical gameplay truth，Bevy / pixel-world 只覆盖空间/视觉层，不替代玩法因果。
  - `live_browser_30m_playthrough` 必须在同一个 browser session 中按 0-1m、1-3m、3-5m、5-7m、7-10m、8-12m、12-18m、18-23m、20-25m、25-30m 顺序执行 action + assertion，并为每个 beat 写出 state artifact；不得只用单步 smoke 代替逐项执行。
  - `live_browser_30m_ui_click_playthrough` 必须用真实 player-visible UI controls 点击推进同一组 10 个 beat；`window.__AW_TEST__` 只允许用于 readiness、state assertion 和 artifact capture，不允许作为 gameplay progression action mechanism。
  - `live_aw_test_completeness_playthrough` 必须逐步证明 `window.__AW_TEST__` 本身完备：`describeControls` / `fillControlExample` / `getState` / `select` / `focus` / `sendGameplayAction(request_snapshot)` / `sendControl(step)` / `runSteps` / recommended action submit 都能在真实 browser session 中跑通并写出 per-step artifacts。
  - required tier 可以使用 scenario driver 生成 deterministic fixture / unit evidence；但 summary 必须标明 `viewer_fixture_only` / `runtime_backed` / `visual_only` / `live_verified` 等来源，不能把 mock 推进包装成真实 0-30 分钟 live gameplay。
  - required 自动化入口: `./scripts/verify-gameplay-attraction-automation.sh --tier required`；其中 `summary_writer_contract` 必须验证 summary JSON/Markdown 均报告 `content_volume_pass`、6 个内容段和 truth coverage
  - live 自动化入口: `./scripts/verify-gameplay-attraction-automation.sh --tier live`（启动真实 browser/player-path 与 pure API gameplay 栈；只有 live tier summary 才能作为真实玩家路径 / pure API gameplay 证据）
  - deterministic-provider-backed attraction evidence 已由 `gameplay_attraction_scenario.js` 生成并纳入 summary：3 个 attraction cards 记录 `hook_score`、`replay_intent`、`action_effect_feedback`、`biggest_boredom_point`、`no-op or follow-up route`
  - 30 分钟 motivation-density card 已纳入 summary，记录 `meaningful_decision_count`、`reward_or_unlock_count`、`stall_or_wait_periods`、`branch_offer_clarity`、`continue_reason`、`return_hook`、`leverage_class`
  - 30 分钟 content-volume card 已纳入 summary，区分“目标链路逐项覆盖”和“内容量足够 30 分钟”；当前 deterministic evidence 为 `34/30` 分钟有效内容、`22/18` 次玩家操作，标记 `content_volume_pass`
  - gameplay_designer content-volume supplement 已落到 deterministic-provider-backed scenario evidence：追加 `6-30m` 内的任务诊断校准、资源换路线、微型生产委托、小事故修复、邻近机会探测、回访封装，并覆盖局部需求、共享项目贡献、玩家造成的世界变化、回访目标和恢复动作字段
  - 玩家代理评审后的二遍可玩性优化已纳入 scenario evidence：`second_run_design_card.status=second_run_hook_pass`，`route_branch_regression.status=pass`；`route_tradeoff` 必须影响后续至少 2 个 beat，`micro_commission` 必须生成可截图成果卡，`opportunity_scan` / `return_package` 必须引用本局选择并产生分支化回访目标
  - 玩家代理复评后的 anti-script guard 已纳入 scenario evidence：`anti_script_design_card.status=anti_script_pass`，`boredom_negative_regression.status=pass`；路线分支必须有中途可见指标差异，本地交付必须推进 `local_demand_progress_after_delivery`，第二局首屏必须复现上局选择记忆，连续 `step/wait/refresh` 推荐动作必须被判为 `attraction_weak`，`quick_patch` / `root_cause_fix` 必须有可见代价差异
  - 多人大世界战略参考已收敛为首局可实现的小切片：局部后勤短缺、本地订单、共享项目贡献、机会扫描和回访目标；不得在 TASK-GAME-076 实现阶段扩成完整大地图、全服市场、联盟战争或多跳供应链系统。
  - weak-sample regression 已纳入 `attraction_sufficiency_cards`：`weak_high_progress` 必须产出 `progression_pass_but_attraction_weak`
  - fresh active-LLM / headed UI 样本仍作为 release/playtest 补证，不是 deterministic design sufficiency gate 的前置条件
  - 若样本能推进但缺少新选择、奖励、回访理由或玩家因果感，记录 `progression_pass_but_attraction_weak` 并路由到 `PRD-GAME-012/014/015` 对应专题
  - `rg -n "TASK-GAME-076|first-10-30-minute-attraction-hardening|0-30 分钟吸引力玩法脚本|自动化覆盖矩阵|Scenario driver|gameplay_attraction_scenario|write-gameplay-attraction-automation-summary|gameplay-attraction-summary-writer|summary_writer_contract|viewer-gameplay-attraction-playthrough|viewer-gameplay-attraction-ui-click-playthrough|viewer-aw-test-completeness-playthrough|live_browser_30m_playthrough|live_browser_30m_ui_click_playthrough|live_aw_test_completeness_playthrough|aw_test_completeness_guard|verify-gameplay-attraction-automation|attraction_sufficiency_cards|attraction_sufficiency_status|motivation_density_pass|runtime_backed|viewer_fixture_only|live_verified|progression_pass_but_attraction_weak|hook_score|meaningful_decision_count|What I caused|New option|Why continue|Playwright|Bevy" doc/game/project.md doc/game/prd.md doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.project.md doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.design.md crates/oasis7_viewer/software_safe_src/gameplay_attraction_scenario.js crates/oasis7_viewer/scripts/write-gameplay-attraction-automation-summary.mjs crates/oasis7_viewer/scripts/gameplay-attraction-summary-writer.test.mjs scripts/viewer-gameplay-attraction-playthrough.sh scripts/viewer-gameplay-attraction-ui-click-playthrough.sh scripts/viewer-aw-test-completeness-playthrough.sh scripts/verify-gameplay-attraction-automation.sh`
- `p0-control-proof-surface` / 首局控制证明 surface
  - `rtk node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`
  - `rtk npm run test:ui -- software_safe_src/main.test.jsx`
  - 验证 `Control Proof` 从既有 `player_gameplay` / feedback 真值派生，且在 `Formal Gameplay Summary` 顶部同卡展示 `Player Intent / World Consequence / Recovery Move / Next Move`

## Done Definition

- `TASK-GAME-061`
  - [x] 新专题 PRD / design / project 已创建并挂到 `game` 根入口与 `gameplay` 主入口
  - [x] 根任务编号、owner role、test tier 与建议标题已冻结
  - [x] 当前阶段仍明确为 `internal_playable_alpha_late`
- `TASK-GAME-062`
  - [x] 首次进入不再依赖手动 reopen/reload 才可控
  - [x] `software_safe` 不再把明确 `blocked` / `timeout_no_progress` 压扁成伪 timeout，前台会回填正确控制反馈
  - [x] viewer-side regression、execution log 与相关证据已回写；active-LLM formal re-certification 已交由 `TASK-GAME-065` 复核，该轮曾形成 historical `hold` baseline，当前 fresh formal truth 已由后续样本更新为 `trust gate = pass`、`first capability gate = pass`
- `TASK-GAME-063`
  - [x] 首个持续能力链已有独立 canonical 包，不再被要求在单个 10 分钟 trust 样本内闭环
  - [x] 建厂/首产出/停机恢复/扩产取舍均有 canonical 状态与前台反馈锚点
- `TASK-GAME-064`
  - [x] 首屏主目标不再被无关历史噪音/operator 语义抢焦点
  - [x] 玩家能直接读到代价、阻塞、恢复和奖励
- `viewer-economic-readability-first-capability-surface`
  - [x] 玩家能直接读懂 first capability 的 `投入 / 产出 / 新用途 / 修复动作 / 下一步价值`
  - [x] 工业成长反馈不再主要依赖库存/产量上涨
- `TASK-GAME-065`
  - [x] QA 已区分 active-LLM 正式 lane 与 debug/probe lane
  - [x] `viewer` formal floor 已在 real-main-config rerun 中恢复；`software_safe` 仅作为 compat / historical evidence 入口保留
  - [x] 历史 `10-minute trust gate = hold` 裁决已保留为 baseline，不再作为当前 blocker
  - [x] fresh formal truth 已更新为 `trust gate = pass`、`first capability gate = pass`；更宽的 release / liveops 边界仍需独立复核
- `TASK-GAME-076`
  - [x] 0-30 分钟 target beat 脚本、测试矩阵和现有 surface 覆盖已逐项落地或明确记录缺口；这不等于新增 content-volume supplement 已完成 runtime/viewer/agent 实现
  - [x] 自动化覆盖矩阵已逐项落地：`./scripts/verify-gameplay-attraction-automation.sh --tier required` 通过；每个 beat 至少有一条 Playwright / agent-browser、viewer semantic contract（`test:feedback-contract`）/ Vitest、pure API / Rust runtime harness 或 Bevy / pixel-world 自动化断言；任何缺口必须以 `*_unverified` / `covered_by_live_when_run` 状态记录，不能用人工试玩替代
  - [x] Playwright 已具备逐项 30 分钟 playthrough：`live_browser_30m_playthrough` 在同一 browser session 中按 10 个 beat 顺序执行 action + assertion，并为每个 beat 写出 state artifact；该命令已纳入 `--tier live`
  - [x] 已补实际 UI 点击版 30 分钟 playthrough：`live_browser_30m_ui_click_playthrough` 通过真实可见按钮选择目标、刷新快照、推进一步、点击推荐动作；测试 API 只用于断言/取证，不用于推进玩法；该命令已纳入 `--tier live`
  - [x] 已补 `__AW_TEST__` 完备性 playthrough：`aw_test_completeness_guard` 覆盖 API 面静态防回退；`live_aw_test_completeness_playthrough` 在真实 browser session 中逐步跑通 API discovery、state read、select/focus、snapshot、step、runSteps、recommended action，并写出 per-step artifacts；该命令已纳入 `--tier live`
  - [x] 已补 summary writer contract：`summary_writer_contract` 覆盖 `gameplay-attraction-summary-writer.test.mjs`，确认 required summary JSON/Markdown 不会漏报 `content_volume_pass`、6 个内容段或 truth coverage
  - [x] 共享 scenario driver 已落地：同一份 canonical scenario 能导出 runtime snapshot、viewer snapshot、Bevy/pixel-world render input 与 attraction evidence；summary 清楚标注 `runtime_backed` / `viewer_fixture_only` / `visual_only` / `live_verified`，避免 mock 数据冒充真实推进
  - [x] live tier 自动化已按需执行：真实玩家路径或 pure API gameplay 证据只能来自 `./scripts/verify-gameplay-attraction-automation.sh --tier live` 或等价 live evidence，不能由 required tier summary 单独声明
  - [x] 3 个 deterministic-provider-backed 样本完成 attraction card，且不把 `trustGateResult=pass` 直接当作 attraction pass；fresh active-LLM / headed UI 样本保留为 release/playtest 补证
  - [x] 30 分钟 motivation-density card 已完成，且 weak-sample regression 能识别 `progression_pass_but_attraction_weak`
  - [x] 30 分钟 content-volume gate 已完成并达标：`content_volume_card.status=content_volume_pass`，`effective_play_minutes=34/30`，`player_operation_count=22/18`
  - [x] gameplay_designer 内容量修复包已落到 deterministic-provider-backed scenario evidence，并在 summary 中报告 `content_volume_supplement_complete=true`
  - [x] 玩家代理评审后的二遍可玩性 guard 已补齐：`second_run_design_card` 验证路线承诺、可截图委托成果、本局选择生成机会和选择记忆回访；`route_branch_regression` 验证 `accelerate` / `stabilize` 产生不同后续事故与回访目标
  - [x] 玩家代理复评后的 anti-script guard 已补齐：`anti_script_design_card` 验证中途路线后果、本地需求交付进度、第二局首屏选择记忆、boredom 负例和修复代价差异；`boredom_negative_regression` 验证连续被动 CTA 不能通过 attraction gate
  - [ ] 若触发旁观感、纯 grind、小玩家无价值感或 world activity only，已路由到 `PRD-GAME-014/015` 并记录 owner follow-up
- `p0-control-proof-surface`
  - [x] `software_safe` summary 已发布 viewer-derived `controlProof`，不新增 runtime schema
  - [x] 正式玩法摘要顶部已展示 `Control Proof`，把玩家意图、世界后果、恢复动作与下一步并排呈现
  - [x] contract / UI 测试覆盖 blocked 与 completed 控制证明语义

## 依赖

- `doc/game/gameplay/gameplay-top-level-design.prd.md`
- `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
- `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
- `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md`
- `doc/world-simulator/m4/m4-resource-product-system-playability-2026-02-27.prd.md`
- `doc/playability_test_result/playability_test_card.md`
- `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md`
- `testing-manual.md`

## 状态

- 更新日期: 2026-06-27
- 当前状态: in_progress
- 当前 owner: `producer_system_designer`
- 下一任务: `first-10-30-minute-attraction-hardening` / `TASK-GAME-076` required tier 已收口；后续只在改动 runtime/viewer/agent 或需要真实玩家留存判断时复跑 live/provider playtest。当前 required pass 不等同于生产 provider 放行或真实玩家留存结论。

- 2026-06-25 P0 control proof follow-up:
  - 已把制作人落点“首局 KPI 从世界活着改为玩家控制被证明”落到 `viewer` 正式入口：`buildGameplaySummary()` 聚合 `controlProof`，`WorldSummaryPanel()` 在 `Formal Gameplay Summary` 顶部显示 `Control Proof` 卡片；`software_safe` 仅作为 compat alias 复核。
  - 本切片只强化 viewer summary 与 UI hierarchy；canonical truth 仍来自 runtime `player_gameplay` / feedback 字段，不能把该卡片单独包装成 `10-minute trust gate` 或 `first capability gate` 新 verdict。
  - 对应测试：`rtk node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`、`rtk npm run test:ui -- software_safe_src/main.test.jsx`。
- 2026-06-25 P1/P2 continuation follow-up:
  - 已在同一 `Formal Gameplay Summary` 中追加 `Agency Moves`、`First Win & Anti-Grind`、`Mature-World Continuation` 和 `Share Replay`，把 P1 打断/重排/纠偏、P1 首胜 anti-grind leverage、P2 repair/rebuild/pivot 与可分享回放落成 viewer 可读 surface。
  - 本切片仍只消费或派生既有 `player_gameplay` 字段；PRD-GAME-015 的 runtime lane truth、agent specialization contract 与 QA mature-world verdict 仍按各自后续任务判定。
  - 对应测试：`rtk node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`、`rtk npm run test:ui -- software_safe_src/main.test.jsx`。

- 口径更新（2026-05-17）: `PRD-GAME-012` 当前正式 verdict 继续拆成两层。`10-minute trust gate` 只判断“控制可信、主目标可读、后果可见、是否愿意继续玩”；`first capability gate` 单独判断“首个持续能力”是否在后续 `30` 分钟或 `1~3` 次会话内闭环。2026-04-15 的 `trust gate = hold / capability gate = not_run` 保留为历史 baseline；当前 fresh active-LLM formal truth 已更新为 `trust gate = pass`、`first capability gate = pass`，证据见 `doc/testing/evidence/issue-160-first-capability-closeout-2026-05-17.md`。
- 说明:
  - 本专题不改变当前阶段，也不改变 active-LLM 正式游玩前置。
  - 本专题优先级高于新的宏系统扩面与宣传性包装。
  - `TASK-GAMEPLAY-RR-001~004` 已完成并回写 `.pm`；其中 `TASK-GAMEPLAY-RR-002/003/004` 分别收口了控制门控与 ack 语义、工业中循环 canonical 包，以及首屏噪音/后果可见化。
  - runtime follow-up `task_7bdbbf9839c74c9eb7bb8c7c161e87de` 已修复 formal lane 在 prior progress 后收到 `blocked` / `completed_no_progress` 反馈时被错误映射回 `first_session_loop` 的问题；这说明样本 B/C 里的“掉回新手态”至少有一部分是快照阶段机口径缺口，而不是完整的真实阶段回滚。
  - runtime follow-up `task_fb967ddaadde459786e286b484bc4b0c` 已补齐另一条独立 freeze path：formal lane 一旦在 prior progress 之后遇到瞬时 LLM access / decision failure，后台 `play` 过去会直接关闭 `session.playing`，把一次短暂 provider 抖动放大成 `logicalTime/eventSeq` 长时间不再前进；当前已改成有限预算重试，并用 runtime-live `auth_actions` 回归固定住“短暂失败可重试、预算耗尽仍停机”的边界。
  - runtime follow-up `task_8d2e20dd7f5c47fd8303ff55159227ba` 已清除另一条更前置的 startup blocker：当前 `NodeRuntimeExecutionDriver` 会在 fresh execution world / simulator mirror 启动时立即落盘 `snapshot.json` 与 `journal.json`，因此 `run-game-test --json-ready` 不再因 `reward-runtime-execution-world` 缺少初始持久化文件而在 Viewer HTTP ready 前退出。该切片只恢复 trust sample 的启动前提，不单独改变 `trust gate` / `first capability gate` verdict。
  - runtime follow-up `task_319c1fc645b04dd185f3afb45dcd00ee` 已把当前 20% 长停的第三条独立签名钉住为 industrial schema drift，而且不是单点文案问题：`llm_agent` prompt/runtime helper 还在声明 assembler-only `factory_kind/recipe_id`，`recipe_coverage` 只跟踪 assembler 三条配方，而 shadow kernel `recipe_plan()` 甚至不会接受 `recipe.smelter.*`；但 `PostOnboarding` canonical 目标链与 `runtime_live` gameplay actions 已切到 smelter-first bootstrap。这样 formal lane 的 active LLM 即使持续推进 world time，也可能始终拿不到、或在 shadow decision path 里直接拒掉，`factory.smelter.mk1` / `recipe.smelter.*` 这些首条能力链动作，表现为一直停在 `post_onboarding.establish_first_capability / 20%`。当前已同步更新 LLM 工业提示、factory/recipe fallback、tracked recipe coverage、shadow kernel recipe support 与定向回归测试，用来消除这条“世界在动但能力链没法被决策命中”的 stall 来源。
  - viewer follow-up `task_a0173315eb4d44c9b83073dd55442f48` 已补齐上一条修复里仍残留的 advanced industrial recipe surface drift：`player_gameplay` 现在会显式暴露 runtime 已支持的 `scale_out` / `governance` 配方动作，active-LLM recipe truth 也扩到 runtime 已开放的 smelter / assembler 高阶配方，shadow kernel 决策面不再漏掉 `recipe.smelter.alloy_plate`、`recipe.assembler.gear`、`recipe.assembler.sensor_pack`、`recipe.assembler.module_rack`、`recipe.assembler.factory_core`。这条 follow-up 的目标是避免 canonical gameplay、LLM 提示与 shadow decision path 继续各说各话，把 runtime 明明可执行的工业能力链留在“支持但永远不会被选中”的灰区。
  - runtime follow-up `task_ed2dd76639264739a61a25c0d89c3352` 已收口当前 retention slice 的另一组 canonical truth regressions：`player_gameplay` 现在会优先跟随当前主线能力链，而不是被字典序更靠前的次级 blocked 工厂劫持；`industry_progress.stage` 也会在回收最后一座已完成产出的工厂后按现存工厂完成度重新回退，不再让历史累计完成数把失效能力误报成 `choose_first_expansion_tradeoff` 或 `choose_midloop_path`。该切片只修复真值误判，不替代新的 active-LLM formal retention 样本。
  - runtime follow-up `task_167a5da426df4c42bf0aa4de26ec1b61` 已收口另一组确定性 progression regressions：`runtime_live` 现在只会在真实玩家控制已确认产生前向增量后，才把后续 `blocked` / `completed_no_progress` 归入 `post_onboarding.recover_capability`，不再把 fresh session 的 bootstrap tick 或背景时间推进误判成正式阶段推进；同时 gameplay industrial action 的建厂门槛已改成与 runtime `BuildFactory` 真值一致，按 `agent ledger -> world fallback` 判断 smelter/assembler build 是否可执行，避免前台 action 卡片仅因忽略 agent ledger 而把可执行扩产链误报为材料不足。
  - `TASK-GAME-065` 的历史正式结论保留为 failure archive：当时 active-LLM `software_safe` floor 虽恢复，但 trust/capability 仍未通过。
  - 后续 `issue-160-first-capability-closeout` 已继续拆除“冻结放大器”与“20% 长停”两条 blocker，并以 `issue160-trust-refresh-fix11-capability-window` 把 formal lane 推进到 `post_onboarding.choose_first_expansion_tradeoff / 92%`。
  - 因此本专题当前应把 2026-04-15 样本集视为历史基线，而不是继续当作今天的 active blocker 文案。
