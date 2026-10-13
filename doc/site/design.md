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

<a id="manual-upgrade-first-write"></a>
### 推荐资产校验、手动升级首写与安全回退

本节承接[玩家接入与发行 PRD 的 SC-8 / SC-10](../product/player-entry-distribution/prd.md#download-integrity-and-upgrade)，
是下载说明、平台预检、Launcher 与 runtime 之间的目标编排合同，不表示这些入口已经实现或平台验收已通过。
沿用手动下载/覆盖的产品范围，不新增自动更新服务、自动迁移、跨目录状态保留或默认 host 运维权限。

| 主责 | 接收边界 |
| --- | --- |
| site / viewer_engineer | 推荐资产与安装说明绑定同一候选；平台/Launcher 预检负责在所有可能触碰原状态的入口前执行本合同，不能只在窗口显示后检查 |
| packaging owner | [来源、新鲜度与最终安装布局](../engineering/workflow/package-source-security.md#final-artifact-freshness)；为候选提供可核对的字节身份与准确的平台/状态兼容信息 |
| runtime_engineer | 旧 execution-world 格式/代际的只读识别、允许的版本转换及恢复边界；世界权威不由 Launcher 重定义 |
| qa_engineer | 按下表逐受支持平台保留真实预检、首写边界与恢复证据，不能以网页静态 gate 代签安装/状态兼容 |

**1. 先验证要用的那一份资产。** 从 `eng-cc/oasis7` 的受信 Release 入口固定候选版本/发布标识、
平台/架构、资产名、checksum 算法和预期值。下载到不覆盖现有安装、配置、备份或世界数据的独立暂存位置，
用本机校验工具重算实际字节摘要；缺 checksum、来源/候选关联无法核实、算法不支持、读文件失败、
平台不符或摘要不匹配均停止，不能运行该 `.exe`、开始复制 `.app` 或安装 `.deb`。
页面展示 checksum 和下载成功不等于已执行核对，checksum 也不代替签名/notarization 或发行者认证。

验证通过后，预检、解包与安装必须消费同一受保护文件对象；固定版本 URL、文件名、大小或 mtime 不足以
排除替换。平台入口须控制暂存对象的链接/替换/并发写入，使用保持对象身份的读取方式，发生身份变化则重验
或拒绝；不能把一次旧摘要用于后来重新下载的同名文件。不防御已控制相同 OS 账号的恶意进程，不能借此
声称本机已可信。无法约束校验到使用的链路时，不提供“继续覆盖”的成功路径。

**2. 在读取兼容信息前固定原状态与写入方。** 在用户获准的本地升级窗口中停止或隔离该安装的写入方，
包括 Launcher 自动保存、关联进程和本地 runtime；不能擅自停止共享服务或其他用户的进程，无法静止就阻塞。
从实际安装/profile 解析原位置，不能把当前工作目录当成状态目录。清点以下三类及安装器额外会接触的状态：

| 状态 | 预检至少读取的事实 |
| --- | --- |
| `config.toml` | 原文件/目录身份、存在或缺失、实际配置格式/版本、候选是否支持读取；不能先写默认值再验证 |
| `.oasis7_launcher_ux_state.json` | 实际 schema/版本、关联安装/profile 与候选读取能力；窗口启动、上次运行提示等自动保存也是写入 |
| `output/chain-runtime/<node_id>/reward-runtime-execution-world/` | 解析后的实际目录、world/genesis/profile、格式与 generation/checkpoint；只读核对，不先恢复/启动世界来猜是否兼容 |

备份绑定原安装版本、平台、primary mode、实际状态位置与格式/代际，验证副本完整性与可读取性；不存在的
文件也记录为不存在，不能由空默认文件替代。备份与原状态分离保存，权限不放宽；原始路径、内容和状态摘要
留在受控本地诊断，不上传到公开页面。runtime checkpoint 或单个目录的原子性不能代签另两类状态。

**3. 给出有来源的兼容结论。** 候选必须提供版本化的兼容描述，逐类列出允许读取的原格式/版本、
是否需要迁移、目标格式与已验证的转换；平台预检只读取该描述和原状态，不启动默认写路径。
首期可只支持已验证的原格式直接读取，明确拒绝需要迁移或未知的组合；不得用“有备份”代替迁移实现。
额外迁移只有专业域合同与适用验证齐备后才可声明支持。不能从 semver 大小、进程曾启动或安装器 exit=0
推断兼容；读取新世界协议/历史的能力也须按 runtime 主责核对。

预检输出是有版本的本地 `UpgradeAssessmentV1` 目标记录：
`assessment_id / candidate(version, platform, arch, asset, algorithm, expected_digest, actual_digest) /
previous_install / primary_mode / state_inventory(format, generation, backup_ref, read_result) /
phase / result / reason / permitted_next_action`。
结果只使用 `compatible / blocked_unchanged / recovery_required`；各类读取失败、未知 schema 和缺失证据
分别保留原因。该记录不是新网络 API、签名证书或世界回执，也不将以上字段假称当前 Launcher 已导出。

**4. 首写边界覆盖整条路径。** 安装器及其脚本、Launcher 初次载入/自动保存、runtime 打开数据目录、
恢复/迁移和选择新 generation 都必须纳入预检后的写入边界。只有资产有效、备份已核对且各类状态兼容后，
才进入已授权的写阶段；不能先让新 binary 自动写状态再补做检查。候选暂存和受控预检日志可写在原状态之外，
这不构成原状态改写，也不能成为隐蔽覆盖原路径的别名。

在可能写原状态前，将 `state_write_started` 与 assessment/candidate/原状态身份持久记录到独立的受控
升级诊断位置。记录失败则不开始写入；开始标记后的异常、标记完整性无法核实或进程中断均按可能已改写处理。
`blocked_unchanged` 必须同时有受约束的写入口、写入前正常停止记录、持续静止的写入方，以及三类原状态
和安装器涉及状态的前后存在性/内容/代际核对；单靠缺少日志、mtime 未变、UI 显示失败或旧备份存在不能证明。
这个保守标记不声称实现跨配置、UX 与 runtime 的原子事务。

| 结论 | 允许的下一步与禁止项 |
| --- | --- |
| compatible | 仅继续该候选和已验证状态组合的安装/启动；完成后仍按各专业域回读模式、原任务与权威结果，不自动恢复 session/Agent 授权 |
| blocked_unchanged | 原状态核对成立后可退出新候选；需要恢复时仅使用绑定来源且已验证的备份与明确旧资产，再核对旧资产完整性及其对当前网络协议/历史的支持。不得只说“下载旧版” |
| recovery_required | 停止新启动和反复覆盖，保留原数据/备份/诊断；进入适用的当前版本重装、隔离数据或 runtime 恢复路径。修复方案未确定时明确阻塞，不建议旧程序打开可能升级的状态 |

任何本地回退都不重写远端世界历史、已接受行动、receipt、identity/session 或 Agent 授权。
已发生写入但后来成功启动，也不能抹去之前的未知结果；专业恢复另行核对。旧版本清理仅沿
[成功提交后的 retention 合同](../engineering/workflow/package-source-security.md#release-upgrade-retention)，
不能为了腾空间提前删除唯一恢复资产或备份。本节不重复定义 SOURCEPLAN、新鲜度、安装布局或 retention。

<a id="manual-upgrade-validation"></a>
### SC-8 / SC-10 的平台验证映射

下列用例在每个声明支持手动升级的平台/primary mode 上分别登记；Windows `.exe`、macOS `.dmg`
及其实际 `.app` 替换、Linux `.deb` 的安装机制不能相互代签。不支持的组合注明范围与原因，不能填通过。
确定性判断可先用普通单元/合同测试，真实平台验证必须覆盖最终发布资产及实际入口；文档 PR 不运行 host 升级。

| 场景 / 上游 | 给定、故障点与通过判据 |
| --- | --- |
| missing / unverifiable / mismatch，SC-10 | 分别移除 checksum、使其来源关联/读取不可完成、修改资产字节；所有实际入口在安装/覆盖和原状态写入前停止，给出预期/实际结果或未知原因 |
| replaced candidate，SC-10 | 校验后替换同名文件、换平台或换版本；后续入口拒绝对象漂移或重验新候选，不能复用旧检查结果 |
| compatible，SC-8 | 三类已知旧格式与有效备份，实际资产从预检到进入所声明模式；验证状态连续性和原 operation 回查，不把进程健康当全部状态兼容 |
| unchanged failure，SC-8 | 分别在不支持的配置/UX/runtime 格式以及写入前正常失败处停止；三类状态、额外安装器状态与原代际不变，明确旧资产/备份可验证，恢复不会复活世界 authority |
| uncertain write，SC-8 | 在首写标记前后、配置/UX 写入及 runtime 迁移/代际选择处中断，另注入标记损坏；无法证明未写时为 recovery_required，没有盲目降级入口，不清理原资料 |
| retrieval and retry，SC-10 / SC-8 | 重新获取正确资产后重新绑定并校验，不继承原候选成功值；后续仍过独立的兼容检查，完整性通过不解除已有状态恢复阻塞 |

证据绑定 source/build、Release/平台/模式/资产摘要、兼容描述版本、原/目标格式、备份来源、故障点和
首写边界前后结果；受控本地记录可保留状态摘要，公开结论只保留脱敏关联。QA 逐条记录 pass/fail/blocked/not_run
及剩余缺口，不从某平台绿色结果外推其他平台。签名/notarization、公开 claim 升阶与真实世界恢复仍消费各自
专业证据；本节的目标合同不能作为这些能力已交付的证明。

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
