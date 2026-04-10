# oasis7

> 一个由 LLM 驱动、支持可编程社会结构与去中心化共识的持久多主体文明模拟系统。

oasis7 是一个持久化的多主体 AI 世界模拟框架。在该系统中，自主智能体在资源约束、空间约束与制度演化的规则下持续运行。

本项目探索以下方向：

- 多主体 LLM 系统
- 可编程社会结构（基于 WASM）
- 持久化世界运行时
- 基于区块链的世界状态共识
- 由资源约束驱动的涌现行为

---

## 项目概述

在 oasis7 中：

- 每个实体都是一个 AI Agent
- Agent 由 LLM 驱动决策
- Agent 拥有长期记忆
- Agent 在能源与算力约束下运行
- Agent 可以编写并部署 WASM 模块扩展能力

系统只提供最小化基础规则。  
市场、组织、协议等结构由 Agent 自主构建。

---

## 设计原则

### 1. World-first

所有行为必须通过世界规则验证。  
禁止绕过运行时直接修改状态。

### 2. Emergence-first

不预设剧情。  
行为由资源与规则约束自然产生。

### 3. Persistent

世界状态可落盘与恢复。  
单个玩家离线不影响世界持续运行。

### 4. Auditable

关键状态变更以事件形式记录。  
支持回放与审计。

### 5. Extensible

高阶逻辑通过 WASM 模块扩展实现。

---

## 玩家模型

玩家不是世界中的实体，而是外部策略提供者。

玩家可以：

- 提供高层目标
- 调整 Agent 的提示词
- 指导开发 WASM 模块

玩家不能：

- 直接控制 Agent
- 修改底层世界规则
- 绕过共识层修改状态

控制是间接的，Agent 保持自主性。

---

## 模拟模型

### 空间模型

- 默认空间：100km × 100km × 10km 破碎小行星带
- 小行星直径：500m–10km
- 最小间距：≥500m
- 空间分辨率：1cm（规则抽象层）

### 文明设定

- 硅基智能体
- 能源来源：辐射 → 电能
- 无生物需求
- 关键约束维度：电力、算力、存储、带宽

资源与约束塑造结构。

当前实现采用“最小内建资源 + 模块扩展资源”的分层：

- 内建资源（由 runtime/共识直接校验）：`Electricity`、`Data`
- 模块扩展资源（由 WASM 模块定义）：包括原先 `Compound`/`Hardware` 在内的产业资源，以及信用、许可、税票、组织积分等制度化资产

其中，算力/存储/带宽在当前实现中由内建资源组合近似表达，细粒度规则由模块层决定。

---

## 可编程层

Agent 可以：

1. 使用 Rust 编写逻辑
2. 编译为 WASM
3. 部署至世界运行时
4. 安装至自身或基础设施

WASM 模块：

- 有资源成本
- 可交易
- 可升级
- 可审计
- 作为游戏内实体存在

社会结构不是预设，而是可编程产物。

---

## 高层架构

LLM Agents
↓
WASM Modules
↓
World Runtime
↓
Consensus Layer (Blockchain)
↓
Distributed Storage & Networking


世界状态通过去中心化共识维护。  
每个玩家可运行节点（推荐 native 进程）。  
Web 端默认定位为 Viewer/间接控制客户端，通过 `oasis7_viewer_live --web-bind` 网关桥接接入，不承担完整分布式节点职责。

---

## 仓库结构

对外品牌与当前 workspace / crate 命名已统一为 `oasis7`；仓库内的当前实现、脚本与入口均以 `oasis7` 为准。

- `oasis7_proto` — 协议与共享数据模型
- `oasis7_net` — 网络层
- `oasis7_consensus` — 共识与协调层
- `oasis7_node` — 节点运行时
- `oasis7_distfs` — 分布式存储
- `oasis7_wasm_*` — WASM 执行与路由层
- `oasis7` — 核心模拟层
- `oasis7_viewer` — 可视化与调试工具

---

## 项目状态

当前项目处于**技术预览阶段（尚不可玩）**。

- 当前对外可确认内容：架构、验证链路、开发预览构建包与文档入口已开放。
- 当前不应误读的内容：这不是面向玩家/社区的正式可玩发布，也不代表赛季已上线。
- 当前公开说明状态：正式公告仍在准备中；GitHub Releases 与站点下载区当前主要承载开发预览构建说明。
- 推荐入口：先查看站点首页与文档总入口，再决定是否进入完整构建与深度文档。

相关入口：[`site/index.html`](./site/index.html) · [`doc/README.md`](./doc/README.md) · [`testing-manual.md`](./testing-manual.md)

欢迎讨论与贡献。

## 从这里开始

如果你现在只是想快速找到正确入口，先按目标选路径：

| 你的目标 | 先读 | 再读 |
| --- | --- | --- |
| 想确认项目现在公开到了什么程度 | [`site/index.html`](./site/index.html) | [`site/doc/cn/index.html`](./site/doc/cn/index.html) |
| 想本地验证 Viewer / Web / API 链路 | [`testing-manual.md`](./testing-manual.md) | [`doc/world-simulator/viewer/viewer-manual.manual.md`](./doc/world-simulator/viewer/viewer-manual.manual.md) |
| 想理解世界规则、玩法和玩家边界 | [`world-rule.md`](./world-rule.md) | [`doc/game/gameplay/gameplay-top-level-design.prd.md`](./doc/game/gameplay/gameplay-top-level-design.prd.md) |
| 想参与开发或继续治理文档/代码 | [`doc/README.md`](./doc/README.md) | [`doc/core/prd.md`](./doc/core/prd.md) |


## 深入阅读

`README` 只负责快速对齐项目定位与入口，不再在这里重复维护世界规则摘要。继续深入时，直接进入对应权威文档：

- 世界规则与系统边界：[`world-rule.md`](./world-rule.md)
- 玩家访问模式与技术预览边界：[`doc/core/player-access-mode-contract-2026-03-19.prd.md`](./doc/core/player-access-mode-contract-2026-03-19.prd.md)
- Viewer / Web / 运行使用说明：[`doc/world-simulator/viewer/viewer-manual.manual.md`](./doc/world-simulator/viewer/viewer-manual.manual.md)
- 闭环测试与套件矩阵：[`testing-manual.md`](./testing-manual.md)
- 游戏玩法顶层设计：[`doc/game/gameplay/gameplay-top-level-design.prd.md`](./doc/game/gameplay/gameplay-top-level-design.prd.md)
- 游戏玩法工程架构：[`doc/game/gameplay/gameplay-engineering-architecture.md`](./doc/game/gameplay/gameplay-engineering-architecture.md)
