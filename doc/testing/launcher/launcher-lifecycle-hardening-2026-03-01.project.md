# oasis7: 启动器生命周期与就绪硬化（2026-03-01）（项目管理）

- 对应设计文档: `doc/testing/launcher/launcher-lifecycle-hardening-2026-03-01.design.md`
- 对应需求文档: `doc/testing/launcher/launcher-lifecycle-hardening-2026-03-01.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] LCH-1 (PRD-TESTING-LAUNCHER-HARDEN-001/003): 完成专题设计文档与项目管理文档建档。
- [x] LCH-2 (PRD-TESTING-LAUNCHER-HARDEN-001/002): `oasis7_game_launcher` 完成信号清理、启动失败回滚、就绪存活联动、IPv6 解析/URL 修复。
- [x] LCH-3 (PRD-TESTING-LAUNCHER-HARDEN-002/003): `oasis7_client_launcher` 地址解析与 URL 规则对齐并补单测。
- [x] LCH-4 (PRD-TESTING-LAUNCHER-HARDEN-003): 完成定向回归、文档收口与 devlog 记录。
- [x] LCH-5 (PRD-TESTING-LAUNCHER-HARDEN-001/003): 清理未使用测试包装函数并整理测试模块路径，降低 `--all-targets` 构建噪音。
- [x] LCH-6 (PRD-TESTING-004): 专题文档按 strict schema 人工重写，并切换命名到 `.prd.md/.project.md`。

## 依赖
- doc/testing/launcher/launcher-lifecycle-hardening-2026-03-01.prd.md
- `crates/oasis7/src/bin/oasis7_game_launcher.rs`
- `crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/distfs_probe_runtime.rs`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs（`#[cfg(test)]`）`
- `crates/oasis7_client_launcher/src/main.rs`
- `doc/testing/prd.md`
- `doc/testing/project.md`
- `doc/devlog/README.md`

## 状态
- 更新日期：2026-03-03
- 当前阶段：已完成
- 阻塞项：无
- 下一步：无（当前专题已收口）
