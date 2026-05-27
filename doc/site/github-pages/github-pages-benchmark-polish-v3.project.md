# GitHub Pages 对标优化（三期）项目管理文档

- 对应设计文档: `doc/site/github-pages/github-pages-benchmark-polish-v3.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-benchmark-polish-v3.prd.md`

审计轮次: 5

## 审计备注
- 项目主入口：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`。
- 本文件仅维护对标优化（三期）的增量任务。

## 任务拆解

### 0. 文档与基线
- [x] 新增三期设计文档（`doc/site/github-pages/github-pages-benchmark-polish-v3.prd.md`）
- [x] 新增三期项目管理文档（本文件）
- [x] 明确本轮对标落地验收项（内容/视觉/验证）

### 1. 内容结构优化（双语同构）
- [x] Hero 文案与 CTA 重排（体验路径优先）
- [x] 新增 `#path` 三段式理解路径模块（30 秒 / 3 分钟 / 30 分钟）
- [x] 新增 `#proof` 真实运行证据模块（截图/事件/更新）
- [x] 更新中英文导航锚点与 section 顺序一致

### 2. 视觉层级优化
- [x] 新增 Hero 世界观视觉素材（SVG）并接入双语页面
- [x] 收敛通用卡片边框/发光强度，减少“全高亮”视觉噪声
- [x] 强化主 CTA 与关键卡片视觉优先级
- [x] 新增叙事路径与证据模块样式（与既有主题一致）

### 3. 交互与体验微调
- [x] 新增叙事路径步骤高亮交互（滚动触发）
- [x] 确保新增模块在移动端可读（<= 860px）
- [x] 保持无障碍降级（`prefers-reduced-motion`）
- [x] 新增 Proof 场景事件时间线切换（中英文）
- [x] 新增 Proof 切换的键盘可访问性（按钮组）

### 4. 验证与收口
- [x] 使用 agent-browser 进行页面截图回归（中英文）
- [x] 执行 `env -u RUSTC_WRAPPER cargo check`
- [x] 更新 `README.md` 展示站说明（补充三期亮点）
- [x] 回写本项目管理文档状态
- [x] 追加当日开发日志（`doc/devlog/README.md`）
- [x] 回归新增 Proof 切换交互截图（中英文）
- [x] 追加本次增量任务日志（`doc/devlog/README.md`）

## 依赖
- 继续沿用当前 `site/` 静态部署结构与 Pages 工作流。
- 现有截图素材可复用，仅新增轻量 SVG。

## 状态
- 当前阶段：已完成（含 Proof 交互增强）
- 最近更新：完成 Proof 场景事件时间线切换、可访问性与回归校验（2026-02-10）
- 下一步：可继续迭代“事件详情抽屉”或“场景对比视图”增强证据表达。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
