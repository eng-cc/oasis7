# oasis7：玩家访问模式总契约设计（2026-03-19）

- 对应需求文档: `doc/core/player-access-mode-contract-2026-03-19.prd.md`
- 对应项目管理文档: `doc/core/player-access-mode-contract-2026-03-19.project.md`

审计轮次: 8

## 1. 设计定位
将玩家访问模式正式收口为 `viewer / pure_api` 两项：`viewer` 是唯一正式 Web 入口，`pure_api` 是唯一正式 no-UI 入口。`software_safe` 是当前 Viewer 的兼容渲染/页面入口 alias，不再作为独立玩家访问模式；`player_parity / headless_agent` 继续只是 execution lane；`agent_direct_connect/provider_loopback_http` 继续只是兼容 alias。

## 2. 核心设计决策
- 保留两种玩家访问模式：
  - `viewer`：默认 Web 正式入口；当前软件安全渲染页面可继续以 `software_safe` alias 暴露。
  - `pure_api`：无 UI、自动化、长稳与集成入口。
- 删除旧第二 Viewer 入口的现行模式地位：
  - 不再作为玩家入口。
  - 不再作为 release / QA / playability 的当前 claim 目标。
  - 若历史文档仍提到它，只能按归档理解。
- 继续采用 claim-first：
  - 先绑定 `mode_id`，再输出结论。
  - execution lane 与 provider 只作附加维度。

## 3. 设计结构
### 3.1 Mode Registry
- `viewer`
- `pure_api`

### 3.2 Routing
- formal Web gameplay -> `viewer`
- 无 UI / 自动化 / CLI 长稳 -> `pure_api`
- `non-3D` / `2D 优先` 只表示当前优先级，不表示模式

### 3.3 Evidence
- 所有证据包必须挂一个主 `mode_id`
- `execution_lane` / `agent_provider_*` 只能补充，不得替代

## 4. 关键约束
- `viewer` 不能代签 `pure_api` 结论；`software_safe` 证据必须标注为 `viewer` mode 下的当前渲染 alias。
- `pure_api` 不能代签 Web / headed UI 结论。

## 5. 失败与降级语义
- Web 缺 LLM 或命中 blocker：记为 `viewer blocked`
- 纯接口缺 canonical gameplay 语义：记为 `pure_api observer_only`
- 旧第二 Viewer 入口文案：视为历史残留，需清理
