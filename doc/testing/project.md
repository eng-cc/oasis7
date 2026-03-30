# testing PRD Project

审计轮次: 7

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-TESTING-001 (PRD-TESTING-001) [test_tier_required]: 完成 testing PRD 改写，建立分层测试设计入口。
- [x] TASK-TESTING-002 (PRD-TESTING-001/002) [test_tier_required]: 对齐 S0~S10 与改动路径触发矩阵。
  - 产物文件:
    - `testing-manual.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "套件触发总表|改动路径矩阵|选择规则|S0|S10" testing-manual.md`
- [x] TASK-TESTING-003 (PRD-TESTING-002/003) [test_tier_required]: 建立发布证据包模板（命令、日志、截图、结论）。
  - 产物文件:
    - `doc/testing/templates/release-evidence-bundle-template.md`
  - 验收命令 (`test_tier_required`):
    - `test -f doc/testing/templates/release-evidence-bundle-template.md`
    - `rg -n "执行命令|UI / 体验证据|长跑 / 在线证据|结论摘要|PRD-ID" doc/testing/templates/release-evidence-bundle-template.md`
- [x] TASK-TESTING-004 (PRD-TESTING-003) [test_tier_required]: 建立测试质量趋势跟踪（通过率/逃逸率/修复时长）。
  - 产物文件:
    - `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.prd.md`
    - `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.design.md`
    - `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.project.md`
    - `doc/testing/evidence/testing-quality-trend-baseline-2026-03-11.md`
  - 验收命令 (`test_tier_required`):
    - `test -f doc/testing/governance/testing-quality-trend-tracking-2026-03-11.prd.md`
    - `test -f doc/testing/evidence/testing-quality-trend-baseline-2026-03-11.md`
    - `rg -n "首次通过率|阶段内逃逸率|平均修复时长|红黄绿阈值" doc/testing/governance/testing-quality-trend-tracking-2026-03-11.prd.md doc/testing/evidence/testing-quality-trend-baseline-2026-03-11.md`
