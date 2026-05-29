# Viewer Pixel World Commercial Rendering Loop（2026-05-28）

- 对应设计文档: `doc/world-simulator/viewer/viewer-pixel-world-commercial-rendering-loop-2026-05-28.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-pixel-world-commercial-rendering-loop-2026-05-28.project.md`
- 关联专题:
  - `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.prd.md`
  - `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.prd.md`
  - `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod-2026-05-27.prd.md`

审计轮次: 1

## 1. Executive Summary
- Pixel-world 不能只追求把模拟器数据画全；商业化目标要求首屏直接回答“当前目标、下一步动作、玩家造成了什么变化”。
- 本轮把 pixel-world 主舞台从 renderer/debug 状态面板收口为游戏指挥棋盘：Agent、路线、目标和 player leverage 优先，Fragment terrain 保持世界背景。
- 本轮不扩 runtime snapshot 协议，不新增 3D workstream，不把 Fragment block 变成交互对象。

## 2. User Experience & Functionality

## 目标
- 玩家 5 秒内能读出当前目标、下一步动作和关键阻塞。
- Agent / route / action feedback 的视觉权重大于 Location marker 和 renderer 诊断。
- Fragment terrain 继续作为世界身体存在，但在 Agent 可读视角下保持背景信息。
- renderer 状态、DTO JSON 和调试按钮默认进入诊断折叠区。

## 范围
- 范围内：host render state 的商业化语义层、PixelWorldHost 首屏 HUD、fallback DOM route 表达、定向 UI 测试、生成产物刷新。
- 范围外：runtime snapshot 字段扩展、设施/生产线正式协议、dense terrain 聚合、3D renderer、正式美术资产。

### In Scope
- `buildPixelWorldRenderState` 新增 `commercial_surface`，由现有 `buildGameplaySummary`、agents、links、fragment terrain 派生。
- `commercial_surface` 至少表达 objective、next action、active agent、player leverage、blocker、route count 与 fragment count。
- Pixel-world stage 顶部显示商业化 HUD，优先展示目标、下一步和玩家杠杆。
- fallback DOM 绘制 agent-location route 线，使关系不只存在于 WASM renderer。
- renderer 诊断 badges、控制按钮和 raw DTO 默认折叠。

### Out of Scope
- 不新增 world server / runtime snapshot 字段。
- 不改变 `pixel_world_bridge` 反序列化 schema；新增 host 字段由 wasm 侧忽略。
- 不把 action button 直接搬进像素地图；命令面仍在现有 Command surface。
- 不把 Fragment block 设为 hover/select target。

## 3. User Stories
- As a player, I want the pixel-world stage to show my current objective and next move before renderer diagnostics, so that the screen feels like a game surface instead of an operator console.
- As a producer, I want the stage to separate player leverage from ambient world activity, so that a lively simulation is not mistaken for meaningful play.
- As a viewer engineer, I want commercial HUD semantics to be derived from existing snapshot/gameplay data, so that the first slice ships without widening runtime protocol risk.

## 4. Technical Specifications

## 接口 / 数据
- 输入：现有 `WorldSnapshot.player_gameplay`、`model.agents`、`model.locations`、fragment blocks 与 recent events。
- 输出：Pixel-world host render state 新增 `commercial_surface`；WASM bridge 继续消费原有 `world_bounds / fragment_terrain / locations / agents / links / visual_hotspots / selection`。
- 兼容：没有 gameplay summary 时，HUD 使用 agents/routes/fragments 的静态世界读数，不阻断 renderer mount/update。

### 4.1 Commercial Surface DTO
- `objective`: 当前目标标题与描述。
- `next_action`: 推荐动作标签或 next-step hint。
- `active_agent_id`: 推荐动作、accepted intent、selection 或首个 agent 推导出的当前玩家杠杆对象。
- `player_leverage`: execution state、accepted intent summary、world change/effect detail。
- `blocker`: blocker label/detail。
- `world_read`: agents/routes/fragments/hotspots 的可扫描计数。

### 4.2 UI Layering
- Stage summary: `World Command Board` / `世界指挥棋盘`。
- Commercial HUD: objective、next move、player leverage 三列。
- Canvas foreground: selected entity / goal / blocker callouts。
- Canvas middle: agents, routes, hotspots。
- Canvas background: Fragment terrain and subdued Location anchors。
- Diagnostics: renderer status, runtime source, camera, raw DTO and test controls.

### 4.3 Player Leverage Rule
- `player_leverage.summary` 优先使用 accepted intent / recent world change / execution cause。
- 若没有玩家意图，则明确显示尚无 player-facing accepted intent，不用“世界活跃”冒充玩家进展。
- UI test 必须断言 `commercial_surface.player_leverage` 与目标/下一步同屏可见。

## 5. Risks & Roadmap

## 里程碑
- M1：专题 PRD/design/project 与模块入口回写。
- M2：host DTO、commercial HUD、fallback route 与诊断折叠实现。
- M3：targeted UI test、build、PM/doc/diff hygiene 完成。

## 风险
- 当前数据里设施/生产线语义不足，第一期只能用 gameplay summary 和 agent-location routes 表达商业闭环。
- HUD 如果继续堆 renderer badges，会退回 debug dashboard；本轮通过默认折叠诊断控制。
- 后续若要展示设施、生产、库存流，需要另开 runtime/viewer 协议扩展任务。

## 6. Acceptance Criteria
- AC-1: `buildPixelWorldRenderState` 返回 `commercial_surface`，并可在无 runtime 协议扩展时从现有 snapshot 派生。
- AC-2: PixelWorldHost 首屏展示 objective、next action、player leverage，renderer diagnostics 默认折叠。
- AC-3: fallback DOM 绘制 agent-location route，Fragment terrain 仍在背景层且非交互。
- AC-4: UI tests 覆盖 commercial surface DTO、HUD 可见性和诊断折叠。
- AC-5: `npm --prefix crates/oasis7_viewer run test:ui -- pixel_world_host`、`npm --prefix crates/oasis7_viewer run build:software-safe`、`./scripts/doc-governance-check.sh`、`./scripts/pm/lint.sh` 与 `git diff --check` 通过。

## 7. Validation & Decision Record
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-046 | `task_b399bf37eff94c44a300c55f5db739d3` | `test_tier_required` | `npm --prefix crates/oasis7_viewer run test:ui -- pixel_world_host` + `npm --prefix crates/oasis7_viewer run build:software-safe` + `./scripts/doc-governance-check.sh` + `./scripts/pm/lint.sh` + `git diff --check` | Pixel-world host DTO、商业化 HUD、fallback route、diagnostics 信息架构、checked-in Viewer Web bundle |

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-WS-046-COMM-1 | 第一阶段只扩 host 商业语义层，不扩 runtime snapshot 协议 | 立即新增设施/生产线/资源流协议 | 现有 gameplay summary 已能表达目标、阻塞、下一步与玩家意图；先压实 player leverage 展示，避免横跨 runtime/viewer 的高风险大改。 |
| DEC-WS-046-COMM-2 | renderer diagnostics 默认折叠 | 保留 renderer badges 与调试按钮在 pixel-world 首屏 | 商业化入口应让玩家先读目标、Agent 与世界变化；renderer 状态属于诊断信息。 |
