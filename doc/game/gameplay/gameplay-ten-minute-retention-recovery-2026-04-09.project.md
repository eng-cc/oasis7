# Gameplay 10 分钟留存修复计划（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`

审计轮次: 2

## 任务拆解

- [x] TASK-GAMEPLAY-RR-001 (`PRD-GAME-012`) [test_tier_required]: `producer_system_designer` 已冻结未来两周只优先推进 5 条 retention lane，并完成 `game` 根入口、`gameplay` 主文档与当前 task execution log 挂载。
- [x] TASK-GAMEPLAY-RR-002 (`PRD-GAME-012`) [test_tier_required + test_tier_full]: `viewer_engineer` 已收口首次进入与最小控制地板的前台控制门控与 ack 语义，让 headed Web/UI 与 `software_safe` 不再把明确 `blocked` / `no_progress` 压扁成伪 timeout；fresh active-LLM formal lane 的 floor blocker 与恢复状态由 `TASK-GAMEPLAY-RR-005` 持续跟踪。
- [x] TASK-GAMEPLAY-RR-003 (`PRD-GAME-012`) [test_tier_required]: `runtime_engineer` 已将 `PostOnboarding` 后 10 分钟工业中循环加厚为“韧性生产 -> 第一次扩产取舍 -> 通用 mid-loop”的可复跑目标包，补齐首座工厂、首个制成品、停机恢复与扩产取舍的 canonical 语义。
- [x] TASK-GAMEPLAY-RR-004 (`PRD-GAME-012`) [test_tier_required]: `viewer_engineer` 已收口首屏噪音、玩家身份和后果可见化，把玩家身份、当前主目标、主阻塞、立即下一步以及代价/奖励反馈抬到前台主语义。
- [x] TASK-GAMEPLAY-RR-005 (`PRD-GAME-012`) [test_tier_required]: `qa_engineer` 已区分 active-LLM formal lane 与 debug/probe lane，并在复制 `main` 的 real provider `config.toml` 后完成 `3` 条 active-LLM 10 分钟正式样本；当前 gate 已从 `watch` 收口为 `hold`，因为 formal lane 虽恢复 `software_safe` first-step floor，但仍卡在 `post_onboarding.establish_first_capability / 20%`，且其中 `2` 条样本回退到 `first_session_loop.create_first_world_feedback / 0%` 并伴随 `logicalTime/eventSeq` 冻结。

## 任务建议标题（给后续 owner 直接开 task 用）

| 根任务 | owner role | 建议标题 |
| --- | --- | --- |
| `TASK-GAME-061` | `producer_system_designer` | Freeze gameplay 10-minute retention recovery scope and owner lanes |
| `TASK-GAME-062` | `viewer_engineer` | Stabilize first-session control floor across headed Web/UI and software_safe |
| `TASK-GAME-063` | `runtime_engineer` | Ship the first 10-minute industrial midloop package after PostOnboarding |
| `TASK-GAME-064` | `viewer_engineer` | Reduce first-screen noise and surface player-facing consequences/rewards |
| `TASK-GAME-065` | `qa_engineer` | Establish active-LLM 10-minute retention gate and producer continue/hold verdict |

## Handoff Matrix

| 根任务 | 发起角色 | 接收角色 | 输入 | 期望输出 |
| --- | --- | --- | --- | --- |
| `TASK-GAME-062` | `producer_system_designer` | `viewer_engineer` | 最近 playability 卡片、`software_safe` 阻断事实、首连/控制 floor 指标 | 正式入口稳定性收口与回归证据 |
| `TASK-GAME-063` | `producer_system_designer` | `runtime_engineer` | 工业引导卡组、`PostOnboarding` 阶段口径、M4 工业链目标 | 10 分钟中循环 canonical 状态、事件与恢复逻辑 |
| `TASK-GAME-064` | `producer_system_designer` | `viewer_engineer` | 首屏主目标优先级、噪音样本、当前奖励反馈缺口 | 主界面信息层级与反馈可见化收口 |
| `TASK-GAME-065` | `producer_system_designer` | `qa_engineer` | active-LLM 正式 lane 定义、debug lane 边界、阶段当前真值 | `continue_playing` / `hold` gate 与 producer 裁决输入 |

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
- `TASK-GAME-063` / 工业中循环包
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::economy:: -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer ui_text_industrial -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer feedback_tone_for_event_maps_warning_positive_and_info -- --nocapture`
  - `./scripts/run-game-test.sh`
  - 按 `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md` 复跑卡片 A/B/C
- `TASK-GAME-064` / 首屏降噪与后果可见化
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer push_feedback_toast_uses_runtime_industry_friendly_detail -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer sync_agent_chatter_bubbles_formats_runtime_industry_feedback -- --nocapture`
  - headed Web/UI 首屏截图对比与 Mission HUD/summary/toast/chatter 人工复核
