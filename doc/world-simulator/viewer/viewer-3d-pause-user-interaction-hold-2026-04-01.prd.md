# Viewer 3D 可视化暂停与用户交互分支暂存（2026-04-01）

- 对应设计文档: `doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: 当前 Viewer 线同时承载 3D scene/camera/rendering 打磨与正式交互闭环，资源被持续摊薄；继续并行推进会直接挤占 `software_safe`、runtime interaction、launcher 与 formal gameplay 所需的主链路收口。
- Proposed Solution: 自 2026-04-01 起暂停所有新的 3D 可视化相关工作，把当前用户交互分支冻结为“暂存态”参考，不再作为 active delivery 分支；活跃实现路径统一收口到非 3D 交互、`software_safe`、launcher 与 runtime/playability 闭环。
- Success Criteria:
  - SC-1: 所有新的 3D scene/camera/render/material/light/post-process 需求都被标记为 `paused`，不再进入 active delivery。
  - SC-2: 当前用户交互分支被定义为 `hold`，仅作为未来恢复时的参考上下文，不再继续承接新实现。
  - SC-3: `world-simulator` 主 PRD / project / index 与本专题文档在 2026-04-01 同步回写，后续协作不再混淆“taxonomy 仍存在”和“工作仍在推进”。
  - SC-4: 当前 Viewer 主路径明确为非 3D / `software_safe` 优先，允许的维护修改只服务 formal gameplay、QA 闭环或现有链路不腐烂。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要冻结当前优先级，避免资源继续漂移到 3D 美术与可视化细节。
  - `viewer_engineer`: 需要明确哪些改动仍可继续，哪些必须停下并挂入恢复池。
  - `qa_engineer`: 需要知道后续应该验证哪条主链路，以及 3D 证据不再代表当前交付目标。
  - `liveops_community`: 需要避免对外继续形成“3D 正在持续冲刺”的错误口径。
- User Scenarios & Frequency:
  - 新 Viewer 需求分流：每次新增或变更需求时执行一次。
  - 阶段 review / 版本评审：每个版本候选至少 1 次，确认 3D 仍处于暂停态。
  - 恢复评审：仅当制作人显式要求恢复 3D workstream 时触发。
- User Stories:
  - As a `producer_system_designer`, I want 3D visualization work paused, so that scarce delivery capacity stays on gameplay closure instead of visual polish.
  - As a `viewer_engineer`, I want the current user-interaction branch treated as a hold branch, so that I do not accidentally keep stacking new work onto a paused stream.
  - As a `qa_engineer`, I want clear allowed vs paused scopes, so that required regression stays aligned with current product claims.
- Critical User Flows:
  1. Flow-V3P-001（需求分流）:
     `收到新的 Viewer 需求 -> 判断是否属于 3D visualization -> 若是则登记 paused，不进入 active delivery -> 若否则继续走非 3D 主链路`
  2. Flow-V3P-002（允许修改判定）:
     `收到现有 3D 相关文件修改请求 -> 判断是否只为保持 build / doc / software_safe 主链路不腐烂 -> 若是则允许最小维护 -> 若否则拒绝并保留到恢复池`
  3. Flow-V3P-003（恢复门禁）:
     `制作人要求恢复 3D -> 复核当前阶段目标、交互主链路稳定性、QA 资源与明确恢复范围 -> 通过后才能把 hold 分支重新转为 active`
  4. Flow-V3P-004（对外口径）:
     `需要解释当前 Viewer 方向 -> 先说明 3D taxonomy 仍存在但研发暂停 -> 再声明当前正式交互主路径为非 3D / software_safe`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 3D workstream registry | `workstream_id`、`pause_status`、`paused_since`、`resume_gate`、`paused_topics` | 3D 新需求统一登记为 `paused` | `active -> paused -> hold -> resumed` | 先按是否属于 3D scene/camera/render/material/light/post-process 分类 | `producer_system_designer` 冻结 |
| 用户交互分支状态 | `branch_role`、`branch_state`、`scope_note`、`resume_note` | 当前用户交互分支只保留为暂存参考，不再承接新增任务 | `active_delivery -> hold_reference -> resumed_delivery` | 若未通过恢复门禁，不得从 `hold_reference` 直接接续实现 | `producer_system_designer` 决定恢复；`viewer_engineer` 执行 |
| 允许修改范围 | `change_type`、`justification`、`touches_3d_files`、`affects_mainline` | 仅允许 build/doc/governance/compat 或为 `software_safe` 主链路避腐烂的最小维护 | `proposed -> allowed/rejected -> executed` | 若主要目的仍是推进 3D 体验，则直接判为 rejected | owner + implementation role 联审 |
| 恢复门禁 | `stage_ready`、`mainline_stable`、`qa_capacity`、`explicit_goal` | 满足全部条件后才允许恢复 3D active delivery | `blocked -> review_ready -> resumed` | 缺一不可；恢复必须明确到专题而不是“全部 3D 自动解冻” | `producer_system_designer` 终审 |
- Acceptance Criteria:
  - AC-1: 本专题明确把 3D scene/camera/render/material/light/post-process 相关工作定义为暂停态。
  - AC-2: 当前用户交互分支被记录为 `hold`，只作为暂存参考，不再作为 active delivery 分支。
  - AC-3: `world-simulator` 主 PRD / project / index 已同步挂载本专题。
  - AC-4: 文档中明确列出“允许的最小维护范围”和“禁止继续推进的新 3D 需求”边界。
  - AC-5: 恢复 3D workstream 需要显式恢复门禁，不允许通过默认 backlog 或临时需求隐式恢复。
  - AC-6: 当前正式交互主路径被明确收口到非 3D / `software_safe` / launcher / runtime interaction。
- Non-Goals:
  - 不删除现有 3D 代码、脚本或历史文档。
  - 不把 `standard_3d` 从玩家访问模式 taxonomy 中移除。
  - 不在本专题中直接重排 runtime / launcher / OpenClaw 的其它优先级细节。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - `./scripts/doc-governance-check.sh`
  - `doc/world-simulator/prd.md`
  - `doc/world-simulator/project.md`
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- Evaluation Strategy:
  - 检查主文档是否已把 3D workstream 标成 `paused`。
  - 检查当前主路径是否仍然明确指向非 3D / `software_safe`。
  - 检查是否存在“把 hold 分支误写成 active branch”或“把 taxonomy 当成交付优先级”的漂移表述。

## 4. Technical Specifications
- Architecture Overview:
  - 本专题只定义 Viewer 工作流治理，不重写 3D 实现。
  - `software_safe`、launcher、runtime interaction 继续作为当前活跃主链路。
  - 3D 相关专题继续保留在文档树中，但状态统一视为“暂停推进、等待恢复门禁”。
- Integration Points:
  - `doc/world-simulator/prd.md`
  - `doc/world-simulator/project.md`
  - `doc/world-simulator/prd.index.md`
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
  - `doc/core/player-access-mode-contract-2026-03-19.prd.md`
- Edge Cases & Error Handling:
  - 若 3D 相关文件因编译、依赖、治理或主链路耦合问题必须修改，只允许做“最小不腐烂维护”，不得借机继续追加 3D feature。
  - 若某个 3D bug 直接阻断 `software_safe`、launcher 或 runtime 主链路，允许做 blocker fix，但必须在提交说明中写明不是恢复 3D workstream。
  - 若用户显式要求恢复 3D，可新开恢复任务，但必须先通过恢复门禁，不得直接在 hold 分支上继续堆叠。
- Non-Functional Requirements:
  - NFR-1: 自 2026-04-01 起，100% 新 Viewer 需求都必须先经过“是否属于 3D paused scope”判断。
  - NFR-2: 所有活跃文档在 1 个工作日内完成暂停状态回写，避免口径漂移。
  - NFR-3: 恢复门禁必须可追溯到正式文档，不允许只停留在口头或聊天上下文。
- Security & Privacy:
  - 本专题不新增权限或数据面要求。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 冻结 3D workstream、定义 hold branch 语义、同步模块主文档。
  - v1.1: 在后续版本 review 中持续复核 3D 暂停态是否被错误冲破。
  - v2.0: 若未来恢复 3D，只恢复被明确点名的专题，不自动恢复全部 3D 方向。
- Technical Risks:
  - 风险-1: 若暂停边界写得不清楚，团队会继续把小改动当作 3D workstream 续做。
  - 风险-2: 若直接删除上下文而不是 hold，后续恢复会丢失设计依据。
  - 风险-3: 若把 taxonomy 和 delivery priority 混在一起，会错误地得出“standard_3d 已废弃”的结论。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-041 | TASK-WORLD_SIMULATOR-285 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `rg -n "暂停|暂存|恢复门禁|software_safe 优先" doc/world-simulator/prd.md doc/world-simulator/project.md doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.{prd,design,project}.md` + `git diff --check` | Viewer 工作流治理、3D 暂停边界、未来恢复可审计性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-V3P-001` | 暂停新的 3D 可视化推进，优先把资源投到非 3D 正式交互主链路 | 继续并行推进 3D 视觉专题 | 当前阶段最关键的是 formal gameplay 与可验证交互闭环。 |
| `DEC-V3P-002` | 把当前用户交互分支定义为 `hold` 参考 | 直接删除或继续堆叠新任务 | 删除会丢失恢复上下文，继续堆叠会让暂停策略失效。 |
| `DEC-V3P-003` | 恢复 3D 必须经过显式恢复门禁 | 允许通过零散任务隐式恢复 | 没有正式 gate，暂停状态无法被稳定执行。 |
