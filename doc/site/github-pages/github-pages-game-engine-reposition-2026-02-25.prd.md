# GitHub Pages 游戏+引擎定位重写（2026-02-25）设计文档

- 对应设计文档: `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`

审计轮次: 5

## ROUND-002 主从口径
- 本文件为 github-pages 主文档（master）。
- `doc/site/github-pages/github-pages-architecture-svg-refresh.prd.md`、`doc/site/github-pages/github-pages-benchmark-polish-v3.prd.md`、`doc/site/github-pages/github-pages-content-sync-2026-02-12.prd.md` 为本批增量子文档（slave）。

- 对应标准执行入口: `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`

## 目标
- 将 GitHub Pages 从“世界模拟器”单一叙事，重写为“游戏 + 游戏引擎”双定位叙事。
- 保留现有页面视觉设计优点：深色科技基调、Hero 动态网格、分层卡片、证据切换与滚动 reveal 动效。
- 保持中英文页面结构同构，保证首页与文档中心口径一致。

## 视觉方向（viewer visual ideation）
- Purpose：让首次访问者在 30 秒内理解项目既是可玩的游戏系统，也是可扩展的 WASM 游戏引擎。
- Tone：industrial/utilitarian + editorial（工业科技感 + 清晰信息层级）。
- Constraints：纯静态页面（HTML/CSS/JS），继续兼容现有 `site/assets/app.js` 交互标记。
- Differentiation：保留“可运行证据”模块，但证据内容升级为“玩法循环 + 引擎能力 + 生产链路”三位一体。

## 范围
- 范围内
  - 重写首页：`site/index.html`、`site/en/index.html`。
  - 重写文档中心：`site/doc/cn/index.html`、`site/doc/en/index.html`。
  - 必要的样式微调（不破坏现有视觉优势）：`site/assets/styles.css`。
  - 页面链接改为当前主叙事文档入口（README + game docs + viewer manual）。
- 范围外
  - 不引入新前端框架/构建链路。
  - 不改 `site/assets/app.js` 交互协议。
  - 不扩写 Viewer 手册正文章节（本次聚焦首页与文档中心重写）。

## 接口/数据
- 内容基线
  - `README.md`
  - `doc/game/gameplay/gameplay-top-level-design.prd.md`
  - `doc/game/gameplay/gameplay-engineering-architecture.md`
  - `doc/world-simulator/viewer/viewer-manual.md`
- 页面文件
  - `site/index.html`
  - `site/en/index.html`
  - `site/doc/cn/index.html`
  - `site/doc/en/index.html`
  - `site/assets/styles.css`（可选微调）

## 里程碑
- M0：建档与任务拆解完成。
- M1：首页（CN/EN）完成游戏+引擎定位重写并保留视觉骨架。
- M2：文档中心（CN/EN）完成目录重构，新增游戏与引擎入口。
- M3：完成校验、文档状态回写、devlog 收口。

## 风险
- 风险：改文案后丢失现有视觉辨识度。
  - 缓解：保留 Hero 结构、颜色体系、动效和证据切换组件，只改信息内容与层级。
- 风险：中英文内容偏移。
  - 缓解：按同任务双语同构改写，保持同锚点和同模块顺序。
- 风险：定位切换过猛导致老用户困惑。
  - 缓解：在文档中心保留 Viewer 手册主入口，新增游戏/引擎入口而非替换删除。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
