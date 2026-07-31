# site 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/site/prd.md`
- 对应项目管理文档: `doc/site/prd.md`
- 对应文件级索引: `doc/site/prd.index.md`

## 1. 设计定位
`site` 模块的 `design.md` 负责描述站点、静态文档、GitHub Pages 与内容发布的总体设计。

## 2. 阅读顺序
1. `doc/site/prd.md`
2. `doc/site/design.md`
3. `doc/site/prd.md`
4. `doc/site/prd.index.md`
5. 下钻 `github-pages/`、`manual/` 等专题目录

## 3. 设计结构
- 展示层：首页、展示页、下载页与内容组织。
- 发布层：静态站构建、发布与同步机制。
- 内容层：文档内容与页面内容的一致性要求。

## 4. 集成点
- `doc/readme/prd.md`
- `site/doc/`
- `doc/engineering/doc-governance/doc-structure-standard.prd.md`

## 5. 专题导航
- 页面与体验优化进入 `github-pages/`
- 静态文档与手册迁移进入 `manual/`

### GitHub Pages 当前视觉与交互权威

首页以陌生访客为第一阅读对象：首屏先说明游戏、玩家位置与当前可做
事项，再渐进披露证据、下载、文档与技术细节。中英文路由须保持同一信息
顺序与预览边界；`limited playable technical preview` 必须可见，且不得被视觉
素材、截图或 CTA 表达成正式发布或公开网页可玩承诺。

响应式与可访问性要求包括：窄屏无横向溢出、可达的导航与触控操作、键盘
焦点和 skip navigation、无 JS 导航以及 reduced-motion 行为。生成式世界图仅
承担氛围与层级，不能替代真实 Viewer 截图、状态证据或 HTML 中的有效文案。

- 需求与公开边界：`doc/site/prd.md`
- 首页页面级层级基线：`doc/site/github-pages/github-pages-homepage-page-2026-06-19.design.md`
- 当前首屏内容与响应式验收：`doc/site/github-pages/github-pages-visual-content-refresh-2026-07-18.design.md`
- 当前后果链节奏与渐进披露：`doc/site/github-pages/github-pages-cinematic-consequence-refresh-2026-07-19.design.md`

### 静态页面实现与页面级覆盖合同

- `site/index.html` 与 `site/en/index.html` 共享静态 HTML/CSS/JS 实现；页面改动不得因此引入必需的前端框架或构建链。中英文路由的结构、主阅读顺序、关键 CTA 与公开 claim 必须保持同构，英文较长标签应自然换行而非以不可读缩放处理。
- Hero Canvas、指针响应、滚动步骤高亮与证据时间线是可选的渐进增强：缺少对应 DOM、Canvas 上下文失败、脚本失效或 `prefers-reduced-motion` 时，正文、导航和 CTA 仍须可达；动态效果不得遮挡正文或把诊断/证据交互置于访客主路径之前。
- 共享架构图仍使用 `site/assets/images/architecture-flow.svg`。更新图稿时保留稳定引用路径和 SVG 的 `title`、`desc`、`role`、`aria-labelledby` 语义，不得以未说明的位图替代或破坏缩放可读性。
- 页面级设计证据按 route family 管理：中英文共享视觉系统时可复用一份明确列出路径、语言长度风险与响应式约束的设计稿；真实浏览器桌面/移动截图与门禁结果仍是实现验证，Image2/概念图和设计稿不能替代运行证据或 QA 结论。fixture、debug、vendor 与纯构建输出不是产品页面，须在其所属模块明确分类而非纳入公开首页覆盖。
- 任何新增公开页面、Hero Canvas/架构图/时间线的实质变更，或现有页面的视觉层级重排，均须先更新本节或对应的非日期化专业设计 authority，并补齐 `1440x900`、`390x844`、`360x800` 的真实浏览器证据。这是未清偿的页面设计治理债务触发器，不得恢复已退役微专题作为替代。

## 设计目标
- 提供 `site` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/site/prd.md`
- 执行入口：`doc/site/prd.md`
- 索引入口：`doc/site/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
