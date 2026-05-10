# Gameplay 物理尺度与间接控制对齐（2026-05-07） PRD v0.1

- 对应设计文档: `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.project.md`

审计轮次: 1

## 1. Executive Summary

- Problem Statement: 当前 oasis7 已经把 `1cm` 落成底层世界坐标与持久化合同，但产品层还缺少一份正式方案来解释“物理世界有尺度”与“玩家默认并不做 Minecraft 式逐块直接操作”之间的边界。缺少这份边界会导致设计、实现、Viewer 表达和 QA 口径混写，把“厘米是真值”误读成“玩家应当默认拥有具身/方块编辑玩法”。
- Proposed Solution: 新增 `PRD-GAME-013`，把 gameplay 的尺度语义拆成四层合同：`canonical physical scale`、`subsystem native resolution`、`player interaction scale`、`presentation scale`。同时明确当前正式主路线继续保持“间接控制的文明模拟”，把具身/方块编辑只保留为未来候选能力门，而不是当前产品承诺。
- Success Criteria:
  - SC-1: `game` 根 PRD、`gameplay` 主文档与本专题统一采用“四层尺度合同”口径，不再把 `1cm` 与“默认玩家可逐块编辑世界”混写。
  - SC-2: 当前正式玩家动作面被明确冻结为间接控制语义：`agent/location/facility/recipe/governance` 等动作继续是主路径；不把 block placement / digging / terraforming 写成当前正式玩法承诺。
  - SC-3: runtime / viewer / agent / QA 各自拥有明确 follow-up 任务，能够验证“厘米真值、粗粒度子系统、表现层夸张、玩家动作边界”四件事是否一致。
  - SC-4: 文档正式要求任何 coarse-grained 子系统都声明自己的 native resolution 和与厘米真值的映射，不再只靠实现代码暗含。
  - SC-5: Viewer 正式口径明确区分“物理真值距离/尺寸”和“为了可读性进行的视觉夸张”，避免把 marker 放大误读成世界真尺寸。
  - SC-6: 未来若要引入 `player_parity` / embodied / block-editing 之类具身能力，必须满足独立 gate，不得绕过当前 `internal_playable_alpha_late` 的间接控制主路线。

## 2. User Experience & Functionality

- User Personas:
  - `producer_system_designer`: 需要把“1cm 到底意味着什么、不意味着什么”冻结成正式玩法边界。
  - `runtime_engineer`: 需要明确哪些数据结构必须保持厘米真值，哪些系统允许保留更粗 native resolution，但必须显式声明。
  - `viewer_engineer`: 需要知道哪些视觉放大/LOD/marker 是可接受的表达层夸张，哪些会误导玩家。
  - `agent_engineer`: 需要明确当前动作语义仍是间接控制，不要提前把具身动作 contract 包装成正式可玩入口。
  - `qa_engineer`: 需要一份尺度一致性矩阵，判断“代码是厘米”“UI 看起来像米/块”“动作还是地点级”的组合是否符合产品真值。
  - 高期望玩家 / 对标 Minecraft 的外部观察者: 需要知道 oasis7 的物理世界不是抽象无尺度表，而是有物理尺寸的文明模拟；同时当前玩法不是第一人称逐块建造。
- User Scenarios & Frequency:
  - 玩法边界评审：每次空间、移动、工业、Viewer 表达或 action schema 变化时至少 1 次。
  - Viewer 表达验收：每个影响距离、半径、marker、地图缩放的改动至少 1 次。
  - runtime / action contract 评审：每次新增动作、观测字段或子系统分辨率时至少 1 次。
  - 对外口径审查：每次需要回答“像不像 Minecraft / 有没有物理尺度 / 玩家能不能直接建造”时重复使用。
- User Stories:
  - PRD-GAME-013: As a 制作人与玩法 owner, I want the gameplay docs to distinguish physical-world scale from player interaction scale, so that oasis7 keeps a real metric world without accidentally promising Minecraft-style direct block play before the product is ready.
