# 本地启动会话连续性与恢复迁移追踪

## 文档身份

- 配对产品 PRD：[`local-launch-session-continuity-and-recovery.prd.md`](local-launch-session-continuity-and-recovery.prd.md)
- 配对产品设计：[`local-launch-session-continuity-and-recovery.design.md`](local-launch-session-continuity-and-recovery.design.md)
- 上位产品 PRD：[`prd.md`](prd.md)
- 追踪范围：产品文档语义迁移
- Owner role：`repository_health_engineer`

配对文档的 canonical 路径为 `doc/product/player-entry-distribution/local-launch-session-continuity-and-recovery.prd.md` 和 `doc/product/player-entry-distribution/local-launch-session-continuity-and-recovery.design.md`。

本文只记录产品语义归位与源文件删除条件，不维护实现任务、测试命令、CI、发布或当前会话状态。

## 迁移映射

| 历史源专题 | 归位语义 | 未提升为产品承诺 |
| --- | --- | --- |
| `game-client-launcher-chain-runtime-decouple-2026-02-28` 三件套 | 本地 Launcher 只承载已声明模式的交付和 session 过渡；进程启动不代签进入权威世界 | chain/runtime 进程、CLI、拓扑、状态 API、资产与打包 |
| `game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09` 三件套 | 本地输出与会话恢复需要可理解、可诊断的边界 | execution-world 路径、目录规则、文件与参数构造 |
| `game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12` 三件套 | 陈旧本地状态必须导向真实的恢复、重新进入或安全停止，不静默复用旧 authority | 陈旧状态识别、清理/恢复算法、持久化与 runtime 合同 |
| `game-client-launcher-graceful-stop-2026-03-02` 三件套 | 停止与重启是本地 session 生命周期，不自动确认、撤销或回放世界结果 | signal、子进程终止、超时、进程树与验证 |
| `game-client-launcher-llm-settings-panel-2026-03-02` 三件套 | 设置是待验证输入；编辑、保存或请求受理不等于应用、Agent 行为或世界结果 | provider/LLM schema、秘密、存储、控制面与 UI 实现 |
| `game-client-launcher-web-wasm-time-compat-2026-03-04` 三件套 | Web Launcher 故障必须真实可读且有适用恢复，不伪装为 session 或世界成功 | WASM 时钟、轮询、浏览器 API、错误签名、自动化与证据 |

## 删除条件

- 六组源专题仅在当前 Launcher、runtime、设置和 Web/WASM 专业 authority 已完整承接其实现、兼容、存储、恢复和验证真值后才能整组删除。
- 活跃索引和所有 incoming references 必须改指 current professional authority；产品文档不能成为任何技术合同的唯一后继。
- 源文件删除后，历史任务与验证只从 Git history 和 GitHub task issue evidence 追溯。

## 产品边界

- 本专题不宣布新的玩家模式、发行等级、可玩结论或公开可用性。
- 根 `README.md` 继续拥有公开状态与 claim envelope；专业域继续拥有当前 Launcher/runtime 行为及验证结论。

## 任务拆解

不适用。本文件不维护实现计划、checkbox、PR、CI 或任务状态；相关真值只进入 GitHub task issue evidence 和专业域 project。

## 依赖

- [`doc/product/player-entry-distribution/prd.md`](prd.md) 的入口与发行边界。
- [`doc/world-simulator/launcher/README.md`](../../world-simulator/launcher/README.md) 的当前 Launcher 专业入口。
- [`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md) 与 [`doc/testing/prd.md`](../../testing/prd.md) 的实现、恢复和验证权威。

## 状态

- 文档生命周期：`active`。
- 迁移收据：`finalized`。六组三件套共 18 个历史源文件已在 `a7149afb9` 删除；当前专业后继为 Launcher runtime-session-continuity、cross-surface-action-parity 与 control-plane-and-machine-interface authority，历史过程只从 Git 与 GitHub task evidence 追溯。
