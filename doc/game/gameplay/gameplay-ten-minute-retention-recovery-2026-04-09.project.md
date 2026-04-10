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
  - [x] viewer-side regression、execution log 与相关证据已回写；fresh active-LLM formal re-certification 由 `TASK-GAME-065` 复核后更新为 `watch`
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

- 更新日期: 2026-04-10
- 当前状态: in_progress
- 当前 owner: `producer_system_designer`
- 下一任务: 由 `producer_system_designer` 按 `TASK-GAME-065` 的正式阻断签名拆出下一轮 runtime/viewer 修复切片，再重新申请 formal retention 复验。
- 说明:
  - 本专题不改变当前阶段，也不改变 active-LLM 正式游玩前置。
  - 本专题优先级高于新的宏系统扩面与宣传性包装。
  - `TASK-GAMEPLAY-RR-001~004` 已完成并回写 `.pm`；其中 `TASK-GAMEPLAY-RR-002/003/004` 分别收口了控制门控与 ack 语义、工业中循环 canonical 包，以及首屏噪音/后果可见化。
  - `TASK-GAME-065` 的最新正式结论是：active-LLM `software_safe` formal floor 已恢复，不再以 `Responses API` 10 秒超时作为当前阻断项；但 `3` 条 10 分钟正式样本均未支持 `continue_playing`，其中 `1` 条长期停在 `post_onboarding.establish_first_capability / 20%`，另 `2` 条出现 `post_onboarding -> first_session_loop` 回退并冻结世界时间，因此当前 producer verdict 为 `hold`。
