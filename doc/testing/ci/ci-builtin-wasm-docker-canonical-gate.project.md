# oasis7: Builtin Wasm Docker Canonical Gate（项目管理）

- 对应设计文档: `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.design.md`
- 对应需求文档: `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.prd.md`

审计轮次: 2

## 任务拆解（含 PRD-ID 映射）
- [x] T1 (PRD-TESTING-CI-WASMHARD-001/002/003): 专题文档对齐到 Docker canonical gate 目标态。
- [x] T2 (PRD-TESTING-CI-WASMHARD-001): builtin wasm manifest 收敛到单 canonical token。
- [x] T3 (PRD-TESTING-CI-WASMHARD-001): sync 脚本启用 strict 模式并彻底禁止 legacy token 写回。
- [x] T4 (PRD-TESTING-CI-WASMHARD-003): 独立 gate 收敛到 `.github/workflows/wasm-determinism-gate.yml`。
- [x] T5 (PRD-TESTING-CI-WASMHARD-003): required checks 自动化默认上下文收敛到 `Wasm Determinism Gate`。
- [x] T6 (PRD-TESTING-CI-WASMHARD-002): identity / receipt 输入收敛为源码白名单与稳定 tracked 文件集合。
- [x] T7 (PRD-TESTING-CI-WASMHARD-002): identity 输入移除 workspace 根 `Cargo.lock`，改为模块级 lockfile 策略。
- [x] T8 (PRD-TESTING-CI-WASMHARD-003): 落地“本地仅 `--check`、写入需显式授权”策略并同步测试手册。
- [x] wasm-determinism-gate-ondemand-scope (PRD-TESTING-CI-WASMHARD-003) [test_tier_required]: 为 `wasm-determinism-gate` 补 changed-path scope planner，保持 `verify-wasm-determinism (m1|m4|m5)` required contexts 稳定，同时把 docs-only / 无关 PR 收口为 job 内 no-op success。 Trace: .pm/tasks/task_3db20911d0a141cda3f990ea75bc5ea7.yaml

## 依赖
- `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.prd.md`
- `scripts/sync-m1-builtin-wasm-artifacts.sh`
- `scripts/sync-m4-builtin-wasm-artifacts.sh`
- `scripts/sync-m5-builtin-wasm-artifacts.sh`
- `scripts/ci-m1-wasm-summary.sh`
- `scripts/ci-verify-m1-wasm-summaries.py`
- `scripts/wasm-release-evidence-report.sh`
- `scripts/ci-ensure-required-checks.py`
- `crates/oasis7_distfs/src/bin/sync_builtin_wasm_identity.rs`
- `.github/workflows/wasm-determinism-gate.yml`
- `testing-manual.md`
- `doc/testing/prd.md`
- `doc/testing/project.md`
- `testing-manual.md`

## 状态
- 更新日期：2026-04-15
- 当前阶段：已完成（现行口径已收敛；PR/push 改为 planner 先判 scope，无关改动 job 内 no-op；外部跨宿主 full-tier 证据按需补充）
- 阻塞项：无
- 下一步：无（等待新一轮治理需求）
