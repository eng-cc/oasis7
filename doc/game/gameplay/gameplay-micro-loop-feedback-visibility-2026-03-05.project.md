# Gameplay 微循环反馈可见性闭环（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md`

## ROUND-002 主从口径
- 主入口：`doc/game/gameplay/gameplay-top-level-design.prd.md`
- 本文仅维护增量计划，不重复主文档口径。

## 执行目标（How / When / Who）
- How：按“runtime 协议 -> runtime_live 映射 -> viewer 反馈 -> 门禁回归”顺序交付，避免前后口径漂移。
- When：以 5 个任务切片推进（D0 文档基线 + D1/D2 实现 + D3 回归）。
- Who：按责任域分工（runtime / viewer / test-release），每个任务单独提交并记录 devlog。

## 任务拆解（含 PRD-ID 映射）

| Task ID | PRD-ID | Owner | 计划窗口 | 核心改动 | 交付物 | 测试层级 | 状态 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TASK-GAMEPLAY-MLF-000 | PRD-GAME-004 | game-doc-owner | D0 (2026-03-05) | PRD 与项目文档建模、索引挂载 | `gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md` / `.project.md` | `test_tier_required`（文档治理） | completed |
| TASK-GAMEPLAY-MLF-001 | PRD-GAME-004-01/02 | runtime-owner | D1 | 新增 `ActionAccepted` 协议原语并接入 gameplay action 流程，保证“动作被接受”可观测 | runtime 事件模型 + action/event 处理代码 + gameplay 协议回归测试 | `test_tier_required` | completed |
| TASK-GAMEPLAY-MLF-002 | PRD-GAME-004-02 | runtime-live-owner | D1-D2 | runtime_live 对 gameplay DomainEvent 与 ACK 的结构化映射与兼容降级 | runtime_live mapping + 兼容降级摘要规则 + 映射回归测试 | `test_tier_required` | completed |
| TASK-GAMEPLAY-MLF-003 | PRD-GAME-004-01 | viewer-owner | D2 | 微循环 HUD（倒计时/最近动作状态）与无进展诊断建议 | viewer right panel / i18n / tone 规则 | `test_tier_required` | completed |
| TASK-GAMEPLAY-MLF-004 | PRD-GAME-004-03 | test-release-owner | D3 | required/full 回归与卡片评分复核，形成门禁证据包 | 测试日志 + playability 卡片 + 结论 | `test_tier_required` + `test_tier_full` | completed |
| TASK-GAMEPLAY-MLF-005 | PRD-GAME-004-04 | viewer-owner | D4 | Mission HUD 增加控制结果显著条，压缩重复反馈信息 | `egui_right_panel_player_guide.rs` + 任务回归测试 | `test_tier_required` | completed |
| TASK-GAMEPLAY-MLF-006 | PRD-GAME-004-04 | viewer-owner | D4 | 玩家模式默认 Mission 预设 + 模块开关渐进披露 + 可见性持久化缺字段默认策略修正 | `egui_right_panel.rs` + `right_panel_module_visibility.rs` + 回归测试 | `test_tier_required` | completed |
| TASK-GAMEPLAY-MLF-007 | PRD-GAME-004-05 | viewer-owner | D4-D5 | 选中强调与 2D marker 可见性/尺寸增强 | `selection_emphasis.rs` / `camera_controls.rs` / `scene_helpers_entities.rs` + 回归测试 | `test_tier_required` | completed |
| TASK-GAMEPLAY-MLF-008 | PRD-GAME-004-03/04/05 | test-release-owner | D5 | 手动实操迭代截图（每任务后）并给出视觉评估结论 | `output/playwright/playability/manual-*` 证据集 + 结论摘要 | `test_tier_required` | completed |

