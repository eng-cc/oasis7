# `game/gameplay` 热点子域入口

更新时间: 2026-06-20

## 从这里开始
- 想快速理解核心玩法骨架：先读 `gameplay-top-level-design.prd.md`
- 想确认当前冲刺窗口、10 分钟留存修复和下一步体验目标：先读 `gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
- 想确认间接控制为什么仍然应该让玩家感觉自己在控制：先读 `gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
- 想确认成熟世界里小玩家/新玩家靠什么继续形成独立价值：先读 `gameplay-small-player-progression-lane-2026-05-17.prd.md`
- 想确认 1cm 物理世界、动作粒度和表现夸张边界：先读 `gameplay-physical-scale-indirect-control-2026-05-07.prd.md`
- 想确认 limited preview、closed beta 或 release readiness：先读 `gameplay-limited-preview-execution-2026-03-22.prd.md` 与 `gameplay-closed-beta-readiness-2026-03-21.prd.md`
- 想确认 agent claim token cost、claim bond、upkeep、reclaim 或 restricted grant：先读 `gameplay-agent-claim-token-cost-2026-03-27.prd.md`，再按需读 `gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md`
- 想精确找某份 gameplay 专题文档，而不是按问题阅读：回到 `../prd.index.md`

## 入口分工
- 当前页只承担 `gameplay/` 子目录 landing page 职责，不复制完整长表。
- `../README.md` 是 game 模块级 landing page，负责在模块 PRD、执行台账、文件级索引和少量高频 gameplay 专题之间分流。
- `../prd.md` 是 game 模块目标态与阶段口径真值。
- `../project.md` 是 game 模块执行入口，适合确认 retention、preview、经济规则与放行门禁当前推进状态。
- `../prd.index.md` 是完整文件级索引，适合已知主题后按文件名查找。

## 密度快照
- 当前 inventory 快照（`bash scripts/doc-inventory-report.sh`，2026-06-20）:
  - `doc/game/gameplay/`: 82 份 Markdown
  - `doc/game/`: 87 份 Markdown
- 该子域已经达到热点阈值；本页目标是降低首读扫描成本，不在本批直接减少文件数。

## 首读主题簇

### 1. 核心玩法骨架
- 首读入口: `gameplay-top-level-design.prd.md`
- 适合问题:
  - 游戏的核心循环、玩家目标和世界互动骨架是什么
  - gameplay 主题之间的关系如何理解
  - 新增玩法专题应该挂到哪个主线下

### 2. 当前体验窗口与留存修复
- 首读入口:
  - `gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
  - `gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md`
  - `gameplay-post-onboarding-stage-2026-03-18.prd.md`
- 适合问题:
  - 进入游戏后的前 10 分钟为什么会掉线
  - micro-loop、反馈可见性和 post-onboarding 阶段如何衔接
  - 哪些问题应该先作为体验修复，而不是系统重写

### 3. Agency、间接控制与物理尺度
- 首读入口:
  - `gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
  - `gameplay-physical-scale-indirect-control-2026-05-07.prd.md`
  - `gameplay-small-player-progression-lane-2026-05-17.prd.md`
- 适合问题:
  - 间接控制如何保留玩家主因果感
  - 1cm 物理世界和表现层夸张的边界是什么
  - 小玩家在成熟世界里如何继续产生 leverage

### 4. Preview / beta / release gate
- 首读入口:
  - `gameplay-limited-preview-execution-2026-03-22.prd.md`
  - `gameplay-closed-beta-readiness-2026-03-21.prd.md`
  - `gameplay-release-gap-closure-2026-02-21.prd.md`
- 适合问题:
  - limited preview 可以放什么、不能承诺什么
  - closed beta candidate gate 如何判断
  - release gap 和 production closure 相关证据在哪里

### 5. Economy、claim 与运营规则
- 首读入口:
  - `gameplay-agent-claim-token-cost-2026-03-27.prd.md`
  - `gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md`
  - `gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
- 适合问题:
  - agent claim 的 token cost、bond、upkeep、reclaim 如何组织
  - restricted grant 如何发放、撤销、过期和 incident 处理
  - economy / longrun hardening 何时需要 QA 或 LiveOps 参与

## 定向检索边界
- 如果你已经知道准确文件名，直接回 `../prd.index.md`。
- runbook、evidence、checklist、handoff 和历史执行补充材料继续保留可检索性，但默认不作为首读入口。
- 如果问题需要玩法正确性、平衡、release 放行或对外口径结论，本页只提供文档入口；结论必须回到 `gameplay_designer`、`qa_engineer` 或 `liveops_community` 对应任务证据。
- 历史专题不因出现在本页附近而重新成为当前真值；当前判断以模块 PRD/project、活跃专题和 `.pm` task trace 为准。

## 维护约定
- 新增 gameplay 专题后，若改变默认首读路径，应同步更新本页。
- 玩法行为、进度、经济规则、preview/beta gate 或对外承诺变化时，必须同步评估是否更新 `../prd.md`、`../project.md` 与相关高频专题。
- 本页只维护簇级入口，不维护完整文件清单。
