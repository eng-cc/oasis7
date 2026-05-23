# Viewer Web 单一真值构建与 Legacy Core 拆分（2026-05-19）

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-single-source-build-truth-2026-05-19.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-single-source-build-truth-2026-05-19.project.md`
- 关联主专题:
  - `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.prd.md`
  - `doc/world-simulator/viewer/viewer-pixel-world-bridge-render-optimization-2026-05-17.prd.md`
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`

审计轮次: 1

## 1. Executive Summary
- 当前 `crates/oasis7_viewer/software_safe_src/legacy_core.js` 同时承载状态、auth、反馈、控制发送、DOM 渲染与 bootstrap，多职责耦合导致主入口继续演进时改动半径过大。
- 当前 viewer Web 产物流在文档上宣称 `viewer.js` 为 canonical 发布名，但仓库实际只跟踪 `software_safe.js` 生成 bundle，再由脚本复制成 `viewer.js`，`pixel-world-bridge/` 生成目录也缺少同一条明确的 canonical 生成链。
- 本专题要同时解决两类工程债：把 `legacy_core.js` 下沉成多模块实现；把 Web 生成产物收口为明确的单一真值流程，避免继续由 compat 名称承担 canonical 语义。

## 目标
- 将 `software_safe_src` 主入口实现从单个 `legacy_core.js` god module 拆成职责明确的子模块，同时保留现有对外 API 与测试入口。
- 将 Viewer Web 生成 bundle 的 canonical 真值收口到 `viewer.js`，把 `software_safe.js` 降级为兼容 alias，而不是反过来。
- 将 `pixel-world-bridge/` 生成目录纳入与主 bundle 同一条显式 finalize 流，统一表达“哪些文件是源码真值，哪些文件是 generated artifacts，哪些文件只是 compat alias”。

## 范围
- 范围内：
  - `crates/oasis7_viewer/software_safe_src/**`
  - `crates/oasis7_viewer/scripts/finalize-software-safe-build.mjs`
  - `crates/oasis7_viewer/software_safe.js`
  - `crates/oasis7_viewer/viewer.js`
  - `crates/oasis7_viewer/pixel-world-bridge/**`
  - 直接消费上述文件的 Viewer Web dist / bundle / regression scripts
- 范围外：
  - runtime 协议、world DTO、Prompt / Chat / hosted access 业务语义变更
  - Pixel world bridge 的渲染行为、host fallback 语义或 wasm ABI 调整
  - `viewer` / `software_safe` taxonomy rename 或 public copy 改版

## 2. User Experience & Functionality

## 3. User Stories
- As a `viewer_engineer`, I want the software-safe entry implementation split into multiple modules, so that evolving auth, rendering, or command surfaces no longer requires editing one 4k+ line file.
- As a `qa_engineer`, I want the Web bundle and pixel-world runtime artifacts to have one explicit finalize flow, so that repo-owned tests and bundle freshness checks can tell canonical assets from compat aliases.
- As a `producer_system_designer`, I want the viewer entry docs to match repo truth about `viewer.js` vs `software_safe.js`, so that the canonical browser entry no longer depends on an implementation mismatch.

## 4. Technical Specifications

### 4.1 Legacy Core Split Boundary
- `legacy_core.js` 保留为单入口 facade，允许继续作为 `main.jsx`、`pixel_world_host.jsx` 与现有测试的稳定 import path。
- 主实现必须下沉到 `software_safe_src/` 子模块，至少把以下职责拆开：
  - viewer state / locale / render hook / snapshot-derived utilities
  - auth / hosted access / session surface derivation
  - semantic feedback / gameplay summary / display model
  - DOM rendering / event binding / bootstrap composition
- 拆分过程中不得改动以下对外合同：
  - `initializeSoftwareSafeCore()`
  - `__AW_TEST__`
  - `state`
  - 现有 `main.jsx` / `pixel_world_host.jsx` / repo-owned tests 依赖的导出函数名

