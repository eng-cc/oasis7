# Gameplay 物理尺度与间接控制对齐（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md`

审计轮次: 1

## 任务拆解

- [x] gameplay-physical-scale-contract-freeze (PRD-GAME-013) [test_tier_required]: `producer_system_designer` 已冻结“四层尺度合同”，完成 `game` 根入口、`gameplay` 主文档、索引与当前 task execution log 挂载，明确当前正式主路线继续是“间接控制的文明模拟”。 Trace: .pm/tasks/task_5dfbbe7c8c0c4557bef2b49612da3081.yaml

## 后续待建任务

| topic slug | owner role | status | 目标 |
| --- | --- | --- | --- |
| `runtime-native-resolution-declaration` | `runtime_engineer` | `done` | 盘点并声明现有 coarse-grained 子系统的 native resolution（如 chunk / voxel / location / facility），补齐到厘米真值的映射与定向测试。 |
| `viewer-scale-surface-truth-labeling` | `viewer_engineer` | `done` | 收口主入口距离/尺寸/marker/zoom 语义，明确哪些是物理真值、哪些是视觉夸张，并补齐主界面/语义地图的玩家可读锚点。 |
| `agent-action-contract-boundary-alignment` | `agent_engineer` | `done` | 对齐 dual-mode / action schema 文档，明确当前正式动作面仍是间接控制，不把 embodied / block-editing 写成现行 contract。 |
| `qa-scale-consistency-matrix` | `qa_engineer` | `done` | 已建立尺度一致性矩阵，验证“厘米真值 / coarse native resolution / 表现层夸张 / 动作边界”四项合同没有漂移。 |

## 已完成切片

- [x] runtime-native-resolution-declaration (PRD-GAME-013) [test_tier_required]: `runtime_engineer` 已在 `crates/oasis7/src/simulator/native_resolution.rs` 新增 runtime 原生分辨率声明表，显式冻结以下 contract，并补齐定向测试。 Trace: .pm/tasks/task_303dedfe38b04036a198c256cc858e29.yaml

### runtime-native-resolution-declaration / 声明表

| subsystem_id | native resolution | cm mapping rule | rounding / snapping | repo truth |
| --- | --- | --- | --- | --- |
| `canonical-physical-space` | `SPACE_UNIT_CM = 1` | 世界位置/半径/尺寸直接以整数厘米存储 | 无额外 rounding；保持整数 cm | `GeoPos` / `space_distance_cm` / `radius_cm` / `CuboidSizeCm` |
| `chunk-grid` | `20km × 20km × 10km` 固定 chunk | `GeoPos` 通过 chunk 常量映射到 `ChunkCoord` | 整数除法 floor；世界上边界 clamp 到最后一个 chunk | `chunk_coord_of` / `chunk_bounds` |
| `asteroid-fragment-voxel` | `AsteroidFragmentConfig.voxel_size_km`（默认 `10km`，最小 `1km`） | chunk 局部 voxel bounds 先转成 cm，再生成碎片中心 | 采样后的中心坐标 round 到最近整数 cm | `generate_fragments` |
| `asteroid-fragment-spacing` | `min_fragment_spacing_cm`（默认 `50_000cm`，最小 `0cm`） | 表面最小间距直接以 cm 真值校验 | 负值 sanitize 为 `0` | `generate_fragments` / `generate_chunk_at` |
| `movement-energy-cost` | `1km` 计费桶 | 真实 `distance_cm` 先转 km 再计算移动成本 | 任意正距离按 km 向上取整 | `movement_cost` |
| `power-transfer-distance` | `1km` 传输桶 | 真实 `distance_cm` 先转 km 再判断损耗/上限 | 任意正距离按 km 向上取整 | `power_transfer_distance_km` |
| `location-site-actions` | `LocationId` 离散站点锚点 | 动作先解析到 `Location.pos` / `radius_cm` 再落到物理世界 | 不支持 sub-location offset；靠 `location_id` 绑定 | `MoveAgent` / `BuildFactory` / `MineCompound` / `ensure_colocated` |
| `fragment-block-geometry` | `1cm` 最小 block edge | block 几何仍以 cm 表示 | 任意 `<1cm` 边长 clamp 到 `1cm` | `CuboidSizeCm::sanitized` |

