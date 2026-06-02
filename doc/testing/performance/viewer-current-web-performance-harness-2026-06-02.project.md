# Viewer 当前 Web 性能 Harness（项目管理文档）

- 对应需求文档: `doc/testing/performance/viewer-current-web-performance-harness-2026-06-02.prd.md`

## 任务拆解
- [x] VCPH-1: 新增当前 `crates/oasis7_viewer` 可执行浏览器性能 probe。
- [x] VCPH-2: 新增核心指标统计与 gate 判定单测。
- [x] VCPH-3: 增加 repo root wrapper 与 npm script。
- [x] VCPH-4: 更新 `testing-manual.md`，移除旧 `viewer-owr4-stress` 作为当前活跃入口的口径。

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
