# Viewer Web 可玩性解阻设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.project.md`

## 1. 设计定位
定义 Web Player 首次进入和测试链路的可玩性解阻方案：修复 Web Test API 参数契约、自动 `Play` 与错误探针噪音，让 tick 能稳定推进。

## 2. 设计结构
- 接口兼容层：`runSteps` 同时接受 DSL、数字和 `{count}`，非法入参只告警不 panic。
- 控制白名单层：`sendControl` 仅接受 `play/pause/step/seek` 并稳定返回。
- 自动开局层：wasm + Player 模式连接成功后自动发送一次 `Play`。
- 探针降噪层：修正脚本的 WS 就绪探针，避免 `HandshakeIncomplete` 假故障。

## 3. 关键接口 / 入口
- `runSteps(payload)`
- `sendControl(action, payload)`
- wasm + Player 自动 `Play`
- `scripts/run-game-test.sh` WS 探针

## 4. 约束与边界
- 不重构 viewer live 协议，也不改核心世界规则。
- 自动 `Play` 只限 wasm + Player，不能影响导演模式/离线模式。
- 接口放宽要兼容历史脚本，不能把非法输入变成 panic。
- 探针优化目标是降噪，不是掩盖真实连接失败。

## 5. 设计演进计划
- 先修 Web Test API 入参契约。
- 再落自动 `Play`。
- 最后清理脚本探针噪音并通过回归。