- [x] viewer-scale-surface-truth-labeling (PRD-GAME-013) [test_tier_required]: `viewer_engineer` 已在 `crates/oasis7_viewer/software_safe_src/{legacy_core.js,main.jsx}` 补齐 formal Web entry 的尺度真值表面，把 `1cm` 真值、world bounds、选中锚点坐标/半径、最近地点距离样本，以及“marker/zoom 只服务可读性、不可误读为真实几何尺寸”的说明挂到 `software_safe` 主入口，并新增前端回归锁定该 contract。 Trace: .pm/tasks/task_103c448874b7494a8312418995889098.yaml

- [x] agent-action-contract-boundary-alignment (PRD-GAME-013) [test_tier_required]: `agent_engineer` 已对齐 `doc/world-simulator/llm/llm-provider-agent-dual-mode-2026-03-16.prd.md` 与 supporting contract，把当前正式动作面明确冻结为 `wait / wait_ticks / move_agent / speak_to_nearby / inspect_target / simple_interact`，并把 `jump / attack / use_item / block_editing` 改回 future embodied candidate，而不是现行正式能力。 Trace: .pm/tasks/task_15890765ee3b4188a1e2766973f392fc.yaml

- [x] qa-scale-consistency-matrix (PRD-GAME-013) [test_tier_required]: `qa_engineer` 已在 `doc/testing/evidence/gameplay-scale-consistency-matrix-2026-05-07.md` 建立正式矩阵，复核 runtime/viewer/agent 三侧的一致性，并记录四层合同的 pass/blocker 签名；当前结论为 `pass`，且明确不把该结论扩写成 `PRD-GAME-012` trust/capability gate 恢复。 Trace: .pm/tasks/task_8205baa6d2fb46388b11c1eed340fdf5.yaml

### viewer-scale-surface-truth-labeling / formal Web entry 语义

| surface | 物理真值 | 表现层夸张说明 | repo truth |
| --- | --- | --- | --- |
| `software_safe / world scale` | 显示 `1cm` canonical unit、`snapshot.config.space` world bounds、选中锚点坐标、地点半径、最近地点真实距离 | 文案明确要求玩家以数值标签为准，不要把屏幕上的 marker 直径读成真实尺寸 | `software_safe_src/legacy_core.js::buildWorldScaleSurface` |
| `software_safe / locations list` | 地点列表直接显示 `radius` 真值标签 | 列表只补真值，不把资源条目或卡片权重包装成空间尺度本身 | `software_safe_src/main.jsx::TargetsPanel` |
| `standard viewer / overview zoom` | 继续以 runtime cm 真值为底层真值 | 文案明确 `overview/detail zoom tiers` 只切换表现语义，不改写世界尺度 | `software_safe_src/legacy_core.js::presentationScale.zoomTruthNote` + `doc/world-simulator/viewer/viewer-overview-map-zoom.prd.md` |

## 任务建议标题（给后续 owner 直接开 task 用）

| topic slug | owner role | 建议标题 |
| --- | --- | --- |
| `gameplay-physical-scale-contract-freeze` | `producer_system_designer` | Freeze physical scale vs indirect control gameplay contract |
| `runtime-native-resolution-declaration` | `runtime_engineer` | Declare subsystem native resolutions against centimeter truth |
| `viewer-scale-surface-truth-labeling` | `viewer_engineer` | Separate physical truth from presentation exaggeration in player surfaces |
| `agent-action-contract-boundary-alignment` | `agent_engineer` | Align current action contract with deferred embodied capabilities |
| `qa-scale-consistency-matrix` | `qa_engineer` | Build gameplay scale consistency matrix and blocker signatures |

## Handoff Matrix

| topic slug | 发起角色 | 接收角色 | 输入 | 期望输出 |
| --- | --- | --- | --- | --- |
| `runtime-native-resolution-declaration` | `producer_system_designer` | `runtime_engineer` | `PRD-GAME-013` 四层合同、现有厘米真值证据、粗粒度子系统清单 | 子系统声明表、映射规则与回归测试 |
| `viewer-scale-surface-truth-labeling` | `producer_system_designer` | `viewer_engineer` | 距离/尺寸真值口径、当前 marker/zoom 夸张问题、3D hold 边界 | 主入口尺度表达规范、可读锚点与 regression |
| `agent-action-contract-boundary-alignment` | `producer_system_designer` | `agent_engineer` | dual-mode 文档、当前正式动作集、future embodied gate | current vs deferred action surface 对账结果 |
| `qa-scale-consistency-matrix` | `producer_system_designer` | `qa_engineer` | runtime / viewer / agent 对账产物 | pass/block 矩阵与 blocker 归档 |

