# Gameplay 间接控制 control-feeling 合同（2026-05-14） PRD v0.1

- 对应设计文档: `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.project.md`

审计轮次: 1

## 1. Executive Summary

- Problem Statement: oasis7 当前正式主路线已经明确是“通过 agent 间接控制文明”，但还缺少一份正式合同来回答：玩家在不直接操纵每一步执行的前提下，什么时候仍然会觉得“这是我在控制”，而不是“我只是在旁观 AI 自己决定”。缺少这份合同会让 retention、Viewer 表达、action contract 与 playability verdict 继续各说各话。
- Proposed Solution: 新增 `PRD-GAME-014`，把间接控制的 control-feeling 正式拆成一组可验证的交互保证：`intent legibility before/at commit`、`execution acknowledgement with causality`、`interrupt/reprioritize/recover hooks`、`bounded consequence readability`。同时明确这些保证怎样映射到 headed Web/UI、pure API、runtime canonical 语义与 QA gate。
- Success Criteria:
  - SC-1: `game` 根 PRD、`gameplay` 主文档与本专题对“间接控制下何谓玩家仍然在控制”采用统一口径，不再只停留在笼统的“有 goal/progress/blocker/next_step”描述。
  - SC-2: 至少冻结 3 条可验证的 interaction guarantees，并明确每条 guarantee 的字段、状态、失败签名与 owner role。
  - SC-3: headed Web/UI 与 pure API 都能直接回答 4 个问题：`我刚刚让系统做什么`、`系统是否接受了`、`为什么当前这样推进/没推进`、`我现在最有效的下一步是什么`。
  - SC-4: future UX / runtime / agent contract 变更可以被 QA 按本专题直接判定为 `strengthens / preserves / weakens control-feeling`，而不是只看 smoke 是否还能跑。
  - SC-5: 本专题不改变 `PRD-GAME-012` 当前正式 verdict；它负责定义 trust/capability 修复所依赖的“控制感合同”，而不是替代 active-LLM 留存验证本身。

## 2. User Experience & Functionality

- User Personas:
  - 新玩家 / 试玩玩家：需要在不直接操控每一步的前提下，仍然感觉“我的决定真的改变了世界”。
  - 回流玩家：需要快速恢复上一次意图、当前执行状态、阻塞原因与最短下一步，而不是重新猜测 agent 在做什么。
  - `producer_system_designer`: 需要把“间接控制是否仍然像控制”冻结成正式可裁决合同。
  - `viewer_engineer`: 需要知道哪些首屏与反馈语义是正式能力地板，而不是可有可无的 polish。
  - `runtime_engineer` / `agent_engineer`: 需要知道哪些 canonical 状态、ack、override、blocked reason 与 resume hook 是产品合同，而不是实现细节。
  - `qa_engineer`: 需要一个比“能点按钮、有日志”更强的 playability 验收基准。
- User Scenarios & Frequency:
  - 首次正式会话：每位新玩家首次进入 `PostOnboarding` 后都会经历。
  - active-LLM 10 分钟 trust gate 回归：每个候选版本至少 3 条正式样本。
  - `PostOnboarding -> first capability gate` 跟踪：每个候选版本至少 1 组 `30` 分钟或 `1~3` 次会话样本。
  - headed Web/UI 或 pure API 行为面变更：每个影响 action/ack/goal/blocker/next_step 的改动至少 1 次合同复核。
- User Stories:
  - PRD-GAME-014: As a 玩家, I want indirect agent gameplay to preserve a concrete sense of control, so that I feel I am directing outcomes rather than merely observing an autonomous simulation.
  - PRD-GAME-014-A: As a 玩家, I want the system to show what intent it accepted and why it changed course or got blocked, so that I can still trust my decisions.
  - PRD-GAME-014-B: As a 回流玩家, I want clear resume and reprioritization hooks, so that I can recover agency without rereading raw logs.
  - PRD-GAME-014-C: As a QA / gameplay owner, I want future UX and runtime changes to be evaluated against an explicit control-feeling contract, so that “still playable” does not silently drift into “technically alive but emotionally passive”.
- Critical User Flows:
  1. Flow-CFC-001: `玩家给出目标/动作 -> 系统在 commit 前展示意图边界或提交后立即回写 accepted intent -> 玩家确认“系统正在执行我刚让它做的事”`
  2. Flow-CFC-002: `执行中出现等待/阻塞/改道 -> 系统把当前状态归类为 executing / blocked / overridden / completed_no_progress 之一 -> 玩家读懂主因果`
  3. Flow-CFC-003: `玩家对当前方向不满意 -> 使用 interrupt / reprioritize / focus goal / change target -> 系统在同一语义面上反馈新旧意图交接`
  4. Flow-CFC-004: `玩家中断会话后返回 -> 系统恢复最近 accepted intent、当前主阻塞、最近可见后果与下一步建议 -> 玩家无需翻 raw history 即可继续`
  5. Flow-CFC-005: `QA 评审某项 UX/runtime/agent 变更 -> 对照 guarantees 判断该改动是增强、保持还是削弱 control-feeling -> 输出 blocker 或 pass`
