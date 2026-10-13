# oasis7 Viewer：当前 Web 性能采集与评测 Harness（2026-06-02）

- 可变 task 状态与历史: PR、实际 CI 与评审记录（Issue 按需）

## 目标
- Problem Statement: 当前 `crates/oasis7_viewer` canonical `viewer` Web 入口使用时出现卡顿感，而历史 `viewer-owr4-stress` / `perf_probe` 入口已经不再是活跃实现，缺少可执行的当前 viewer 性能门禁。
- Proposed Solution: 新增 `scripts/viewer-performance-probe.sh` / `crates/oasis7_viewer/scripts/viewer-performance-probe.mjs`，使用 repo 既有 `agent-browser` 链路在真实浏览器里注入合成高密度快照，采集 rAF frame timing、FPS、Long Task、DOMContentLoaded/load event、DOM 规模与截图，并输出 JSON/Markdown gate 结果。
- Success Criteria:
  - SC-1: harness 可从 repo root 和 `crates/oasis7_viewer` 两处启动。
  - SC-2: 输出 `summary.json`、`summary.md`、截图，并在 gate fail 时返回非零退出。
  - SC-3: 核心指标统计与 gate 判定有 Vitest 覆盖。
  - SC-4: `testing-manual.md` 不再把历史 `viewer-owr4-stress` 作为当前活跃 Viewer 性能入口。
  - SC-5: 可选 interaction p95 在统计、gate 与 Markdown 中一致区分缺样本和有效 `0 ms`，按下文缺样本合同验收。

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
  - 先执行 `npm --prefix crates/oasis7_viewer run build:viewer:visual-test`；probe 只从 `.viewer-test-dist/` 读取显式测试产物，release 产物不含测试 API。
  - `./scripts/viewer-performance-probe.sh --profile smoke`
  - `./scripts/viewer-performance-probe.sh --profile release --duration-ms 8000`
  - `npm --prefix crates/oasis7_viewer run test:performance -- --profile smoke`
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

### 可选交互指标的缺样本语义

以下为 [#3425 D-G4](https://github.com/eng-cc/oasis7/issues/3425)（原 [#4429](https://github.com/eng-cc/oasis7/issues/4429)）的待实现验收合同；本节补齐不代表实现修复或回归验收已完成。`summarizeViewerPerformance → evaluateViewerPerformance → buildViewerPerformanceMarkdown` 必须保留同一语义，不能在数值转换时把缺失值 `null` 变成 `0`。

| 原始 `interactionLatencies` | `metrics.interactionSamples` / `metrics.interactionP95Ms` | `interaction_p95_ms` gate 的 `actual` / `status` | Markdown 判读 |
| --- | --- | --- | --- |
| 缺失或 `[]` | `0` / `null` | `null` / `skip` | 指标和 gate actual 显示未采样或缺值占位，gate 保留 `skip`，不得显示为 `0 ms` 或该项 `pass` |
| 有效 `[0]` | `1` / `0` | `0` / 按既有阈值判定 | 显示有效 `0 ms` 与相应判定，不得当作缺样本 |
| 有效非空样本且 p95 超过既有阈值 | 实际样本数 / 实际 p95 | 实际 p95 / `fail` | 显示实际值与失败判定，不得跳过 |

交互延迟仍是 optional 指标；该项 `skip` 本身不使整体报告 `fail`，整体 `pass` 也不能充当已采集交互性能的证据。其他 gate 继续按既有规则判定和聚合；CI 的 report-only 包装与 runtime/source/canvas/fallback、缺失或无效产物的处理保持[性能补测矩阵](performance-coverage-gap-matrix-2026-06-09.md)的现有边界。本合同不新增采样要求、阈值或 blocking gate。

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
- SC-5 定向回归（待补）：在既有 `crates/oasis7_viewer/software_safe_src/performance_metrics.test.js`（`npm --prefix crates/oasis7_viewer run test:perf-harness`）中，分别用省略输入、`[]`、`[0]` 和超阈值有效样本串联统计、评估与 Markdown，核对上表及 optional `skip` 的整体判定边界。
- Required narrow checks:
  - `npm --prefix crates/oasis7_viewer run test:ui -- performance_metrics`
  - `./scripts/viewer-performance-probe.sh --profile smoke --duration-ms 1000 --min-frame-samples 20 --min-fps 20`
  - `git diff --check`
- Broader follow-up when chasing player-visible lag:
  - Run the release profile in a headed GPU browser and compare `summary.json` between commits.
  - Preserve failing summaries as lag signatures before changing UI layout or rendering behavior.
