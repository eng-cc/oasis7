# issue #162 industrial chain legibility closeout（2026-05-04）

审计轮次: 1

## Meta
- 关联 issue: `#162`
- 关联任务: `issue-162-industrial-chain-legibility-closeout`
- Trace: `.pm/tasks/task_4da3948c1c2c457c9529ee661e4af03d.yaml`
- owner: `producer_system_designer`
- 当前结论: `closeout_ready`

## 本轮要解决的问题
- `#162` 要求把工业链状态、停机原因与修复建议收成玩家可读反馈，而不是继续停留在 raw log / debug text。
- 仓库里已经有前期工业引导、`PostOnboarding` 能力链、`software_safe`/pure API shared gameplay contract 等多处实现与证据，但缺一份专门面向 `#162` 的现行 closeout trace。
- 这次 closeout 只回答“工业链状态与停机修复是否已经达到可关闭 `#162` 的程度”，不把 active-LLM trust/capability gate 的未恢复 blocker 混写成同一结论。

## Acceptance criteria mapping

| `#162` 验收点 | 当前仓库真值 | 现行证据 / 来源 | 结论 |
| --- | --- | --- | --- |
| 玩家无需阅读 raw logs，也能判断产线为什么没推进 | 顶层玩法 PRD 已冻结“状态 + 停机原因 + 修复动作”口径；Viewer 主卡与玩家反馈已消费这些语义 | `doc/game/gameplay/gameplay-top-level-design.prd.md`；`crates/oasis7_viewer/src/egui_right_panel_player_guide/post_onboarding.rs`；`crates/oasis7_viewer/src/egui_right_panel_player_experience.rs` | `pass` |
| 至少 1 个 canonical player surface 能视觉验证首个工业里程碑 | 前期工业 required-tier 卡组已把“首个制成品 / 停机恢复 / 首座工厂单元”固定为正式人工复核最小集 | `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md` | `pass` |
| 停机原因必须是人类可用分类，而不是 freeform debug text | `PostOnboarding` 与顶层玩法口径都已冻结最小 blocker taxonomy，并在 Viewer 侧落为玩家文案 | `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`；`doc/game/gameplay/gameplay-top-level-design.prd.md`；`crates/oasis7_viewer/src/egui_right_panel_player_guide/mod.rs` | `pass` |

## 当前 contract

### 1. 工业状态不再只存在于 debug 日志
- `doc/game/gameplay/gameplay-top-level-design.prd.md` 已明确：
  - 反馈必须区分 `已接受 / 执行中 / 已产出 / 停机或阻塞`
  - 停机必须给出最小原因分类
  - 玩家需要知道“产线是否在推进、哪里停机、下一步该修什么”
- `doc/game/gameplay/gameplay-top-level-design.project.md` 的 T4 已记录 runtime / viewer / QA 三侧闭环完成：
  - runtime 负责生产完成、停机、恢复状态与审计事件
  - viewer 负责主界面显式反馈
  - QA 负责 required-tier 卡组

### 2. Viewer 已把 blocker 和 repair hint 提升到玩家主语义
- `crates/oasis7_viewer/src/egui_right_panel_player_guide/post_onboarding.rs` 已根据运行态输出：
  - 阶段进展
  - blocker detail
  - next step
  - branch hint
- `crates/oasis7_viewer/src/egui_right_panel_player_experience.rs` 已把停机 / 恢复反馈收口为玩家文案，而不是直接泄露原始 `RuntimeEvent`。

### 3. required-tier 证据已经覆盖 issue 关心的三个工业里程碑
- `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md` 已把以下场景固定为正式最小集：
  - `首个制成品`
  - `停机恢复`
  - `首座工厂单元`
- 其中 `卡片 B：停机恢复` 直接要求玩家在 UI 中看见：
  - 目标工厂与阻塞原因
  - `blocked` 与 `resumed` 反馈
  - 玩家友好文案，而不是仅有 ActionRejected 或 debug 文本

### 4. API 侧不再另算一套 blocker 语义
- `doc/testing/evidence/pure-api-shared-player-gameplay-parity-2026-04-28.md` 已确认 `software_safe` 与 `pure_api` 共用 canonical `snapshot.player_gameplay` 问题集。
- 这意味着 API 至少在 `当前被什么阻塞 / 下一步该做什么 / 最近一次关键世界变化是什么` 这些玩家问题上，不再偏离 Viewer 语义。
- 本 issue 的 closeout 仍以“至少一个 canonical player surface 可视验证”作为硬验收锚点，不额外声称 pure API 已具备完整工厂 inspector。

## 作用域边界
- 本 closeout 不声称 active-LLM formal lane 的 `10-minute trust gate` 已恢复。
- 本 closeout 不声称 `first capability gate` 已通过；当前正式口径仍是 `trust gate = hold`、`first capability gate = not_run`。
- 本 closeout 只说明：工业链状态、停机分类、恢复提示与首个工业里程碑的玩家可读反馈，已经达到关闭 `#162` 所需的 repo truth。

## Closeout reasoning
- `#162` 的核心不是“工业链是否已经永远不卡”，而是“卡住时玩家能不能知道为什么、能不能看到恢复路径、能不能验证第一个工业里程碑”。
- 当前 repo 已把这些能力分别固化到：
  - 顶层玩法 PRD / project
  - `PostOnboarding` blocker taxonomy
  - Viewer 玩家文案与主卡
  - required-tier 工业卡组
  - shared player gameplay contract
- 因此继续把 `#162` 保持 open，只会把它和 trust gate / capability gate 的正式 lane blocker 混在一起，降低问题边界清晰度。

## 结论
- issue verdict: `closeout_ready`
- 建议 PR 收口方式: trace-only 文档 PR，PR body 显式 `Closes #162`
- 当前非目标: 不把该 issue 的关闭解读为 active-LLM retention 已恢复，也不把它升级成“工业中循环整体完成”的更大 claim