- Functional Specification Matrix:

| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 意图可见性保证 | `accepted_intent_id`、`intent_summary`、`intent_scope`、`intent_target`、`intent_age_ms` | 玩家发出 `play/step/select/gameplay_action/prompt_control` 后，界面或 API 必须能回读当前被接受的主意图 | `implicit -> acknowledged -> stale/replaced` | 最近一次仍有效的 accepted intent 优先；若被新意图取代，旧意图必须显式标记 `replaced` | 已认证玩家可见自己的当前主意图；不得暴露他人私有 prompt 细节 |
| 因果反馈保证 | `execution_status`、`status_reason`、`blocker_type`、`override_reason`、`last_world_change` | 玩家查看当前结果时，必须看到 “accepted / executing / blocked / overridden / completed_no_progress / completed_with_progress” 之一 | `acknowledged -> executing -> blocked/overridden/completed_*` | 当前主目标相关的执行状态优先于历史日志噪音；同一时刻只高亮 1 个主因果 | 玩家可读；runtime/canonical snapshot 为真值来源，Viewer/API 不得各算一套 |
| 可打断与重排保证 | `can_interrupt`、`can_reprioritize`、`replacement_intent_summary`、`handoff_result` | 提供 `focus goal`、重新选目标、重新提交 prompt/动作等显式重排入口；不允许只剩“等 AI 自己继续” | `continuing -> interrupt_requested -> reprioritized / resume_required` | 当前主阻塞若持续存在且玩家可采取更高收益动作，应优先暴露 reprioritize hook | 正式玩家可对自己的主意图重排；不允许越权改动其他玩家/组织控制面 |
| 后果可读性保证 | `cost_summary`、`progress_delta`、`world_change_summary`、`next_step_hint` | 每次关键动作后必须给出“付出了什么 / 世界改变了什么 / 下一步最该做什么” | `opaque -> readable -> decision_ready` | 当前主目标相关的成本、产出、阻塞、恢复优先于调试日志与次级事件 | 玩家可读；系统生成，owner 冻结字段语义 |
| 恢复与续玩保证 | `resume_anchor`、`last_accepted_intent`、`primary_blocker`、`resume_next_step` | 玩家重连或回流时，直接恢复当前 agency surface，不要求先读原始事件流 | `fresh_session -> resumed -> replanned` | 优先恢复主目标链最近一次 accepted intent；若旧意图已失效，必须显式写出失效原因 | 已认证玩家可恢复自己的续玩面板/API 字段 |

- Acceptance Criteria:
  - AC-1: 本专题至少冻结 4 条 control-feeling guarantees，其中至少 3 条具备可直接验收的字段、状态与失败签名。
  - AC-2: “间接控制仍然像控制”在本专题中被具体定义为：玩家始终能回答 `我让系统做了什么 / 系统有没有接受 / 为什么现在这样 / 我下一步该做什么`，而不是只看到世界在变化。
  - AC-3: 当前正式主路线不要求玩家获得第一人称逐帧操控；但若 accepted intent、主因果、打断重排或续玩恢复四者缺一，则不得宣称 control-feeling 合格。
  - AC-4: 本专题必须显式承接 `PRD-GAME-012` 第 4 条 lane“间接控制因果与下一步”，并解释它与 `PRD-GAME-004` 微循环反馈可见性、`PRD-GAME-007` PostOnboarding、`PRD-GAME-008` pure API parity 的边界。
  - AC-5: headed Web/UI 与 pure API 都必须具备同等级的 agency surface；API 可以更简洁，但不允许缺少 accepted intent、主因果、阻塞分类或 next-step 语义。
  - AC-6: 若系统只是展示原始日志、世界 tick 或泛化状态，而不能把主意图和当前结果绑定到同一语义面，则判定为 control-feeling 失败。
  - AC-7: 若玩家无法显式中断、改道或重新聚焦主目标，只能被动等待 agent 自行推进，则判定为 agency weakened，即便世界仍在持续运行。
  - AC-8: 本专题必须给出未来 UX/runtime/agent 变更的判据：增强型变更至少强化 1 条 guarantee 且不削弱其余 guarantee；任何削弱 accepted intent、主因果、重排入口或续玩恢复的变更都必须经 `producer_system_designer` 显式裁决。
  - AC-9: 本专题完成后，`game` 根 PRD / project、`gameplay` 主文档、索引与当前 task execution log 必须互链到 `PRD-GAME-014` 与 `TASK-GAME-071~075`。
  - AC-10: 当前阶段口径继续保持 `internal_playable_alpha_late`、`trust gate = hold`、`first capability gate = not_run`；本专题不允许被包装成“issue #160 已解决”或“正式留存已恢复”。
