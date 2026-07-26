# 游戏客户端启动器控制面与机器接口项目管理文档

- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.prd.md`
- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.design.md`

## 任务拆解

- [x] launcher-control-plane-machine-interface-migration (PRD-WORLD_SIMULATOR-031) [test_tier_required]: 将五个完成的 launcher 历史专题收敛为稳定 authority，并修复模块与 DecisionProvider 的阅读入口。 Trace: https://github.com/eng-cc/oasis7/issues/2585 (task_dc250ba301164ea5ac21b719a1e3cefe)
- [x] launcher-control-plane-batch23-session-surface-backfill (PRD-WORLD_SIMULATOR-031) [test_tier_required]: 补回受控停止、Web 非崩溃诊断表现边界，并将 runtime/session/WASM 细节路由至新 runtime successor。 Trace: https://github.com/eng-cc/oasis7/issues/2630 (task_54fad990c6904d45b5f7f22820c40541)

## 状态

- 当前阶段: stable authority
- 当前任务: 本文档承接完成的 launcher control-plane 和 GUI-agent 机器接口语义；后续实现变更应在新的 GitHub Project-backed task 中以当前代码和 capability discovery 响应为准。
- owner boundary: `viewer_engineer` 负责 launcher/Web 表现、可观测和受控操作面的合同；runtime 状态、执行和 Agent/DecisionProvider 语义分别归对应专业 authority。
- runtime/session/WASM successor: `game-client-launcher-runtime-session-continuity.{prd,design,project}.md` 负责 launcher 与 runtime 的 session continuity、execution-world/recovery 和浏览器/WASM 运行时细节；本文只保留 surface contract。

## 已收敛能力

- native/Web 共享 launcher UI schema 与服务端状态映射。
- 静态 launcher 资源可与既有 control-plane API 并存。
- operator 可通过 capabilities、state 和统一 action 响应使用 HTTP JSON，而无需解析 GUI。
- hosted 模式的枚举 operator 路由有 peer-IP gate；这不是认证、公开 readiness 或所有路由私有性的结论。

## 依赖

- `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.design.md`
- `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.prd.md`

- `crates/oasis7_client_launcher/` 与 `crates/oasis7_launcher_ui/`：共享表现层实现。
- `crates/oasis7/src/bin/oasis7_web_launcher/`：控制面、GUI-agent 路由和 hosted operator-path gate。
- `testing-manual.md`：实现或可见输出改动时的 S6 Web 验证要求。
- 文档迁移验证：`./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh`、`python3 scripts/product-doc-governance-check.py` 和 `git diff --check`。
