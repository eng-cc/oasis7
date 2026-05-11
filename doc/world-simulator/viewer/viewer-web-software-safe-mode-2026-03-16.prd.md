# Viewer Web Software-Safe Mode（2026-03-16）

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.project.md`

审计轮次: 3

## 目标
- 将 `viewer` 固定为当前仓库唯一正式 Web Viewer / UI 入口，并把 `software_safe` 降为兼容 alias。
- 清除 `standard_3d` 删除后残留的双入口、兼容跳转和旧 QA 口径。
- 保证 launcher、手册、回归脚本与站点镜像围绕同一入口事实收口。

## 范围
- 范围内:
  - 发布产物 `viewer.html`（canonical）/ `software_safe.html`（compat）
  - 发布产物 `viewer.js`（canonical）/ `software_safe.js`（compat）
  - 源码与构建入口 `software_safe.html` / `software_safe.js` / `software_safe_src/**`
  - `scripts/run-viewer-web.sh`
  - `scripts/viewer-primary-web-entry-regression.sh`
  - `scripts/viewer-software-safe-step-regression*.sh`
  - `testing-manual.md`
  - `doc/world-simulator/viewer/viewer-manual*.md`
- 范围外:
  - 恢复任何 `standard_3d` 代码、入口或静态资源。
  - 维护 3D visual QA、截图、贴图、theme 资产链路。
  - 扩大当前 Web Viewer 能力边界到本专题之外的新玩法。

## 接口 / 数据
- 入口 URL: `http://<host>:<port>/?ws=ws://<web-bind>`
- Launcher 输出: 必须显式注入 `render_mode=viewer`
- 静态入口: 发布产物 `viewer.html`（兼容保留 `software_safe.html`）；源码页面文件当前仍由 `software_safe.html` 生成
- 测试契约: 发布产物 `viewer.js` / `software_safe.js` 暴露同一套 `__AW_TEST__` 状态与动作接口；源码 bundle 文件当前仍由 `software_safe.js` 生成
- 验证数据:
  - Web 入口 freshness 检查结果
  - `viewer-primary-web-entry-regression.sh` evidence
  - `viewer` 手册与站点镜像内容

## 里程碑
- M1: 删除 `standard_3d` 代码、静态入口与相关脚本。
- M2: launcher / Web 启动 / regression 全量改为 `viewer` canonical 单入口，并保留 `software_safe` 兼容 alias。
- M3: 手册、PRD、project、testing 文档完成真值回写并通过治理校验。

## 风险
- 历史文档或审读清单继续引用已删除 3D 路径，导致 doc-governance 失败。
- launcher 或脚本残留旧 `render_mode=standard` 参数，造成入口分叉。
- `viewer` canonical 入口如果与兼容 alias / `__AW_TEST__` 契约漂移，会破坏现有 Web 回归闭环。

## 1. Executive Summary
- Problem Statement: 仓库删除 `standard_3d` 代码、静态入口与相关 QA 工具后，Web Viewer 只剩一条正式 UI 入口，但 canonical 名称长期停留在 `software_safe`，与当前产品定位不匹配。本专题需要把旧命名和兼容切换口径收口到单一事实。
- Proposed Solution: 将 `viewer` 固定为唯一正式 Web Viewer / UI 入口；所有 Web 启动、回归、launcher URL、manual 与 evidence 都围绕 `viewer` canonical 入口与对应 `__AW_TEST__` 契约组织，同时保留 `software_safe` 文件名与 URL 参数作为兼容 alias。
- Success Criteria:
  - SC-1: 默认 Web 入口始终落到 `viewer` canonical 入口；旧 `software_safe` 链接继续兼容。
  - SC-2: `viewer` 继续承接连接、观察、目标选择、prompt/chat/rollback、canonical gameplay summary 与 blocked/handoff surface。
  - SC-3: 活跃文档、脚本和 evidence 不再要求 `render_mode=standard` 或任何标准 Viewer 入口。

## 2. User Experience & Functionality
- In Scope:
  - `software_safe.html`
  - `software_safe.js`
  - `software_safe_src/**`
  - `scripts/run-viewer-web.sh`
  - `scripts/viewer-primary-web-entry-regression.sh`
  - `scripts/viewer-software-safe-step-regression*.sh`
  - `testing-manual.md`
  - `viewer-manual*.md`
- Out of Scope:
  - 不恢复 3D/标准 Viewer。
  - 不维护 visual QA / screenshot / texture/theme 资产链路。

## 3. User Stories
- As a 玩家 / QA / 制作人, I want `viewer` to remain the only formal Web / UI entry, so that the browser path has one unambiguous source of truth.
- As a `viewer_engineer`, I want all Web-facing scripts and docs to point to `viewer` as the canonical name, so that no stale 3D fallback contract or old naming ambiguity survives.

## 4. Technical Specifications
### 4.1 Entry Contract
- 默认 URL:
  - `http://<host>:<port>/?ws=ws://<web-bind>`
- `oasis7_game_launcher` 生成的游戏页 URL 必须显式注入 `render_mode=viewer`
- `viewer.html` 是 dist / bundle / release 的唯一 canonical 静态入口页面；`software_safe.html` 只保留兼容别名，源码页面文件暂不改名

### 4.2 Capability Envelope
- 必须保留：
  - 连接状态
  - 世界摘要
  - 目标列表与详情
  - canonical gameplay summary
  - blocked / handoff surface
  - auth/bootstrap 下的 prompt/chat/rollback
  - `__AW_TEST__` 状态与动作契约
- 不再保留：
  - 标准 Viewer 跳转
  - 3D / visual QA 入口
  - texture/theme/截图链路

### 4.3 Freshness
- source-tree Web 入口必须继续阻断 stale dist
- `run-viewer-web.sh` 继续在启动前重建 `viewer` canonical dist，并同步产出兼容 alias 文件；源码 bundle / HTML 文件名仍保留 `software_safe.*`

## 5. Acceptance Criteria
- AC-1: `run-viewer-web.sh` 构建并服务 `viewer` canonical dist，同时保留 `software_safe` 兼容副本。
- AC-2: `viewer-primary-web-entry-regression.sh` 只验证默认入口与 `viewer` 契约，不再验证 `render_mode=standard`；对 `software_safe` 仅做兼容通过处理。
- AC-3: 运行态 `renderMode`、launcher URL、手册和发布产物都不再把 `software_safe` 当作 canonical UI 名称。
- AC-4: Viewer 手册与站点镜像不再引用已删除的 3D/截图/纹理脚本。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-039 | T0-T6 | `test_tier_required` | `./scripts/doc-governance-check.sh` + browser regression + 残留 grep | Web 入口、脚本与文档真值 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-WS-039-1` | `viewer` 成为唯一 canonical Web / UI 入口，`software_safe` 仅保留兼容 alias | 保留已删实现的标准 Viewer 口径，或继续让 `software_safe` 作为 canonical 名称 | 当前仓库已无第二条正式 UI 入口，实现与公开命名必须一致。 |
