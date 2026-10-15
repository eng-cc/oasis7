# Viewer Pixel World Fragment LOD Terrain Rendering 详细设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod.prd.md`
- 历史任务追溯: `task_428db5366f654c5e892ac300807cb9cc` / GitHub issue #986；执行证据见 GitHub task issue evidence comments 与 `.pm/github-project-sync/task-archive.jsonl`。

## Current State
- `pixel_world_host.jsx` already derives world bounds, locations, agents, links, and hotspots from the snapshot.
- `pixel_world_bridge` currently renders grid, location sprites, agent sprites, relationship links, and hotspots.
- `Location` is still visually prominent even though the real visible world should be the fragment terrain under it.

## Design Decision
- Keep `Location` as the logical anchor and selection/link context.
- Derive Fragment terrain in the host from existing snapshot data; do not change runtime snapshot protocol.
- Render Fragment terrain as a background layer with screen-space LOD so Agent-readable zoom stays clean.

## DTO Shape
`fragment_terrain[]` entries contain:
- `id`
- `location_id`
- `pos`
- `footprint_cm`
- `dominant_compound`
- `color`
- `emphasis`

`locations[]` entries gain:
- `marker_role`: `logic_anchor` when fragment terrain exists, otherwise `primary_marker`.
- `marker_alpha`: renderer hint for de-emphasized visual weight.

## Host Algorithm
1. Build locations with normalized position and fragment block access.
2. For each block:
   - find largest `block.compounds.ppm` entry.
   - map the compound to a deterministic terrain color.
   - compute local center from `origin_cm + size_cm / 2`.
   - place it around the location center using `radius_cm` as the local half-extent when available.
   - clamp final position inside world bounds.
3. Attach terrain count and marker role to the owning location.
4. Return `fragment_terrain` with existing render state fields.

## Renderer Algorithm
1. Deserialize optional `fragment_terrain`.
2. Compute screen-space footprint:
   - use the smaller world-to-canvas scale of width/depth.
   - multiply by current camera zoom.
3. Classify:
   - `< 1.5px`: hidden.
   - `< 10px`: background.
   - `>= 10px`: detail.
4. Render fragments below links/agents/hotspots.
5. Do not add fragment hit regions in this slice; selection remains location/agent.

## Fallback DOM
- Rendered DOM draws non-interactive terrain cells behind location and agent buttons.
- Terrain cells are absolutely positioned using world coordinate percentages.
- Location marker remains clickable but visually subdued when `marker_role=logic_anchor`.

## Verification
- RED/GREEN host test:
  - sample snapshot with fragment blocks produces `fragment_terrain`.
  - fragment terrain carries stable compound color.
  - location marker is `logic_anchor`.
  - rendered DOM displays terrain cells behind entities.
- Rust check:
  - wasm bridge accepts new DTO fields.
  - LOD helper thresholds are unit-covered.


## 目标、上游与当前实现边界

Owner：viewer_engineer；共同审读 game_visual_interaction_designer / qa_engineer。目标是让地表提供空间/材料背景，同时保留Agent可读性；不把terrain色块升级为可操作资源或权威业务结果。

上文 Current State、Host Algorithm 和 Fallback DOM 保留原设计slice的历史上下文。当前派生实现位于 `crates/pixel_world_bridge/src/host_state.rs::build_render_state`，`pixel_world_host.jsx` 消费桥接结果；`render_fragment_visuals.rs` 负责实体reconciliation。不得把历史JS派生叙述当成当前source layer。DTO形状、LOD边界与非交互合同不因迁移而取消。

### 2.1 需求承接与分配表