- [x] TASK-TESTING-005 (PRD-TESTING-002/003) [test_tier_required]: 建立模块级专题任务映射索引（2026-03-02 批次）。
- [x] TASK-TESTING-006 (PRD-TESTING-001/002/003) [test_tier_required]: 对齐 strict PRD schema，补齐关键流程/规格矩阵/边界异常/NFR/验证与决策记录。
- [x] TASK-TESTING-007 (PRD-TESTING-004) [test_tier_required]: 完成 `ci-wasm32-target-install` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-008 (PRD-TESTING-004) [test_tier_required]: 继续按批次迁移 testing 活跃 legacy 专题文档（优先 `governance/launcher/longrun/performance/manual`）。
- [x] TASK-TESTING-009 (PRD-TESTING-004) [test_tier_required]: 完成 `ci-testcase-tiering` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-010 (PRD-TESTING-004) [test_tier_required]: 完成 `ci-tiered-execution` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-011 (PRD-TESTING-004) [test_tier_required]: 完成 `ci-test-coverage` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-012 (PRD-TESTING-004) [test_tier_required]: 完成 `ci-builtin-wasm-determinism-gate-m1` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-013 (PRD-TESTING-004) [test_tier_required]: 完成 `ci-builtin-wasm-determinism-gate-required-check-protection` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-014 (PRD-TESTING-004) [test_tier_required]: 完成 `ci-remove-builtin-wasm-hash-checks-from-base-gate` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-015 (PRD-TESTING-004) [test_tier_required]: 完成 `wasm-build-determinism-guard` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-016 (PRD-TESTING-004) [test_tier_required]: 完成 `release-gate-metric-policy-alignment-2026-02-28` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-017 (PRD-TESTING-004) [test_tier_required]: 完成 `llm-skip-tick-ratio-metric` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-018 (PRD-TESTING-004) [test_tier_required]: 完成 `launcher-chain-script-migration-2026-02-28` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-019 (PRD-TESTING-004) [test_tier_required]: 完成 `launcher-lifecycle-hardening-2026-03-01` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-020 (PRD-TESTING-004) [test_tier_required]: 完成 `launcher-viewer-auth-node-config-autowire-2026-03-02` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-021 (PRD-TESTING-004) [test_tier_required]: 完成 `chain-runtime-feedback-replication-network-autowire-2026-03-02` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-022 (PRD-TESTING-004) [test_tier_required]: 完成 `chain-runtime-soak-script-reactivation-2026-02-28` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-023 (PRD-TESTING-004) [test_tier_required]: 完成 `p2p-longrun-continuous-chaos-injection-2026-02-24` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-024 (PRD-TESTING-004) [test_tier_required]: 完成 `p2p-longrun-endurance-chaos-template-2026-02-25` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-025 (PRD-TESTING-004) [test_tier_required]: 完成 `p2p-storage-consensus-longrun-online-stability-2026-02-24` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-026 (PRD-TESTING-004) [test_tier_required]: 完成 `p2p-longrun-feedback-event-injection-2026-03-02` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-027 (PRD-TESTING-004) [test_tier_required]: 完成 `s10-distfs-probe-bootstrap-2026-02-28` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-028 (PRD-TESTING-004) [test_tier_required]: 完成 `s10-five-node-real-game-soak` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-029 (PRD-TESTING-004) [test_tier_required]: 完成 `runtime-performance-observability-foundation-2026-02-25` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-030 (PRD-TESTING-004) [test_tier_required]: 完成 `runtime-performance-observability-llm-api-decoupling-2026-02-25` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-031 (PRD-TESTING-004) [test_tier_required]: 完成 `viewer-perf-bottleneck-observability-2026-02-25` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-032 (PRD-TESTING-004) [test_tier_required]: 完成 `viewer-performance-methodology-closure-2026-02-25` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-033 (PRD-TESTING-004) [test_tier_required]: 完成 `systematic-application-testing-manual` 专题文档逐篇人工迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-034 (PRD-TESTING-004) [test_tier_required]: 完成 `web-ui-agent-browser-closure-manual` 专题文档逐篇人工迁移到 strict schema，并补齐 `.project.md` 管理文档。
- [x] TASK-TESTING-042 (PRD-TESTING-002/003/004) [test_tier_required]: 将 Web UI 闭环默认工具口径统一收口到 `agent-browser`，同步主手册、专题分册、Viewer 手册、站内镜像与门禁脚本。
- [x] TASK-TESTING-043 (PRD-TESTING-002/003) [test_tier_required]: 修正文档边界歧义，明确 `Viewer` 页面默认走 `agent-browser`，`oasis7_web_launcher` / launcher Web 控制面默认走 GUI Agent，再用页面做状态与字段校验。
- [x] TASK-TESTING-044 (PRD-TESTING-LAUNCHER-BUNDLE-001) [test_tier_required]: 完成“启动器 bundle-first 试玩入口收敛（2026-03-12）”专题 PRD / design / project 建档，并同步 testing 索引与 README。
- [x] TASK-TESTING-045 (PRD-TESTING-LAUNCHER-BUNDLE-001/002) [test_tier_required]: 为 `run-game-test.sh` 增加 `--bundle-dir`，同步主手册/人工清单帮助口径，并完成 bundle 产物闭环验证与 blocker 归档。
- [x] TASK-TESTING-046 (PRD-TESTING-LAUNCHER-BUNDLE-002) [test_tier_required]: 查明 `run-game-test-ab.sh --headless` 的阻断根因是 `SwiftShader` software renderer，并新增环境快失败与 `browser_env.json` 证据落盘，避免把环境问题误判成 fresh Web 回归。
- [x] TASK-TESTING-047 (PRD-TESTING-LAUNCHER-BUNDLE-001/002) [test_tier_required]: 新增 `run-producer-playtest.sh`，把制作人 bundle-first 试玩收敛成单命令入口，并同步主手册与人工清单。
- [x] TASK-TESTING-048 (PRD-TESTING-LAUNCHER-BUNDLE-001/002) [test_tier_required]: 为 `run-producer-playtest.sh` 增加 `--open-headed`，使制作人可在起栈后自动打开 headed 浏览器，并同步主手册/日志口径。
- [x] TASK-TESTING-049 (PRD-TESTING-LAUNCHER-BUNDLE-001/002) [test_tier_required]: 修复 `run-producer-playtest.sh --open-headed` 退出后残留浏览器窗口的问题，为脚本补充自动关会话收尾，并同步手册/日志口径。
- [x] TASK-TESTING-050 (PRD-TESTING-LAUNCHER-BUNDLE-001/002) [test_tier_required]: 固化 headed Viewer Web 的默认硬件 WebGL 启动参数，并把 headed 命中 `SwiftShader` / software renderer 统一收口为环境阻断，同步脚本/手册/专题文档。
- [x] TASK-TESTING-051 (PRD-TESTING-LAUNCHER-BUNDLE-001/002) [test_tier_required]: 为 bundle-first 试玩入口增加 freshness manifest 守卫，默认阻断或自动重建 stale bundle，避免旧 Viewer Web 产物与新 runtime 二进制混跑。
- [x] TASK-TESTING-035 (PRD-TESTING-004) [test_tier_required]: 完成 archive 专题 `ci-required-m1-wasm-hash-check` 文档迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-036 (PRD-TESTING-004) [test_tier_required]: 完成 archive 专题 `wasm-platform-canonical-hash-manifest` 文档迁移到 strict schema，并统一 `.prd` 命名。
- [x] TASK-TESTING-037 (PRD-TESTING-005) [test_tier_required]: 完成 `ci-builtin-wasm-docker-canonical-gate` 专题 PRD 与项目管理文档建档，建立 1-6 治理项映射。
- [x] TASK-TESTING-038 (PRD-TESTING-005) [test_tier_required]: 落地 m4/m5 keyed hash manifest 迁移与 sync strict 模式（禁 legacy 写回）。
- [x] TASK-TESTING-039 (PRD-TESTING-005) [test_tier_required]: 收敛 builtin wasm identity 的 `source_hash` 输入范围并移除 workspace 根 `Cargo.lock` 依赖。
- [x] TASK-TESTING-040 (PRD-TESTING-005) [test_tier_required]: 收敛 builtin wasm 独立 gate 到 `wasm-determinism-gate` / required checks 保护与本地只读校验策略；GitHub-hosted 默认 runner 为 Linux，外部 Docker-capable macOS summary 作为 full-tier 补充证据。
- [x] TASK-TESTING-041 (PRD-TESTING-002/003) [test_tier_required]: 完成“启动器全功能可用性审查与闭环验收（2026-03-08）”专题执行（脚本审查 + agent-browser 真闭环 + 风险分级结论）。

