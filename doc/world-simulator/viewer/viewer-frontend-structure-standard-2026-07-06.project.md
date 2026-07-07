# viewer-frontend-structure-standard-2026-07-06 项目管理

- 对应需求文档: `doc/world-simulator/viewer/viewer-frontend-structure-standard-2026-07-06.prd.md`
- 对应设计文档: `doc/world-simulator/viewer/viewer-frontend-structure-standard-2026-07-06.design.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] viewer-frontend-structure-standard-baseline (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 建立 Viewer Web `js/html/jsx` 结构标准，定义 source/generated/compat taxonomy、分层模型、拆分触发条件、accepted/rejected split patterns 与验证矩阵。 Trace: #2119 (task_15caf5a4ca0c4924967c388b0d510954)
- [ ] viewer-frontend-tooling-gate-evaluation (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 评估是否引入 ESLint/Prettier 或现有 formatter/lint wrapper 来自动化 JS/JSX/HTML 结构卫生；必须先给出规则集合、CI 成本和存量修复策略。 Trace: #2119 (task_15caf5a4ca0c4924967c388b0d510954)
- [ ] viewer-legacy-core-facade-burndown-next-slice (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 下次触碰 `legacy_core.js` 时按本标准继续抽离一个 coherent boundary，并记录 before/after line counts、owner 与验证命令。 Trace: #2119 (task_15caf5a4ca0c4924967c388b0d510954)
- [ ] viewer-main-jsx-component-boundary-next-slice (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 下次触碰 `main.jsx` 大型 UI surface 时，优先抽离一个 named widget/feature component 或 display model helper，并补对应 UI/narrow test。 Trace: #2119 (task_15caf5a4ca0c4924967c388b0d510954)

## 当前任务证据
- 用户问题: 当前前端 `js/html/jsx` 是否缺少拆分和抽象规范；随后要求搜索外部备选并拼一套 Viewer 标准。
- 外部研究摘要: Feature-Sliced Design 适合作为结构主框架；Solid docs 适合作为组件/props/JSX 基线；Google HTML/CSS、ESLint、Prettier 适合作为卫生和自动化 gate；Atomic Design / Airbnb 只作为局部参考。
- 本任务产物: 新增本三件套，并更新 Viewer README、world-simulator PRD index、`viewer_engineer` / `repository_health_engineer` / `qa_engineer` 角色卡。
- 验证结果:
  - `git diff --check`: passed.
  - `./scripts/pm/workflow-lint.sh --task-uid task_15caf5a4ca0c4924967c388b0d510954 --phase current`: passed in the canonical task worktree with GitHub issue evidence for #2119.
  - 本任务局部检查 passed: PRD canonical section headings present; triplet mutual links present; `viewer/README.md`, `prd.index.md`, and related role cards expose the standard; touched files have no trailing whitespace.
  - `./scripts/doc-governance-check.sh`: initially blocked by disk-full temp-file failure; rerun after disk space recovered on 2026-07-07 and passed (`doc-governance-check: OK`).
  - `./scripts/pm/lint.sh`: passed.
  - new-doc whitespace check via `git diff --check --no-index /dev/null <new-doc>` before staging: passed for PRD, design, and project files.
  - project task-row policy regex matching `scripts/doc-governance-check.sh`: passed for `Trace: #2119 (task_15caf5a4ca0c4924967c388b0d510954)`.
- 专业复核:
  - `viewer_engineer`: standard is technically appropriate for Viewer Web/SolidJS/generated-artifact reality; P1 was PRD heading governance shape, fixed by adding canonical `目标` / `范围` / `接口 / 数据` / `里程碑` / `风险` sections.
  - `repository_health_engineer`: no conflict with `viewer-web-single-source-build-truth` or Rust governance; P1 same PRD heading issue, fixed; P2 evidence writeback gap, addressed in this project evidence section.
  - `qa_engineer`: verification matrix is testable for docs-only baseline; stale doc-governance blocker and untracked-file whitespace coverage findings were fixed and reverified.
  - 2026-07-07 optimization pass: normalized PRD headings/table labels, removed stale section-number references, refreshed README metadata, and aligned task acceptance with the implemented standard scope.

## 依赖
- `doc/world-simulator/viewer/viewer-web-single-source-build-truth-2026-05-19.{prd,design,project}.md`
- `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.{prd,project}.md`
- `doc/world-simulator/viewer/viewer-page-module-design-2026-06-18.design.md`
- `crates/oasis7_viewer/package.json`
- `crates/oasis7_viewer/vite.software-safe.config.mjs`
- `crates/oasis7_viewer/scripts/finalize-software-safe-build.mjs`
- `testing-manual.md`

## 状态
- 更新日期: 2026-07-07
- 当前状态: baseline_standard_added_and_review_clean
- 当前任务: `task_15caf5a4ca0c4924967c388b0d510954`
- 下一任务: 若该标准在 review 中通过，后续按 project 中的 tooling gate、legacy_core facade burn-down、main.jsx component boundary slices 进入独立 GitHub-backed tasks。
