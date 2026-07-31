# 客户端启动器区块链浏览器设计（当前 authority）

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer.prd.md`
> 历史迁移、验收与 task 状态：GitHub task issue evidence。

## 表现结构

1. **共用承载层**：native 与 Web 继续复用 launcher 的同源 `egui` explorer 表面；Web 控制面只代理已有 `/api/chain/explorer/*` 只读请求。
2. **概览与命令层**：顶部保留刷新、重置当前筛选、当前 tab 与请求/链可用状态；概览以已有 height、identity、hash 与状态计数形成可扫描分组，而不是把日志行当作主界面。
3. **七视图层**：`Blocks`、`Txs`、`Contracts`、`Mempool` 使用已有列表到详情的选择路径；`Search` 显示查询和命中说明；`Address`、`Assets` 以 summary-first 卡片与次级列表呈现既有返回字段。
4. **状态层**：`loading`、`empty`、`not_ready` 与结构化 `error_code + error` 留在当前 tab；筛选、分页、清空与选择严格复用既有请求参数和状态机，不在前端重算链结果。

## 控制与信息边界

- explorer 只显示当前接口可返回的公共账本字段；不得在展示层暴露私钥、凭据或本地敏感配置。
- 该表面只能编排已有查询和结果，不得新增交易、钱包、validator、结算或世界状态控制能力。
- “mainnet-grade”只描述历史信息架构目标；设计不主张 live mainnet、公开可用、network readiness、无重置、全 archive 或最终性。
- 窄宽度可以把双区布局压为纵向，但不能回退为无状态说明的日志式长文本；任何实际视觉/交互改动须由 game_visual_interaction_designer 定义验收并按 `testing-manual.md` S6 取证。

## 代码与协议接点

- launcher：`crates/oasis7_client_launcher/src/explorer_window.rs` 及其拆分的 `explorer_window_*` 视图模块。
- Web 控制面：`/api/chain/explorer/*`，代理 runtime 的现有 `/v1/chain/explorer/*` 查询。
- 本设计记录当前结构，不授权新增 endpoint、字段、轮询频率、索引保留策略或 DOM/UI 改动。