| 上游 requirement / product AC | 具体 obligation 与适用条件 | 本设计条款 | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [多尺度世界浏览](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-003)；Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf | 仅承接Fragment背景随屏幕投影尺寸降级的子义务；输入/兼容依据[配对PRD](viewer-pixel-world-fragment-lod.prd.md#41-fragment-terrain-dto) | [terrain数据与状态](viewer-pixel-world-fragment-lod.design.md#terrain-state) | runtime提供snapshot；Rust host派生DTO；renderer消费 | 不覆盖搜索、观察权限、回到主目标、原生surface及整个产品多尺度验收 |
| [多尺度世界浏览](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-003)；Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf | 以screen-space LOD保留背景/细节层级与Agent可读性，沿用[专业LOD合同](viewer-pixel-world-fragment-lod.prd.md#42-screen-space-lod) | [LOD边界与验证条件](viewer-pixel-world-fragment-lod.design.md#terrain-lod-boundary) | visual owner审读层级；qa_engineer提供像素与真实浏览器证据 | 不实现block picking、采矿/建造、dense terrain索引或体素管线 |

<a id="terrain-state"></a>
## 状态、数据流与兼容边界

snapshot拥有Fragment block与Location事实；host仅派生 `fragment_terrain`、稳定ID/色板和Location逻辑锚点提示，不写回runtime。renderer根据当前DTO与camera/viewport重建或更新显示实体，移除不再出现的terrain，不能保留上一快照的孤立色块。

DTO是可重新生成的显示状态，不是事务日志，也不证明材料交易或world effect。缺少fragment输入时保留既有Location/Agent回退；新增字段由optional/default兼容接收，旧snapshot不得被伪造地形填充。回滚需复核host与renderer配对版本，保持既有mount/update/tick/unmount与选择合同。

<a id="terrain-lod-boundary"></a>
## LOD设计条款、部署与质量边界

沿用 [Renderer Algorithm](#renderer-algorithm) 的 `<1.5px` hidden、`<10px` background、其余detail；屏幕尺寸来自world bounds、canvas与camera zoom，不能改成固定zoom阈值。terrain位于前景Agent/links/hotspots之下，不创建block hit region；detail加强视觉也不授予交互权限。

缺失世界边界时不得继续绘制旧terrain。dense snapshot的sprite churn是容量风险，当前没有密集世界性能承诺；不得用单元阈值测试或fallback DOM截图代替Bevy像素/实际浏览器证据。配对PRD AC-5列出的完整验证组合仍有效，本次没有缩减其历史或后续适用验收边界。

## 11. 验证设计

### 11.1 验证映射表

| 上游 requirement | 本设计条款 | 独立 obligation 与适用条件 | 验证方法 / source / scenario / candidate | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [多尺度世界浏览](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-003) | [terrain数据与状态](viewer-pixel-world-fragment-lod.design.md#terrain-state) | 含fragment blocks的snapshot派生stable ID、compound/color/footprint与logic_anchor；不写回runtime | [测试手册](../../../testing-manual.md)；源码 `crates/pixel_world_bridge/src/host_state_tests.rs`；`rust_host_state_derives_fragment_agent_link_and_commercial_surface`；Rust host DTO派生；固定source、fixture和native Rust target | 实际PR/验证记录：host fixture、source/target、测试输出与退出码，核对terrain数量及各DTO字段断言 | 样例DTO断言不证明生产snapshot、稠密性能、真实渲染或完整交互；稀疏回退/清理变更需另选对应负例 |
| [多尺度世界浏览](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-003) | [LOD边界与验证条件](viewer-pixel-world-fragment-lod.design.md#terrain-lod-boundary) | screen-space footprint决定hidden/background/detail，Agent可读尺度保持terrain低层且非交互 | [测试手册](../../../testing-manual.md)；源码 `crates/pixel_world_bridge/src/lib_tests.rs`；`fragment_lod_uses_screen_space_size`、`fragment_lod_keeps_blocks_background_at_agent_readable_scale`与visual styles断言；LOD helper / renderer分层；固定source、world bounds、canvas尺寸、camera zoom与target；完整AC-5另按配对PRD执行 | 实际PR/验证记录：bounds/canvas/zoom、LOD与style测试输出；完整AC-5另附Bevy render/pixel regression、browser smoke、wasm check和build工件 | helper只证明阈值/层级计算；不代签Bevy像素、浏览器、原生surface及完整AC-5，也不证明block交互 |

## 决策、风险与未决范围

地形作为非交互背景，Location保留逻辑锚点；这避免以材料装饰替代玩家对象与任务。palette是工程映射而非最终美术决定。当前文档整理没有运行host/renderer或浏览器测试，不宣称历史AC-5在新版本通过，也不升级整个产品多尺度功能的实现状态。
