# `world-simulator/viewer` 热点子域入口

更新时间: 2026-04-17

## 从这里开始
- 想执行 Viewer、走 Web 闭环、看命令或手工步骤：先读 `viewer-manual.manual.md`
- 想确认正式浏览器主入口、`software_safe` 边界或弱机/CI 默认路径：先读 `viewer-web-software-safe-mode-2026-03-16.prd.md`
- 想确认 runtime live / event-driven / step-control 现行口径：先读 `viewer-live-full-event-driven-phase10-2026-02-27.prd.md`
- 想确认聊天、右侧面板、Prompt 与输入桥接：先读 `viewer-chat-right-panel-polish.prd.md` 或 `viewer-egui-right-panel.prd.md`
- 想确认 gameplay release / visual QA / commercial polish：先读 `viewer-gameplay-release-experience-overhaul.prd.md`
- 想确认 3D 当前是否在做、是否暂停：先读 `viewer-3d-pause-user-interaction-hold-2026-04-01.prd.md`
- 想精确找某份专题文档，而不是按问题阅读：回到 `../prd.index.md`

## 入口分工
- 当前页只承担 `viewer/` 子目录 landing page 职责，不复制完整长表。
- `viewer-manual.manual.md` 是 Viewer / Web 闭环 / operator 的 canonical 操作手册。
- `../prd.index.md` 是 world-simulator 模块完整文件级索引，适合已知主题后按文件名查找。
- `../README.md` 是 world-simulator 模块级 landing page，负责跨 `viewer / launcher / llm / kernel / scenario / m4` 分流。

## 密度快照
- 治理前快照（`scripts/doc-inventory-report.sh`，2026-04-17）:
  - `doc/world-simulator/viewer/`: 296 份 Markdown
  - `doc/world-simulator/`: 549 份 Markdown
- 当前子域属于仓库最高密度热点路径；本页的目标是压缩首读路径，而不是在本批直接减少文件数。

## 首读主题簇

### 1. 操作手册与执行闭环
- 首读入口: `viewer-manual.manual.md`
- 适合问题:
  - 怎么启动 Viewer
  - Web 闭环怎么跑
  - `software_safe` / bilingual URL / test API 怎么使用
- 说明: 如果你是来“操作”而不是“做治理判断”，这里通常是第一入口。

### 2. `software_safe` 与正式 Web 主入口
- 首读入口:
  - `viewer-web-software-safe-mode-2026-03-16.prd.md`
  - `viewer-web-runtime-fatal-surfacing-2026-03-12.prd.md`
  - `viewer-web-semantic-test-api.prd.md`
- 适合问题:
  - 为什么正式 Web 默认走 `software_safe`
  - 弱机 / CI / 无 GPU 环境下的 canonical 路径是什么
  - 浏览器 fatal、语义测试接口、正式主入口怎么对齐

### 3. runtime live / event-driven / control
- 首读入口:
  - `viewer-live-full-event-driven-phase10-2026-02-27.prd.md`
  - `viewer-live-runtime-world-migration-phase1-2026-03-04.prd.md`
  - `viewer-live-runtime-world-llm-full-bridge-2026-03-05.prd.md`
- 适合问题:
  - runtime live 现在哪些能力已经接管
  - event-driven 阶段的主文档是哪份
  - step/control/live playback 的现行边界是什么
- 说明: `phase8/9` 已物理合并到主文档，当前不应再从旧阶段文件倒推现行口径。

### 4. chat / prompt / right panel
- 首读入口:
  - `viewer-chat-right-panel-polish.prd.md`
  - `viewer-egui-right-panel.prd.md`
  - `viewer-chat-prompt-presets-profile-editing.prd.md`
- 适合问题:
  - 聊天入口、右侧面板、Prompt profile 现在怎样组织
  - 输入法、回车发送、预设编辑这些问题该去哪里看

### 5. release / visual QA / 体验收口
- 首读入口:
  - `viewer-gameplay-release-experience-overhaul.prd.md`
  - `viewer-release-full-coverage-gate.prd.md`
  - `viewer-visual-release-readiness-hardening-2026-03-01.prd.md`
- 适合问题:
  - 首局体验、release readiness、visual QA 的主文档是什么
  - 哪些沉浸阶段已经物理合并，哪些不再是独立首读入口

### 6. 3D / 2D / visual-only 模式
- 首读入口:
  - `viewer-3d-pause-user-interaction-hold-2026-04-01.prd.md`
  - `viewer-visualization-3d.prd.md`
  - `viewer-2d-3d-clarity-improvement.prd.md`
- 适合问题:
  - 3D 当前是暂停、继续还是只保留 QA/视觉用途
  - 2D/3D 表现、清晰度和 visual review 的当前边界是什么

## 定向检索边界
- 如果你已经知道准确文件名，直接回 `../prd.index.md`，不要指望本页替代完整索引。
- 如果你追的是历史状态收口、module status closure 或 review note，允许直接进相应 `.md` supporting spec，但不要把它们重新当作默认首读入口。
- 如果某个主题已经出现“主文档物理合并”，应优先进入主文档，而不是从旧阶段文档开始。

## 维护约定
- 新增 Viewer 专题后，若改变了默认首读路径，应同步更新本页。
- 本页只维护簇级入口，不维护完整文件清单。
- 若未来 `viewer/` 内部继续分裂出更高密度簇，再另开簇内治理专题，而不是把本页扩写成长表。
