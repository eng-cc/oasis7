# oasis7 Simulator：Agent Frag 初始站位优化（设计文档）设计

- 对应需求文档: `doc/world-simulator/scenario/agent-frag-initial-spawn-position.prd.md`
- 对应项目管理文档: `doc/world-simulator/scenario/agent-frag-initial-spawn-position.project.md`

## 1. 设计定位
定义 Agent Frag 初始站位优化设计，统一新生 Agent 在碎片场景中的出生位置选择、避让规则与可重复性。

## 2. 设计结构
- 出生采样层：根据场景与碎片布局生成候选站位。
- 避让约束层：避免与障碍、边界或已有实体发生冲突。
- 首局理由层：把推荐 frag 的材质预期、距离优势和第一工业目标关联转成玩家可读提示。
- 落位确认层：在运行时初始化阶段确定最终出生点。
- 回归验证层：校验不同 seed 与碎片布局下的站位稳定性。

## 3. 关键接口 / 入口
- 初始站位生成入口
- 碎片/边界约束读取
- starter frag reason: `target_frag_id`、`expected_material_hint`、`starter_value_reason`、`distance_or_accessibility_reason`、`first_recipe_relevance`
- Agent 初始化落位
- 站位回归用例

## 4. 约束与边界
- 同一 seed 下站位应可复现。
- 出生点选择不得破坏既有场景边界。
- 推荐 frag 的玩家提示不得承诺精确掉落，只能给粗粒度材质预期和第一工业目标关联。
- 不在本专题扩展完整导航系统。

## 5. 设计演进计划
- 先固化站位选择规则。
- 再补避让与边界守卫。
- 最后沉淀场景回归。
