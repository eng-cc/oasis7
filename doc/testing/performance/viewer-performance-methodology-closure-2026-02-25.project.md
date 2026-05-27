# Viewer 性能测试方法论能力补齐（项目管理文档）

- 对应设计文档: `doc/testing/performance/viewer-performance-methodology-closure-2026-02-25.design.md`
- 对应需求文档: `doc/testing/performance/viewer-performance-methodology-closure-2026-02-25.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] VPMC-1 (PRD-TESTING-PERF-VPMC-001/002): 输出设计文档与项目管理文档建档。
- [x] VPMC-2 (PRD-TESTING-PERF-VPMC-001/002): 增强 `scripts/viewer-owr4-stress.sh`（自动采集、门禁判定、基线对比、结构化输出）。
- [x] VPMC-3 (PRD-TESTING-PERF-VPMC-001/003): 执行脚本验证（语法/help/短窗/baseline smoke）并收口文档状态。
- [x] VPMC-4 (PRD-TESTING-PERF-VPMC-002/003): 补充 Web 渲染性能采样 GPU 前置要求并回写 devlog。
- [x] VPMC-5 (PRD-TESTING-004): 专题文档按 strict schema 人工迁移并统一 `.prd.md/.project.md` 命名。

## 依赖
- doc/testing/performance/viewer-performance-methodology-closure-2026-02-25.prd.md
- `scripts/viewer-owr4-stress.sh`
- `crates/oasis7_viewer/src/perf_probe.rs`
- `testing-manual.md`
- `doc/devlog/README.md`
- `doc/testing/prd.md`
- `doc/testing/project.md`

## 状态
- 更新日期：2026-03-03
- 当前阶段：已完成
- 阻塞项：无
- 下一步：无（项目收口完成）
