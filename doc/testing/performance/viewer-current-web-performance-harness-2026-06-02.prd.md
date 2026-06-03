# oasis7 Viewer：当前 Web 性能采集与评测 Harness（2026-06-02）

- 对应项目管理文档: `doc/testing/performance/viewer-current-web-performance-harness-2026-06-02.project.md`

## 目标
- Problem Statement: 当前 `crates/oasis7_viewer` software-safe Web viewer 使用时出现卡顿感，而历史 `viewer-owr4-stress` / `perf_probe` 入口已经不再是活跃实现，缺少可执行的当前 viewer 性能门禁。
- Proposed Solution: 新增 `scripts/viewer-performance-probe.sh` / `crates/oasis7_viewer/scripts/viewer-performance-probe.mjs`，使用 repo 既有 `agent-browser` 链路在真实浏览器里注入合成高密度快照，采集 rAF frame timing、FPS、Long Task、DOMContentLoaded/load event、DOM 规模与截图，并输出 JSON/Markdown gate 结果。
- Success Criteria:
  - SC-1: harness 可从 repo root 和 `crates/oasis7_viewer` 两处启动。
  - SC-2: 输出 `summary.json`、`summary.md`、截图，并在 gate fail 时返回非零退出。
  - SC-3: 核心指标统计与 gate 判定有 Vitest 覆盖。
  - SC-4: `testing-manual.md` 不再把历史 `viewer-owr4-stress` 作为当前活跃 Viewer 性能入口。

## 范围
- In scope:
  - Browser-side FPS and frame interval p50 / p95 / p99 / max.
  - Long Task count / total / max when browser exposes `PerformanceObserver`.
  - DOMContentLoaded / load event readiness timing.
  - Optional interaction latency fields in the artifact schema.
  - DOM node count, interactive element count, viewport/browser metadata and screenshot.
  - `smoke` and `release` threshold profiles plus CLI threshold overrides.
- Out of scope:
  - GPU timestamp, drawcall, VRAM, WebGL pass timing, or external telemetry systems.
  - Runtime rule changes or LLM behavior tuning.
  - Reviving deleted `viewer-owr4-stress` as an active operator surface.

## 接口 / 数据
- Active commands:
  - `./scripts/viewer-performance-probe.sh --profile smoke`
  - `./scripts/viewer-performance-probe.sh --profile release --duration-ms 8000`
  - `cd crates/oasis7_viewer && npm run test:performance -- --profile smoke`
- Artifacts:
  - `output/playwright/viewer-performance/<run-id>/summary.json`
  - `output/playwright/viewer-performance/<run-id>/summary.md`
  - `output/playwright/viewer-performance/<run-id>/viewer-performance.png`
- Gate fields:
  - `frame_samples`
  - `fps_avg`
  - `frame_p95_ms`
  - `frame_p99_ms`
  - `long_task_count`
  - `long_task_total_ms`
  - `dom_content_loaded_ms`
  - `load_event_ms`
  - `interaction_p95_ms`

## 里程碑
- VCPH-1: 新增当前 `crates/oasis7_viewer` 可执行浏览器性能 probe。
- VCPH-2: 新增核心指标统计与 gate 判定单测。
- VCPH-3: 增加 repo root wrapper 与 npm script。
- VCPH-4: 更新 `testing-manual.md`，移除旧 `viewer-owr4-stress` 作为当前活跃入口的口径。

## 风险
- Browser automation infra failure must be reported separately from performance gate failure.
- Headless browser timing may differ from headed GPU sessions; release triage should preserve artifact metadata and compare like with like.
- Current smoke already captures a severe lag signature; this harness establishes the gate but does not by itself optimize the viewer.
- Long Task support depends on browser `PerformanceObserver`; missing support should not be treated as a pass for frame timing gates.

## 验证
- Required narrow checks:
  - `cd crates/oasis7_viewer && npm run test:ui -- performance_metrics`
  - `./scripts/viewer-performance-probe.sh --profile smoke --duration-ms 1000 --min-frame-samples 20 --min-fps 20`
  - `git diff --check`
- Broader follow-up when chasing player-visible lag:
  - Run the release profile in a headed GPU browser and compare `summary.json` between commits.
  - Preserve failing summaries as lag signatures before changing UI layout or rendering behavior.