## 验收命令（草案）

- `gameplay-physical-scale-contract-freeze` / 文档冻结与挂载
  - `rg -n "PRD-GAME-013|gameplay-physical-scale-contract-freeze|runtime-native-resolution-declaration|viewer-scale-surface-truth-labeling|agent-action-contract-boundary-alignment|qa-scale-consistency-matrix" doc/game/prd.md doc/game/project.md doc/game/prd.index.md doc/game/gameplay/gameplay-top-level-design.prd.md doc/game/gameplay/gameplay-top-level-design.project.md doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.project.md`
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`
- `runtime-native-resolution-declaration` / runtime 尺度声明
  - `rg -n "native_resolution|SPACE_UNIT_CM|x_cm|y_cm|z_cm|distance_cm|radius_cm|voxel_size_km|min_fragment_spacing_cm" crates/oasis7 crates/oasis7_wasm_sdk`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 init_generated_fragments_use_integer_centimeter_positions -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 kernel_segmented_move_keeps_agent_on_centimeter_grid -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 cuboid_size_is_sanitized_to_minimum_1cm -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 native_resolution_ -- --nocapture`
- `viewer-scale-surface-truth-labeling` / viewer 表达对齐
  - `rg -n "cm_to_unit|world_units_per_meter|visual|marker|radius" crates/oasis7_viewer/src`
  - `node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`
  - `cd crates/oasis7_viewer && npm run build:software-safe`
  - headed Web/UI 或 semantic surface 人工复核：确认真实距离/量级锚点与视觉夸张说明可见
- `agent-action-contract-boundary-alignment` / agent contract 对齐
  - `rg -n "move/jump/attack/interact/use_item|headless_agent|player_parity|debug_viewer" doc/world-simulator/llm doc/world-simulator/viewer crates/oasis7/src/simulator`
  - `git diff --check`
- `qa-scale-consistency-matrix` / QA 矩阵
  - fresh bundle 主入口复核
  - 文档与实现交叉对账
  - 输出尺度一致性矩阵与 blocker 签名归档

## Done Definition

- `gameplay-physical-scale-contract-freeze`
  - [x] 新专题 PRD / design / project 已创建并回挂到 `game` 根入口、索引与 `gameplay` 主文档
  - [x] 已明确当前正式主路线不是 Minecraft 式 block-editing
  - [x] 已拆出 runtime / viewer / agent / QA 的后续任务
- `runtime-native-resolution-declaration`
  - [x] 现有 coarse-grained 子系统均有 native resolution 与厘米映射说明
  - [x] 定向测试能证明厘米真值合同仍成立
- `viewer-scale-surface-truth-labeling`
  - [x] Viewer 主入口能区分物理真值与视觉夸张
  - [x] 玩家能读到真实距离/量级锚点
- `agent-action-contract-boundary-alignment`
  - [x] current action surface 与 deferred embodied capabilities 已分离
  - [x] dual-mode 文档不再把 future embodied 动作写成现行正式能力
- `qa-scale-consistency-matrix`
  - [x] QA 尺度一致性矩阵已建立
  - [x] blocker 签名可稳定复现并回写

## 依赖

- `doc/game/prd.md`
- `doc/game/project.md`
- `doc/game/prd.index.md`
- `doc/game/gameplay/gameplay-top-level-design.prd.md`
- `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
- `doc/world-simulator/scenario/world-initialization.prd.md`
- `doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.prd.md`
- `doc/world-simulator/llm/llm-provider-agent-dual-mode-2026-03-16.prd.md`
- `testing-manual.md`

## 状态

- 更新日期: 2026-05-07
- 当前状态: completed
- 当前 owner: `qa_engineer`
- 下一任务: 无；`PRD-GAME-013` 当前规划切片已全部完成。
- 说明:
  - 本专题不改变当前 `PRD-GAME-012` 的 trust/capability 主优先级，只是补齐其背后的尺度边界，避免继续因为“1cm 是否等于逐块玩法”产生路线误读。
  - 本专题不会重开 3D active delivery，也不会提前承诺 embodied / block-editing 主玩法。
