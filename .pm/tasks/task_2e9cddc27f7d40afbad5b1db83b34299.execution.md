# task_2e9cddc27f7d40afbad5b1db83b34299 Execution Log

- task_uid: task_2e9cddc27f7d40afbad5b1db83b34299
- title: viewer performance probe uses real renderer
- owner_role: viewer_engineer
- worktree_hint: null

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
  - Action: ...
  - Validation Command: ...
  - Expected Result: ...
  - Actual Result: ...
  - Blocker / Next Action: ...
-->

## 2026-06-09 12:48:00 CST / viewer_engineer
- 完成内容: Updated the viewer performance probe so performance measurements no longer opt out of the pixel-world wasm renderer. The probe now prepares an official copied viewer web-dist via `scripts/copy-viewer-web-dist.sh`, serves that web-dist instead of `crates/oasis7_viewer` source root, opens the viewer without `pixel_world_renderer=defer`, and waits for `pixelWorldRuntimeStatus="ready"` plus `pixelWorldRuntimeSource="wasm_bindgen_runtime"` before sampling frames.
- Action: Modified `crates/oasis7_viewer/scripts/viewer-performance-probe.mjs` to use `web-dist` as the static root, require `pixel-world-bridge/pixel_world_bridge.js`, extend the real-renderer browser-open timeout, and fail fast if the wasm renderer does not become ready. Updated `crates/oasis7_viewer/software_safe_src/performance_metrics.js` and `.test.js` so performance summaries/markdown include pixel-world runtime and fallback-shell evidence.
- Validation Command: `node --check crates/oasis7_viewer/scripts/viewer-performance-probe.mjs`; `rg -n "pixel_world_renderer=defer" crates/oasis7_viewer/scripts/viewer-performance-probe.mjs scripts/viewer-performance-probe.sh crates/oasis7_viewer/software_safe_src/performance_metrics.js crates/oasis7_viewer/software_safe_src/performance_metrics.test.js || true`; `rtk npm --prefix crates/oasis7_viewer run test:ui -- software_safe_src/performance_metrics.test.js`; `rtk ./scripts/build-viewer-software-safe.sh`; `rtk ./scripts/viewer-performance-probe.sh --profile smoke --duration-ms 1000 --min-frame-samples 5 --min-fps 1 --max-frame-p95-ms 1000 --max-frame-p99-ms 1000 --max-long-task-count 999 --max-long-task-total-ms 999999 --max-dom-content-loaded-ms 60000 --max-load-event-ms 60000 --max-interaction-p95-ms 1000 --agents 10 --locations 4 --out-dir output/playwright/viewer-performance/real-renderer-smoke-20260609-final`; `git diff --check`
- Expected Result: Probe URL does not include `pixel_world_renderer=defer`; `/pixel-world-bridge` assets are served from official copied web-dist; performance sampling starts only after the real wasm renderer is ready; summary artifacts expose renderer evidence; targeted tests and whitespace check pass.
- Actual Result: All checks passed. Final probe URL was `http://127.0.0.1:57943/software_safe.html?test_api=1&connect=0&locale=en&hosted_bootstrap=0&t=1780980419705`; summary status `pass`; `pixelWorldRuntimeStatus="ready"`; `pixelWorldRuntimeSource="wasm_bindgen_runtime"`; module URL `/pixel-world-bridge/pixel_world_bridge.js`; `pixelWorldFatal=null`; `lastError=null`; `renderedCanvasCount=1`; `fallbackShellCount=0`. Markdown includes `Pixel-world runtime: ready / wasm_bindgen_runtime` and `Pixel-world renderer DOM: rendered canvas 1, fallback shell 0`.
- 遗留事项: The short smoke command used relaxed thresholds and a small fixture to verify the real-renderer harness behavior. Release/profile thresholds may need recalibration now that the probe measures the heavier real renderer path instead of the deferred shell.