- Critical User Flows:
  1. Flow-PSIC-001: `producer 复盘现有 1cm 证据 -> 冻结四层尺度合同 -> 回挂根入口与 gameplay 主文档 -> 后续 owner 按统一口径实施`
  2. Flow-PSIC-002: `runtime 新增或修改空间/碎片/移动子系统 -> 显式声明 native resolution -> 给出与厘米真值的映射 -> QA 对照文档验收`
  3. Flow-PSIC-003: `viewer 调整 marker/zoom/LOD/距离表达 -> 区分物理真值与视觉夸张 -> 玩家能看懂“这是读图放大，不是世界单位变化”`
  4. Flow-PSIC-004: `讨论未来 embodied / block-editing 方向 -> 先通过本专题 gate 判断是否仍服务间接控制主路线 -> 若不满足则保持 deferred`
- Functional Specification Matrix:

| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| canonical physical scale | `space_unit_cm`、`GeoPos{x_cm,y_cm,z_cm}`、`distance_cm`、`radius_cm`、`size_cm` | 不新增玩家按钮；用于定义世界真值 | `draft -> frozen -> audited` | 一切物理位置与持久化坐标以整数厘米为唯一真值 | `runtime_engineer` 实现；`producer_system_designer` 冻结；`qa_engineer` 复核 |
| subsystem native resolution | `subsystem_id`、`native_resolution_kind`、`native_resolution_value`、`cm_mapping_rule`、`rounding_rule` | 子系统 owner 在 PRD / design / code 中显式声明 | `implicit -> declared -> verified` | coarse-grained 子系统允许保留 km/voxel/location 级 native resolution，但必须给出到 cm 的映射 | 对应子系统 owner 负责；`producer_system_designer` 审核边界 |
| player interaction scale | `interaction_surface`、`action_schema_version`、`current_action_granularity`、`deferred_embodied_capabilities` | 当前正式动作继续围绕 `move_agent / inspect / interact / harvest / mine / build_factory / recipe / governance`；不新增 block-edit 按钮 | `observer_only -> indirect_control_playable -> embodied_candidate` | 先保证间接控制的文明模拟成立，再评估具身能力是否能强化主循环 | `producer_system_designer` 拍板；`runtime_engineer` / `agent_engineer` 实现 |
| presentation scale | `physical_distance_label`、`visual_exaggeration_reason`、`visual_scale_floor_m`、`zoom_tier` | UI 可显示真实距离、近似量级和视觉放大说明 | `opaque -> readable -> trustworthy` | 玩家先看到对决策有用的真实距离/量级；视觉夸张只服务可读性，不改物理真值 | `viewer_engineer` owner；`qa_engineer` 验收 |
| future embodied gate | `embodied_lane_status`、`player_parity_status`、`supports_block_editing`、`supports_collision_fidelity`、`supports_local_physics_feedback` | 不在当前主入口暴露；仅作为 future candidate checklist | `deferred -> candidate -> approved_for_prototype` | 只有当具身能力能强化间接控制主路线、且不稀释当前 retention blocker 时，才允许进入原型 | `producer_system_designer` 最终拍板 |

- Acceptance Criteria:
  - AC-1: 本专题明确写出四层尺度合同：`canonical physical scale`、`subsystem native resolution`、`player interaction scale`、`presentation scale`。
  - AC-2: `PRD-GAME-013` 明确声明当前正式主路线仍是“间接控制的文明模拟”，而不是第一人称逐块建造游戏。
  - AC-3: 文档显式允许 runtime 子系统采用比 `1cm` 更粗的 native resolution，但要求声明映射规则、四舍五入/截断规则和 QA 验证方式。
  - AC-4: 文档显式禁止把 Viewer 的 marker 放大、LOD floor 或 2D/semantic 地图抽象误写成世界物理真值本身。
  - AC-5: 至少拆出 `producer_system_designer`、`runtime_engineer`、`viewer_engineer`、`agent_engineer`、`qa_engineer` 五类后续任务，并给出 test tier 与建议验收命令。
  - AC-6: `game` 根 PRD / project、`gameplay-top-level-design` 主文档与 `prd.index` 必须能路由到本专题。
  - AC-7: 本专题必须给出“现在不做什么”：不把 Minecraft 式 block editing、实时具身 3D、碰撞/跳跃/攻击动作集写成当前正式承诺。
  - AC-8: 本专题必须给出“未来什么时候才可以做”：只有当间接控制 trust/capability 主路径稳定、且具身能力能增强而不是稀释主循环时，才允许进入候选原型。
  - AC-9: QA 后续矩阵必须能同时检查 4 件事：厘米真值是否保持、子系统 coarse resolution 是否声明、Viewer 是否误导、动作面是否仍符合间接控制边界。
