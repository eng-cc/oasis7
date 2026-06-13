# viewer-web-playability-unblock-2026-02-26

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.project.md`

审计轮次: 5

## 1. Executive Summary
- 修复 Web 玩家链路中“已连接但无法正常游玩”的主阻塞问题。
- 消除 `__AW_TEST__.runSteps(20)` 触发 wasm panic 的不稳定行为。
- 让默认 Web Player 首次进入后能稳定推进 `tick`，并降低测试链路误报。

## 2. User Experience & Functionality
- `crates/oasis7_viewer/src/web_test_api.rs`
- `crates/oasis7_viewer/src/headless.rs`
- `crates/oasis7_viewer/src/app_bootstrap.rs`
- `scripts/run-game-test.sh`
- 相关单元测试（viewer 侧）

不在范围内：
- 重构 viewer live 协议。
- 变更核心世界规则或经济/战斗数值。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications
- Web Test API `runSteps(payload)`：
  - 兼容 `string` DSL（现有自动化步骤）
  - 新增兼容 `number` / `{count}`（映射为 `ViewerControl::Step { count }`）
  - 非法 payload 仅告警，不 panic。
- Web Test API `sendControl(action, payload)`：
  - 仅接受 `play|pause|step|seek`，非法入参稳定返回并告警。
- Web Player 自动开局：
  - 在 wasm + Player 体验模式下，连接成功后自动发送一次 `Play`。
- 启动脚本探针：
  - 避免用裸 TCP 写入 WS 端口导致 `HandshakeIncomplete` 假故障噪音。

## 5. Risks & Roadmap
1. M1: 修复 Web Test API 参数契约和 panic 问题。
2. M2: 落地 Web Player 自动 `Play`，验证 `tick` 可推进。
3. M3: 修复 `run-game-test.sh` WS 就绪探针误报。
4. M4: 回归测试、文档与 devlog 回写。

### Technical Risks
- 自动 `Play` 可能改变部分导演模式/离线模式预期，需要限制在 wasm + Player。
- `runSteps` 语义扩展后，历史脚本若依赖旧行为需要兼容验证。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
