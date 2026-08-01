# `game/gameplay` 热点子域入口

更新时间: 2026-07-06

## 从这里开始
- 想快速理解核心玩法骨架：先在 `../prd.md` 确认活跃基线与路由，再读 `gameplay-top-level-design.prd.md` 的 bounded 专业合同
- 想确认首局、前 10/30 分钟吸引力和持续游玩合同：先读 `../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md` 的产品承诺，再读 `gameplay-top-level-design.prd.md` 的 early-retention 专业合同；当前 verdict 由同候选 GitHub task evidence 与 `../../testing/evidence/` 确认。
- 想确认间接控制为什么仍然应该让玩家感觉自己在控制：先读 `gameplay-indirect-control-agency-contract.prd.md`
- 想确认成熟世界里小玩家/新玩家靠什么继续形成独立价值：先读 `../../product/world-rules-core-gameplay/mature-world-progression.prd.md`，再读 `gameplay-top-level-design.prd.md` 的专业合同
- 想确认 1cm 物理世界、动作粒度和表现夸张边界：先读 `../../product/world-rules-core-gameplay/prd.md` 的产品承诺，再读 `gameplay-top-level-design.prd.md` 的玩法合同
- 想确认访问模式、limited preview 或 release readiness：先读 `../../product/player-entry-distribution/access-modes-and-release-readiness.prd.md`；当前 preview 执行状态由 GitHub task truth 确认。已关闭 Round 1 的发布、渠道 fallback、信号与关闭证据从 Git history 和 GitHub issue `eng-cc/oasis7#48` 追溯
- 想确认 agent claim token cost、claim bond、upkeep、reclaim、restricted grant、starter OC 或 first chat gate：先读 `gameplay-agent-claim-economy-contract.prd.md`，再按需读 `gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md`
- 想精确找某份 gameplay 专题文档，而不是按问题阅读：回到 `../prd.index.md`

## 入口分工
- 当前页只承担 `gameplay/` 子目录 landing page 职责，不复制完整长表。
- `../README.md` 是 game 模块级 landing page，负责在模块 PRD、执行台账、文件级索引和少量高频 gameplay 专题之间分流。
- `../prd.md` 是 game 模块目标态与阶段口径真值。
- `gameplay-top-level-design.prd.md` 只拥有核心玩法骨架与 `PRD-GAME-012` early-retention 的详细合同；它不替代 `../prd.md` 的活跃路由/状态职责，也不覆盖其他 topic 或 `doc/product/` 的声明范围。
- GitHub Project task status 与 issue evidence comments 是 retention、preview、经济规则和放行门禁的执行状态入口。
- `../prd.index.md` 是完整文件级索引，适合已知主题后按文件名查找。

## 文档密度
- 当前 inventory 与热点判定统一以仓库根目录执行的 `./scripts/doc-inventory-report.sh` 为准；本页不维护容易漂移的文件数量快照。
- 该子域是持续治理的热点目录；本页目标是降低首读扫描成本，并将退役的一次性 handoff 语义收敛到正式 PRD/project/evidence surfaces。

## 首读主题簇

### 1. 核心玩法骨架
- 首读入口: 先 `../prd.md`，再 `gameplay-top-level-design.prd.md`
- 适合问题:
  - 游戏的核心循环、玩家目标和世界互动骨架是什么
  - gameplay 主题之间的关系如何理解
  - 新增玩法专题应该挂到哪个主线下

### 2. 当前体验窗口与留存修复
- 首读入口:
  - `../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md`（产品承诺）
  - `gameplay-top-level-design.prd.md`
- 适合问题:
  - 进入游戏后的前 10 分钟为什么会掉线
  - micro-loop、反馈可见性和 post-onboarding 阶段如何衔接
  - 哪些问题应该先作为体验修复，而不是系统重写
  - 当前 `TASK-GAME-076` 的 required tier 自动化/诊断/content-volume supplement 已补齐到 `content_volume_pass`；真实留存或生产 provider 体验只由对应 GitHub task evidence 与 `../../testing/evidence/` 的正式样本判断。

### 3. Agency、间接控制与物理尺度
- 首读入口:
  - `gameplay-indirect-control-agency-contract.prd.md`
  - `../../product/world-rules-core-gameplay/prd.md`（产品承诺）与 `gameplay-top-level-design.prd.md`（玩法合同）
  - `../../product/world-rules-core-gameplay/mature-world-progression.prd.md`
  - `gameplay-regional-infrastructure-micro-depot-contract.prd.md`
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
  - release gap、production closure、runtime governance 与 base-runtime/WASM split 旧专题正文已退役；当前首读走 `gameplay-top-level-design.prd.md`、world-runtime/WASM 专业权威与 GitHub task issue evidence comments。

### 5. Economy、claim 与运营规则
- 首读入口:
  - `gameplay-agent-claim-economy-contract.prd.md`
  - `gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md`
  - `../../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md`
  - `../../product/world-infrastructure/deterministic-world-execution.prd.md`
- 适合问题:
  - agent claim 的 token cost、bond、upkeep、reclaim 如何组织
  - restricted grant 如何发放、撤销、过期和 incident 处理
  - starter OC / first chat gate 如何与 restricted starter claim balance 区分
  - economy / longrun hardening 何时需要 QA 或 LiveOps 参与

## 定向检索边界
- 如果你已经知道准确文件名，直接回 `../prd.index.md`。
- runbook、evidence、checklist 和必要的历史执行补充材料继续保留可检索性，但默认不作为首读入口；一次性 role handoff brief 在对应 evidence 或 GitHub task issue evidence comments 已能追溯后应退役删除。旧 MLF008 viewer-to-QA handoff、limited-preview 与 closed-beta role handoff briefs 已退役删除；历史 TASK-GAME-018、029~032、036/037 从 GitHub task evidence、专业 evidence surfaces、core review logs、Git history 与 GitHub issue `eng-cc/oasis7#48` 追溯。
- 如果问题需要玩法正确性、平衡、release 放行或对外口径结论，本页只提供文档入口；结论必须回到 `gameplay_designer`、`qa_engineer` 或 `liveops_community` 对应任务证据。
- 历史专题不因出现在本页附近而重新成为当前真值；当前判断以模块 PRD/project、活跃专题、正式 evidence surfaces 与 GitHub task issue evidence comments 为准。

## 维护约定
- 新增 gameplay 专题后，若改变默认首读路径，应同步更新本页。
- 玩法行为、进度、经济规则、preview/beta gate 或对外承诺变化时，必须同步评估是否更新 `../prd.md`、相关高频专题、产品 PRD 与对应 GitHub task evidence。
- 本页只维护簇级入口，不维护完整文件清单。
