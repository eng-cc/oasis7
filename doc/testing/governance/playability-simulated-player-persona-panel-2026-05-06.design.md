# oasis7: 模拟玩家 persona 评审面板（2026-05-06）设计

- 对应需求文档: `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.prd.md`
- 对应项目管理文档: `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.project.md`

审计轮次: 1

## 1. 设计定位
把“agent 扮演多个不同风格玩家”从临时灵感收口成 testing/governance 层的固定面板设计，专门补内部主观体验假设。

## 2. 系统结构
- Orchestrator:
  - `producer_system_designer`
- Gatekeeper:
  - `qa_engineer`
- Persona panel:
  - `new_player_confused`
  - `impatient_action_player`
  - `systems_optimizer`
  - `narrative_curiosity_player`
  - `chaos_tester`
- Role handoff:
  - persona cards 不能直接成为最终 verdict，必须回流到标准角色 review card。

## 3. 数据契约
- Input:
  - `persona review packet`
- Output:
  - `persona card`
- Aggregation:
  - `persona divergence summary`
  - `role review card`

## 4. 调度策略
- 先由 `producer_system_designer` / `qa_engineer` 判断本次改动是否值得开启 persona panel。
- 再按 changed surface 选择最小 persona 子集并行执行。
- 最后把 persona cards 交给命中的标准角色 subagent 收口。

## 5. 约束
- persona panel 不是 `.agents/roles/` 正式角色。
- persona panel 只能产出内部体验假设、分歧和风险线索。
- persona panel 不能直接替代 L4 结构化真人试玩或 L5 外部信号。
