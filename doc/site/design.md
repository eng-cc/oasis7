# site 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/site/prd.md`
- 执行记录: Git、PR、实际 CI 和评审记录；Issue 与 Project 按需使用
- 对应文件级索引: `doc/site/prd.index.md`

## 1. 设计定位
`site` 模块的 `design.md` 负责描述站点、静态文档、GitHub Pages 与内容发布的总体设计。

## 2. 阅读顺序
1. `doc/site/prd.md`
2. `doc/site/design.md`
3. `doc/site/prd.index.md`
4. 下钻 `github-pages/`、`manual/` 等专题目录

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

<a id="homepage-layout-authority"></a>
### GitHub Pages 当前视觉与交互权威

首页现行布局采用聚焦的后果链方案：一个资源压力问题、一段玩家位置与持久后果说明、
一个主要操作及一个次要文字链接，并保留明确可见的 `limited playable technical preview`。
首屏不恢复三列事实摘要；其有效信息由标题、副标题和后续说明承接。本条确定文档优先关系，
不改变当前 HTML、按钮目的地、公开能力承诺或已有验收结论。

7 月 19 日补充负责后果链节奏与渐进披露；7 月 18 日补充继续承接双语、响应式、可访问性
与生成素材边界，但其“三列事实摘要”布局已经被本节替代。两者发生布局冲突时以本节为准，
不是以文件日期自行覆盖所有专业要求。生成图公开标签按 7 月 18 日文档记录的 #2459 amendment：
不强制展示 Image2／non-gameplay 生产标签；来源保留在仓库，生成图仍不是玩法或发布证据。

首页以陌生访客为第一阅读对象：首屏先说明游戏、玩家位置与当前可做
事项，再渐进披露证据、下载、文档与技术细节。中英文路由须保持同一信息
顺序与预览边界；`limited playable technical preview` 必须可见，且不得被视觉
素材、截图或 CTA 表达成正式发布或公开网页可玩承诺。

响应式与可访问性要求包括：窄屏无横向溢出、可达的导航与触控操作、键盘
焦点和 skip navigation、无 JS 导航以及 reduced-motion 行为。生成式世界图仅
承担氛围与层级，不能替代真实 Viewer 截图、状态证据或 HTML 中的有效文案。

- 需求与公开边界：`doc/site/prd.md`
- 首页页面级层级基线：`doc/site/github-pages/github-pages-homepage-page-2026-06-19.design.md`
- 响应式、双语与素材边界补充：[7 月 18 日文档](github-pages/github-pages-visual-content-refresh-2026-07-18.design.md)；旧三列布局不再生效。
- 后果链节奏与渐进披露补充：[7 月 19 日文档](github-pages/github-pages-cinematic-consequence-refresh-2026-07-19.design.md)；服从本节的当前布局与图注边界。

### 静态页面实现与页面级覆盖合同

- `site/index.html` 与 `site/en/index.html` 共享静态 HTML/CSS/JS 实现；页面改动不得因此引入必需的前端框架或构建链。中英文路由的结构、主阅读顺序、关键 CTA 与公开 claim 必须保持同构，英文较长标签应自然换行而非以不可读缩放处理。
- Hero Canvas、指针响应、滚动步骤高亮与证据时间线是可选的渐进增强：缺少对应 DOM、Canvas 上下文失败、脚本失效或 `prefers-reduced-motion` 时，正文、导航和 CTA 仍须可达；动态效果不得遮挡正文或把诊断/证据交互置于访客主路径之前。
- 共享架构图仍使用 `site/assets/images/architecture-flow.svg`。更新图稿时保留稳定引用路径和 SVG 的 `title`、`desc`、`role`、`aria-labelledby` 语义，不得以未说明的位图替代或破坏缩放可读性。
- 页面级设计证据按 route family 管理：中英文共享视觉系统时可复用一份明确列出路径、语言长度风险与响应式约束的设计稿；真实浏览器桌面/移动截图与门禁结果仍是实现验证，Image2/概念图和设计稿不能替代运行证据或 QA 结论。fixture、debug、vendor 与纯构建输出不是产品页面，须在其所属模块明确分类而非纳入公开首页覆盖。
- 任何新增公开页面、Hero Canvas/架构图/时间线的实质变更，或现有页面的视觉层级重排，均须先更新本节或对应的非日期化专业设计 authority，并补齐 `1440x900`、`390x844`、`360x800` 的真实浏览器证据。这是未清偿的页面设计治理债务触发器，不得恢复已退役微专题作为替代。

<a id="static-manual-design"></a>
### 静态手册的共享设计

