# Viewer Web Software-Safe Mode 设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.project.md`

审计轮次: 3

## 1. 设计定位
`viewer` 是唯一正式 Web Viewer / UI 入口；`software_safe` 只保留为兼容 alias。设计目标不再是“主入口 + 标准 Viewer 分流”，而是围绕单一静态入口维护稳定浏览器玩法与观测闭环。

## 2. 核心设计决策
- 保留 `software_safe.html` + `software_safe.js` + `software_safe_src/**` 作为源码/兼容资产，同时在 dist / bundle / release 中额外产出 `viewer.html` + `viewer.js`。
- 删除标准 Viewer 跳转、3D 模式入口与相关 UI 文案。
- 保留 `__AW_TEST__` 作为统一自动化契约。
- 保留 freshness gate，防止 stale dist。

## 3. 设计结构
### 3.1 Entry
- `run-viewer-web.sh` 负责构建并服务 `viewer` canonical dist，同时保留 `software_safe` 兼容副本
- `oasis7_game_launcher` 负责生成 `render_mode=viewer` 的 URL

### 3.2 UI
- 继续提供世界摘要、目标列表、详情、事件流、blocked/handoff、prompt/chat/rollback
- `Language and Viewer Entry` 菜单只保留语言切换

### 3.3 Automation
- `viewer-primary-web-entry-regression.sh` 验证默认入口
- `viewer-software-safe-step-regression*.sh` 验证 gameplay/blocked 契约

## 4. 关键约束
- 不再暴露标准 Viewer 跳转
- 不再维护 `render_mode=standard` 的当前产品语义
- 不再维护 texture/theme/capture 工具链