## 专题任务映射（2026-03-02 批次）
- [x] SUBTASK-TESTING-20260302-001 (PRD-TESTING-002/003) [test_tier_required]: `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.project.md`
- [x] SUBTASK-TESTING-20260302-002 (PRD-TESTING-002/003) [test_tier_required]: `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.project.md`
- [x] SUBTASK-TESTING-20260302-003 (PRD-TESTING-002/003) [test_tier_required]: `doc/testing/longrun/p2p-longrun-feedback-event-injection-2026-03-02.project.md`

## 专题任务映射（2026-03-03 批次）
- [x] SUBTASK-TESTING-20260303-001 (PRD-TESTING-004) [test_tier_required]: `doc/testing/ci/ci-wasm32-target-install.project.md`
- [x] SUBTASK-TESTING-20260303-002 (PRD-TESTING-004) [test_tier_required]: `doc/testing/ci/ci-testcase-tiering.project.md`
- [x] SUBTASK-TESTING-20260303-003 (PRD-TESTING-004) [test_tier_required]: `doc/testing/ci/ci-tiered-execution.project.md`
- [x] SUBTASK-TESTING-20260303-004 (PRD-TESTING-004) [test_tier_required]: `doc/testing/ci/ci-test-coverage.project.md`
- [x] SUBTASK-TESTING-20260303-005 (PRD-TESTING-004) [test_tier_required]: `doc/testing/ci/ci-builtin-wasm-determinism-gate-m1.project.md`
- [x] SUBTASK-TESTING-20260303-006 (PRD-TESTING-004) [test_tier_required]: `doc/testing/ci/ci-builtin-wasm-determinism-gate-required-check-protection.project.md`
- [x] SUBTASK-TESTING-20260303-007 (PRD-TESTING-004) [test_tier_required]: `doc/testing/ci/ci-remove-builtin-wasm-hash-checks-from-base-gate.project.md`
- [x] SUBTASK-TESTING-20260303-008 (PRD-TESTING-004) [test_tier_required]: `doc/testing/governance/wasm-build-determinism-guard.project.md`
- [x] SUBTASK-TESTING-20260303-009 (PRD-TESTING-004) [test_tier_required]: `doc/testing/governance/release-gate-metric-policy-alignment-2026-02-28.project.md`
- [x] SUBTASK-TESTING-20260303-010 (PRD-TESTING-004) [test_tier_required]: `doc/testing/governance/llm-skip-tick-ratio-metric.project.md`
- [x] SUBTASK-TESTING-20260303-011 (PRD-TESTING-004) [test_tier_required]: `doc/testing/launcher/launcher-chain-script-migration-2026-02-28.project.md`
- [x] SUBTASK-TESTING-20260303-012 (PRD-TESTING-004) [test_tier_required]: `doc/testing/launcher/launcher-lifecycle-hardening-2026-03-01.project.md`
- [x] SUBTASK-TESTING-20260303-013 (PRD-TESTING-004) [test_tier_required]: `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.project.md`
- [x] SUBTASK-TESTING-20260303-014 (PRD-TESTING-004) [test_tier_required]: `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.project.md`
- [x] SUBTASK-TESTING-20260303-015 (PRD-TESTING-004) [test_tier_required]: `doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.project.md`
- [x] SUBTASK-TESTING-20260303-016 (PRD-TESTING-004) [test_tier_required]: `doc/testing/longrun/p2p-longrun-continuous-chaos-injection-2026-02-24.project.md`
- [x] SUBTASK-TESTING-20260303-017 (PRD-TESTING-004) [test_tier_required]: `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.project.md`
- [x] SUBTASK-TESTING-20260303-018 (PRD-TESTING-004) [test_tier_required]: `doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.project.md`
- [x] SUBTASK-TESTING-20260303-019 (PRD-TESTING-004) [test_tier_required]: `doc/testing/longrun/p2p-longrun-feedback-event-injection-2026-03-02.project.md`
- [x] SUBTASK-TESTING-20260303-020 (PRD-TESTING-004) [test_tier_required]: `doc/testing/longrun/s10-distfs-probe-bootstrap-2026-02-28.project.md`
- [x] SUBTASK-TESTING-20260303-021 (PRD-TESTING-004) [test_tier_required]: `doc/testing/longrun/s10-five-node-real-game-soak.project.md`
- [x] SUBTASK-TESTING-20260303-022 (PRD-TESTING-004) [test_tier_required]: `doc/testing/performance/runtime-performance-observability-foundation-2026-02-25.project.md`
- [x] SUBTASK-TESTING-20260303-023 (PRD-TESTING-004) [test_tier_required]: `doc/testing/performance/runtime-performance-observability-llm-api-decoupling-2026-02-25.project.md`
- [x] SUBTASK-TESTING-20260303-024 (PRD-TESTING-004) [test_tier_required]: `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.project.md`
- [x] SUBTASK-TESTING-20260303-025 (PRD-TESTING-004) [test_tier_required]: `doc/testing/performance/viewer-performance-methodology-closure-2026-02-25.project.md`
- [x] SUBTASK-TESTING-20260303-026 (PRD-TESTING-004) [test_tier_required]: `doc/testing/manual/systematic-application-testing-manual.project.md`
- [x] SUBTASK-TESTING-20260303-027 (PRD-TESTING-004) [test_tier_required]: `doc/testing/manual/web-ui-agent-browser-closure-manual.project.md`

