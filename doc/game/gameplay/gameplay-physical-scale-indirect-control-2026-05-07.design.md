# Gameplay 物理尺度与间接控制对齐（2026-05-07）设计文档

- 对应需求文档: `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.project.md`

审计轮次: 1

## 设计目标
- 把“世界有真实物理尺度”与“当前玩家默认通过间接控制推进文明”同时写成正式合同。
- 给 runtime / viewer / agent / QA 提供同一套尺度语义坐标系，避免各写各的。
- 为未来 embodied / block-editing 候选原型设门，而不是现在就做产品承诺。

## 四层尺度模型

### 1. Canonical Physical Scale
- 世界位置、距离、半径、尺寸的最终真值统一落在厘米整数合同。
- 该层解决的是“世界在存什么、算什么”，不是“玩家默认怎么操作”。

### 2. Subsystem Native Resolution
- 某些系统允许继续用更粗粒度工作，例如 chunk / voxel / location / facility / recipe。
- 但每个子系统都必须显式回答 3 个问题：
  - native resolution 是什么。
  - 如何映射到厘米真值。
  - 何时发生 rounding / truncation / snapping。

### 3. Player Interaction Scale
- 当前正式动作面继续是间接控制：
  - 目标选择
  - 移动到地点
  - 观察与交互
  - 采集、建厂、排产
  - 治理与组织决策
- 该层明确不等于：
  - block placement
  - block digging
  - 第一人称 collision/jump/attack
  - 手搓局部地形编辑

### 4. Presentation Scale
- Viewer 可以为了可读性放大、抽象、聚合，但不能改写物理真值。
- 设计要求是“可读但不欺骗”：
  - 有真实距离/量级锚点。
  - 有视觉夸张原因。
  - 不让玩家把 marker 大小误读成真实几何尺寸。

## 设计原则
- 先真值，再体验：先写清真值层合同，再写玩家会看到什么。
- 先主路线，再候选线：间接控制主路线优先于 embodied 候选线。
- 先声明，再扩展：现有 coarse-grained 实现先声明和校验，再考虑新增精细能力。
- 先解释，再美化：Viewer 的视觉优化不能抢先于语义解释。

## 角色切片
- `producer_system_designer`
  - 冻结四层尺度合同。
  - 裁决“什么属于现在的正式承诺，什么必须 deferred”。
- `runtime_engineer`
  - 列出现有 coarse-grained 子系统与厘米真值映射。
  - 明确动作面仍以间接控制 schema 为主。
- `viewer_engineer`
  - 收口距离/尺寸/marker/zoom 的玩家表达。
  - 标识 presentation exaggeration 与 physical truth 的边界。
- `agent_engineer`
  - 对齐 dual-mode / action contract 文档，避免把 future embodied 能力写成 current action surface。
- `qa_engineer`
  - 建立尺度一致性矩阵，并定义 blocker 签名。

## 实施顺序
1. `TASK-GAME-066`: 冻结专题，回挂根入口与主文档。
2. `TASK-GAME-067`: runtime 盘点现有 coarse-grained 子系统，并补声明/测试。
3. `TASK-GAME-068`: viewer 收口真实距离/视觉夸张表达。
4. `TASK-GAME-069`: agent/provider 文档对齐当前动作 contract 与 future embodied gate。
5. `TASK-GAME-070`: QA 建立矩阵并给出 pass/block。

## Future Gate
- 未来 embodied / block-editing 原型只有在以下条件同时满足时才可进入 candidate：
  - `PRD-GAME-012` 的 trust gate 与 capability gate 主路径不再是当前最高 blocker。
  - 新动作面能强化间接控制主循环，而不是变成第二套产品方向。
  - Viewer / runtime 至少有一条非欺骗性的具身反馈链路，而不是只补 marketing 文案。

## 验证口径
- required:
  - 文档互链与 root/project/task 映射。
  - runtime / viewer / agent contract 对账。
  - QA 矩阵样板与 blocker 定义。
- full:
  - fresh bundle 下主入口语义复核。
  - 对未来 embodied 候选提案的 gate 复盘。

## 风险
- 如果只补设计文档、不补 runtime/viewer/QA 任务，主题会再次退回抽象讨论。
- 如果直接把未来具身动作写成 current contract，会制造“产品已经承诺但实现没跟上”的假口径。