- `TASK-GAME-065` / 10 分钟 retention gate
  - active-LLM 正式 lane：至少 3 轮 `./scripts/run-game-test.sh` + headed Web/UI 10 分钟样本
  - `software_safe` floor：至少 1 轮正式入口复核
  - 回写 `doc/playability_test_result/card_*.md` 与 QA 汇总结论

## Done Definition

- `TASK-GAME-061`
  - [x] 新专题 PRD / design / project 已创建并挂到 `game` 根入口与 `gameplay` 主入口
  - [x] 根任务编号、owner role、test tier 与建议标题已冻结
  - [x] 当前阶段仍明确为 `internal_playable_alpha_late`
- `TASK-GAME-062`
  - [x] 首次进入不再依赖手动 reopen/reload 才可控
  - [x] `software_safe` 不再把明确 `blocked` / `timeout_no_progress` 压扁成伪 timeout，前台会回填正确控制反馈
  - [x] viewer-side regression、execution log 与相关证据已回写；fresh active-LLM formal re-certification 已交由 `TASK-GAME-065` 复核并形成当前 `hold` 裁决
- `TASK-GAME-063`
  - [x] 10 分钟工业中循环包可在同一会话完成
  - [x] 建厂/首产出/停机恢复/扩产取舍均有 canonical 状态与前台反馈锚点
- `TASK-GAME-064`
  - [x] 首屏主目标不再被无关历史噪音/operator 语义抢焦点
  - [x] 玩家能直接读到代价、阻塞、恢复和奖励
- `TASK-GAME-065`
  - [x] QA 已区分 active-LLM 正式 lane 与 debug/probe lane
  - [x] `software_safe` formal floor 已在 real-main-config rerun 中恢复
  - [x] 已完成 `3` 条 active-LLM 10 分钟正式样本与最终 `hold` 裁决回写

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