承接 [静态手册需求](manual/site-manual-static-docs.prd.md#static-manual-requirements)。
本节是该专题的当前系统设计入口；旧同名 design 仅保留兼容路由，不再维护第二份设计正文。

- **输入与内容主责**：Viewer 内容以 [canonical 手册](../world-simulator/viewer/viewer-manual.manual.md) 为准；
  `site/doc/cn/` 与 `site/doc/en/` 是公开镜像，不反向决定 Runtime 合同或发布状态。内容维护者维护原稿，
  镜像维护者同步译文和页面；验证者核对实际候选。任务状态从 Git、PR、实际 CI 与评审读取。
- **页面与接口**：保留 CN/EN 的 `index.html`、`viewer-manual.html`、主页入口及语言互链；维护既有
  `site/assets/styles.css` 与 `site/assets/app.js`，手册样式限定在适用的 `.docs-*` 范围。
  不引入 CMS、SSG、服务端渲染或新的必需构建链，不把一次性迁移扩成全仓文档搬迁。
- **同步顺序**：修改原稿及其适用边界，再同步两种语言的正文、目录、导航、锚点和元数据。
  保持公开 URL 与可用深链；确需迁移时先提供兼容去向并核对消费者，不以新目录存在代替链接验收。
  中英文采用相同信息结构和能力口径；翻译缺失、链接缺失或来源不明必须显式处理，不能用旧镜像冒充同步完成。
- **失败与发布**：入口失效、主次语言矛盾或 unsupported claim 属于本次同步的缺陷；页面可打开、
  静态构建成功或内容合入都不证明部署已完成、游戏可玩或新候选通过。沿现有发布链交付，不新增发布审批层。
- **验证**：按 [测试手册](../../testing-manual.md) 的适用 Site/S6 路径，对同候选原稿和 CN/EN 页面核对
  内容来源、首页到目录到正文、语言往返、深链、窄屏和键盘可达性。可见布局改动仍需适用真实浏览器证据；
  文档合并只验证文本、引用和职责，不生成截图、冷启动、真实 Provider 或部署 PASS。

| 上游需求 | 设计与验证承接 | 未证明范围 |
| --- | --- | --- |
| [稳定 URL、双语与纯静态范围](manual/site-manual-static-docs.prd.md#static-manual-requirements) | 本节的页面接口、成对同步与来源核对；按测试手册验证真实页面 | 不证明线上更新或游戏发行 |
| [内容基线与镜像漂移风险](manual/site-manual-static-docs.prd.md#风险) | 原稿→双语镜像→导航/锚点；来源缺失保持明确缺口 | 不用历史验收替代当前候选 |

<a id="story-reader-design"></a>
### 故事入口与 reader 状态机

承接 [STORY-AC-1–5](prd.md#story-entry-and-state-contract)，复用 `site/assets/app.js`、
`site/assets/styles.css` 与 `site/story/index.html`，不新增应用框架或独立内容源。

路由以页面显式声明的语言映射或等价 allowlist 为依据，不以路径以 `/`、`index.html` 结尾推导双语支持。
先读取手动语言选择，再排除文档及无目标语言的页面，仅在有实际目标页的入口进行首次自动选择。
路径解析保留部署前缀、query 和 hash；故事页无英文对应时原路由不动，也不写入“首页首次跳转已完成”标记。
存储不可用时退化为无持久化选择，不因读取 localStorage 失败中断正文。

章节状态由一个渲染入口同时设置正文、提示、选中按钮与 `aria-busy`，`aria-live` 保留可访问通知。

| 输入/结果 | 新状态 | 视图行为 |
| --- | --- | --- |
| 选择章节或显式重试 | loading | 增加 requestId，绑定当前 src；清除旧正文与错误，只呈现当前章节 loading，busy=true |
| 当前请求非空正文 | ready | 渲染该正文，busy=false；不显示异常/空状态示例 |
| 当前请求 200 空串或全空白 | empty | 显示章节暂无内容及原文/重试入口，busy=false；不宣告已加载正文 |
| 当前请求 404/网络拒绝 | error | 显示真实加载失败与当前 src 的恢复入口，busy=false；不同时显示 loading/empty |
| 旧请求成功或失败 | 不变 | 在所有 UI 提交点核对 requestId，忽略过期结果，不改变新选择或 busy |

保留现有请求序号防护；取消请求可作为优化，但不能代替迟到结果校验。缓存不能使错误永久化，显式重试
应使当前空内容缓存失效或绕过该缓存；非空内容缓存命中也须经过当前请求身份判断。状态示例从生产 UI 移除，
或仅在显式 debug 页面展示；CSS 不能用任一状态显示整个示例集合。

| 上游验收 | 验证设计与证据边界 |
| --- | --- |
| STORY-AC-1/2 | 根域/项目子路径 × `/story/`/`story/index.html` × en-US/中文 × query/hash；另测手动选择、存储不可用、首页双语和 docs 排除 |
| STORY-AC-3/4 | 受控 fetch 延迟、404、拒绝、200 空串/空白与正常章节；检查单一状态、正文与 aria-busy，以及可操作的当前章节重试/原文链接 |
| STORY-AC-5 | A→B 快切：A 的迟到成功和迟到失败均不改 B；覆盖缓存命中和 retry；由入口 locale 用例继续走同一 reader 路径 |

以上为应实现的行为与验证设计，非本页已运行结果。文档检查、静态 Pages 门禁、行为测试、真实浏览器及
适用视觉证据分别记录；不以生成图或旧内容发布 PR 代替本次阅读行为验收。

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
