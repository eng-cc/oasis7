# 游戏客户端启动器运行时会话连续性（项目与历史追溯）

审计轮次: 5

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-runtime-session-continuity.prd.md`
> 对应设计: `doc/world-simulator/launcher/game-client-launcher-runtime-session-continuity.design.md`

## 状态

- 状态：`documented_current_authority`。
- 本轮完成：将 Launcher/chain-runtime 职责、execution-world 输出、严格 stale 分类与安全恢复、运行态存储深链及 Web/WASM 兼容边界收敛到稳定专业三件套。
- 本轮未做：不修改进程、runtime、浏览器、WASM、存储策略、测试或产品承诺。

## 吸收范围

| 历史专题 | 已完成范围 | 当前归属 |
| --- | --- | --- |
| `game-client-launcher-chain-runtime-decouple-2026-02-28` | Launcher 与 chain runtime 职责、托管进程 | 本专题的编排边界；链执行细节仍归 runtime。 |
| `game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09` | 双入口显式 execution-world 输出 | 本专题的 node-scoped 输出边界。 |
| `game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12` | 严格 stale 分类与 fresh-node 恢复建议 | 本专题的非破坏恢复边界；持久化恢复归运行态存储治理。 |
| `game-client-launcher-web-wasm-time-compat-2026-03-04` | 浏览器平台时间兼容与回归证据 | 本专题的非崩溃/可诊断边界；具体实现和验证留在代码与测试。 |

## 任务拆解

- [x] launcher-runtime-session-continuity-authority (PRD-WORLD_SIMULATOR-031) [test_tier_required]: 建立稳定 Launcher runtime session continuity authority，并将活跃依赖从日期化来源切换到本专题。 Trace: https://github.com/eng-cc/oasis7/issues/2630 (task_54fad990c6904d45b5f7f22820c40541)
- [x] launcher-runtime-session-storage-boundary (PRD-WORLD_SIMULATOR-031) [test_tier_required]: 保留 execution-world persistence、replay、checkpoint、GC 与 recovery 的 runtime 深链，不复制其专业合同。 Trace: https://github.com/eng-cc/oasis7/issues/2630 (task_54fad990c6904d45b5f7f22820c40541)
- [x] launcher-runtime-session-wasm-boundary (PRD-WORLD_SIMULATOR-031) [test_tier_required]: 固定浏览器/WASM 非崩溃、可诊断和验证边界，不把时钟或轮询实现升级为产品承诺。 Trace: https://github.com/eng-cc/oasis7/issues/2630 (task_54fad990c6904d45b5f7f22820c40541)

## 依赖与验证责任

- [运行态存储体积治理](../../world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md) 是 execution-world retention、GC、checkpoint、replay 和 latest-state recovery 的当前 runtime authority。
- Launcher 改动应验证参数构造、状态分类和恢复建议；runtime storage 改动应运行其 retention/recovery/determinism 回归；浏览器可见改动应补 S6 证据。
- 文档迁移验证：`./scripts/doc-governance-check.sh && ./scripts/readme-link-check.sh && git diff --check`。

历史专题完成证据保留在 Git history 和 GitHub task issue evidence comments；它们不再作为当前默认入口。
