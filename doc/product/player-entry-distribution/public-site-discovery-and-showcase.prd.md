# 官网发现、玩法展示与持续关注

## 文档身份

- 所属产品模块：玩家接入与发行
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：active
- 状态边界：静态实现与完整验收分别表达，不代表已经部署或发行升级
- Owner role：`producer_system_designer`
- Last reviewed：2026-10-15
- 设计判定：`paired-design`。
- 配对产品设计：[`public-site-discovery-and-showcase.design.md`](public-site-discovery-and-showcase.design.md)
- 专业系统设计：[`官网发现与展示系统设计`](../../site/github-pages/public-site-discovery-and-showcase.design.md)
- 专业权威：[`site PRD`](../../site/prd.md)、[`site 设计`](../../site/design.md#homepage-layout-authority)、[`公开口径`](release-communications-and-public-claims.prd.md#public-claim-four-layer-model)、[`README`](../../../README.md)

## 1. 问题与目标

一位没参与过开发的访客听说 oasis7，打开官网。他先想知道自己在游戏里做什么、AI 替自己做什么、一个选择会带来什么后果，再决定观看、了解预览或继续关注。他不应先学会读测试日志、PR 和架构文档，也不应把设计稿、示例回放或小说当作当前可玩的能力。

目标是形成“理解玩家位置 → 看懂一次选择 → 判断当前可参与范围 → 找到下一步”的公开发现路径。它承接上位 SC-1/2/3/4/6 的发现与说明部分，不替代后续真实下载安装、进入世界和发行验收。

**已核对事实。** 以 `116c2e6d4f4891c316b305764a1f755a3005d3ae` 为只读基线，[首页源码](https://github.com/eng-cc/oasis7/blob/116c2e6d4f4891c316b305764a1f755a3005d3ae/site/index.html)已有危机场景切换、说明性事件、Viewer 截图、平台下载和文档入口；[小说页](https://github.com/eng-cc/oasis7/blob/116c2e6d4f4891c316b305764a1f755a3005d3ae/site/story/index.html)已有阅读器。源码阅读不是线上浏览器、素材真实性或完整试玩验收。

**设计假设。** 展示具体决策、集中说明参与条件、提供短篇进展，可能比继续增加宏观概念更帮助陌生访客理解。该假设由真实访客观察验证；若访客仍把自己理解为旁观者、认为公共世界已经开放，或找不到下一步，应修改对应展示而非仅美化页面。转化与留存基线未验证，不设虚构提升百分比。

## 2. 范围与 Non-Goals

首期范围：中英文首页、一个分步玩法展示、精选静态观测记录、面向玩家的开发日志入口、技术预览速览与 FAQ、世界观小说导读、对应设计稿及渐进增强/失败恢复。

不在范围：实时星图或游戏服务接入、账号/手机号预约、支付、投稿平台、创作者市场、公开网页加入、自动更新客户端、第三方追踪服务、全站框架迁移、游戏规则改造、小说正文重写。媒体资料包和实时数据是后续候选，不是首期隐藏依赖。不保证固定更新周期、模型费用、邀测时间或新平台支持。

产品文档与生成式设计稿确定目标与验证方法，不宣布持久世界开放。当前静态实现以功能状态中的固定版本为依据；真实素材、完整验收和发行仍消费各自主责证据。

## 3. 体验与决策

| 访客选择 | 可见信息与机会成本 | 结果和下一步 |
| --- | --- | --- |
| 先看玩法 | 标明素材类型的目标、约束、Agent 响应与结果；只花阅读时间 | 可以回看、比较已记录步骤或了解预览；不触发世界动作 |
| 阅读观测/进展 | 日期、适用版本、证据范围与尚存限制 | 深读一条记录或文章，返回列表；不把历史记录当当前服务状态 |
| 尝试技术预览 | 当前可体验范围、平台、必要配置、模型提供方/费用责任和安装风险 | 自主进入受支持下载/文档；不自动安装、连接 provider 或发送凭据 |
| 暂不安装 | 小说、开发进展、现有 GitHub 跟进入口 | 无需注册即可继续了解或离开，不损失权益 |

主阅读顺序为：定位与主展示入口 → 一次具体选择 → 精选观测记录 → 当前阶段与进展 → 预览速览/下载/FAQ → 世界观小说 → 技术与反馈入口。首屏仍保留一个资源压力问题、一个主要按钮、一个次要文字链接和可见技术预览状态，不恢复三列事实条。

适用异常包括素材缺失、来源失效、记录撤回、文章为空、版本信息获取失败、不支持的平台、脚本不可用、语言目标不存在和小说加载失败。页面必须保留真实原因及安全下一步；不自动切换成别的世界、模式、版本或未声明语言，不以“成功”占位隐藏缺失。

## 4. 产品要求与功能状态

以下功能 ID 同时对应独立 REQ；状态只覆盖本专题定义的首期范围。共同核对基线为 2026-10-14、上述源提交、远程源码阅读环境；实现基线与对应检查范围见各功能点；尚未完成陌生访客理解和完整发行验收。系统设计“已覆盖”仅指条款可逐项定位，不等于已通过独立评审。

<a id="req-site-discovery-001"></a>
### REQ-SITE-DISCOVERY-001：访客理解与主路径

Trace ID: O7-REQ-4544302c-40be-4aa3-a557-21fe6f475c19

- 功能点：F-SITE-DISCOVERY-001；首期 CN/EN 首页；验收 [AC-SITE-DISCOVERY-001](#ac-site-discovery-001)。
- 要求：首屏和首个说明段必须让访客辨认游戏类型、玩家与 Agent 的分工、资源压力和当前可做事项；主按钮进入真实存在的展示，次链接进入预览说明，不用“立即进入世界”代替未开放入口。按钮不能直接调用模型或世界写接口。
- 系统设计状态：已覆盖
- 系统设计依据：[DES-SITE-ENTRY](../../site/github-pages/public-site-discovery-and-showcase.design.md#des-site-entry)，含旧锚点与无脚本入口。
- 实现状态：已实现待验证
- 实现与验证依据：[固定实现](https://github.com/eng-cc/oasis7/tree/cde5a650e0aca7cb444b92699cbe5aff4e9ed17d/site)、[浏览器回归方法](../../../scripts/site-discovery-browser.test.cjs)与[静态派生检查](../../../scripts/site-discovery-check.py)；完整验收和未覆盖项由实施 PR 承接。
- 核对日期与基线：2026-10-15；本专题要求；实现 `cde5a650e0aca7cb444b92699cbe5aff4e9ed17d`；本地静态服务器与外部 Google Chrome；本功能 AC 的静态页面范围。
- 剩余范围：陌生访客理解观察；当前页面变化不证明游戏可玩或线上更新。

<a id="req-site-discovery-002"></a>
### REQ-SITE-DISCOVERY-002：一次选择及证据性质

Trace ID: O7-REQ-4b01d2d4-ec3a-4dbb-90f8-fca0c4b0a822

- 功能点：F-SITE-DISCOVERY-002；首期分步展示与记录详情；验收 [AC-SITE-DISCOVERY-002](#ac-site-discovery-002)。
- 要求：每个展示必须把目标/授权边界、Agent 解释、实际或说明性结果、下一次选择分开；概念图、说明性受控示例、真实运行记录和小说不得混称。真实记录只展示来源能证明的范围；素材缺失/失效时收窄或停止相关展示，不用生成图补成“实机”。浏览、切换和回放均不产生世界效果。
- 系统设计状态：已覆盖
- 系统设计依据：[DES-SITE-EVIDENCE](../../site/github-pages/public-site-discovery-and-showcase.design.md#des-site-evidence)，含来源校验、失效与竞态隔离。
- 实现状态：已实现待验证
- 实现与验证依据：[固定实现](https://github.com/eng-cc/oasis7/tree/cde5a650e0aca7cb444b92699cbe5aff4e9ed17d/site)、[浏览器回归方法](../../../scripts/site-discovery-browser.test.cjs)与[静态派生检查](../../../scripts/site-discovery-check.py)；完整验收和未覆盖项由实施 PR 承接。
- 核对日期与基线：2026-10-15；本专题要求；实现 `cde5a650e0aca7cb444b92699cbe5aff4e9ed17d`；本地静态服务器与外部 Google Chrome；本功能 AC 的静态页面范围。
- 剩余范围：可公开真实运行材料的选择与来源核对；本候选只交付明确说明性示例，不声称 runtime_capture。

<a id="req-site-discovery-003"></a>
### REQ-SITE-DISCOVERY-003：值得回访且可纠正的内容

Trace ID: O7-REQ-2c62937a-f257-4112-a04e-4044662d1b04

- 功能点：F-SITE-DISCOVERY-003；首期观测记录与开发日志；验收 [AC-SITE-DISCOVERY-003](#ac-site-discovery-003)。
- 要求：访客必须能免注册阅读有日期、适用范围和稳定链接的内容；观测解释一件事，日志解释玩家问题、变化及限制。无内容时明确为空，不编造新闻、人数、日期、成功故事或更新频率。撤回内容保留安全的纠正入口，不继续推荐错误摘要；敏感内容不以历史保留为由继续公开。
- 系统设计状态：已覆盖
- 系统设计依据：[DES-SITE-PUBLISHING](../../site/github-pages/public-site-discovery-and-showcase.design.md#des-site-publishing)，含静态来源、排序、RSS 与撤回。
- 实现状态：已实现待验证
- 实现与验证依据：[固定实现](https://github.com/eng-cc/oasis7/tree/cde5a650e0aca7cb444b92699cbe5aff4e9ed17d/site)、[浏览器回归方法](../../../scripts/site-discovery-browser.test.cjs)与[静态派生检查](../../../scripts/site-discovery-check.py)；完整验收和未覆盖项由实施 PR 承接。
- 核对日期与基线：2026-10-15；本专题要求；实现 `cde5a650e0aca7cb444b92699cbe5aff4e9ed17d`；本地静态服务器与外部 Google Chrome；本功能 AC 的静态页面范围。
- 剩余范围：可公开世界运行记录；本候选提供真实空列表及首篇双语站点开发说明，不承诺更新频率或留存。

<a id="req-site-discovery-004"></a>
### REQ-SITE-DISCOVERY-004：预览选择与安全分流

Trace ID: O7-REQ-d23033f5-0d4d-40b6-8745-1374374af0c3

- 功能点：F-SITE-DISCOVERY-004；首期速览、下载与 FAQ；验收 [AC-SITE-DISCOVERY-004](#ac-site-discovery-004)。
- 要求：下载前必须能读到适用版本/平台、当前可体验范围、必要环境与 provider 配置、费用责任、签名和数据保留/升级限制。未知值明确未知；无对应资产时不能猜下载地址或暗示支持。关键警示留在对应按钮附近，其余详细说明可渐进展开；保留不安装的选择。静态网站不提供凭据输入框。
- 系统设计状态：已覆盖
- 系统设计依据：[DES-SITE-DOWNLOAD](../../site/github-pages/public-site-discovery-and-showcase.design.md#des-site-download)，消费现行下载与升级 authority。
- 实现状态：已实现待验证
- 实现与验证依据：[固定实现](https://github.com/eng-cc/oasis7/tree/cde5a650e0aca7cb444b92699cbe5aff4e9ed17d/site)、[浏览器回归方法](../../../scripts/site-discovery-browser.test.cjs)与[静态派生检查](../../../scripts/site-discovery-check.py)；完整验收和未覆盖项由实施 PR 承接。
- 核对日期与基线：2026-10-15；本专题要求；实现 `cde5a650e0aca7cb444b92699cbe5aff4e9ed17d`；本地静态服务器与外部 Google Chrome；本功能 AC 的静态页面范围。
- 剩余范围：真人复述参与成本/安装限制；当前浏览器验证不代签发布资产、安装器或平台兼容性。

<a id="req-site-discovery-005"></a>
### REQ-SITE-DISCOVERY-005：小说与世界记录分离

Trace ID: O7-REQ-ee159d08-7998-44e2-bac5-930b7fa8a0ae

- 功能点：F-SITE-DISCOVERY-005；首期首页小说导读与既有阅读器入口；验收 [AC-SITE-DISCOVERY-005](#ac-site-discovery-005)。
- 要求：小说必须明确为世界观创作，不作为当前玩法证据；首次入口优先简介、开始阅读和无剧透背景，版本清单/总纲置于次级入口。故事加载失败保留真实错误、原文与重试；不存在英文正文时明确中文内容，不伪造翻译或英文路由。不更改小说正文。
- 系统设计状态：已覆盖
- 系统设计依据：[DES-SITE-STORY](../../site/github-pages/public-site-discovery-and-showcase.design.md#des-site-story)，复用阅读器恢复合同。
- 实现状态：已实现待验证
- 实现与验证依据：[固定实现](https://github.com/eng-cc/oasis7/tree/cde5a650e0aca7cb444b92699cbe5aff4e9ed17d/site)、[浏览器回归方法](../../../scripts/site-discovery-browser.test.cjs)与[静态派生检查](../../../scripts/site-discovery-check.py)；完整验收和未覆盖项由实施 PR 承接。
- 核对日期与基线：2026-10-15；本专题要求；实现 `cde5a650e0aca7cb444b92699cbe5aff4e9ed17d`；本地静态服务器与外部 Google Chrome；本功能 AC 的静态页面范围。
- 剩余范围：更多正文内容与小说质量；中文 reader 不提供英文正文，不将具身叙事变成玩法承诺。

<a id="req-site-discovery-006"></a>
### REQ-SITE-DISCOVERY-006：一致且可恢复的阅读

Trace ID: O7-REQ-de38c7a9-3a48-418d-be60-2c527bc1e8eb

- 功能点：F-SITE-DISCOVERY-006；首期全部受影响公开路由；验收 [AC-SITE-DISCOVERY-006](#ac-site-discovery-006)。
- 要求：CN/EN 必须保留同等主路径、素材类型及公开边界；键盘、窄屏、减少动效和脚本失效时仍能读正文并抵达有效下一步。失效语言目标、媒体或网络请求只影响所属内容，不吞掉页面或显示虚假成功。设计稿不能替代这些入口的浏览器证据。
- 系统设计状态：已覆盖
- 系统设计依据：[DES-SITE-ACCESSIBILITY](../../site/github-pages/public-site-discovery-and-showcase.design.md#des-site-accessibility)，含响应式、语言与降级验证。
- 实现状态：已实现待验证
- 实现与验证依据：[固定实现](https://github.com/eng-cc/oasis7/tree/cde5a650e0aca7cb444b92699cbe5aff4e9ed17d/site)、[浏览器回归方法](../../../scripts/site-discovery-browser.test.cjs)与[静态派生检查](../../../scripts/site-discovery-check.py)；完整验收和未覆盖项由实施 PR 承接。
- 核对日期与基线：2026-10-15；本专题要求；实现 `cde5a650e0aca7cb444b92699cbe5aff4e9ed17d`；本地静态服务器与外部 Google Chrome；本功能 AC 的静态页面范围。
- 剩余范围：完整无障碍与性能测量；既有桌面/窄屏/键盘/无脚本与故障场景的实际结果只覆盖已记录范围。

## 5. 验收与证据

<a id="ac-site-discovery-001"></a>
### AC-SITE-DISCOVERY-001：理解玩家位置
- 覆盖要求：REQ-SITE-DISCOVERY-001
给定未接触项目的访客，当阅读首屏及第一段展示后，则能用自己的话区分玩家目标、Agent 决策与世界约束，并找到当前允许的下一步；不能认为已开放公共网页加入。记录误解和原话，不用作者自测代签。30 秒是沿用 site SC-8 的理解目标，不是已实测结果。

<a id="ac-site-discovery-002"></a>
### AC-SITE-DISCOVERY-002：展示不伪造事实
- 覆盖要求：REQ-SITE-DISCOVERY-002
给定概念、说明性、真实记录以及来源缺失四种材料，当浏览与快速切换步骤时，则类型标识与当前内容一致；缺失/失效来源不能进入真实记录陈述，旧响应不覆盖新选择，任何步骤均无世界写请求。人能分清“目标”“结果未知”和“已确认结果”。

<a id="ac-site-discovery-003"></a>
### AC-SITE-DISCOVERY-003：空内容与纠正可理解
- 覆盖要求：REQ-SITE-DISCOVERY-003
给定零条、一条、多条、历史和撤回内容，当进入列表、详情、旧链接或订阅源时，则顺序与日期一致，无假填充；撤回理由和当前说明可达，敏感正文及错误推荐被移除。只记录站点内容行为，不推导游戏活跃或留存。

<a id="ac-site-discovery-004"></a>
### AC-SITE-DISCOVERY-004：预览选择不诱导
- 覆盖要求：REQ-SITE-DISCOVERY-004
给定支持/不支持平台、版本请求超时、缺资产和签名未完成，当访客选择预览时，则看见对应限制与文档/重试/退出路径；不会拼接不存在的资产、绕过信任警示或把未知版本标为最新。安装成本、费用责任和保留语义能被正确复述。

<a id="ac-site-discovery-005"></a>
### AC-SITE-DISCOVERY-005：小说不变成玩法承诺
- 覆盖要求：REQ-SITE-DISCOVERY-005
给定英文首页、无英文小说、章节正常/空/失败与快速切换，当进入导读和阅读器时，则读者知道是小说、知道实际语言、可回首页或重试；不错误跳转，不把具身叙事当当前操作模式，不把迟到章节当当前章节。

<a id="ac-site-discovery-006"></a>
### AC-SITE-DISCOVERY-006：降级仍有完整路径
- 覆盖要求：REQ-SITE-DISCOVERY-006
给定 CN/EN、1440×900、390×844、360×800、200% 缩放、键盘、无 JS 与 reduced-motion，当沿主路径浏览时，则文字、限制、导航、CTA 均可达，没有横向溢出或仅靠颜色传意；媒体失效仍有文字与替代入口。真实浏览器结果按路由和环境分别记录。

验证方法由[系统设计验证矩阵](../../site/github-pages/public-site-discovery-and-showcase.design.md#verification)和现有 [testing-manual](../../../testing-manual.md)承接。自动结构检查不证明人类理解，生成稿不证明浏览器结果，页面合入不证明上线或游戏发行。

## 6. 取舍与未决问题

采用精选静态展示而非实时世界服务，牺牲即时性以减少首期依赖；类型、日期和范围必须显著。采用一个完整案例而非堆叠宏观场景，牺牲题材覆盖以验证因果可读性。采用现有静态技术栈而非 CMS，接受少量编辑同步成本，不引入运行服务。

素材选择仍待当前证据决定：由 `game_visual_interaction_designer` 与 `qa_engineer` 在实施前选定可公开记录；未取得时只保留明确的说明性示例，不声明真实首产物、续接或真人结果。费用/平台信息由 `liveops_community` 对照当前发行文档核实，未知则明确未知。详细排期与执行事实只进入当前 PR 或后续实施 PR。

## 7. 官网参考及采纳边界

核对日期：2026-10-14；官网公开页面读取，不是竞品实机或转化研究。

| 官方来源 | 观察到的结构 | 本专题采纳 / 不采纳 |
| --- | --- | --- |
| [SpaceMolt](https://spacemolt.com/) | 星图、战斗、经济、新闻和更新入口 | 采纳可观察世界的入口；首期不复制实时服务和全量导航 |
| [Screeps](https://screeps.com/) | 明确程序员定位，区分 World/Arena 与参与节奏 | 采纳清晰玩家位置；不把编程当预览先决条件 |
| [Colony](https://parallel.life/colony) | Quick Facts、具体动作、下载与 FAQ | 采纳动作和参与条件速览；不照搬已发售口径 |
| [Factorio](https://factorio.com/) | 玩法介绍、Demo、版本、开发日志与 RSS | 采纳玩家视角进展和稳定内容入口；不承诺固定频率 |
| [BitCraft Discover](https://bitcraftonline.com/discover) | 角色与分篇世界故事 | 采纳短导读连接小说；不把故事当运行历史 |
| [EVE Frontier](https://evefrontier.com/en) | 游戏、Builders、Vision、Roadmap 与文档分层 | 采纳读者分流；不复制账户、市场或发布范围 |

只借鉴信息组织与交互原则；不复制竞品美术、标志、截图或文案。生成设计稿是 oasis7 的方案表达，来源与非证据边界见配对产品设计。

## 专业 owner、authority 与测试层级追踪

| REQ / AC | 专业 owner | 专业权威 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| [REQ-SITE-DISCOVERY-001](#req-site-discovery-001) / [AC-SITE-DISCOVERY-001](#ac-site-discovery-001) | `viewer_engineer` / `qa_engineer` | [site authority](../../site/design.md#homepage-layout-authority) | [系统验证计划](../../site/github-pages/public-site-discovery-and-showcase.design.md#verification)，实现后取得同候选结果；本 PR 只验证文档 | `test_tier_required`；页面实现另按 S6 选择真实浏览器范围 |
| [REQ-SITE-DISCOVERY-002](#req-site-discovery-002) / [AC-SITE-DISCOVERY-002](#ac-site-discovery-002) | `viewer_engineer` / `qa_engineer` | [site authority](../../site/design.md#homepage-layout-authority) | [系统验证计划](../../site/github-pages/public-site-discovery-and-showcase.design.md#verification)，实现后取得同候选结果；本 PR 只验证文档 | `test_tier_required`；页面实现另按 S6 选择真实浏览器范围 |
| [REQ-SITE-DISCOVERY-003](#req-site-discovery-003) / [AC-SITE-DISCOVERY-003](#ac-site-discovery-003) | `viewer_engineer` / `qa_engineer` | [site authority](../../site/design.md#homepage-layout-authority) | [系统验证计划](../../site/github-pages/public-site-discovery-and-showcase.design.md#verification)，实现后取得同候选结果；本 PR 只验证文档 | `test_tier_required`；页面实现另按 S6 选择真实浏览器范围 |
| [REQ-SITE-DISCOVERY-004](#req-site-discovery-004) / [AC-SITE-DISCOVERY-004](#ac-site-discovery-004) | `viewer_engineer` / `qa_engineer` | [site authority](../../site/design.md#homepage-layout-authority) | [系统验证计划](../../site/github-pages/public-site-discovery-and-showcase.design.md#verification)，实现后取得同候选结果；本 PR 只验证文档 | `test_tier_required`；页面实现另按 S6 选择真实浏览器范围 |
| [REQ-SITE-DISCOVERY-005](#req-site-discovery-005) / [AC-SITE-DISCOVERY-005](#ac-site-discovery-005) | `viewer_engineer` / `qa_engineer` | [site authority](../../site/design.md#homepage-layout-authority) | [系统验证计划](../../site/github-pages/public-site-discovery-and-showcase.design.md#verification)，实现后取得同候选结果；本 PR 只验证文档 | `test_tier_required`；页面实现另按 S6 选择真实浏览器范围 |
| [REQ-SITE-DISCOVERY-006](#req-site-discovery-006) / [AC-SITE-DISCOVERY-006](#ac-site-discovery-006) | `viewer_engineer` / `qa_engineer` | [site authority](../../site/design.md#homepage-layout-authority) | [系统验证计划](../../site/github-pages/public-site-discovery-and-showcase.design.md#verification)，实现后取得同候选结果；本 PR 只验证文档 | `test_tier_required`；页面实现另按 S6 选择真实浏览器范围 |