- 更新日期: 2026-04-14
- 当前状态: in_progress
- 当前 owner: `producer_system_designer`
- 下一任务: 由 `producer_system_designer` 按 `TASK-GAME-065` 的正式阻断签名拆出下一轮 runtime/viewer 修复切片，再重新申请 formal retention 复验。
- 说明:
  - 本专题不改变当前阶段，也不改变 active-LLM 正式游玩前置。
  - 本专题优先级高于新的宏系统扩面与宣传性包装。
  - `TASK-GAMEPLAY-RR-001~004` 已完成并回写 `.pm`；其中 `TASK-GAMEPLAY-RR-002/003/004` 分别收口了控制门控与 ack 语义、工业中循环 canonical 包，以及首屏噪音/后果可见化。
  - runtime follow-up `task_7bdbbf9839c74c9eb7bb8c7c161e87de` 已修复 formal lane 在 prior progress 后收到 `blocked` / `completed_no_progress` 反馈时被错误映射回 `first_session_loop` 的问题；这说明样本 B/C 里的“掉回新手态”至少有一部分是快照阶段机口径缺口，而不是完整的真实阶段回滚。
  - runtime follow-up `task_fb967ddaadde459786e286b484bc4b0c` 已补齐另一条独立 freeze path：formal lane 一旦在 prior progress 之后遇到瞬时 LLM access / decision failure，后台 `play` 过去会直接关闭 `session.playing`，把一次短暂 provider 抖动放大成 `logicalTime/eventSeq` 长时间不再前进；当前已改成有限预算重试，并用 runtime-live `auth_actions` 回归固定住“短暂失败可重试、预算耗尽仍停机”的边界。
  - runtime follow-up `task_319c1fc645b04dd185f3afb45dcd00ee` 已把当前 20% 长停的第三条独立签名钉住为 industrial schema drift，而且不是单点文案问题：`llm_agent` prompt/runtime helper 还在声明 assembler-only `factory_kind/recipe_id`，`recipe_coverage` 只跟踪 assembler 三条配方，而 shadow kernel `recipe_plan()` 甚至不会接受 `recipe.smelter.*`；但 `PostOnboarding` canonical 目标链与 `runtime_live` gameplay actions 已切到 smelter-first bootstrap。这样 formal lane 的 active LLM 即使持续推进 world time，也可能始终拿不到、或在 shadow decision path 里直接拒掉，`factory.smelter.mk1` / `recipe.smelter.*` 这些首条能力链动作，表现为一直停在 `post_onboarding.establish_first_capability / 20%`。当前已同步更新 LLM 工业提示、factory/recipe fallback、tracked recipe coverage、shadow kernel recipe support 与定向回归测试，用来消除这条“世界在动但能力链没法被决策命中”的 stall 来源。
  - viewer follow-up `task_a0173315eb4d44c9b83073dd55442f48` 已补齐上一条修复里仍残留的 advanced industrial recipe surface drift：`player_gameplay` 现在会显式暴露 runtime 已支持的 `scale_out` / `governance` 配方动作，active-LLM recipe truth 也扩到 runtime 已开放的 smelter / assembler 高阶配方，shadow kernel 决策面不再漏掉 `recipe.smelter.alloy_plate`、`recipe.assembler.gear`、`recipe.assembler.sensor_pack`、`recipe.assembler.module_rack`、`recipe.assembler.factory_core`。这条 follow-up 的目标是避免 canonical gameplay、LLM 提示与 shadow decision path 继续各说各话，把 runtime 明明可执行的工业能力链留在“支持但永远不会被选中”的灰区。
  - runtime follow-up `task_ed2dd76639264739a61a25c0d89c3352` 已收口当前 retention slice 的另一组 canonical truth regressions：`player_gameplay` 现在会优先跟随当前主线能力链，而不是被字典序更靠前的次级 blocked 工厂劫持；`industry_progress.stage` 也会在回收最后一座已完成产出的工厂后按现存工厂完成度重新回退，不再让历史累计完成数把失效能力误报成 `choose_first_expansion_tradeoff` 或 `choose_midloop_path`。该切片只修复真值误判，不替代新的 active-LLM formal retention 样本。
  - runtime follow-up `task_167a5da426df4c42bf0aa4de26ec1b61` 已收口另一组确定性 progression regressions：`runtime_live` 现在只会在真实玩家控制已确认产生前向增量后，才把后续 `blocked` / `completed_no_progress` 归入 `post_onboarding.recover_capability`，不再把 fresh session 的 bootstrap tick 或背景时间推进误判成正式阶段推进；同时 gameplay industrial action 的建厂门槛已改成与 runtime `BuildFactory` 真值一致，按 `agent ledger -> world fallback` 判断 smelter/assembler build 是否可执行，避免前台 action 卡片仅因忽略 agent ledger 而把可执行扩产链误报为材料不足。
  - `TASK-GAME-065` 的最新正式结论是：active-LLM `software_safe` formal floor 已恢复，不再以 `Responses API` 10 秒超时作为当前阻断项；但 `3` 条 10 分钟正式样本均未支持 `continue_playing`，其中 `1` 条长期停在 `post_onboarding.establish_first_capability / 20%`，另 `2` 条出现 `post_onboarding -> first_session_loop` 回退并冻结世界时间，因此当前 producer verdict 为 `hold`。
  - 最新 `240s` active-LLM 对照复跑没有再复现硬冻结，但仍稳定停在 `post_onboarding.establish_first_capability / 20%`；这说明“冻结放大器”与“长期不推进”是两条相关但独立的 blocker，后者仍是当前 formal retention gate 的主阻断。
  - 即便去掉上述快照误回退、缓解瞬时失败放大器并对齐 industrial schema，本专题仍未重新拿到 fresh active-LLM formal 复验证据，也没有证明所有历史 `logicalTime/eventSeq` 冻结都已消失；在新的正式样本确认 `20%` 长停消失前，formal retention gate 仍不得回收 `hold`。