## 执行顺序与依赖
- 串行主链：`MLF-001 -> MLF-002 -> MLF-003 -> MLF-004 -> MLF-005 -> MLF-006 -> MLF-007 -> MLF-008`。
- 依赖约束：
  - `MLF-002` 依赖 `MLF-001` 的事件 schema 稳定。
  - `MLF-003` 依赖 `MLF-002` 的映射字段与摘要口径。
  - `MLF-004` 依赖 `MLF-001/002/003` 全部合入后执行。
  - `MLF-006` 依赖 `MLF-005` 的反馈显著条落位，避免重复显示冲突。
  - `MLF-007` 与 `MLF-006` 可并行编码，但验收截图需在两者完成后统一复核。
  - `MLF-008` 依赖 `MLF-005/006/007` 全部完成。

## 任务级完成标准（Definition of Done）

### TASK-GAMEPLAY-MLF-001
- `ActionAccepted` 在 runtime 事件流可持久化与可回放。
- 对旧 schema 兼容，不破坏既有事件解析。
- 关键测试：`env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::gameplay_protocol:: -- --nocapture`。

### TASK-GAMEPLAY-MLF-002
- gameplay 关键 DomainEvent 映射为结构化可读字段，不再仅显示笼统 `RuntimeEvent`。
- ACK 与 DomainEvent 均可被 viewer 协议消费；缺失字段时有降级文案。
- 关键测试：`env -u RUSTC_WRAPPER cargo test -p oasis7 viewer::runtime_live::tests:: -- --nocapture`。

### TASK-GAMEPLAY-MLF-003
- 右侧面板可显示 war/governance/crisis/contract 倒计时与最近动作状态。
- 无进展提示包含“原因 + 可执行建议”并支持 `zh/en`。
- 关键测试：`env -u RUSTC_WRAPPER cargo test -p oasis7_viewer -- --nocapture`。

### TASK-GAMEPLAY-MLF-004
- required/full 测试通过且产物可追溯。
- playability 卡片评分达标：题 15/16/17 平均 `>= 4.0`；有效控制命中率 `>= 90%`。
- 关键测试：
  - `./scripts/ci-tests.sh required`
  - `./scripts/ci-tests.sh full`
  - Web 闭环用例按 `testing-manual.md` 执行并归档到 `doc/playability_test_result/`。

### TASK-GAMEPLAY-MLF-005
- Mission HUD 顶部新增“控制结果显著条”，可区分 `executing/completed_advanced/completed_no_progress/blocked`。
- 在不丢失恢复动作（recover/retry）的前提下，减少重复反馈块。
- 关键测试：`env -u RUSTC_WRAPPER cargo test -p oasis7_viewer player_mission_tests:: -- --nocapture`。

### TASK-GAMEPLAY-MLF-006
- 玩家模式首次进入默认 Mission 布局；模块开关改为折叠展开。
- 修复模块可见性持久化“缺字段默认全开”问题，回落到运行时默认值。
- 关键测试：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer right_panel_module_visibility::tests:: -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer egui_right_panel_tests:: -- --nocapture`。

### TASK-GAMEPLAY-MLF-007
- 选中强调 halo 与 2D marker 在玩家常用视角下可稳定辨识。
- 可读性增强不触发渲染退化门禁。
- 检查清单：`doc/game/gameplay/gameplay-micro-loop-readable-world-checklist-2026-03-10.md`。
- 关键测试：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer camera_controls_tests:: -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer selection_emphasis::tests:: -- --nocapture`。

### TASK-GAMEPLAY-MLF-008
- 每个实现任务完成后执行一轮手动实操并截图，输出视觉评估结论。
- 产物至少包含：baseline + 每轮改动后对照图 + 简要结论。
- 关键验证：截图人工审阅通过并归档。
- 统一证据模板：`doc/game/gameplay/gameplay-micro-loop-visual-closure-evidence-template-2026-03-10.md`。
- 2026-03-10 ROUND-009 迭代记录：
  - 证据目录：`output/playwright/playability/manual-20260310-round009/`
  - 证据包：`doc/game/gameplay/gameplay-micro-loop-visual-closure-evidence-2026-03-10-round009.md`
  - 关键截图：
    - `mlf007-baseline-3d-mid.png`（3D 中景未选中 baseline）
    - `mlf007-selected-3d-mid.png`（3D 中景选中态）
    - `mlf007-selected-3d-near.png`（3D 近景选中态）
    - `mlf007-selected-2d-marker.png`（2D marker 选中态）
  - 语义状态：`selectedKind=agent`、`connectionStatus=connected`、`errorCount=0`；`console.errors.log` 为空。
  - 本轮结论：`qa_engineer` 已复核通过并回写 `doc/playability_test_result/card_2026_03_10_23_27_43.md`；`MLF-008` 完成。
