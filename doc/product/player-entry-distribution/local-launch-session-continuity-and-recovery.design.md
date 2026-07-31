# 本地启动会话连续性与恢复产品设计

## 文档身份

- 配对产品 PRD：[`local-launch-session-continuity-and-recovery.prd.md`](local-launch-session-continuity-and-recovery.prd.md)
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`

配对产品 PRD 的 canonical 路径为 `doc/product/player-entry-distribution/local-launch-session-continuity-and-recovery.prd.md`。

本文定义跨 native/Web Launcher 的稳定玩家阅读与恢复顺序，不冻结任何进程、目录、配置字段、凭据、时钟或控件实现。

## 1. 稳定会话顺序

1. 当前入口和 primary mode。
2. 本地 session 当前状态：准备、可用、blocked、停止或恢复中。
3. 主要 blocker、可信原因和安全下一步。
4. 待验证的设置或配置及其来源、阶段与影响范围。
5. 已确认的当前 session 结果；世界结果仍由权威世界反馈表达。

运行日志、端口、路径、进程诊断和浏览器技术错误可以按需展开，但不能压过当前 session、主要 blocker 和恢复动作。

## 2. 生命周期与恢复

- 启动、停止、重试、修复、重新进入和安全返回使用可辨识的状态与结果；停止或重启不暗示世界回退、保存或重放。
- 恢复面必须表明它恢复的是本地 session、连接或配置，而不是自动恢复玩家 authority、世界进度或未完成行动。
- 同一状态在 native 与 Web 可以用不同布局表达，但不得把一端的局部成功外推为另一端、另一模式或公开发行成功。

## 3. 设置边界

- 设置按“当前来源 → 草稿/暂存 → 提交 → 权威结果 → 恢复”组织。
- 配置操作的目标和影响范围在提交前可读；凭据、私密路径和原始 provider/runtime 诊断默认不作为玩家层内容展示。
- 只有专业 authority 确认时才显示已应用或可继续；本地保存、页面刷新或请求受理保持各自语义。

## 4. Web 失败表达

- 初始化、轮询或渲染失败必须从加载/可用状态中分离，并给出当前 authority 支持的重试、返回或其他恢复入口。
- 自动恢复、重新打开页面或重新启动本地服务只能说明该本地步骤已尝试；不能形成世界已健康或可玩的视觉暗示。

## 5. 非承诺

- 不规定 Launcher 界面、按钮、步骤、CLI、子进程、signal、目录、文件、端口或浏览器 API。
- 不规定 LLM/provider 字段、secret 存储、WASM 时间实现、错误签名、重试次数、自动恢复策略或测试证据格式。
- 不把 Launcher 状态或当前技术表面提升为 primary-mode 可玩性、发布就绪或公开 claim。
