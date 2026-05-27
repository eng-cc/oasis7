# Viewer 性能瓶颈观测能力补齐（项目管理文档）

- 对应设计文档: `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.design.md`
- 对应需求文档: `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] VPBO-1 (PRD-TESTING-PERF-VPBO-001/002): 完成设计文档与项目管理文档建档。
- [x] VPBO-2 (PRD-TESTING-PERF-VPBO-001/002): 扩展 `RenderPerfSummary` 字段与 `PerfHotspot` 推断规则（含单测）。
- [x] VPBO-3 (PRD-TESTING-PERF-VPBO-002/003): 扩展 `perf_probe` 与 `scripts/viewer-owr4-stress.sh` 输出/解析（含脚本回归）。
- [x] VPBO-4 (PRD-TESTING-PERF-VPBO-003): 测试回归、文档状态与 devlog 收口。
- [x] VPBO-5 (PRD-TESTING-004): 专题文档按 strict schema 人工迁移并统一 `.prd.md/.project.md` 命名。

## 依赖
- doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.prd.md
- `crates/oasis7_viewer/src/render_perf_summary.rs`
- `crates/oasis7_viewer/src/perf_probe.rs`
- `crates/oasis7_viewer/src/egui_right_panel.rs`
- `scripts/viewer-owr4-stress.sh`
- `testing-manual.md`
- `doc/devlog/README.md`
- `doc/testing/prd.md`
- `doc/testing/project.md`

## 状态
- 更新日期：2026-03-03
- 当前阶段：已完成
- 阻塞项：无
- 下一步：无（项目收口完成）
