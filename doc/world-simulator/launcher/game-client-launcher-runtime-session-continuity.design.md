# 游戏客户端启动器运行时会话连续性设计

审计轮次: 5

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-runtime-session-continuity.prd.md`
> 对应项目: `doc/world-simulator/launcher/game-client-launcher-runtime-session-continuity.project.md`

## 结构

1. **编排边界**：Launcher 构造 node-scoped 启动参数、托管子进程并读取既有状态；chain runtime 保持执行与持久化权威。
2. **输出边界**：托管入口显式传递 execution-world 目录，避免 cwd 将 runtime 产物写入源码树。
3. **恢复边界**：只将严格匹配的状态根冲突提升为 stale execution world；默认给出 fresh node 建议，破坏性重置须显式确认。
4. **存储边界**：Launcher 读取 runtime 发布的 profile、指标和降级状态；head、checkpoint、replay 与 GC 的确定性归运行态存储治理专题。
5. **浏览器边界**：WASM 路径使用可用的计时/刷新实现；失败保持可诊断，并以目标编译和浏览器证据防止平台回归。

## 不变量

- Launcher 不把本地进程可启动、状态可读或建议可见误报为 chain health、最终性或玩家承诺。
- stale 分类不得吞没其他启动错误；fresh node 建议不得删除或修改旧 execution-world 数据。
- 任何清理只能由 runtime 的 pin/manifest/replay 合同或明确确认的实现路径决定，不能由 Launcher 以文件年龄推断。
- Web/WASM 兼容层不定义 WASM ABI、runtime 时钟或永久轮询 API。

## 代码与验证接点

- Launcher 参数与进程/浏览器适配：`oasis7_client_launcher`、`oasis7_game_launcher`、`oasis7_web_launcher`。
- runtime execution-world 存储和恢复：`oasis7_chain_runtime/execution_bridge` 与运行态存储治理专题。
- 行为修改应覆盖受影响参数构造、stale 分类/恢复、进程或浏览器路径测试；可见浏览器变化另按 `testing-manual.md` S6 取证。