- Non-Goals:
  - 不把 oasis7 改成第一人称直接操作或逐块建造游戏。
  - 不在本专题中直接修复 active-LLM provider、runtime freeze 或 capability gate 本身；这些仍由对应实现任务负责。
  - 不把“更多按钮”误当作 control-feeling 改善；本专题关注的是 agency 合同，而不是动作数量膨胀。
  - 不要求 v1 引入复杂概率预览、未来分支树模拟器或完整 plan diff 可视化。

## 3. AI System Requirements (If Applicable)

- Tool Requirements:
  - active-LLM 正式游玩样本与 `agent-browser` headed Web/UI 证据，用于验证 trust gate 语义面是否满足 control-feeling 合同。
  - pure API 对账脚本与 canonical snapshot 检查，用于验证 accepted intent / status / blocker / next_step 没有只留在 UI 私有组装层。
  - playability 卡片与 QA evidence，用于记录“玩家是否真的感觉在控制”而非仅依赖技术成功率。
- Evaluation Strategy:
  - 先做合同级评估，再做样本级评估。合同级评估检查 guarantees 是否在 surface 上存在且一致；样本级评估再看玩家能否在真实 session 中持续读懂 accepted intent、主因果、重排行为与下一步。
  - 对 active-LLM 正式样本，若出现 `world time 没推进`、`goal regress`、`override without explanation`、`raw log 抢焦点`、`resume only through debug history` 中任一签名，则 control-feeling 至少有 1 条 guarantee 失败。

## 4. Technical Specifications

- Architecture Overview:
  - `PRD-GAME-014` 是 gameplay 层的交互合同，不新增单独玩法系统，而是收束已有 `player_gameplay`、goal chain、action ack、blocker taxonomy 与 feedback surfaces 的共同完成定义。
  - runtime / canonical snapshot 负责提供 accepted intent、execution status、blocker/override reason、world change summary、resume anchor 等真值字段。
  - Viewer 与 pure API 负责把这些字段呈现为玩家可用的 agency surface，而不是让玩家去拼原始日志。
  - QA 负责把这些 guarantees 固化为验证矩阵，并在 trust/capability 样本中独立记录哪一条 guarantee 失效。
