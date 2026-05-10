# Viewer Web Software-Safe Mode（2026-03-16）

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.project.md`

审计轮次: 3

## 目标
- 将 `software_safe` 固定为当前仓库唯一正式 Web Viewer 入口。
- 清除 `standard_3d` 删除后残留的双入口、兼容跳转和旧 QA 口径。
- 保证 launcher、手册、回归脚本与站点镜像围绕同一入口事实收口。

## 范围
- 范围内:
  - `software_safe.html`
  - `software_safe.js`
  - `software_safe_src/**`
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
- Launcher 输出: 必须显式注入 `render_mode=software_safe`
- 静态入口: `software_safe.html`
- 测试契约: `software_safe.js` 暴露的 `__AW_TEST__` 状态与动作接口
- 验证数据:
  - Web 入口 freshness 检查结果
  - `viewer-primary-web-entry-regression.sh` evidence
  - `software_safe` 手册与站点镜像内容

## 里程碑
- M1: 删除 `standard_3d` 代码、静态入口与相关脚本。
- M2: launcher / Web 启动 / regression 全量改为 `software_safe` 单入口。
- M3: 手册、PRD、project、testing 文档完成真值回写并通过治理校验。

## 风险
- 历史文档或审读清单继续引用已删除 3D 路径，导致 doc-governance 失败。
- launcher 或脚本残留旧 `render_mode=standard` 参数，造成入口分叉。
- `software_safe` 入口如果与 `__AW_TEST__` 契约漂移，会破坏现有 Web 回归闭环。

## 1. Executive Summary
- Problem Statement: 仓库删除 `standard_3d` 代码、静态入口与相关 QA 工具后，Web Viewer 只剩 `software_safe`。本专题需要把旧“双入口/兼容切换”口径收口到单一事实。
- Proposed Solution: 将 `software_safe` 固定为唯一正式 Web Viewer 入口；所有 Web 启动、回归、launcher URL、manual 与 evidence 都围绕 `software_safe.html` 与对应 `software_safe.js`/`__AW_TEST__` 契约组织。
- Success Criteria:
  - SC-1: 默认 Web 入口始终落到 `software_safe`。
  - SC-2: `software_safe` 继续承接连接、观察、目标选择、prompt/chat/rollback、canonical gameplay summary 与 blocked/handoff surface。
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
- As a 玩家 / QA / 制作人, I want `software_safe` to remain the only formal Web entry, so that the browser path has one unambiguous source of truth.
- As a `viewer_engineer`, I want all Web-facing scripts and docs to point only at `software_safe`, so that no stale 3D fallback contract survives.

## 4. Technical Specifications
### 4.1 Entry Contract
- 默认 URL:
  - `http://<host>:<port>/?ws=ws://<web-bind>`
- `oasis7_game_launcher` 生成的游戏页 URL 必须显式注入 `render_mode=software_safe`
- `software_safe.html` 是唯一静态入口页面

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
- `run-viewer-web.sh` 继续在启动前重建 `software_safe`

## 5. Acceptance Criteria
- AC-1: `run-viewer-web.sh` 只构建并服务 `software_safe`。
- AC-2: `viewer-primary-web-entry-regression.sh` 只验证默认入口与 `software_safe` 契约，不再验证 `render_mode=standard`。
- AC-3: `software_safe.js` 不再包含“打开标准 Viewer”或等价文案。
- AC-4: Viewer 手册与站点镜像不再引用已删除的 3D/截图/纹理脚本。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-039 | T0-T6 | `test_tier_required` | `./scripts/doc-governance-check.sh` + browser regression + 残留 grep | Web 入口、脚本与文档真值 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-WS-039-1` | `software_safe` 成为唯一 Web 入口 | 保留已删实现的标准 Viewer 口径 | 当前仓库已无对应代码与静态入口。 |