- Non-Goals:
  - 不把 oasis7 当前产品方向改成 Minecraft 式第一人称逐块建造。
  - 不在本专题里恢复或扩大 3D workstream 的 active delivery 承诺。
  - 不在本专题里新增 runtime 物理碰撞、跳跃、攻击、装备、方块放置/挖掘的正式实现需求。
  - 不要求所有子系统都真的按 `1cm` 精度模拟；本专题要求的是声明与对齐，而不是统一到同一数值步长。

## 3. AI System Requirements (If Applicable)

- Tool Requirements:
  - 文档审计与代码真值比对工具（`rg`、`doc-governance`、定向测试）。
  - active-LLM / pure API / viewer 主路径证据，用于评估“尺度表达是否帮助理解，而不是制造错觉”。
- Evaluation Strategy:
  - 使用“合同一致性”而不是“美术效果”作为核心评估口径：同一个 feature 必须同时回答 `物理真值是什么`、`玩家动作粒度是什么`、`表现层有没有夸张`、`QA 怎么验证`。

## 4. Technical Specifications

- Architecture Overview:
  - game 模块负责冻结产品层尺度合同。
  - runtime / simulator 继续持有厘米真值、粗粒度子系统分辨率与动作 schema。
  - viewer 负责把物理真值与视觉夸张分开表达。
  - agent / provider 相关 contract 只能在不打破间接控制主路线的前提下演进。
- Integration Points:
  - `doc/game/prd.md`
  - `doc/game/project.md`
  - `doc/game/prd.index.md`
  - `doc/game/gameplay/gameplay-top-level-design.prd.md`
  - `doc/game/gameplay/gameplay-top-level-design.project.md`
  - `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
  - `doc/world-simulator/scenario/world-initialization.prd.md`
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
  - `doc/world-simulator/llm/llm-provider-agent-dual-mode-2026-03-16.prd.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 某个 runtime 子系统继续使用 km / voxel / location 级 native resolution，但没有文档声明：判定为 contract incomplete，不得宣称“1cm 设计已贯穿始终”。
  - Viewer 为可读性放大 marker，但没有任何真实距离/尺寸锚点：判定为 presentation misleading。
  - 文档开始把 `1cm` 当成“玩家应可逐块编辑世界”的依据：判定为 scope drift，回退到 deferred。
  - agent / provider 文档先声明 `jump/attack/use_item/...`，但 runtime 当前正式动作集并不支持：必须明确标记为 candidate contract，不得写成现行正式语义。
  - 某个 future 具身提案虽然技术上可做，但会抢占当前 `trust gate / first capability gate` 主路径资源：默认 deferred。
- Non-Functional Requirements:
  - NFR-PSIC-1: `PRD-GAME-013` 的根入口互链必须在 1 个工作日内完成并可检索。
  - NFR-PSIC-2: 任一涉及空间、距离、大小、动作粒度的 active 文档，不得同时出现“厘米是真值”和“默认 block-edit 主玩法”两种互相冲突口径。
  - NFR-PSIC-3: 任何新的 coarse-grained 子系统在进入 active delivery 前，100% 具备 native resolution 与厘米映射说明。
  - NFR-PSIC-4: QA 的尺度一致性回归必须可在 fresh bundle 本地复跑，并输出 pass/block 结论。
  - NFR-PSIC-5: 当前阶段公开口径继续保持 `limited playable technical preview`；不得因为补齐尺度方案就扩大为“开放世界建造游戏已可玩”。