- Integration Points:
  - `doc/game/prd.md`
  - `doc/game/project.md`
  - `doc/game/prd.index.md`
  - `doc/game/gameplay/gameplay-top-level-design.prd.md`
  - `doc/game/gameplay/gameplay-top-level-design.project.md`
  - `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md`
  - `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
  - `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
  - `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
  - `crates/oasis7/src/viewer/runtime_live/gameplay_snapshot.rs`
  - `crates/oasis7/src/bin/oasis7_pure_api_client.rs`
  - `crates/oasis7_viewer/software_safe.js`
  - `crates/oasis7_viewer/software_safe_src/main.jsx`
  - `crates/oasis7_viewer/software_safe_src/legacy_core.js`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - accepted intent 已被新动作替换，但表面仍像旧意图在执行：必须显式标记 `replaced` 或 `reprioritized`，否则判定为因果漂移。
  - agent 因世界约束自行改道，但玩家只看到“执行了别的事”而没有 override reason：判定为 agency break，而不是普通日志缺失。
  - world 有变化，但当前主目标没有任何 progress/cost/next-step 解释：不得把“世界活着”视为 control-feeling pass。
  - pure API 能读到 stage/goal，却读不到 accepted intent 或 interruption hook：判定为 parity 不完整，不得宣称 API 也满足 control-feeling 合同。
  - 玩家重连后只能从 raw history 猜测最近状态，而没有恢复锚点：判定为 resume guarantee 失败。
  - 系统提供过多 operator/debug 语义，淹没当前主意图与主因果：判定为 presentation-level control-feeling regression。
  - active-LLM lane 因 provider 问题卡死时，不得用 deterministic `--no-llm` 样本代替本专题正式验收；debug lane 只能帮助定位哪条 guarantee 先失效。
- Non-Functional Requirements:
  - NFR-CFC-1: `PRD-GAME-014` 的 active 入口互链必须在 1 个工作日内完成，并可通过 grep 直接定位到根 PRD / project、主文档、索引与专题三件套。
  - NFR-CFC-2: headed Web/UI 与 pure API 的 control-feeling 关键字段覆盖率必须为 100%：`accepted_intent`、`execution_status`、`primary_reason`、`next_step` 四类字段不得缺任一类。
  - NFR-CFC-3: 任何导致 accepted intent 与当前世界结果脱钩、且没有 override/replaced 解释的回归，都必须被 QA 标记为 blocker，而不是低优先级文案问题。
  - NFR-CFC-4: control-feeling 合同验证必须可在 fresh bundle 本地复跑，并能区分 formal active-LLM lane 与 debug/probe lane。
  - NFR-CFC-5: 本专题下所有 follow-up 结论必须继续遵守当前 claim envelope，不得借“控制感合同已定义”扩大对外承诺。
- Security & Privacy:
  - 本专题不新增玩家隐私采集；accepted intent 与 resume anchor 只要求表达 canonical 玩家状态，不要求暴露私有 prompt 全文或实现内部 trace。

## 5. Risks & Roadmap

- Phased Rollout:
  - R0: 冻结 `PRD-GAME-014`，完成根入口、主文档、索引与任务映射。
  - R1: 对齐 current canonical surface，把 accepted intent / 主因果 / next-step / resume anchor 的正式合同写入 runtime/viewer/API 语义面。
  - R2: 补齐 interrupt / reprioritize / resume hooks 的实现与表面一致性。
  - R3: QA 建立 control-feeling matrix，并将其接入 `PRD-GAME-012` trust/capability 样本复核。
  - R4: 只有在上述 guarantees 稳定后，才允许继续讨论更复杂的 plan preview、branch simulation 或更宽动作面。
- Technical Risks:
  - 风险-1: 如果只补 UI 文案、不补 canonical accepted intent / override / resume truth，合同会再次退化成展示层口号。
  - 风险-2: 如果把 control-feeling 简化为“世界在动、按钮有响应”，会继续把系统原型误判为足够像游戏。
  - 风险-3: 如果为了“更像控制”而盲目扩动作面，反而会打散当前间接控制主路线和 retention 修复优先级。

## 6. Validation & Decision Record

- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-GAME-014 | `TASK-GAME-071` | `test_tier_required` | 文档治理检查、根入口/主文档/索引/执行日志互链核验 | control-feeling 合同冻结与任务挂载 |
| PRD-GAME-014 | `TASK-GAME-072` | `test_tier_required` | canonical snapshot / feedback surface 对账，确认 accepted intent、execution status、override/blocker reason、next-step truth 已定义 | runtime / viewer / player_gameplay 语义一致性 |
| PRD-GAME-014 | `TASK-GAME-073` | `test_tier_required` | formal Web entry 与 pure API agency surface 对账，人工复核主意图、主因果、重排入口、续玩恢复 | headed Web/UI 与 pure API control-feeling surface |
| PRD-GAME-014 | `TASK-GAME-074` | `test_tier_required` | dual-mode / action contract / interruption semantics 对账，确认重排与打断能力不再隐形 | agent contract、override 解释与 reprioritize hook |
| PRD-GAME-014 | `TASK-GAME-075` | `test_tier_required` + `test_tier_full` | QA control-feeling matrix、active-LLM trust samples、pure API parity 抽样与 blocker 签名归档 | 正式 control-feeling 合同、trust/capability 样本解释力 |

- Decision Log:

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-CFC-001 | 将 control-feeling 定义为一组 interaction guarantees，而不是一句“间接控制也要有控制感” | 继续让留存、Viewer、runtime、QA 各自用模糊语言描述 | issue #164 的问题不是缺观点，而是缺一份可裁决、可实现、可测试的正式合同。 |
| DEC-CFC-002 | 把 accepted intent、主因果、打断重排、续玩恢复列为 agency 地板 | 只要求“世界在动”或“有 next_step 提示” | 玩家真正缺的不是更多世界变化，而是缺“这是我造成的、我现在能改什么”的稳定心智模型。 |
| DEC-CFC-003 | 本专题承接 `PRD-GAME-012` 第 4 条 lane，但不替代 trust/capability gate | 直接把 issue #164 写成 retention gate verdict 本身 | control-feeling 是 formal gate 的前置合同，不是 gate 结果本身；混写会再次污染 verdict 语义。 |
| DEC-CFC-004 | 继续坚持间接控制主路线，通过更强的 agency surface 解决“像旁观 AI”的问题 | 直接把产品方向改成更多 direct control 或 embodied 操作 | 当前主问题是间接控制表达不够完整，而不是缺一套完全不同的直接控制产品方向。 |
