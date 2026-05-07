# Local Provider 双轨模式 Observation / Action Contract 冻结（2026-03-16）

- 关联 PRD: `PRD-WORLD_SIMULATOR-040`
- 关联项目: `doc/world-simulator/llm/llm-provider-agent-dual-mode-2026-03-16.project.md`
- owner: `agent_engineer`
- 联审: `producer_system_designer`、`runtime_engineer`、`viewer_engineer`、`qa_engineer`
- 执行任务: `TASK-WORLD_SIMULATOR-149`

## 1. 目的
- 冻结 `player_parity` / `headless_agent` 的 observation/action contract，避免后续实现阶段边做边改导致 parity、回放与回归口径漂移。
- 明确哪些信息可暴露给 `player_parity`，哪些只允许在 `headless_agent` 中以结构化形式提供。
- 明确所有模式共享的动作语义、失败语义、schema version 与禁止事项。

## 2. 统一原则
1. 所有模式共享同一 runtime 权威动作校验，不允许模式专属捷径动作。
2. `player_parity` 与 `headless_agent` 的区别只在 observation 表达层，不在动作语义层。
3. `debug_viewer` 只读订阅 runtime 权威输出，不是 Agent 必需输入源。
4. 所有 benchmark / replay / summary 样本必须记录 `mode`、`observation_schema_version`、`action_schema_version`。
5. 若 provider 或 adapter 不识别 schema version，必须返回结构化失败，不得静默降级。

## 3. 模式定义

### 3.1 `player_parity`
- 目标：评估“这像不像玩家在玩”。
- 允许输入：受约束的局部环境信息、附近实体、当前任务提示、当前角色状态、最近少量事件摘要。
- 禁止输入：全图真值、隐藏碰撞拓扑、未来事件、未被正常感知的 AI 内部状态。
- 推荐用途：producer/QA 对照、默认体验评审、parity 证据采样。

### 3.2 `headless_agent`
- 目标：评估“在无 GUI / 无 GPU 条件下是否稳定可跑、可回放、可批量回归”。
- 允许输入：结构化局部状态、平台/碰撞拓扑摘要、附近敌人/机关状态、任务目标、最近事件、最近失败动作摘要。
- 禁止输入：绕过 runtime 规则的直接状态修改能力、未来事件、未声明字段。
- 推荐用途：CI、夜间回归、低配/无图形环境、批量 benchmark。

### 3.3 `debug_viewer`
- 目标：观战、解释、分诊。
- 输入来源：runtime 权威事件、trace、summary、mode metadata。
- 约束：关闭 `debug_viewer` 不影响 Agent 主闭环；开启时也不能反向提供隐藏真值给 Agent。

## 4. Observation Contract（冻结版）

### 4.1 顶层元数据
所有模式都必须携带以下字段：
- `mode`: `player_parity` | `headless_agent`
- `observation_schema_version`: 首期固定为 `oc_dual_obs_v1`
- `action_schema_version`: 首期固定为 `oc_dual_act_v1`
- `fixture_id?`: benchmark / replay / parity 采证时必填
- `world_time`
- `agent_id`
- `agent_profile`
- `seed?`
- `replay_id?`

### 4.2 公共 observation 段
所有模式共享以下逻辑段，但允许粒度不同：
- `self_state`
  - `location_ref`
  - `pose_hint`
  - `velocity_hint`
  - `health_state`
  - `status_flags`
  - `equipped_item?`
- `mission_context`
  - `goal_id`
  - `goal_summary`
  - `priority`
  - `blocked_reason?`
- `nearby_entities`
  - `entity_ref`
  - `kind`
  - `relation`
  - `relative_hint`
  - `interaction_hint?`
- `recent_events`
  - `event_ref`
  - `kind`
  - `summary`
  - `age_ticks`
- `last_action`
  - `action_ref`
  - `result`
  - `failure_reason?`
  - `decision_rewrite?`
- `action_catalog_ref`
  - 引用统一动作白名单版本，而不是每轮展开未冻结的新字段

### 4.3 `player_parity` 观测约束
`player_parity` 允许的增强仅限“人类玩家可感知信息的结构化压缩”，例如：
- 允许把附近平台/实体用 `relative_hint` 表达为“前方可落脚平台”“左上敌人”
- 允许把 UI 可见状态压缩成 `health_state`、`equipped_item`
- 允许把最近显式可见事件压缩成 `recent_events`

`player_parity` 明确禁止：
- `full_map_graph`
- `hidden_collision_mesh`
- `future_spawn_schedule`
- `unrevealed_enemy_ai_state`
- `global_optimal_path`
- 任意未通过玩家正常感知可获得的精确真值