## 专题任务映射（2026-03-06 批次）
- [x] SUBTASK-TESTING-20260306-001 (PRD-TESTING-005) [test_tier_required]: `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.project.md`

## 专题任务映射（2026-03-08 批次）
- [x] SUBTASK-TESTING-20260308-001 (PRD-TESTING-002/003) [test_tier_required]: `doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.project.md`

## 专题任务映射（2026-03-10 批次）
- [x] SUBTASK-TESTING-20260310-001 (PRD-TESTING-LAUNCHER-MANUAL-001/002/003) [test_tier_required]: `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.project.md`
- [x] TASK-TESTING-006 (PRD-TESTING-001) [test_tier_required]: 同步 `doc/testing/README.md` 的模块入口索引，补齐近期专题、模块职责与根目录收口口径。
- [x] TASK-TESTING-052 (PRD-TESTING-002/003) [test_tier_required]: 把前期工业引导的 required-tier 人工回归链路挂入 `testing-manual.md`，并与 playability 专题卡组互链，覆盖 `首个制成品 / 停机恢复 / 首座工厂单元`。
  - 产物文件:
    - `testing-manual.md`
    - `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n 'industrial-onboarding-required-tier-cards-2026-03-15|首个制成品|停机恢复|首座工厂单元' testing-manual.md doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md`