- Security & Privacy:
  - 本专题不引入新的隐私采集；只约束语义合同与验证口径。

## 5. Risks & Roadmap

- Phased Rollout:
  - R0: 冻结 `PRD-GAME-013`，完成根入口、主文档与执行任务挂载。
  - R1: runtime 声明并校验现有 coarse-grained 子系统的 native resolution。
  - R2: viewer 收口距离/尺寸/marker 语义，显式区分物理真值与表现夸张。
  - R3: QA 建立尺度一致性矩阵并给出 pass/block。
  - R4: 只有在 `PRD-GAME-012` 主路径稳定后，才允许重新评估 embodied / block-editing 原型资格。
- Technical Risks:
  - 风险-1: 若只强调 `1cm`，不强调动作面边界，会持续制造“为什么不能像 Minecraft 一样直接玩”的预期落差。
  - 风险-2: 若只强调“不是 Minecraft”，不强调真实物理尺度，又会把世界退化成抽象表格游戏印象。
  - 风险-3: 若 Viewer 继续大量使用视觉夸张但不解释，会放大“实现不一致”的错觉，削弱玩家信任。

## 6. Validation & Decision Record

- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-GAME-013 | `gameplay-physical-scale-contract-freeze` | `test_tier_required` | 文档治理检查、根入口/专题入口/执行日志互链核验 | `game` 模块尺度边界与任务冻结 |
| PRD-GAME-013 | `runtime-native-resolution-declaration` | `test_tier_required` | runtime / simulator 尺度字段与 native resolution 声明核对、定向单测与 contract grep | 厘米真值、粗粒度子系统映射、动作语义边界 |
| PRD-GAME-013 | `viewer-scale-surface-truth-labeling` | `test_tier_required` | Viewer 距离/尺寸/marker 表达人工复核 + UI/semantic regression | 物理真值与表现层夸张的一致性 |
| PRD-GAME-013 | `agent-action-contract-boundary-alignment` | `test_tier_required` | dual-mode / action schema 文档对账，确认当前正式动作面与 deferred embodied 能力边界 | provider / agent 文档口径与当前动作 contract 一致性 |
| PRD-GAME-013 | `qa-scale-consistency-matrix` | `test_tier_required` + `test_tier_full` | QA 尺度一致性矩阵、fresh bundle 主入口复核与 blocker 归档 | 跨 runtime/viewer/agent 的最终尺度对齐结论 |

- Decision Log:

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PSIC-001 | 把尺度语义拆成四层合同，而不是只写“最小单位 1cm” | 继续只保留底层坐标声明 | 当前问题不在有没有厘米，而在产品层没有解释厘米和玩法粒度的关系。 |
| DEC-PSIC-002 | 继续坚持“间接控制文明模拟”为正式主路线 | 直接把产品方向转成具身/逐块编辑游戏 | 当前 `PRD-GAME-012` 的 trust/capability blocker 还没清空，切产品主路线只会稀释主问题。 |
| DEC-PSIC-003 | 允许 coarse-grained 子系统保留自己的 native resolution，但必须声明映射 | 强制所有子系统都按 `1cm` 等步长模拟 | 实现层已有 km voxel、location/facility 级抽象；真正缺的是可审计声明，而不是盲目统一数值精度。 |
| DEC-PSIC-004 | 允许 Viewer 做视觉夸张，但必须区分物理真值与展示层 | 要么完全禁止夸张，要么继续不解释 | 完全禁止会损失可读性；不解释会损失信任。 |
| DEC-PSIC-005 | embodied / block-editing 只保留为 future gate | 先在文档里按理想动作面承诺 `jump/attack/use_item/block_place` | 当前 runtime 正式动作集和 3D 主路径都不支撑这类承诺，先写进去会制造伪能力。 |