- 2026-03-07 ROUND-008 迭代记录：
  - 证据目录：`output/playwright/playability/manual-20260307-round008/`
  - 关键截图：
    - `d7-runtime-throttle-initial.png`（默认停驻）
    - `d8-runtime-throttle-playing.png`（播放态）
    - `d9-runtime-throttle-paused.png`（暂停态）
  - 手动量化（`window.__AW_TEST__.getState().tick` 按 1s 窗口采样）：
    - 修复前：播放稳态约 `+11~12 tick/s`，首秒可达 `+17 tick/s`。
    - 修复后：播放首秒 `+2 tick`，后续稳态 `+1 tick/s`；暂停 `+0 tick/s`。
  - 本轮实现：`runtime_live` 播放循环增加间隔节流，默认 `play_step_interval=800ms`，并新增间隔门控回归测试。

### TASK-GAMEPLAY-MLF-004 执行记录（2026-03-06）
- CI 问题修复：`./scripts/ci-tests.sh full` 首次失败于 `m4_builtin_modules.identity.json missing hash token`，通过 `./scripts/sync-m4-builtin-wasm-artifacts.sh` 对齐 m4 hash/identity 清单后复跑通过。
- 门禁结果：
  - `./scripts/ci-tests.sh required`：PASS
  - `./scripts/ci-tests.sh full`：PASS
- Web 闭环与卡片证据：
  - A/B 闭环产物：`output/playwright/playability/20260306-124312/ab_metrics.md`
  - 有效控制命中率：`3/3 (100.0%)`
  - 卡片：`doc/playability_test_result/card_2026_03_06_12_43_31.md`
  - 卡片题 15/16/17：`4/4/4`（平均 `4.0`）

## 风险与应对
- 风险: runtime builtin identity 校验失败会阻塞 required tier。
- 应对: 在 `MLF-004` 前先执行最小定向测试；若失败为仓库基线问题，记录阻塞并按团队既定流程处理，不回滚本任务改动（本轮已通过同步 m4 builtin identity 清单解除阻塞）。
- 风险: 事件噪声过大影响可读性。
- 应对: viewer 仅突出微循环关键事件，其余降级为信息提示。

## 依赖
- `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md`
- `doc/game/gameplay/gameplay-top-level-design.prd.md`
- `doc/game/gameplay/gameplay-war-politics-mvp-baseline.md`
- `doc/playability_test_result/playability_test_card.md`
- `testing-manual.md`

## 状态
- 更新日期: 2026-03-10
- 当前状态: completed
- 当前完成: `TASK-GAMEPLAY-MLF-000`、`TASK-GAMEPLAY-MLF-001`、`TASK-GAMEPLAY-MLF-002`、`TASK-GAMEPLAY-MLF-003`、`TASK-GAMEPLAY-MLF-004`、`TASK-GAMEPLAY-MLF-005`、`TASK-GAMEPLAY-MLF-006`、`TASK-GAMEPLAY-MLF-007`、`TASK-GAMEPLAY-MLF-008`
- 当前证据骨架: `doc/game/gameplay/gameplay-micro-loop-visual-closure-evidence-template-2026-03-10.md`
- 下一任务: `已完成`（后续仅需按 evidence linkage 回填到 `playability_test_result` / `testing` / `core`）
- 阻塞项: 无
- 说明: 过程记录在 `doc/devlog/README.md`、`doc/devlog/README.md` 与 `doc/devlog/README.md`

审计轮次: 4