- [x] TASK-TESTING-053 (PRD-TESTING-002/003) [test_tier_required]: 优化 release packaging Web 资产复用链路，在 `build-web-dist` 一次性产出 viewer/launcher 两份静态包，并让 `package-native` 直接复用 artifact，避免每个平台重复 `trunk install` / `trunk build`。
  - 产物文件:
    - `.github/workflows/release-packages.yml`
    - `scripts/release-prepare-bundle.sh`
    - `scripts/build-game-launcher-bundle.sh`
  - 验收命令 (`test_tier_required`):
    - `bash -n scripts/release-prepare-bundle.sh scripts/build-game-launcher-bundle.sh`
    - `python -c "import pathlib, yaml; yaml.safe_load(pathlib.Path('.github/workflows/release-packages.yml').read_text())"`
    - `tmpdir=$(mktemp -d) && mkdir -p "$tmpdir/viewer" "$tmpdir/launcher" "$tmpdir/out" && printf "<html></html>" > "$tmpdir/viewer/index.html" && printf "<html></html>" > "$tmpdir/launcher/index.html" && ./scripts/build-game-launcher-bundle.sh --dry-run --out-dir "$tmpdir/out" --web-dist "$tmpdir/viewer" --web-launcher-dist "$tmpdir/launcher" >/dev/null`
- [x] TASK-TESTING-054 (PRD-TESTING-002/003) [test_tier_required]: 继续压缩 release 关键路径，让 `release-gate-web` 与 `build-web-dist` 共享同一组 Web wasm/cargo cache，并把 bundle 原生二进制构建收敛到单次 cargo 调用，减少重复 bootstrap 与 metadata 解析。
  - 产物文件:
    - `.github/workflows/release-packages.yml`
    - `scripts/build-game-launcher-bundle.sh`
    - `doc/testing/prd.md`
  - 验收命令 (`test_tier_required`):
    - `bash -n scripts/build-game-launcher-bundle.sh`
    - `python -c "import pathlib, yaml; yaml.safe_load(pathlib.Path('.github/workflows/release-packages.yml').read_text())"`
    - `rg -n "release-packages-web-wasm-v2|BUNDLE_NATIVE_BUILD_ARGS|oasis7_client_launcher" .github/workflows/release-packages.yml scripts/build-game-launcher-bundle.sh doc/testing/prd.md`
- [x] TASK-TESTING-055 (PRD-TESTING-002/003) [test_tier_required]: 继续拆分 `release-gate-runtime`，把 `ci-tests.sh full` 收敛为 `full-core` / `full-support` 两个 shard，并把 builtin wasm sync 检查独立成第三个并行 job，由聚合 gate 统一裁决放行，压缩 runtime 关键路径。
  - 产物文件:
    - `.github/workflows/release-packages.yml`
    - `scripts/ci-tests.sh`
    - `doc/testing/prd.md`
  - 验收命令 (`test_tier_required`):
    - `bash -n scripts/ci-tests.sh`
    - `python -c "import pathlib, yaml; yaml.safe_load(pathlib.Path('.github/workflows/release-packages.yml').read_text())"`
    - `rg -n "full-core|full-support|release-gate-runtime-core|release-gate-runtime-support|release-gate-runtime-sync" scripts/ci-tests.sh .github/workflows/release-packages.yml doc/testing/prd.md`
- [x] TASK-TESTING-056 (PRD-TESTING-002/003) [test_tier_required]: 基于 `runtime-core` 热点复盘重平衡 shard，把 `oasis7 --features wasmtime --lib --bins` 从 `full-core` 挪到 `full-support`，降低最长 runtime shard。
  - 产物文件:
    - `scripts/ci-tests.sh`
    - `doc/testing/prd.md`
  - 验收命令 (`test_tier_required`):
    - `bash -n scripts/ci-tests.sh`
    - `rg -n "full-core|full-support|oasis7 --features wasmtime --lib --bins" scripts/ci-tests.sh doc/testing/prd.md doc/testing/project.md`
