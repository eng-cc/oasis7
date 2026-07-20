# `game/gameplay` 热点子域入口

更新时间: 2026-07-06

## 从这里开始
- 想快速理解核心玩法骨架：先读 `gameplay-top-level-design.prd.md`
- 想确认当前冲刺窗口、10 分钟留存修复和下一步体验目标：先读 `gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
- 想确认间接控制为什么仍然应该让玩家感觉自己在控制：先读 `gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
- 想确认成熟世界里小玩家/新玩家靠什么继续形成独立价值：先读 `../../product/world-rules-core-gameplay/mature-world-progression.prd.md`，再读 `gameplay-top-level-design.prd.md` 的专业合同
- 想确认 1cm 物理世界、动作粒度和表现夸张边界：先读 `../../product/world-rules-core-gameplay/prd.md` 的产品承诺，再读 `gameplay-top-level-design.prd.md` 的玩法合同
- 想确认访问模式、limited preview 或 release readiness：先读 `../../product/player-entry-distribution/access-modes-and-release-readiness.prd.md`；当前 preview 执行状态看 `../project.md` 与对应 round execution record
- 想确认 agent claim token cost、claim bond、upkeep、reclaim、restricted grant、starter OC 或 first chat gate：先读 `gameplay-agent-claim-token-cost-2026-03-27.prd.md`，再按需读 `gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md`
- 想精确找某份 gameplay 专题文档，而不是按问题阅读：回到 `../prd.index.md`

## 入口分工
- 当前页只承担 `gameplay/` 子目录 landing page 职责，不复制完整长表。
- `../README.md` 是 game 模块级 landing page，负责在模块 PRD、执行台账、文件级索引和少量高频 gameplay 专题之间分流。
- `../prd.md` 是 game 模块目标态与阶段口径真值。
- `../project.md` 是 game 模块执行入口，适合确认 retention、preview、经济规则与放行门禁当前推进状态。
- `../prd.index.md` 是完整文件级索引，适合已知主题后按文件名查找。

## 密度快照
- 当前 inventory 快照（`bash scripts/doc-inventory-report.sh`，2026-07-06）:
  - `doc/game/gameplay/`: 78 份 Markdown
  - `doc/game/`: 83 份 Markdown
- 该子域已经达到热点阈值；本页目标是降低首读扫描成本，并将退役的一次性 handoff 语义收敛到正式 PRD/project/evidence surfaces。

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
  - `../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md`
  - `gameplay-top-level-design.prd.md`
- 适合问题:
  - 进入游戏后的前 10 分钟为什么会掉线
  - micro-loop、反馈可见性和 post-onboarding 阶段如何衔接
  - 哪些问题应该先作为体验修复，而不是系统重写
  - 当前 `TASK-GAME-076` 的 required tier 自动化/诊断/content-volume supplement 已补齐到 `content_volume_pass`；若要判断真实留存或生产 provider 体验，继续看 retention topic project 的 live/provider playtest 边界

### 3. Agency、间接控制与物理尺度
- 首读入口:
  - `gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
  - `../../product/world-rules-core-gameplay/prd.md`（产品承诺）与 `gameplay-top-level-design.prd.md`（玩法合同）
  - `../../product/world-rules-core-gameplay/mature-world-progression.prd.md`
  - `gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.prd.md`
- 适合问题:
  - 间接控制如何保留玩家主因果感
  - 1cm 物理世界和表现层夸张的边界是什么
  - 小玩家在成熟世界里如何继续产生 leverage
  - 可编程区域设施如何作为中后期区域专业化能力落地，而不变成新手期自由建造或任意脚本

### 4. Preview / beta / release gate
- 首读入口:
  - `../../product/player-entry-distribution/access-modes-and-release-readiness.prd.md`（访问模式、受控 preview 与发行就绪产品承诺）
- 适合问题:
  - limited preview 可以放什么、不能承诺什么
  - closed beta candidate gate 如何判断
  - 当前 release gate 与候选级放行边界在哪里
- 历史 closure / production provenance:
  - release gap、production closure 与 runtime governance closure 旧专题只作为历史证据入口保留；当前首读先走 `gameplay-top-level-design.prd.md`、`gameplay-top-level-design.project.md`、`doc/game/prd.index.md#历史-closure--provenance-入口` 与 GitHub task issue evidence comments。

### 5. Economy、claim 与运营规则
- 首读入口:
  - `gameplay-agent-claim-token-cost-2026-03-27.prd.md`
  - `gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md`
  - `../../product/world-infrastructure/world-continuity-governance-and-recovery.prd.md`
- 适合问题:
  - agent claim 的 token cost、bond、upkeep、reclaim 如何组织
  - restricted grant 如何发放、撤销、过期和 incident 处理
  - starter OC / first chat gate 如何与 restricted starter claim balance 区分
  - economy / longrun hardening 何时需要 QA 或 LiveOps 参与

## 定向检索边界
- 如果你已经知道准确文件名，直接回 `../prd.index.md`。
- runbook、evidence、checklist 和必要的历史执行补充材料继续保留可检索性，但默认不作为首读入口；一次性 role handoff brief 在对应 project / evidence / GitHub task issue evidence comments 已能追溯后应退役删除。旧 MLF008 viewer-to-QA handoff、limited-preview 与 closed-beta role handoff briefs 已退役删除；TASK-GAME-018 当前追溯通过 micro-loop topic project / evidence surfaces / core review logs / GitHub task issue evidence comments，TASK-GAME-029~032 当前追溯通过 closed beta 专题 project、runtime / viewer / QA / liveops evidence surfaces 与 GitHub task issue evidence comments，TASK-GAME-036/037 当前追溯通过 limited preview 专题 project、round-1 execution record 与 QA gate evidence。
- 如果问题需要玩法正确性、平衡、release 放行或对外口径结论，本页只提供文档入口；结论必须回到 `gameplay_designer`、`qa_engineer` 或 `liveops_community` 对应任务证据。
- 历史专题不因出现在本页附近而重新成为当前真值；当前判断以模块 PRD/project、活跃专题、正式 evidence surfaces 与 GitHub task issue evidence comments 为准。

## 维护约定
- 新增 gameplay 专题后，若改变默认首读路径，应同步更新本页。
- 玩法行为、进度、经济规则、preview/beta gate 或对外承诺变化时，必须同步评估是否更新 `../prd.md`、`../project.md` 与相关高频专题。
- 本页只维护簇级入口，不维护完整文件清单。
