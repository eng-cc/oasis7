# Viewer 当前 Web 性能 Harness（项目管理文档）

- 对应需求文档: `doc/testing/performance/viewer-current-web-performance-harness-2026-06-02.prd.md`

## 任务拆解
- [x] viewer-current-web-performance-probe (PRD-TESTING-PERF-VCPH-001) [test_tier_required]: 新增当前 `crates/oasis7_viewer` 可执行浏览器性能 probe。 Trace: .pm/tasks/task_5e6444132d414bf19d18d40f194a1a6f.yaml
- [x] viewer-performance-metrics-gate-tests (PRD-TESTING-PERF-VCPH-002) [test_tier_required]: 新增核心指标统计与 gate 判定单测。 Trace: .pm/tasks/task_5e6444132d414bf19d18d40f194a1a6f.yaml
- [x] viewer-performance-command-entrypoints (PRD-TESTING-PERF-VCPH-003) [test_tier_required]: 增加 repo root wrapper 与 npm script。 Trace: .pm/tasks/task_5e6444132d414bf19d18d40f194a1a6f.yaml
- [x] viewer-performance-manual-current-entry (PRD-TESTING-PERF-VCPH-004) [test_tier_required]: 更新 `testing-manual.md`，移除旧 `viewer-owr4-stress` 作为当前活跃入口的口径。 Trace: .pm/tasks/task_5e6444132d414bf19d18d40f194a1a6f.yaml

## 依赖
- `crates/oasis7_viewer/software_safe.html`
- `crates/oasis7_viewer/software_safe_src/performance_metrics.js`
- `crates/oasis7_viewer/scripts/viewer-performance-probe.mjs`
- `scripts/viewer-performance-probe.sh`
- `agent-browser`

## 状态
- 更新日期：2026-06-02
- 当前阶段：优化验证中
- 阻塞项：无
- 下一步：基于 release profile 剩余 `frame_p99_ms` 尖刺继续定位长窗口 frame pacing。
