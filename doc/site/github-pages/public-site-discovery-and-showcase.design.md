# 官网发现与展示系统设计

- 状态：active（当前静态实现合同；不表示部署或完整验收）
- Owner role：viewer_engineer
- 审读基线：2026-10-14；`116c2e6d4f4891c316b305764a1f755a3005d3ae`；远程源码阅读，无线上浏览器结论
- 上游产品：[`官网发现、玩法展示与持续关注 PRD`](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md)
- 产品体验设计：[`配对产品 design`](../../product/player-entry-distribution/public-site-discovery-and-showcase.design.md)
- 专业入口：[`site PRD`](../prd.md)、[`当前布局与页面覆盖合同`](../design.md#homepage-layout-authority)、[`文件级索引`](../prd.index.md)

## 1. 问题、目标与非目标

现有首页有氛围图、说明性危机场景、下载与文档入口。目标是在同一静态技术栈上形成具体决策展示、可公开的观测与开发进展、准确预览分流和可恢复阅读，而不是引入游戏服务依赖。

本设计补充现行 root design，不替代其权限、双语、可访问性、下载、手册和故事 reader 合同。保留首屏一个资源压力问题、一个主动作、一个次要文字链接和可见技术预览状态；通过后续区段承担具体玩法，不恢复三列事实条。后续实现若反转 root 约束，须同步 root，而非以新文件日期静默覆盖。

静态实现沿本设计维护首页、内容列表/详情、RSS、下载增强及小说恢复；实现来源见上游产品功能状态。生成式概念稿继续只承担视觉方向。本文不升级发行状态、游戏规则或运行服务，也不以页面代码存在代替真人理解、完整浏览器验收或线上部署。

## 2. 上游约束与相关角色

产品 owner 决定访客价值和范围；视觉 owner 决定信息层级与素材；viewer_engineer 承担静态实现；liveops_community 核对可公开内容与当前口径；qa_engineer 核对来源和验证范围。本文不要求固定 subagent 数量或新增审批流程。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-4544302c-40be-4aa3-a557-21fe6f475c19；[REQ-SITE-DISCOVERY-001](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-001) | 首页先解释玩家、Agent、资源压力及有效下一步 | [DES-SITE-ENTRY](#des-site-entry) | 产品/视觉；root 布局 | 不证明真人理解或游戏可玩 |
| Trace refs: O7-REQ-4b01d2d4-ec3a-4dbb-90f8-fca0c4b0a822；[REQ-SITE-DISCOVERY-002](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-002) | 同一案例的四步与素材性质一致；来源缺失不补假事实 | [DES-SITE-EVIDENCE](#des-site-evidence) | QA/玩法 owner；可公开原始材料 | 不接实时世界写/读服务 |
| Trace refs: O7-REQ-2c62937a-f257-4112-a04e-4044662d1b04；[REQ-SITE-DISCOVERY-003](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-003) | 稳定列表/详情/RSS 与纠正、空状态 | [DES-SITE-PUBLISHING](#des-site-publishing) | 内容 owner；公开口径 authority | 不做 CMS、订阅账户或投稿 |
| Trace refs: O7-REQ-d23033f5-0d4d-40b6-8745-1374374af0c3；[REQ-SITE-DISCOVERY-004](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-004) | 版本、平台、成本及限制一致；缺资产不猜地址 | [DES-SITE-DOWNLOAD](#des-site-download) | 发行/Launcher；root 下载合同 | 不改安装器、签名或资产校验机制 |
| Trace refs: O7-REQ-ee159d08-7998-44e2-bac5-930b7fa8a0ae；[REQ-SITE-DISCOVERY-005](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-005) | 小说导读与真实记录分离，失败/语言目标可恢复 | [DES-SITE-STORY](#des-site-story) | 小说 owner；现有 reader 合同 | 不翻译或重写正文 |
| Trace refs: O7-REQ-de38c7a9-3a48-418d-be60-2c527bc1e8eb；[REQ-SITE-DISCOVERY-006](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-006) | 双语、无 JS、窄屏/键盘/动效降级不丢主路径 | [DES-SITE-ACCESSIBILITY](#des-site-accessibility) | 视觉/QA；现有 S6 | 概念稿不代签浏览器/性能证据 |
| Trace refs: O7-REQ-6b906d19-8858-4b62-9526-5952cfc9ad8b；[REQ-ENTRY-CANDIDATE-001](../../product/player-entry-distribution/access-modes-and-release-readiness.prd.md#req-entry-candidate-001) | 版本、日期、Release 说明、实际资产与 checksum 以同一候选快照原子更新；失败不混版 | [已有设计](#des-site-download) | 发行/QA/LiveOps 与对应产品 owner | 两 primary mode、跨域 QA/经济/恢复及完整 release gate 不由网站下载片段覆盖 |
| Trace refs: O7-REQ-e20ed8c7-6f8d-400a-a883-57b2e772acff；[REQ-ENTRY-CLAIM-001](../../product/player-entry-distribution/release-communications-and-public-claims.prd.md#req-entry-claim-001) | runtime_capture 绑定版本/场景/证据窗口与可公开来源；来源不足不背书运行事实 | [已有设计](#des-site-evidence) | 发行/QA/LiveOps 与对应产品 owner | 全渠道专业事实、QA verdict 和 LiveOps 审核闭包仍外部 |
| Trace refs: O7-REQ-f71ac2e9-1586-44af-89b7-e98ff364e70c；[REQ-ENTRY-CLAIM-002](../../product/player-entry-distribution/release-communications-and-public-claims.prd.md#req-entry-claim-002) | 同 URL 纠正/撤回同步 CN/EN、详情/摘要/RSS，回滚不复活错误正文 | [已有设计](#des-site-publishing) | 发行/QA/LiveOps 与对应产品 owner | 其他 active public surfaces、自动漂移检测与第三方缓存收回不在本设计 |
| Trace refs: O7-REQ-f9218cff-4a0e-4a40-8a2f-133d9c824385；[REQ-ENTRY-CLAIM-003](../../product/player-entry-distribution/release-communications-and-public-claims.prd.md#req-entry-claim-003) | concept/illustrative/runtime_capture 与小说类型分离，目标/概念不冒充真实运行来源 | [已有设计](#des-site-evidence) | 发行/QA/LiveOps 与对应产品 owner | 其他专业领域、非官网渠道及全部公开审核仍外部 |

现行[公开事实四层模型](../../product/player-entry-distribution/release-communications-and-public-claims.prd.md#public-claim-four-layer-model)和[下载完整性/升级](../../product/player-entry-distribution/prd.md#download-integrity-and-upgrade)被消费，不在本设计重新定义。

## 3. 当前状态、目标状态与差距

| 对象/能力 | 当前状态（基线） | 目标状态 | 差距/假设 | 证据或 owner |
| --- | --- | --- | --- | --- |
| 首页 | 静态 CN/EN HTML、共享 CSS/JS、场景切换与示例日志 | 角色优先、同一案例四步、清楚分流 | 需要内容重排与真实访客验证 | [固定首页源码](https://github.com/eng-cc/oasis7/blob/116c2e6d4f4891c316b305764a1f755a3005d3ae/site/index.html) |
| 记录/日志 | 首页链接 Release/文档；本专题独立内容链未核实 | 静态列表/详情、三条精选、纠正与 RSS | 需要可公开首篇内容，不用占位成功填充 | liveops_community |
| 下载 | HTML 静态回退与 JS 最新 Release 刷新并存 | 速览和版本/资产一致处理 | 不能只凭静态旧日期判断运行显示过期 | [固定 app.js](https://github.com/eng-cc/oasis7/blob/116c2e6d4f4891c316b305764a1f755a3005d3ae/site/assets/app.js) |
| 小说 | 中文阅读器与版本资料导航已存在 | 导读优先，语言/失败遵守现有合同 | 本文不宣布 reader 恢复缺陷已修复 | [reader 设计](../design.md#story-reader-design) |
| 视觉证据 | 本 PR 增加生成式概念板 | 后续按路由/状态取得真实浏览器证据 | 图稿不证明实现或页面覆盖 | 视觉/QA |

## 4. 边界与结构

<a id="des-site-entry"></a>
### DES-SITE-ENTRY：页面与内容所有权

沿用 `site/index.html`、`site/en/index.html`、`site/assets/styles.css`、`site/assets/app.js`，无必需的 React、CMS、SSG、服务端渲染或新框架构建链。正文、关键状态、链接在 HTML 内，JS 仅增强。

```text
README / 现行产品与专业文档 / 已复核可公开原始材料
    -> 人工编辑的 canonical 内容详情 HTML
    -> 首页精选 / 列表摘要 / RSS（派生摘要，同一变更校对）
    -> GitHub Pages 静态发布
    -> 浏览器阅读与渐进增强（无世界效果）

Release 元数据 -> 现有下载刷新层 -> 同候选版本/资产展示
小说原文 -> 现有 reader -> 真实加载状态与恢复入口
```

箭头表示内容读取/派生，不是世界提交。Pages 不拥有世界时间线、Agent 授权或玩家账户。所有官网交互只改变阅读状态；无模型调用、钱包连接和世界写接口。

| 路由族（目标） | 中文 | 英文 | 内容与兼容 |
| --- | --- | --- | --- |
| 首页 | `/` | `/en/` | 同构主路径；相对 URL 支持项目子路径 |
| 观测列表/详情 | `/observations/`、`/observations/<slug>/` | `/en/observations/`、`/en/observations/<slug>/` | 独立稳定 slug；可只有空列表，不虚构文章 |
| 开发日志列表/详情 | `/devlog/`、`/devlog/<slug>/` | `/en/devlog/`、`/en/devlog/<slug>/` | 玩家视角文章，不自动搬 PR 标题 |
| 订阅 | `/feed.xml` | `/en/feed.xml` | 静态 XML，无登录/数据收集 |
| 小说 | 既有 `/story/` | 不新增虚假英文页 | 英文首页链接明确标为中文小说 |

四个主导航对应玩法、世界记录、开发进展、获取预览；文档/GitHub 放页尾，小说入口仅在页尾或次级渐进披露区域提供，遵守 site PRD 的 AC-19。旧锚点 `#proof`、`#world`、`#download`、`#contribute`、`#world-laws` 在新语义区域保留唯一 ID，旧分享链接不失效。详情返回链接不依赖 history 或来源页状态。

## 5. 关键运行流程

<a id="des-site-evidence"></a>
### DES-SITE-EVIDENCE：分步展示与来源

进入页面先显示类型、标题、范围和文字；分步内容预写 HTML，不为切换步骤新增运行服务请求。四步为目标/边界、Agent 回应、后果、再选择。四步在桌面横排、窄屏纵排，始终同时可读；增强按钮仅同步当前步骤高亮与 `aria-pressed`，不隐藏其他步骤。无 JS 时保留完整顺序及步骤标题。

图片保持独立 alt、caption 和完整原图入口。加载失败只替换该媒体节点的状态，不删文字/标签。若使用异步大图 overlay，以当前 record ID + media ID 校验提交，迟到 load/error 不覆盖新选择；Esc 关闭并将焦点还给触发控件。无此能力时直接打开原图，不新造复杂 lightbox。

来源类型固定为 `concept`（概念）、`illustrative`（说明性）、`runtime_capture`（真实运行）；小说与日志使用内容类型区分，不作为运行来源类型。标签没有“live”。运行来源缺少必要版本/场景/证据关系时不得标 runtime_capture；可以明确收窄为原本就属于说明性的材料，但不能把失效真实记录偷偷改成未说明的“示例”。

原始 Agent 自由文本一律视为不可信内容，只以转义后的文字呈现；不得作为 HTML、脚本、页面跳转或模型指令执行。脱敏后仍需能理解因果，不能把关键失败截掉只留成功。

<a id="des-site-publishing"></a>
### DES-SITE-PUBLISHING：编辑、发布与纠正

每条内容的详情 HTML 是公开正文与元数据的唯一编辑源。发布正文放在唯一 `data-content-body` 区域；撤回必须移除此区域并提供 `data-content-correction`，同 GUID 的 RSS 描述与纠正正文一致；关联的 CN/EN 内容必须在同一变更中撤回并同步状态，不能让另一语言继续发布旧正文。首页/列表/RSS 保留必要摘要，作为派生副本，在同一变更中同步并由检查核对 id、标题、类型、日期、语言、版本、链接及状态；不在 PRD 中另存文章台账。静态派生一致性检查消费 `scripts/site-discovery-check.py`，负例覆盖 `scripts/site-discovery-check.test.py`；当前不引入生成器依赖，未来批量化再单独决定。

发布顺序为核对上游事实与可公开性、编写详情、同步 CN/EN 与摘要、运行适用检查、沿现有 Pages 发布链部署。文档通过、合入和部署是不同事实。

列表按 `published_at` 倒序，同时间以 slug 升序；首页最多三条。编辑更正更新 `updated_at` 但不伪造原发布时间。draft 不进入公开页面/RSS；历史已发布文章保留原版本并明确不代表当前可用性。撤回后同 URL 改为安全说明，移除错误正文和推荐；RSS 同 GUID 更新为纠正说明。客户端已缓存或第三方转发内容无法被承诺收回。

敏感材料必须移除正文、图片和派生摘要；必要仓库历史/缓存处置依现有安全与事件流程，由对应 owner 承担，不用静态 tombstone 假装已经彻底删除泄露。

<a id="des-site-download"></a>
### DES-SITE-DOWNLOAD：速览与同候选展示

预览速览消费 README、对应版本发布说明与现有安装/升级 authority。字段为可体验范围、平台/架构、配置要求、编程要求、模型来源/费用责任、公开网页状态、保存/升级/签名限制。未知字段明确未知，不填写估算 token 花费或未经验证的支持矩阵。

保留每平台一个推荐主包和手动切换。平台检测失败只显示选择器与范围说明，不把未知设备判为已支持。API 刷新必须把版本、日期、发布说明、该版本实际存在的目标资产及 checksum 作为一个候选快照核对并一次性更新；不混用新版 tag 与旧版文件/校验项，不仅按旧 filename 拼接新链接。找不到资产时停止该项下载推荐并给发行说明。

刷新失败可以保留完整的静态已知候选，但必须明确“最后记录版本，未确认最新”；静态无 JS 入口同样不得自称在线已验证最新。无有效候选时只保留发行说明/恢复入口。checksums 不是平台签名，不提供绕过系统策略的步骤；本文不改变 [root 手动升级首写合同](../design.md#manual-upgrade-first-write)。

<a id="des-site-story"></a>
### DES-SITE-STORY：小说入口与 reader 复用

首页的次级渐进披露区域或页尾仅提供小说名、无剧透摘要、世界观小说标签与开始阅读；不新增正文副本。故事页首层优先简介/开始阅读/背景，已有版本说明、总纲和正文索引保留可达但降为次级。

沿用 [reader 状态机](../design.md#story-reader-design)：loading/ready/empty/error、requestId 防迟到、明确重试与原文入口。当前请求身份核对先于正文或缓存写入，迟到的旧请求不能污染显式重试后的缓存。语言跳转仅消费存在的 allowlist 映射；`/story/` 不能按路径猜为 `/story/en/`。存储不可用时不阻断阅读，不自动改写小说语言。小说中的具身描述不生成任何游戏能力标记。

## 6. 接口与数据合同

这些字段是目标静态内容约定，不是游戏协议、运行回执或新增数据库。可用 HTML `data-*` / `time` / 语义标签承载，不要求浏览器执行数据构建。

| 内容字段 | 生产者 → 消费者 | 身份/顺序 | 成功与错误 | 兼容/大小边界 |
| --- | --- | --- | --- | --- |
| `id, slug, locale, translation_of` | 内容维护者 → 详情/摘要/RSS | slug 发布后不复用；语言映射显式 | 缺译文说明实际语言；非法路径拒绝 | slug 仅小写 ASCII、数字和连字符；禁止路径穿越 |
| `kind, provenance, status` | 内容 owner/QA → 标签/筛选 | kind=observation/devlog/lore；status=draft/published/withdrawn；provenance 仅观测必需 | 未知枚举、互斥组合在发布检查失败 | 不映射成游戏 readiness；无 live 来源 |
| `title, summary, published_at, updated_at` | 详情 → 首页/列表/RSS | ISO 8601 含时区；发布时间排序 | 缺日期不伪造；空正文不发布成文章 | title 建议 ≤80 字符、summary ≤180；超过先编辑，不隐藏事实 |
| `version, scenario, capture_window, evidence_url` | 原始材料/QA → runtime_capture 详情 | 真实运行必须有版本、场景、窗口及可公开来源关联 | 缺失/失效来源不能背书运行事实 | evidence_url 只允许经复核公开 HTTPS 或站内相对路径 |
| `steps, media, alt, caption` | 编辑者 → 分步展示/浏览器 | 四步属于同 record；media 绑定所属步骤 | 媒体失败留文字，快速切换不串条目 | 图片有尺寸、图注、来源；不嵌不可信 HTML |
| 版本/资产快照 | 现有 GitHub Release 读取 → 下载区 | 同一个合法 release 与实际 assets 集合 | 超时/缺资产/不支持均有独立提示 | 沿用现有接口；仅 HTTPS/允许的仓库域名 |

静态内容检查须拒绝错误 id、重复 slug、失效站内链接、语言配对漂移和不完整 runtime_capture 元数据；形式通过不证明原始材料真实。来源复核保留人工判断。

## 7. 状态、事务与持久化

官网没有世界事务。浏览/切步骤不提交 Agent 意图、不消费资源、不连接私有 provider。阅读状态可保留在 URL 锚点；现有语言偏好存储保持可失败，不新增账户或持久个人画像。

| 事件 | 当前视图/状态 | 处理及恢复边界 |
| --- | --- | --- |
| 首次/无 JS | 静态内容可读 | 直接锚点/链接；不存在必需的 loading 遮罩 |
| 切换步骤 | 当前 record/step | 同步设置面板及可访问状态；不加载另一候选来补失败 |
| 媒体加载错误 | 内容仍有效、媒体 error | 显示替代文字与原文；不切换素材类型 |
| 撤回内容 | withdrawn | 安全纠正页、当前入口；不展示旧正文/错误推荐 |
| Release 刷新成功 | verified metadata snapshot | 元数据/资产一起替换；仅代表来源元数据，不证明游戏验收 |
| Release 刷新失败 | last-known / unavailable | 保留完整旧候选并标记未确认最新，或仅说明入口 |
| reader 迟到结果 | 与当前 requestId 不符 | 忽略；不修改当前正文/错误或 busy 状态 |

内容纠正涉及正文、摘要、语言和 RSS 的成套静态变更；静态发布不是分布式事务，部署期间旧缓存可能存在。因此每份内容都自带日期/版本/状态，不依赖首页提示来限制历史文章。

## 8. 部署、安全与运行约束

继续现有 GitHub Pages 发布链，不新增特权 workflow、Secrets、后台服务或审批层。新内容放 `site/**` 对应语言路径，设计资产只放 `doc/site/github-pages/assets/`，不自动复制到公开首页作为游戏素材。

素材在进入公开 Git 历史前脱敏；不得包含 API key、session、私有路径、非公开账号、玩家私信或部署拓扑。外部链接限定 HTTPS 并有明确去向；富文本按站点现有安全策略处理，禁止 `javascript:`、事件属性和用户脚本。第三方视频只在明确点击后加载，并保留 poster/文字；首期优先站内轻量图片，不依赖外部服务完成阅读。

不新增行为追踪 SDK、邮件/手机号表单或隐形采集。浏览器不得获得世界写接口、管理 token 或供应商凭据。公开素材使用原创/有许可资源，不复制竞品画面。

## 9. 质量与容量

<a id="des-site-accessibility"></a>
### DES-SITE-ACCESSIBILITY：响应式与可访问性

桌面容器沿用现有最大宽度约 1160px，正文以 16px 为基准、长文控制行宽；首屏标题桌面约 48–64px、手机约 32–40px，以真实中英换行检查为准。手机单列重排，不以缩小整页取代响应式。保持现有深色/薄荷绿视觉，琥珀仅用于风险或资源压力；状态必须有文字。

实现检查目标：普通文字对比度至少 4.5:1，大字/关键非文本控件至少 3:1；触控目标以 44×44 CSS px 为设计目标。键盘焦点明显、跳转正文可用、展开信息可键盘访问；用原生链接和按钮，不做仅 hover 信息。200% 缩放不丢正文或操作。

保留 `1440x900`、`390x844`、`360x800` 的真实浏览器证据；CN/EN 各核对阅读顺序、长标签换行、无横向溢出。无 JS 时菜单/正文/CTA 可用；reduced-motion 下不启动 Hero Canvas、自动计数或强制滚动。语言 allowlist、query/hash 和部署前缀保留，不猜站点根域。

首期容量预算是设计目标而非已测结果：首屏必要图片总传输量尽量 ≤500KB，非首屏图片 lazy-load；不自动下载视频，不新增必需字体或大 JS 库。图片声明宽高、适配尺寸；在现有 S6 性能路径记录请求量/总字节/LCP/布局偏移，未测时不写性能 PASS。若真实截图压缩影响文字可读性，优先缩略图 + 按需原图，不牺牲证据清晰度。

### 设计资产归档

设计图文件：[public-site-discovery-and-showcase.webp](assets/public-site-discovery-and-showcase.webp)。生成方式为本次 ChatGPT 原生生图；保存压缩展示副本，未以脚本绘图冒充生图。图内全景/控件均属概念表达，原始生成图与提交的展示副本可以不同编码/尺寸。实际文件字节、尺寸、hash 以 PR 的资产核对和 Git 对象为准，不填写未知模型版本/seed。

图稿负责视觉方向，配对产品 design 负责文案与体验，本系统设计负责实现和验证。图中与正文不一致的细节不自动成为要求；后续 HTML 和浏览器证据独立取得。生图样例不进入 runtime_capture 素材集合。

## 10. 兼容、迁移与回滚

保留既有首页锚点、CN/EN、docs、story 和 download 路由，不移动世界/手册 authority。新增路由相对链接支持自定义域与项目子路径；文章发布后 slug 不重用，迁移保留旧地址到明确新地址的静态说明/链接。

后续实现先维护内容和既有 claim gate 的语义，再重排 HTML，最后补增强脚本与回归。若检查依赖旧精确文案，应同时更新对应测试，使其继续检查公开状态、素材性质、真实 CTA 和下载边界，而不是删检查或用隐藏文本满足旧断言。本设计不要求删减或跳过任何门禁。

站点回滚只回退到已知静态提交，不回滚游戏、世界历史或安装器状态。已撤回/纠正的敏感或错误内容不能随整站回滚复活；必要时在回退提交上前向保留纠正。RSS 和第三方缓存可能保留旧副本，应公开纠正而非声称已全网删除。

## 11. 验证设计与可追溯性

<a id="verification"></a>
下表是验证方法与证据目标，不是执行结果。沿用 [testing-manual](../../../testing-manual.md) 的 Site/S6 和现有站点脚本；操作方法见[官网发现验证手册](public-site-discovery-validation.manual.md)。静态检查与故障回归分别由现有站点门禁和 `scripts/site-discovery-browser.test.cjs` 承担；实际运行结果只在实施 PR 记录。

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [REQ-SITE-DISCOVERY-001](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-001) / AC-SITE-DISCOVERY-001 | [ENTRY](#des-site-entry) | 主路径/旧锚点/CNEN可达，玩家位置可理解 | [官网发现验证手册](public-site-discovery-validation.manual.md)：S6-DISC-01：同候选静态服务器，点开主次 CTA 和五个旧锚点；另做陌生访客观察并记录原话 | 实施 PR：浏览器/访客记录，绑定 source HEAD、语言和入口 | 自动化不能证明理解或可玩 |
| [REQ-SITE-DISCOVERY-002](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-002) / AC-SITE-DISCOVERY-002 | [EVIDENCE](#des-site-evidence) | 类型/内容同属一案例，来源错误不升级，切换无世界写 | [官网发现验证手册](public-site-discovery-validation.manual.md)：S6-DISC-02：concept/illustrative/runtime_capture/缺来源样本；破坏图片、快速切步骤、延迟旧媒体；核对网络无世界/模型调用 | 实施 PR：fixture、断言、脱敏来源审查、当前版本截图 | fixture 不证明素材真实或运行结果 |
| [REQ-SITE-DISCOVERY-003](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-003) / AC-SITE-DISCOVERY-003 | [PUBLISHING](#des-site-publishing) | 零/一/多/同时间排序、双语更正、撤回旧 URL/RSS | [官网发现验证手册](public-site-discovery-validation.manual.md)：S6-DISC-03：静态 fixture 和 XML 解析；核对 id/日期/范围/状态，撤回后详情/首页/列表/RSS同步 | 实施 PR：内容检查输出与页面证据 | 不证明外部缓存清除或活跃留存 |
| [REQ-SITE-DISCOVERY-004](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-004) / AC-SITE-DISCOVERY-004 | [DOWNLOAD](#des-site-download) | 版本与实际资产同候选，不支持/超时有安全出口 | [官网发现验证手册](public-site-discovery-validation.manual.md)：S6-DISC-04：模拟 Release 成功/超时/缺asset/缺checksum/错误平台；核对无混版链接、last-known不称最新、无凭据框 | 实施 PR：同候选 mock API 与浏览器记录；实际资产另核实 | 不证明安装器安全、签名或发行就绪 |
| [REQ-SITE-DISCOVERY-005](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-005) / AC-SITE-DISCOVERY-005 | [STORY](#des-site-story) | 小说/运行记录区分，语言目标与 reader 恢复有效 | [官网发现验证手册](public-site-discovery-validation.manual.md)：S6-DISC-05：英文首访到中文小说；正常/空/404/超时/A→B迟到成功与失败；复用 root reader 用例 | 实施 PR：准确路由、请求身份与当前状态截图 | 不证明小说质量或具身玩法 |
| [REQ-SITE-DISCOVERY-006](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#req-site-discovery-006) / AC-SITE-DISCOVERY-006 | [ACCESSIBILITY](#des-site-accessibility) | CN/EN/三视口/键盘/缩放/无JS/reduced-motion可用 | [官网发现验证手册](public-site-discovery-validation.manual.md)：S6-DISC-06：同候选按上述矩阵验证首页、两类列表/详情、小说入口；对比度与资源加载单独记录 | 实施 PR：真实浏览器截图、键盘路径、网络/性能结果 | 概念图不证明像素/无障碍/性能达标 |
| Trace refs: O7-REQ-6b906d19-8858-4b62-9526-5952cfc9ad8b；[REQ-ENTRY-CANDIDATE-001](../../product/player-entry-distribution/access-modes-and-release-readiness.prd.md#req-entry-candidate-001) | [已有设计](#des-site-download) | 版本、日期、Release 说明、实际资产与 checksum 以同一候选快照原子更新；失败不混版 | [官网验证手册](public-site-discovery-validation.manual.md)：S6-DISC-04，按原 fixture/同候选浏览器方法核对；仅本行网站子范围 | PR 记录来源审查与适用页面/负例结果，不由链接代替执行 | 两 primary mode、跨域 QA/经济/恢复及完整 release gate 不由网站下载片段覆盖；本次未运行 |
| Trace refs: O7-REQ-e20ed8c7-6f8d-400a-a883-57b2e772acff；[REQ-ENTRY-CLAIM-001](../../product/player-entry-distribution/release-communications-and-public-claims.prd.md#req-entry-claim-001) | [已有设计](#des-site-evidence) | runtime_capture 绑定版本/场景/证据窗口与可公开来源；来源不足不背书运行事实 | [官网验证手册](public-site-discovery-validation.manual.md)：S6-DISC-02，按原 fixture/同候选浏览器方法核对；仅本行网站子范围 | PR 记录来源审查与适用页面/负例结果，不由链接代替执行 | 全渠道专业事实、QA verdict 和 LiveOps 审核闭包仍外部；本次未运行 |
| Trace refs: O7-REQ-f71ac2e9-1586-44af-89b7-e98ff364e70c；[REQ-ENTRY-CLAIM-002](../../product/player-entry-distribution/release-communications-and-public-claims.prd.md#req-entry-claim-002) | [已有设计](#des-site-publishing) | 同 URL 纠正/撤回同步 CN/EN、详情/摘要/RSS，回滚不复活错误正文 | [官网验证手册](public-site-discovery-validation.manual.md)：S6-DISC-03，按原 fixture/同候选浏览器方法核对；仅本行网站子范围 | PR 记录来源审查与适用页面/负例结果，不由链接代替执行 | 其他 active public surfaces、自动漂移检测与第三方缓存收回不在本设计；本次未运行 |
| Trace refs: O7-REQ-f9218cff-4a0e-4a40-8a2f-133d9c824385；[REQ-ENTRY-CLAIM-003](../../product/player-entry-distribution/release-communications-and-public-claims.prd.md#req-entry-claim-003) | [已有设计](#des-site-evidence) | concept/illustrative/runtime_capture 与小说类型分离，目标/概念不冒充真实运行来源 | [官网验证手册](public-site-discovery-validation.manual.md)：S6-DISC-02，按原 fixture/同候选浏览器方法核对；仅本行网站子范围 | PR 记录来源审查与适用页面/负例结果，不由链接代替执行 | 其他专业领域、非官网渠道及全部公开审核仍外部；本次未运行 |

文档阶段适用的现有命令为 `./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh` 与 `git diff --check`；实际是否运行、范围和结果只写 PR。若环境无法克隆或运行全仓门禁，如实保留未执行，而不是用局部链接检查冒充全量 PASS。

未来页面实现另执行 `./scripts/site-homepage-claim-check.sh`、`./scripts/site-download-check.sh` 及 S6 适用测试，并在同 PR 补相应负例。生成图、结构检查、真人理解和线上部署分别判定。新增内容/媒体不能独立升级 current claim。

## 12. 决策、长期风险与未决问题

采用静态精选而非实时 API：降低服务依赖与敏感数据风险，代价是非实时；以后接实时只读接口时独立设计新鲜度/断线/权限，不能直接删除“非实时”标签。采用原生 HTML 而非 CMS/框架：保留无 JS 路径，代价是少量摘要同步；依靠轻量检查减少漂移，不增加运营平台。

采用一个生成式概念板 + 精确文字/状态合同：用于讨论视觉方向，不能替代每个路由的真实实现证据。选择真实材料而非虚构成功故事可能导致空列表；真实空状态优于假活跃。

待决项只有材料和验证输入：视觉/QA owner 在实现前选定可公开案例与原图，发行 owner 在发布前核对版本/费用/平台，内容 owner 决定首篇文章。没有来源时排除 runtime_capture，没有真实译文时不发布该语言文章；不阻止当前文档 PR 创建，也不强制新 Issue。

### 概念板构图的实现边界

首页主图作为 hero 内的绝对定位视觉层，以渐变遮罩融入底色；窄屏不再堆叠独立图片块，文字和主操作保留实色对比。四步桌面以方向连接符和明确标为概念氛围的裁切配图形成阅读链，手机使用纵向连接线。案例类型与无世界效果说明始终可见，较长来源说明使用可键盘展开的原生 disclosure。记录、进展、预览与规则入口采用紧凑图文节奏；小说仍留在后面的次级展开区，真实空记录和小说次级地位不因构图改变。预览入口合并到首页图文区，保留本地环境、模型配置与未开放公共加入的预览摘要；平台与安装详情在原生 disclosure 中按需展开，预览 CTA 和 `#download` 深链在增强模式下自动打开。无 JS 时手动展开仍可访问平台包，完整风险提示与下载按钮同在展开内容中。

### 公开首页概念主图

步骤裁切使用原创生成式氛围图 `site/assets/images/hero/asteroid-industrial-cluster-20261015.webp`（1536 × 864），由内置 imagegen 按本概念板的墨绿工业小行星方向生成，再压缩为 WebP。生成工具未提供可核验模型版本或 seed，不作补写。它只表达文明制图的世界氛围，图注与替代文字明确其概念性质；不属于运行截图或世界结果证据。四步中的裁切同属该氛围素材。

首页当前主图 `site/assets/images/hero/asteroid-settlement-planet-20261015.webp`（1535 × 864）由内置 imagegen 参照已采用概念板重建工业聚落、远景行星与逆光，再压缩为 WebP；仅作概念氛围。原图保留，模型版本及 seed 未提供。

四步中的回应与再选择配图分别使用 `site/assets/images/steps/decision-response-control-bay-20261015.webp` 与 `site/assets/images/steps/decision-choice-horizon-20261015.webp`（均为 864 × 288）。它们由内置 imagegen 参照概念板对应低清小图重建，再压缩为 WebP；前者为薄荷光工业控制空间，后者为带远景聚落与分叉物流线的地平线。配图仅用于氛围与阅读节奏，不构成 Agent 行动、运行回执或具身能力证据；沿用同一可见概念配图说明。未知模型版本与 seed 不补写。

首页入口与剩余步骤使用 `site/assets/images/entries/` 下的 768px WebP 缩略副本（`industrial-cluster-thumb.webp`、`industrial-network-thumb.webp`、`civilization-orbit-thumb.webp`），分别从保留的同名方向原图等比例缩小与压缩；不改构图、不新增运行语义。尺寸覆盖 390px 手机与 1440px 桌面验收视口下入口及步骤横幅的两倍像素密度，避免给小图传输完整主图。