- [x] TASK-TESTING-057 (PRD-TESTING-WEB-001/002/003) [test_tier_required]: 为 `renderMode=software_safe` 补专用 prompt/chat 回归方案与 `viewer-software-safe-chat-regression.sh`，覆盖 apply/rollback/chat ack、消息流采样以及 `agent_spoke` 缺失签名。
  - 产物文件:
    - `scripts/viewer-software-safe-chat-regression.sh`
    - `testing-manual.md`
    - `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
    - `doc/testing/manual/web-ui-agent-browser-closure-manual.project.md`
  - 验收命令 (`test_tier_required`):
    - `bash -n scripts/viewer-software-safe-chat-regression.sh`
    - `./scripts/viewer-software-safe-chat-regression.sh --help`
    - `./scripts/viewer-software-safe-chat-regression.sh --viewer-static-dir /tmp/aw-viewer-dist-promptchat3 --viewer-port 4373 --live-bind 127.0.0.1:5323 --web-bind 127.0.0.1:5311`
- [x] TASK-TESTING-058 (PRD-TESTING-WEB-001/002/003) [test_tier_required]: 为 software-safe 消息流回归补 `agent_chat -> AgentSpoke` 的 env-gated runtime echo 验证路径，并修正 runtime 事件形状兼容解析。
  - 产物文件:
    - `crates/oasis7/src/viewer/runtime_live/control_plane.rs`
    - `crates/oasis7/src/viewer/runtime_live/tests.rs`
    - `crates/oasis7_viewer/software_safe.js`
    - `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
  - 验收命令 (`test_tier_required`):
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime_agent_chat_echo_env_enqueues_agent_spoke_virtual_event -- --nocapture`
    - `node --check crates/oasis7_viewer/software_safe.js`
    - `agent-browser` 手工链路：`open software_safe -> sendAgentChat -> runSteps -> getState().chatHistory` 观察 `source=event` 的 `AgentSpoke` 记录
- [x] TASK-TESTING-059 (PRD-TESTING-004) [test_tier_required]: 对 `doc/testing/**` 的仍可读历史专题执行 title-only cleanup，将首行 `oasis7*` 公开标题统一切到 `oasis7*`，保留正文历史证据原文不动。
- [x] TASK-TESTING-060 (PRD-TESTING-004) [test_tier_required]: 清理 `doc/testing/launcher/**` 活跃专题里仍作为当前真值出现的旧品牌 crate/path/env/command，引文统一到 `oasis7*` / `OASIS7_*`。
  - 产物文件:
    - `doc/testing/prd.md`
    - `doc/testing/project.md`
    - `doc/testing/ci/*.md`
    - `doc/testing/governance/*.md`
    - `doc/testing/launcher/*.md`
    - `doc/testing/longrun/*.md`
    - `doc/testing/manual/*.md`
    - `doc/testing/performance/*.md`
    - `doc/devlog/2026-03-19.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "^# oasis7|^# oasis7 Runtime|^# oasis7 Simulator|^# oasis7 Viewer" doc/testing --glob '!third_party/**'`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-TESTING-061 (PRD-TESTING-004) [test_tier_required]: 清理 `doc/testing/{longrun,governance,performance,ci,manual}` 活跃专题里已完成改名但文档仍残留的旧品牌 crate/path/env 当前真值。
- [x] TASK-TESTING-062 (PRD-TESTING-006) [test_tier_required]: 新增“Token 创世分配审计清单（2026-03-22）”专题 PRD / design / project 与执行模板，覆盖比例、custody/treasury 语义、个人上限、创世流通与首年释放门禁，并同步 `testing` / `p2p token` 追踪。
  - 产物文件:
    - `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.prd.md`
    - `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.design.md`
    - `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.project.md`
    - `doc/testing/evidence/token-genesis-allocation-audit-template-2026-03-22.md`
    - `doc/testing/prd.md`
    - `doc/testing/project.md`
    - `doc/testing/prd.index.md`
    - `doc/testing/README.md`
    - `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.project.md`
    - `doc/devlog/2026-03-22.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "sum=10000 bps|genesis_liquid|1500 bps|5000 bps|custody|treasury|verdict" doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.prd.md doc/testing/evidence/token-genesis-allocation-audit-template-2026-03-22.md doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-TESTING-063 (PRD-TESTING-004) [test_tier_required]: 执行 ROUND-009 首批手册载体规范化，为 Web UI 闭环补 canonical `*.manual.md` 操作手册，并同步主手册、模块入口与 PRD/project 职责边界。

- 当前阻断摘要：`doc/testing/openclaw-dual-mode-t4-blocker-2026-03-16.md`

## 依赖
- 模块设计总览：`doc/testing/design.md`
- doc/testing/prd.index.md
- `testing-manual.md`
- `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
- `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
- `scripts/ci-tests.sh`
- `.github/workflows/*`
- `.agents/skills/prd/check.md`

## 状态
- 更新日期: 2026-03-30
- 当前状态: active
- 下一任务: 等待 `producer_system_designer` / `runtime_engineer` 提供真实创世账户表后，用 `token-genesis-allocation-audit-template-2026-03-22` 执行首轮正式审计。
- 最新完成: `TASK-TESTING-063`（已为 Web UI 闭环建立 canonical `*.manual.md` 操作手册，并将 `testing-manual`、testing README 与 PRD/project 职责边界同步收口。）
- 最新完成: `TASK-TESTING-062`（已建立 Token 创世分配 QA 审计清单专题与执行模板，冻结比例/个人上限/流通边界/custody 语义的 QA 门禁。）
- 最新完成: `TASK-TESTING-061`（已清理 `doc/testing/{longrun,governance,performance,ci,manual}` 活跃专题中的旧品牌 crate/path/env 当前真值，统一到 `oasis7*` / `OASIS7_*`）。
- 最新完成: `TASK-TESTING-060`（已清理 `doc/testing/launcher/**` 活跃专题中的旧品牌 crate/path/env/command 当前真值，统一到 `oasis7*` / `OASIS7_*`）。
- 最新完成: `TASK-TESTING-059`（已完成 `doc/testing/**` 历史专题首行标题的 title-only cleanup，旧 `oasis7*` 公开标题已统一切到 `oasis7*`）。
- 最新完成: `TASK-TESTING-058`（为 software-safe 消息流回归补 env-gated runtime echo 与 runtime 事件兼容解析，手工链路已能稳定观测 `AgentSpoke` 进入 `chatHistory`）。
- 最新完成: `TASK-TESTING-057`（为 `renderMode=software_safe` 补专用 prompt/chat 回归方案与执行脚本，沉淀 `agent_spoke` 缺失签名与证据包）。
- 最新完成: `TASK-TESTING-056`（基于 `runtime-core` 热点复盘重平衡 shard，把 `oasis7 --lib --bins` 从 `full-core` 挪到 `full-support`，降低最长 runtime shard）。
- 最新完成: `TASK-TESTING-055`（拆分 `release-gate-runtime` 为 core/support/sync 三个并行 job，并给 `ci-tests.sh` 增加 `full-core` / `full-support` shard 入口）。
- 最新完成: `TASK-TESTING-054`（继续优化 release 关键路径，让 `release-gate-web`/`build-web-dist` 共享 Web wasm cache，并把 bundle 原生二进制构建收敛到单次 cargo 调用）。
- 最新完成: `TASK-TESTING-053`（优化 release packaging Web 资产复用链路，避免 package-native 每平台重复 trunk 安装与构建）。
- 最新完成: `TASK-TESTING-052`（补前期工业引导 required-tier 手动卡组互链与 testing-manual 跳转入口）。
- 最新完成: `TASK-TESTING-051`（为 bundle-first 试玩入口增加 freshness manifest 守卫，默认阻断或自动重建 stale bundle）。
- 最新完成: `TASK-TESTING-050`（固化 headed Viewer Web 的默认硬件 WebGL 启动参数，并把 headed 命中 `SwiftShader` / software renderer 统一收口为环境阻断）。
- 最新完成: `TASK-TESTING-049`（修复 `run-producer-playtest.sh --open-headed` 退出后残留浏览器窗口的问题，为脚本补充自动关会话收尾）。
- 最新完成: `TASK-TESTING-006`（testing 模块 README 入口索引同步）。
- 阶段收口优先级: `P0`
- 阶段 owner: `qa_engineer`（联审：`producer_system_designer`）
- 阻断条件: 在 `TASK-TESTING-002/003` 完成前，跨模块发布评审不得声称“测试范围明确且证据齐备”。
- 承接约束: `TASK-TESTING-002` 完成后进入 `TASK-TESTING-003`；`TASK-TESTING-004` 作为趋势化建设保留在其后。
- 专题映射状态: 2026-03-02 批次 3/3 已纳入模块项目管理文档。
- 专题映射状态补充: 2026-03-06 批次 1/1 已纳入模块项目管理文档。
- 专题映射状态补充: 2026-03-08 批次 1/1 已完成（启动器全功能可用性审查）。
- 专题映射状态补充: 2026-03-10 批次 1/1 已完成（启动器人工测试清单建档）。
- headless-runtime 长稳门禁联动: 已通过 `doc/headless-runtime/templates/headless-runtime-release-gate-linkage.md` 约定证据包字段映射。
- PRD 质量门状态: strict schema 已对齐（含第 6 章验证与决策记录）。
- 模块进展补充（2026-03-11）: 已新增 `doc/testing/evidence/testing-quality-trend-baseline-2026-03-11.md`，以 launcher / game / runtime 三个近期样本建立首次通过率、阶段内逃逸率与修复时长 baseline。
- 说明: 本文档仅维护 testing 模块设计执行状态；过程记录在 `doc/devlog/2026-03-10.md` 与 `doc/devlog/2026-03-11.md`。

## 阶段收口角色交接
### Meta
- Handoff ID: `HO-CORE-20260310-TEST-001`
- Date: `2026-03-10`
- From Role: `producer_system_designer`
- To Role: `qa_engineer`
- Related Module: `testing`
- Related PRD-ID: `PRD-TESTING-001/002/003`
- Related Task ID: `TASK-TESTING-002/003`
- Priority: `P0`
- Expected ETA: `待接收方确认`

### Objective
- 目标描述：建立统一的测试触发矩阵与发布证据包模板，使发布评审不再依赖临时判断。
- 成功标准：任一任务都能反推必跑测试，证据包字段统一且可映射到 PRD-ID / 任务 / 结论。
- 非目标：本轮不要求先完成长期趋势统计。

### Current State
- 当前实现 / 文档状态：`TASK-TESTING-002/003/004` 已完成；testing 模块现已具备触发矩阵、证据包模板与首份趋势 baseline。
- 已确认事实：core 已将 testing 触发矩阵与证据包列为 `P0`。
- 待确认假设：S0~S10 触发矩阵是否需要对现有专题任务映射做进一步合并。
- 当前失败信号 / 用户反馈：测试可跑但“该跑什么、结果怎么看、能不能放”仍缺统一模板。

### Scope
- In Scope: `TASK-TESTING-002`、`TASK-TESTING-003`、`TASK-TESTING-004`（已完成）。
- Out of Scope: 本轮不实现自动趋势面板或脚本化采集。

### Inputs
- 关键文件：`doc/testing/project.md`、`doc/testing/prd.md`、`testing-manual.md`。
- 关键命令：`scripts/ci-tests.sh`、现有 viewer / launcher / playability 闭环命令。
- 上游依赖：各模块现有 `test_tier_required/full` 定义与证据产物。
- 现有测试 / 证据：`2026-03-08` / `2026-03-10` 的 launcher / viewer 闭环与人工清单结果。

### Requested Work
- 工作项 1：完成 S0~S10 与改动路径触发矩阵。
- 工作项 2：建立发布证据包模板（命令、日志、截图、结论）。
- 工作项 3：与 core PRD-ID 映射模板对齐引用方式。

### Expected Outputs
- 代码改动：如需，仅限测试脚本或模板支撑变更。
- 文档回写：`doc/testing/project.md`、相关 testing 分册。
- 测试记录：补齐 `test_tier_required` 的模板验证证据。
- devlog 记录：记录矩阵、模板和遗留趋势项。

### Done Definition
- [ ] 输出满足目标与成功标准
- [ ] 影响面已核对 `producer_system_designer` 与关键模块 owner
- [ ] 对应 `prd.md` / `project.md` 已回写
- [ ] 对应 `doc/devlog/YYYY-MM-DD.md` 已记录
- [ ] required 证据已补齐

### Risks / Decisions
- 已知风险：若先做趋势统计而不先统一触发矩阵与证据包，数据口径会继续漂移。
- 待拍板事项：证据包目录结构是否需要与现有 `output/` 产物强绑定。
- 建议决策：先完成 `002/003`，再推进 `004` 趋势统计。

### Validation Plan
- 测试层级：`test_tier_required`
- 验证命令：以 `rg` 抽样矩阵 / 模板字段，并结合现有闭环产物路径做引用验证。
- 预期结果：任一阶段收口任务都能映射到统一测试范围与证据包格式。
- 回归影响范围：全模块测试治理与发布评审流程。

- 模块进展补充（2026-03-10）: 已新增 `doc/testing/evidence/release-evidence-bundle-task-game-018-2026-03-10.md`，把 `TASK-GAME-018` 的 S6 证据、viewer 定向回归与 playability 卡片纳入统一 testing 证据包。

### Handoff Acknowledgement
- 接收方确认范围：`已接收 TASK-TESTING-002/003；本轮覆盖触发矩阵与发布证据包模板，不含趋势统计`
- 接收方确认 ETA：`TASK-TESTING-002/003/004 已完成，模块主项目已收口`
- 接收方新增风险：`长跑 / UI 产物目录在不同脚本间仍有差异，当前模板先统一字段，不强制统一物理目录`