### 4.4 `headless_agent` 观测约束
`headless_agent` 可额外暴露以下结构化辅助信息：
- `local_navigation_graph`
  - 仅限局部可达拓扑，不得直接给全图解
- `hazard_summary`
  - 附近陷阱/伤害源摘要
- `timing_window_hint`
  - 已建模的跳跃/交互窗口类别摘要，禁止泄露未来 deterministic timeline
- `interaction_targets`
  - 附近可交互对象及其约束

`headless_agent` 仍然禁止：
- 全图最优策略
- 未经声明的直接状态写入
- runtime 内部未来计划或未发生事件

## 5. Action Contract（冻结版）

### 5.1 统一动作 schema
- `action_schema_version`: `oc_dual_act_v1`
- 顶层：
  - `action_ref`
  - `kind`
  - `args`
  - `why?`
  - `confidence?`

### 5.2 首期允许动作
首期双轨模式冻结为与 Local Provider `P0` 对齐的低频动作白名单：
- `wait`
- `wait_ticks`
- `move_agent`
- `speak_to_nearby`
- `inspect_target`
- `simple_interact`

这些动作代表当前正式的间接控制 contract；`jump`、`attack`、`use_item`、`block_editing` 等具身/逐块动作若要进入产品面，必须先升级 schema，并通过独立 gameplay gate，而不是沿用本文件直接宣称为现行能力。

### 5.3 统一动作语义
- `wait`: 当前无低风险有效动作时让出一轮
- `wait_ticks`: 在明确等待外部变化时做有界等待
- `move_agent`: 向局部可达目标移动，不保证瞬移或穿模
- `speak_to_nearby`: 低成本短语义沟通，不承担复杂长对话流程
- `inspect_target`: 请求更多与目标相关的上下文，不等于执行目标动作
- `simple_interact`: 对已暴露的轻量可交互对象执行一次标准交互

### 5.4 参数约束
- 所有动作参数必须来自 `action_catalog_ref` 引用的白名单定义
- 所有枚举/目标引用必须使用 observation 中已暴露的 `*_ref`
- 不允许自由拼接未出现的目标 ID、位置 ID 或动作字段

## 6. 失败与恢复 Contract
- 所有模式共享以下失败语义：
  - `unsupported_schema_version`
  - `unsupported_agent_profile`
  - `invalid_action_kind`
  - `invalid_action_args`
  - `rule_denied`
  - `target_not_visible`
  - `target_not_reachable`
  - `provider_timeout`
  - `provider_unreachable`
  - `insufficient_information`
- 恢复原则：
  - schema / profile 不匹配：立即阻断并升级配置问题
  - 可恢复观察不足：优先 `inspect_target` 或有界 `wait_ticks`
  - 非法动作：下一轮避免复现相同行为，并在 `last_action.failure_reason` 中可见
  - 图形链路失败：不应映射为玩法失败；应通过 `fallback_reason` 切到 headless 或 software-safe 调试链路

## 7. Mode Metadata Contract
以下字段必须写入 replay / summary / parity 样本：
- `mode`
- `observation_schema_version`
- `action_schema_version`
- `agent_profile`
- `provider_id`
- `provider_version?`
- `fallback_reason?`
- `environment_class`（示例：`gpu_webgl` / `software_renderer` / `headless_linux`）

## 8. 禁止事项
- 禁止 `player_parity` 与 `headless_agent` 使用不同动作语义但复用同名动作。
- 禁止在 `debug_viewer` 中引入反向控制能力并默认接回 Agent 主链路。
- 禁止未升级 schema version 就偷偷增删 observation/action 字段。
- 禁止把 headless 增强样本混入 player-parity 验收结果。

## 9. 验证要求
- `test_tier_required`
  - contract 文档冻结评审
  - fixture diff：验证 `player_parity` / `headless_agent` 差异仅在观测层
  - schema 元数据校验：所有样本必须携带 `mode/schema version`
- `test_tier_full`
  - 同场景双模式对照采证
  - 偏差分析：成功率、等待时延、失败签名分布

## 10. 后续交接
- 交给 `runtime_engineer`：实现 `mode metadata`、replay/summary 落盘与统一失败语义
- 交给 `viewer_engineer`：将 `debug_viewer` 收口为旁路订阅层，并显式展示 `mode/fallback_reason/environment_class`
- 交给 `qa_engineer`：基于本 contract 设计双模式对照样本与阻断标准
