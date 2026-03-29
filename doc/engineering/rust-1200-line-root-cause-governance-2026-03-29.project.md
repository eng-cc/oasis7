# Rust 1200 行根治治理（2026-03-29）项目管理

- 对应设计文档: `doc/engineering/rust-1200-line-root-cause-governance-2026-03-29.design.md`
- 对应需求文档: `doc/engineering/rust-1200-line-root-cause-governance-2026-03-29.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-051 (PRD-ENGINEERING-R1200-001/002/005) [test_tier_required]: 产出 Rust 1200 行根治治理专题 `prd/design/project`，并同步回写 engineering 模块入口、索引与 devlog。
- [x] TASK-ENGINEERING-052 (PRD-ENGINEERING-R1200-001/003) [test_tier_required]: 新增 Rust 文件体量检查脚本、冻结当前超限基线，并接入 `scripts/ci-tests.sh required`。
- [x] TASK-ENGINEERING-053 (PRD-ENGINEERING-R1200-002/003) [test_tier_required]: 在门禁中增加 `touch-and-shrink` 与 `split_part/include!` 完成态阻断规则。
- [ ] TASK-ENGINEERING-054 (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required] + [test_tier_full]: 完成 `chain_runtime` 首批目录模块化治理，优先处理 `oasis7_chain_runtime.rs` 与 `execution_bridge.rs`。
- [ ] TASK-ENGINEERING-055 (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required] + [test_tier_full]: 完成 `viewer/runtime_live` 首批目录模块化治理，并补齐对应 live / Web 验证链路。
- [ ] TASK-ENGINEERING-056 (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required] + [test_tier_full]: 完成 `oasis7_viewer` 首批治理，优先处理 `egui_right_panel_player_guide.rs`、`web_test_api.rs` 与相关 automation/UI 混装文件。
- [ ] TASK-ENGINEERING-057 (PRD-ENGINEERING-R1200-004/005) [test_tier_required]: 收口超限测试文件治理策略，冻结残余测试尾债并建立下一批 burn-down 清单。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/engineering/oversized-rust-file-splitting-2026-02-23.prd.md`
- `doc/engineering/oversized-rust-file-splitting-2026-02-23.project.md`
- `testing-manual.md`
- `scripts/ci-tests.sh`
- `scripts/doc-governance-check.sh`
- `doc/.governance/rust-oversized-file-baseline.tsv`
- `doc/.governance/rust-structural-slicing-baseline.tsv`
- `scripts/check-rust-file-size.sh`
- `AGENTS.md`
- Batch A (`TASK-ENGINEERING-052/053`) 依赖 `TASK-ENGINEERING-051` 完成并冻结规则口径。
- Batch B/C/D (`TASK-ENGINEERING-054~057`) 依赖 `TASK-ENGINEERING-052/053` 先把门禁、基线与完成态建立起来。

## 状态
- 更新日期: 2026-03-29
- 当前阶段: active
- 当前任务: `TASK-ENGINEERING-053` 已完成；`TASK-ENGINEERING-054` 为 next task。
- 阻塞项:
  - 现有仓库仍有 31 个生产 Rust 文件和 14 个测试 Rust 文件超过 1200 行，若不先冻结基线与门禁，后续任何重构都可能继续回弹。
  - `runtime_live` / `chain_runtime` / `viewer` 三个高风险域缺少统一目标目录边界，直接并行改代码会高概率冲突。
- 最新完成:
  - `TASK-ENGINEERING-051`：建立 1200 行根治治理专题三件套，并同步回写 engineering 入口、索引和 devlog。
  - `TASK-ENGINEERING-052`：新增 Rust 文件体量检查脚本、冻结 `rust-oversized-file-baseline.tsv`，并接入 `scripts/ci-tests.sh required`。
  - `TASK-ENGINEERING-053`：在同一门禁脚本中补齐 `touch-and-shrink`、`rust-structural-slicing-baseline.tsv` 与 `split_part/include!` 完成态阻断规则。
- 下一步:
  - 先执行 `TASK-ENGINEERING-054`，开始 `chain_runtime` 首批目录模块化治理，并用新门禁持续压降超限基线。
