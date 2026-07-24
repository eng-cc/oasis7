# 客户端启动器引导配置与可用性设计（当前 authority）

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.prd.md`
> 对应执行台账: `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.project.md`

## 表现结构

1. **控制面承载层**：native 与 Web launcher 继续消费既有配置、进程状态和结构化结果；本设计不授权增加 endpoint、状态字段或进程职责。
2. **高频操作层**：状态、既有启动/停止动作和诊断入口保持优先可见；当操作被当前配置或状态阻断时，保留原因和现有修复入口。
3. **配置与恢复层**：低频配置由既有高级配置表面承载。只有精确命中 `DistributedValidationFailed` 与 `latest state root mismatch` 两个签名的 stale 状态可进入既有 non-destructive fresh-node-id 路径；不提供清理、重置或任意错误恢复。
4. **状态呈现层**：loading、empty、not-ready、disabled 与结构化失败留在触发它们的表面；不得将原始日志、空白或缓存表示为启动成功。

## 边界

- launcher 只编排已有配置和进程，不定义或绕过 runtime 的一致性、世界状态或持久化判定。
- 静态目录解析/校验、显式 execution-world-dir 传递、子进程健康及 snapshot/journal 前置条件，均以当前实现为行为真值；本文件不主张静态目录恢复、cwd 无关路径或完整性/readiness 结论。
- 路径、node id、输出目录、错误签名、GUI Agent 动作和恢复步骤均是实现事实；本文件不将历史专题中的具体名称/默认值升格为永久接口。
- 不提供目录删除、重建或覆盖 UI，也不引入 background polling、持久化恢复或续跑；Web 状态只按既有请求触发并沿用节流。
- 任何实际 DOM、样式、响应式布局或交互流程改动，均须由 game_visual_interaction_designer 提供屏幕验收并按 `testing-manual.md` S6 验证；本轮无此类改动。

## 代码与协议接点

- 当前 launcher 实现和其配置/控制面是行为真值；具体文件、CLI 参数与 HTTP 路径随实现演进，以当期代码和 operator 文档为准。
- 本设计只保留 source/generated、native/Web 表现层与 runtime 控制边界；不承诺任何历史构建、脚本、浏览器或本地运行环境可复现。