### 4.2 Generated Artifact Single Source of Truth
- `viewer.js` 必须成为仓库内 canonical Viewer Web bundle 名称。
- `software_safe.js` 只允许作为 compat alias，且其实现必须显式指向 `viewer.js`，不能再承载独立 bundle 真值。
- `software_safe.html` 可以继续作为源码页面文件，但引用脚本时必须对齐 canonical bundle 真值，而不是继续把 compat bundle 当唯一入口。
- `pixel-world-bridge/` 下的 JS / wasm bindgen 产物必须继续由 finalize 脚本生成，但其 canonical 生成边界要与 `viewer.js` 同步写死在同一条 build flow 中。

### 4.3 Script and Bundle Contract
- 所有 Web dist / bundle / rebuild helper 必须按 canonical -> compat 的方向复制：
  - `viewer.html` / `viewer.js` 为 canonical
  - `software_safe.html` / `software_safe.js` 为 compat alias
- freshness / bundle manifest / browser rebuild helper 必须把 canonical `viewer.js` 纳入 source-of-truth scope。
- 若保留 checked-in generated artifact，则必须由单一 finalize 脚本负责写入，避免多个脚本分别“顺手生成”不同变体。

## 接口 / 数据
- 源码入口：
  - `crates/oasis7_viewer/software_safe_src/main.jsx`
  - `crates/oasis7_viewer/software_safe_src/pixel_world_host.jsx`
  - `crates/oasis7_viewer/software_safe_src/legacy_core.js`
- 生成产物：
  - `crates/oasis7_viewer/viewer.js`
  - `crates/oasis7_viewer/software_safe.js`
  - `crates/oasis7_viewer/pixel-world-bridge/*`
- 相关脚本：
  - `crates/oasis7_viewer/scripts/finalize-software-safe-build.mjs`
  - `scripts/run-viewer-web.sh`
  - `scripts/agent-browser-lib.sh`
  - `scripts/build-game-launcher-bundle.sh`
  - `scripts/bundle-freshness-lib.sh`

## 里程碑
- M1：冻结拆分边界、canonical artifact 命名与 compat alias 关系。
- M2：完成 `legacy_core.js` facade + 子模块拆分。
- M3：完成 finalize/build/dist 脚本对 canonical bundle 的一致性调整。
- M4：回跑 repo-owned UI tests、build、bundle freshness 与相关 smoke。

## 风险
- `legacy_core.js` 仍保留大量 render/bootstrap 组装逻辑，若一次性过拆，最容易引入 UI contract 漂移或测试夹具失效。
- canonical `viewer.js` 与 compat `software_safe.js` 若被其他脚本再次反向复制，会让 freshness / bundle manifest 重新失真。
- `pixel-world-bridge/` 属于 checked-in generated runtime；若 finalize flow 没有成为唯一写入口，后续很容易再次出现“bundle 已更新但 runtime 目录还是旧的”分叉。

## 6. Acceptance Criteria
- AC-1: `legacy_core.js` 不再包含全部主入口实现，而是退化为 facade / export assembly；主实现已下沉到多个职责模块。
- AC-2: `viewer.js` 成为仓库内 canonical Viewer Web bundle 名称；`software_safe.js` 仅作为显式 compat alias。
- AC-3: `software_safe.html`、dist rebuild helper、bundle 打包脚本和 browser regression helper 均按同一 canonical/compat 关系工作，不再依赖“先有 `software_safe.js` 再复制成 `viewer.js`”。
- AC-4: `pixel-world-bridge/` generated runtime 继续可用，但其来源明确绑定到 finalize flow，而不是被当成手工维护源码。
- AC-5: 现有 `npm --prefix crates/oasis7_viewer run test:ui`、`npm --prefix crates/oasis7_viewer run build:software-safe` 与相关 repo-owned Node/browser helper 回归通过。

## 7. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-046 | `task_97820fd5e09a450aadcf988a968faad8` | `test_tier_required` | `npm --prefix crates/oasis7_viewer run test:ui` + `npm --prefix crates/oasis7_viewer run build:software-safe` + repo-owned Node contract test + `git diff --check` | Viewer Web 主入口模块边界、canonical bundle flow、pixel-world generated runtime copy chain |
