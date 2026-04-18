# 启动器区块链浏览器主链级信息架构重构设计（2026-04-18）

- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-mainnet-grade-rebuild-2026-04-18.prd.md`
- 对应项目管理文档: `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-mainnet-grade-rebuild-2026-04-18.project.md`

## 1. 设计定位
把当前 explorer 从“debug log + 多段滚动区”重构为主链浏览器式的 operator surface: 顶部命令区、链健康主卡、tab 导航、左侧主列表、右侧详情检查板，以及明确的空态/错误态。

## 2. 视觉方向
- 方向: industrial / utilitarian operator console，而不是营销式彩色看板。
- 记忆点: 第一屏就能完成“链健康判断”，第二屏内完成“列表扫描 + 详情确认”。
- 约束: 保持 `egui` 同源、无额外协议改造、窄屏可退化但不退回日志式长文本。

## 3. 信息架构
- 顶部工具层:
  - `Refresh Current View`
  - `Reset Current Filters`
  - request inflight / blockchain ready 状态
  - quick shortcuts
- 概览层:
  - height trio: `latest / committed / network`
  - identity: `node_id / world_id`
  - hash rail: `last_block / last_exec`
  - status mix: `accepted / pending / confirmed / failed / timeout`
- 主内容层:
  - `Blocks/Txs/Contracts/Mempool`: 双区布局，左侧列表、右侧详情
  - `Search`: 顶部搜索框 + 结果列表 + 命中说明
  - `Address/Assets`: summary cards + secondary list
- 状态层:
  - `loading`
  - `empty`
  - `not_ready`
  - `error_code + error`

## 4. 渲染拆分
- `explorer_window.rs`: 保留数据结构、请求调度、结果应用。
- `explorer_window_view.rs`: 负责 overview、tabs、Blocks/Txs/Search 与通用卡片/徽标/helper。
- `explorer_window_p1.rs`: 负责 Address/Contracts/Assets/Mempool 的新布局与详情板。
- 如 helper 持续增长，可继续抽 `explorer_window_components.rs`，优先控制单文件长度。

## 5. 交互约束
- 列表点击后详情板即时更新；若需要详情请求，则保留选中语义并显式展示加载/空态。
- 所有 tab 的分页、过滤、清空动作保留现有请求参数语义。
- 任何失败都必须在当前 tab 表面给出解释，不要求用户依赖外层日志。
