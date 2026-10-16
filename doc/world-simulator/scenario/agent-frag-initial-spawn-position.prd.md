# oasis7 Simulator：Agent Frag 初始站位优化（设计文档）

- 对应设计文档: `doc/world-simulator/scenario/agent-frag-initial-spawn-position.design.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history。

审计轮次: 5

## 1. Executive Summary
- Agent 初始位置优先生成在 `frag` 附近，缩短开局采矿路径。
- Agent 初始坐标在数据初始化阶段直接生成于 `frag` 上方，并与 `frag` 表面保持约 50m 间距。
- 在 2D 视角下减少 Agent 与 frag 的重叠遮挡，提高开局可见性。
- 首局推荐该 frag 时，玩家应看到“为什么采这个 frag”的材质预期和第一工业目标关联，而不是只看到最近目标。
- 保持确定性：同 seed、同场景输入，生成同样的出生地点与坐标。

## 2. User Experience & Functionality

### In Scope
- 调整 `build_world_model` 出生候选地点策略：当世界中存在 `frag-*` 地点时，优先从 `frag` 集合选取出生地点。
- 在初始化阶段为出生在 frag 的 Agent 生成偏移坐标（严格正上方 + 约 50m 表面间距）。
- 边界保护：当理想坐标越界时，采用确定性降级策略，在空间边界内尽量满足“上方 + 间距”目标。
- Viewer 渲染阶段保留初始化 standoff，不将 Agent 强制贴回 frag 表面。
- 玩家侧首局提示应能读取 starter frag reason：`target_frag_id`、`expected_material_hint`、`starter_value_reason`、`distance_or_accessibility_reason`、`first_recipe_relevance`。
- 补充初始化测试，覆盖 frag 优先出生和坐标偏移语义。

### Out of Scope
- 运行时移动/采矿规则重构。
- 非 frag 场景下的出生策略大改。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications
- 不新增外部配置字段，复用现有 `WorldInitConfig` 与 `LocationProfile.radius_cm`。
- 内部规则：
  - frag 判定：`location_id` 以 `frag-` 前缀开头。
  - frag 出生目标距离：`center_distance = radius_cm + 5_000cm`（约 50m 表面间距）。
  - 方向策略：优先严格正上方（`x/y` 与 frag 中心一致，仅抬升 `z`），确保 2D 顶视角不被 frag 遮挡。
  - 越界降级：在不破坏确定性的前提下递减间距，仍不可用则回退到 frag 中心。
- 玩家提示合同：
  - `target_frag_id`: 被推荐的首个采集 frag。
  - `expected_material_hint`: 基于资源分布策略生成的粗粒度材质预期。
  - `starter_value_reason`: 为什么这个 frag 适合第一步。
  - `distance_or_accessibility_reason`: 为什么现在采它比移动去别处更适合。
  - `first_recipe_relevance`: 该 frag 可能支持的第一工业目标或首个配方方向。
- Edge case: 若首局推荐采集目标但缺少材质预期或第一工业目标关联，标记为 `starter_frag_hint_missing`；该问题属于玩家侧引导可读性，不改变出生或采矿规则。

<a id="fsp-selection-contract"></a>
### 既有专业条款身份：确定性出生选择

Trace ID: O7-REQ-4d98352e-f146-4672-b159-b99545708c78

仅标识 §1、§2、§4 已采用的初始化选择义务：有 `frag-` 候选时优先该集合，同 seed、同场景输入保持确定性；无 frag 的既有策略不在本专题大改。此行的工程范围是 `build_world_model` 候选选择，不授予玩家 Agent 或资源，也不决定正式首 Agent 的 seed/chain 位置身份。精确承接见[选择设计](agent-frag-initial-spawn-position.design.md#fsp-selection-design)。

<a id="fsp-standoff-contract"></a>
### 既有专业条款身份：初始化 standoff 与边界

Trace ID: O7-REQ-3bef4fa6-57cd-41f6-baa3-74272ea8adcc

仅标识 §2/§4 原初始化位置约束与降级：严格正上方、约 50m 表面间距、确定性递减与最终中心回退。此工程身份不包含 Viewer 消费保留或玩家可辨认结果。原数值、坐标轴、空间边界和配置不变；不是新增导航、碰撞求解或正式世界 fallback。精确承接见[站位设计](agent-frag-initial-spawn-position.design.md#fsp-standoff-design)。

<a id="fsp-viewer-preservation-contract"></a>
### Viewer 消费保留的独立产品关系

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；§2 原“Viewer 渲染阶段保留初始化 standoff，不将 Agent 强制贴回 frag 表面”保持不变，独立作为[舞台相关对象可读](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-001)的有限消费者关系，由[Viewer 保留设计](agent-frag-initial-spawn-position.design.md#fsp-viewer-preservation-design)承接，不纳入上面的纯坐标工程身份或其产品 N/A。该关系不反推所有 surface 固定坐标，不保证任意相机无遮挡，也不代签完整 stage001/AC 或改变其原状态。

<a id="fsp-starter-reason-contract"></a>
### 既有条件提示合同的来源

Trace refs: O7-REQ-f6aebb85-7a07-4e0b-a3cf-f23655f293a2；唯一声明位于[产品 FS-7 原条件子义务](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#fs7-starter-resource-reason)。本专题 §2/§4 和 DEC-FSP-001 仅承接推荐首个 frag 时的粗粒度材质预期、当前可达性理由与第一工业目标关联，不承诺产出，也不把采矿增加为每个首局 profile 的前置。五个既有字段和 `starter_frag_hint_missing` 边界保持原义；[理由设计](agent-frag-initial-spawn-position.design.md#fsp-reason-design)不代签整个 FS-7 或完整首局体验。

## 5. Risks & Roadmap
- FSP1：设计文档落地。
- FSP2：初始化逻辑改造（frag 优先 + 站位偏移）。
- FSP3：测试回归、文档收口与开发日志更新。

### Technical Risks
- Agent `location_id` 与几何坐标不完全重合时，观测距离可能与“地点归属”语义出现偏差。
- 极端边界（超大半径 frag 靠近边缘）下，50m 目标可能需要降级。
- 出生点从 region 转为 frag 可能影响既有场景的开局行为节奏，需要通过测试稳定约束。
- 若只优化出生距离而不解释材质预期，开局采集会从“选择第一份原料路线”退化为“点击最近碎片”。

## 6. Validation & Decision Record
- 追溯: GitHub task issue evidence 与 Git history 保留原实施过程；本文与 design 保持现行约束语义。
- DEC-FSP-001: frag 优先出生是路径缩短与可见性优化，不等于自动完成玩家理解；首局推荐 frag 必须解释材质预期和第一工业目标关联。
