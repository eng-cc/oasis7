# 开发指挥客户端技术选型决策

日期：2026-10-10。
状态：本次推荐决策，待实施验证。本文记录方案选择和依据，不代表客户端已经实现或完成性能、兼容性验证。
产品形态与首版范围沿用[客户端形态与首版范围](desktop-client.md)；模块、数据流、接口和运行机制见[客户端架构](client-architecture.md)。

## 目录

- [需求与选择判据](#需求与选择判据)
- [七条技术路线比较](#七条技术路线比较)
- [推荐决定与语言边界](#推荐决定与语言边界)
- [具体技术栈](#具体技术栈)
- [主要取舍与重新决策条件](#主要取舍与重新决策条件)
- [首个验证切片](#首个验证切片)
- [资料与事实基线](#资料与事实基线)

## 需求与选择判据

### 产品工作负载

客户端面向开发者日常使用，macOS 优先，在本地连接 oasis7 仓库、GitHub 和可接入的 Codex 开发会话，远端开发机器按需扩展。
它需要持续回答四个问题：项目要去哪里、现在做到哪里、有哪些有效规范与决定、哪些工作需要人介入。

| 视图 | 主要交互 | 技术选择应服务的能力 |
| --- | --- | --- |
| 目标与进度 | 阶段目标、交付证据、依赖与阻塞的关联和展开 | 多列表、分组筛选、来源跳转、状态局部刷新 |
| 规范与决策 | 中文长文档阅读、搜索、版本比较、决定及理由回顾 | Markdown、代码块、表格、diff、文本选择与复制 |
| 开发执行 | PR/CI、worktree、会话和日志的集中查看 | 异步连接、有序增量、长列表与日志窗口、明确的能力范围 |
| 待我决策 | 聚合等待输入的问题、阅读上下文、给出明确操作 | 中文输入、键盘路径、焦点管理、权限与执行结果反馈 |

这些工作主要是数据与文本交互。第一版的主要难度在于来源关联、状态正确性、会话能力边界和可持续维护，不能用渲染框架替代这些工作。

### 与现有工程的关系

事实基线为 [`main@7039bc376882ab8f84f88ca014f6d7ce642539a8`](https://github.com/eng-cc/oasis7/tree/7039bc376882ab8f84f88ca014f6d7ce642539a8)。
该版本的 [Rust toolchain](https://github.com/eng-cc/oasis7/blob/7039bc376882ab8f84f88ca014f6d7ce642539a8/rust-toolchain.toml) 为 `1.96.0`；[Viewer 的 package 声明](https://github.com/eng-cc/oasis7/blob/7039bc376882ab8f84f88ca014f6d7ce642539a8/crates/oasis7_viewer/package.json) 包含 Solid `^1.9.15`、Vite `^8.3.3`、Vitest `^5.0.3`。
这些是仓库配置和包声明，不能视为本客户端已经验证的锁定依赖组合。

独立客户端可沿用 Rust 和 Solid 的开发知识、基础工具及适合的通用代码；它有自己的入口、构建产物和领域状态，游戏 Viewer 的启动不是使用指挥客户端的前提。
技术选型不要求迁移现有 Viewer 或站点，也不改变仓库的开发流程权威。

### 选择判据

按下列问题评估，不制造未经测量的框架评分：

1. 能否以可接受的工作量实现中文文本、Markdown、diff、日志和多列表交互？
2. 能否让 Rust 持有领域逻辑、本地系统访问、凭据和执行控制？
3. 是否有明确的窗口、菜单、通知、签名、公证和更新路径？
4. 是否能验证真实 macOS 的输入、可访问性、IPC 与生命周期？
5. 升级时会影响哪些模块，维护成本是否集中在产品真正需要的能力上？
6. 许可、依赖发布状态和平台支持是否与分发方式匹配？

包体、启动速度、空闲资源占用和日志峰值表现最终以本产品样机测量。框架主页的最小示例、行数或帧率宣传值不作为本项目已达成的指标。

## 七条技术路线比较

表中“适配判断”是基于当前范围的工程决策；能力事实的官方资料列于文末。

| 路线 | 语言与呈现方式 | 已有能力与主要代价 | 本次适配判断 |
| --- | --- | --- | --- |
| **Tauri 2 + Rust + Solid/TypeScript** | Rust 核心；系统 WebView 呈现 HTML/CSS/JS | 有 commands、channels、权限能力、桌面插件与发布链路；需管理 Rust/TS 契约及系统 WebKit 差异 | **推荐主路线**：文本交互、现有技术经验、Rust 核心和桌面交付之间的综合选择 |
| **GPUI + GPUI Kit** | Rust UI 与业务；GPU 原生渲染，macOS 使用 Metal | Kit 已提供虚拟表格/列表、Markdown、代码编辑、Dock、AccessKit 和测试；GPUI 上游仍为 pre-1.0，常有破坏性升级，安装与更新仍需应用装配 | **主要纯原生备选**：若 UI 也必须 Rust、必须摆脱 WebView，优先评估此组合 |
| **Dioxus Desktop** | Rust UI 与业务；桌面仍由系统 WebView 呈现 | 保留 DOM/CSS 文本能力，提供本地 API 与桌面打包；使用成熟 JS 控件时需要互操作封装 | **Rust UI + WebView 备选**：满足 UI 代码用 Rust 的偏好，但仍要承担 WebKit 兼容性 |
| **egui + eframe** | Rust，立即模式自绘 UI | 有输入控件、可见行渲染、AccessKit 和测试工具；日常长文档、复杂 diff、完整文本工作台要进一步装配 | 适合诊断器、图形子面板或精简工具；本次不作为主要工作台界面 |
| **iced** | Rust，Elm 风格状态与消息，自绘 UI | 0.14 已提供 IME、table/grid、增量 Markdown 和测试能力；应用仍需验证可访问性、复杂文本和桌面集成 | 可实现首版，但相较推荐路线需要承担更多组件与集成验证 |
| **Slint** | Rust 业务 + `.slint` 声明式 UI | 有 ListView、表格、文本编辑和可访问性属性；StyledText 的 Markdown 子集对文档显示有具体限制，分发需选定适用许可 | 更适合界面与交互边界明确的工具；当前长文档范围增加额外实现工作 |
| **Electron + Rust 核心** | Node.js 主进程、Chromium UI、另接 Rust | 桌面与 Web 组件路径明确，渲染器版本可随应用控制；Rust 核心之外增加 Node 主进程与通信边界 | 若出现不可替代 Electron 组件或固定 Chromium 的强需求再考虑 |

GPUI Kit 的当前正式资料已覆盖文本和复杂数据控件，不能以“Rust GUI 没有 Markdown、表格或可访问性”作为排除理由。
选择 Tauri 的理由是当前产品交互、已有经验和交付链路的组合成本，而非断言原生路线做不到。

## 推荐决定与语言边界

**采用 Tauri 2 的受支持稳定补丁版本，使用 Rust 核心与 SolidJS / TypeScript strict / Vite 前端。**

### Rust 的所有权

下列代码放在 Rust 核心：

- 目标、规范、决定、工作项、交付证据和会话的领域模型及关联规则。
- 本地仓库/Git/worktree、GitHub 与 Codex 的适配器。
- 数据来源版本、读取范围、更新时间、不完整状态与冲突处理。
- 本地数据库、文档解析、搜索索引、日志保留和后台刷新。
- 凭据读取、工作区范围检查、命令参数校验、执行授权和结果落地。
- 后台任务、订阅与子进程的生命周期，以及 macOS 原生能力调用。

这些模块不依赖 Solid Signal、Tauri Window 或某个原生 UI 框架的状态类型。桌面壳只负责适配 UI 与核心；以后调整界面技术时，保留领域模型和连接器即可，无须预先建设通用 GUI 框架。

### TypeScript 的职责

TypeScript 负责组件、路由、可访问性、键盘交互、表格和文档呈现，以及筛选、展开、选中项等界面状态。
它通过有限的类型化 bridge 调用 Rust，呈现有来源和状态的数据；不会自行推断任务完成、保存长期凭据或获得任意 shell/文件系统访问。

这满足“Rust 优先”：主要应用能力与控制边界由 Rust 实现，界面保留更直接适配现有交互的技术。
如果“UI 也必须 Rust”成为硬要求，按后文条件重新决策，不把 Rust 业务核心推倒重写。

### 为什么使用 Solid 而非切到 React

现有 Viewer 已声明 Solid、Vite 和 Vitest，沿用这些工具能减少新的维护面。
Solid 提供细粒度响应式；TanStack 官方同时提供 Solid Table/Virtual，Kobalte 提供焦点、键盘和 ARIA 基础组件，足以覆盖当前四视图的核心交互。
这些能力证明方案可装配，并不证明其在本产品中一定比 React 更快。

React + TypeScript 是可接受替代：如果后续确定的关键组件只有成熟 React 版本，或者主要维护者的有效经验集中在 React，应据真实组件与维护成本调整。
当前没有发现必须为四视图新增 React 技术栈的独占需求。

## 具体技术栈

### 组件选择

| 层次 | 具体选择 | 用途与约束 |
| --- | --- | --- |
| 语言与编译 | Rust stable，起点对齐仓库 `1.96.0` | 随实际依赖兼容性确认；桌面应用维护自己的锁定依赖组合 |
| 桌面壳 | Tauri 2、官方桌面插件 | 窗口、菜单、通知、文件选择、窗口状态；使用带安全修复的稳定补丁 |
| UI | SolidJS + TypeScript strict + Vite | SPA；静态资源打包进入 App，开发预览复用同一界面 |
| 基础控件 | Kobalte + CSS 设计变量 | 复用语义、焦点和键盘行为；复杂交互按实际使用选择控件 |
| 表格与列表 | `@tanstack/solid-table`、`@tanstack/solid-virtual` | 对大列表和日志使用虚拟化；简单列表保留普通组件 |
| 异步与进程 | Tokio、`tokio::process`、有界 channel | 网络、子进程 I/O、取消、超时、订阅；阻塞工作进入受限线程 |
| 数据契约 | `serde`、`serde_json`、`ts-rs` | Rust 定义 DTO，生成 TS 类型；bridge 以少量明确命令组织 |
| 本地存储 | SQLite + `rusqlite` 的 `bundled` feature | 独立数据库线程持有连接，经有界消息队列服务；持久化及索引结构见架构分册 |
| Git | 系统 Git CLI | 采用稳定机器输出读取状态、worktree、提交和 diff；结构化参数，不拼接 shell |
| HTTP | `reqwest` + rustls | GitHub 等外部 API、超时、分页、缓存验证、限流与取消均在 Rust 中处理 |
| 文件变化 | `notify` + 去抖与补充重扫 | 变化通知触发校准读取；恢复、手动刷新和必要轮询保障状态收敛 |
| Markdown | `pulldown-cmark` | Rust 解析文档，保留来源位置；受限结构交给 UI，原始 HTML 不直接注入 |
| 凭据 | macOS Keychain | Rust 适配器读取；具体 crate 和账户隔离见[客户端架构](client-architecture.md) |
| 诊断 | `tracing` / `tracing-subscriber` | 结构化本地诊断；与 Agent 业务日志分开，过滤凭据和敏感参数 |
| 测试 | Rust 测试、Vitest、WebdriverIO Tauri service | 分别验证领域与适配器、前端组件、真实桌面链路 |
| 打包与更新 | Tauri bundler / updater | `.app` 与 DMG，Apple 签名公证；独立更新包签名与静态更新清单 |

上表确定技术组件，不把检索时看到的 `latest` 当作可直接安装的精确版本。
实现时核对 MSRV、Cargo feature、前后端 API 兼容性和锁文件；只锁实际通过切片验证的组合。
Tauri 的插件、运行时和前端包各有版本，不要求其版本号表面完全相同。

### IPC、流式数据与类型契约

生产客户端使用 Tauri IPC 连接前端与 Rust 核心，不需要常驻本地 HTTP 服务或 Node 服务端。
command 负责请求响应；channel 传递有序增量；event 只用于少量状态通知。官方明确 event 不适合高吞吐和低延迟数据流。
Channel 的有序交付不能代替消费者背压：批次、队列上限、确认游标、补读和订阅回收由应用定义，详见架构分册。

选 `ts-rs` 是为了稳定生成 DTO，而非宣称完整端到端类型安全。命令名、参数和错误结果仍由 bridge 与契约验证约束，外部数据在 Rust 中按实际协议解析。
Git OID、远端节点 ID 和会话 ID 用字符串；大整数与可空字段的 JSON 表示须明确，避免 TypeScript `bigint` 与普通 JSON number 不一致。

本次不选 `tauri-specta` 作为默认依赖：调研时适配 Tauri 2 的发布为 `2.0.0-rc.25`，其稳定 `latest` 文档仍对应适配 Tauri 1 的 `1.0.2`。
待 Tauri 2 对应路线稳定、维护收益明确时可替换生成工具，DTO 语义和 IPC 访问范围保持一致。

### 存储、Git 与文档能力的取舍

SQLite 适合单机索引、快照和有限执行记录；`rusqlite bundled` 使应用使用自己的 SQLite 构建，减少对用户系统 SQLite 版本的依赖。
独立数据库线程隔离同步数据库调用，避免阻塞 Tokio I/O 与 UI。第一版无需同时引入数据库连接池、通用 ORM 和远端数据库服务。

Git CLI 保持与开发者实际仓库行为接近，首版以机器可读输出实现所需读操作。
使用 Git 不意味着允许执行仓库任意脚本；适配器只调用已定义操作，验证工作目录和参数。后续只有实测进程开销或特定读取能力成为瓶颈时，才评估 `gix`/`git2`。

`notify` 的文件事件存在平台与文件系统差异，不能作为文档和仓库状态的唯一事实来源。
Markdown 解析保留标题、表格、代码与链接等产品需要的结构，展示按白名单元素实现；diff 与日志保持文本语义。第一版不因显示代码就引入完整 IDE 或终端模拟器。

### macOS 与发布路径

Tauri 在 macOS 使用 WKWebView，系统 WebKit 的行为由平台决定。受支持 macOS 版本、中文 IME、焦点和辅助技术必须在真实 App 上验证。
原生菜单、窗口状态、通知和更新入口走 Rust 桌面层；关闭窗口、退出应用和外部开发任务生命周期的关系由产品定义。

发布包启用严格 CSP 和明确 capabilities，app commands 纳入 `AppManifest::commands`；长期凭据和业务授权保留在 Rust 内。
生产 WebView 只加载打包界面，远端内容作为数据阅读，外部网站用系统浏览器打开。
Tauri `2.11.1` 已修复 remote origin 在无 AppManifest 时的自定义 command ACL 绕过，选用包含该修复的受支持稳定补丁。

对外分发采用 Developer ID 签名、公证与 stapling；自动更新继续验证 Tauri 更新包签名。两类签名承担不同职责。
可用静态更新清单和发布文件完成更新分发，第一版无需自建更新服务。

## 主要取舍与重新决策条件

### 第一版不采用的组件

- 不引入 SSR、Next.js、SolidStart 服务端、生产 Vite 服务或 Node 后端；它们没有解决当前桌面工作台的新增需求。
- 不把 Redis、PostgreSQL、消息代理或向量数据库作为单机客户端基础依赖；先以本地来源、SQLite 和明确搜索需求实现。
- 不为桌面 UI 新建常驻 Axum HTTP 服务；未来远端桥接出现独立需求时再添加窄接口。
- 不用全局 event 广播高频日志或敏感状态，不给前端通用 shell、全磁盘文件访问或任意 URL 代理。
- 不同时实现多个 UI 框架原型，不把运行模型任务作为刷新和样机展示的必要条件。

### 重新决策触发

| 触发条件 | 优先重新评估 | 需要拿出的依据 |
| --- | --- | --- |
| 用户将“UI 必须 Rust、不得使用 WebView”设为硬要求 | GPUI + GPUI Kit | 真正的文档、日志、中文输入与 VoiceOver 切片；固定 Kit release 和匹配 GPUI snapshot |
| 仍可用 WebView，但希望 UI 代码也统一 Rust | Dioxus Desktop | 所需表格、Markdown、diff 组件和 JS 互操作成本；锁定 Desktop WebView 路线 |
| WKWebView 在受支持系统上出现不可接受且无法合理修复的关键交互问题 | GPUI Kit 或 Electron | 可复现案例与用户影响；原生化或固定 Chromium 是否实际解决问题 |
| 关键业务控件只有可维护的 React 实现 | Tauri + React/TypeScript | 必需能力、替代成本和维护者经验；无需更换 Rust 核心 |
| UI 性能问题经采样确认来自渲染层而非数据流水线 | 对应 UI 层优化或原生方案 | 固定工作负载、机器、版本、阶段耗时与内存样本 |

GPUI Kit 是认真保留的备选，不把它视作低能力路线；若进入该路线，采用成套 Kit 组件与固定依赖，避免直接追踪 GPUI main。
其打包文档仍要求应用处理安装、签名与发布验证，更新功能也不替应用决定版本和替换安装，切换时应把这些工作计入成本。

## 首个验证切片

只沿推荐技术栈做一个小而真实的纵向切片，验证最容易影响决策的地方。
以固定版本的真实 oasis7 文档、本地临时 Git fixture、GitHub/Codex 协议 fixture 为输入；读取或显示状态不启动模型工作。

| 切片 | 要观察的结果 |
| --- | --- |
| 打开项目并加载一组目标/工作项 | Rust 读取与分页结果可展示，来源、版本及刷新状态可追溯 |
| 阅读一篇中文文档及一份 diff | 标题、表格、代码块、链接、选择复制、搜索定位和受限内容显示正确 |
| 列表筛选与日志持续追加 | 界面保持可操作；队列/在途数据/DOM 受限；慢消费者和重连有明确处理 |
| 中文搜索和给会话追加指令的输入框 | 组合输入、候选选择、撤销与快捷键不冲突，选词 Enter 不误提交 |
| 键盘与 VoiceOver 走完主流程 | 焦点顺序、可读名称、错误提示与虚拟列表上下文可理解 |
| 重载窗口、断开来源、重新打开应用 | 核心状态与缓存可恢复；UI 订阅回收不会擅自终止外部任务 |
| 构建实际 `.app` 并运行桌面测试 | 核实真实 IPC/Channel/文件路径；生产构建不含测试服务和调试执行能力 |

现行 Tauri 官方测试文档已支持 macOS 的 `@wdio/tauri-service` embedded provider；不能沿用“macOS 完全没有 Tauri E2E”的旧结论。
传统直接使用 `tauri-driver` 仍只支持 Windows/Linux。macOS 的内嵌 WebDriver 插件仅放进测试构建，并隔离相应 capabilities。
浏览器测试不能证明真实桌面 IPC 正确，内嵌 WebDriver 也不代替系统 IME、VoiceOver、通知授权和签名包启动检查。

切片记录冷启动、空闲占用、日志高峰响应和峰值内存时，应同时写明机器、系统、构建模式、数据规模与持续时间。
本次文档没有这些测量结果；实现阶段据结果确认局部优化或上述重新决策条件。

## 资料与事实基线

以下为本次读取的官方项目、维护者文档或发布记录。能力描述按调研日期理解，精确依赖以实施时验证的锁文件为准。

| 范围 | 一手资料 | 本文使用的事实 |
| --- | --- | --- |
| 仓库版本 | [固定提交](https://github.com/eng-cc/oasis7/tree/7039bc376882ab8f84f88ca014f6d7ce642539a8)、[Rust toolchain](https://github.com/eng-cc/oasis7/blob/7039bc376882ab8f84f88ca014f6d7ce642539a8/rust-toolchain.toml)、[Viewer package](https://github.com/eng-cc/oasis7/blob/7039bc376882ab8f84f88ca014f6d7ce642539a8/crates/oasis7_viewer/package.json) | 现有工程和包声明基线 |
| Tauri 架构 | [Process Model](https://v2.tauri.app/concept/process-model/)、[Calling the Frontend](https://v2.tauri.app/develop/calling-frontend/)、[Channel API](https://docs.rs/tauri/latest/tauri/ipc/struct.Channel.html) | Rust 核心、系统 WebView、Event 与 Channel 的边界 |
| Tauri 安全 | [Capabilities](https://v2.tauri.app/security/capabilities/)、[CSP](https://v2.tauri.app/security/csp/)、[2.11.1 发布](https://v2.tauri.app/release/tauri/v2.11.1/) | 明确权限、CSP 配置和已修复的来源检查问题 |
| Tauri 测试与分发 | [WebDriver](https://v2.tauri.app/develop/tests/webdriver/)、[WDIO 插件](https://webdriver.io/docs/desktop-testing/tauri/plugin-setup)、[macOS 签名](https://v2.tauri.app/distribute/sign/macos/)、[Updater](https://v2.tauri.app/plugin/updater/) | macOS 测试路线、测试隔离、签名与更新 |
| Solid UI | [Solid 响应式](https://docs.solidjs.com/concepts/intro-to-reactivity)、[Table 框架适配](https://tanstack.com/table/latest/docs/framework)、[Virtual 框架适配](https://tanstack.com/virtual/latest/docs/framework)、[Kobalte](https://kobalte.dev/docs/core/overview/introduction) | 现有响应式、表格、虚拟化和交互组件 |
| GPUI / Kit | [GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md)、[Kit 文档](https://gpui-kit.com/docs/)、[v0.7.1](https://github.com/longbridge/gpui-kit/releases/tag/v0.7.1)、[打包](https://gpui-kit.com/docs/packaging/)、[更新](https://gpui-kit.com/docs/auto-update/) | 原生备选已有的工作台组件、升级与分发职责 |
| Dioxus | [Desktop](https://dioxuslabs.com/learn/0.7/guides/platforms/desktop/)、[Desktop API](https://docs.rs/dioxus-desktop/latest/dioxus_desktop/) | Rust UI 与系统 WebView 的实际关系 |
| egui / iced | [egui](https://github.com/emilk/egui)、[egui 可访问性](https://github.com/emilk/egui/blob/main/docs/accessibility.md)、[iced 发布记录](https://github.com/iced-rs/iced/releases) | 现有输入、测试与组件能力，避免依据旧印象排除 |
| Slint | [StyledText](https://docs.slint.dev/latest/docs/slint/reference/elements/styledtext/)、[Royalty-free 2.0](https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md) | Markdown 范围和可选择的分发许可 |
| Electron | [Process Model](https://www.electronjs.org/docs/latest/tutorial/process-model)、[Security](https://www.electronjs.org/docs/latest/tutorial/security) | Node 主进程、Chromium 渲染及进程安全边界 |
| Rust 数据与 I/O | [Tokio channel](https://tokio.rs/tokio/tutorial/channels)、[rusqlite](https://github.com/rusqlite/rusqlite)、[reqwest](https://docs.rs/reqwest/latest/reqwest/)、[notify](https://docs.rs/notify/latest/notify/)、[Git status](https://git-scm.com/docs/git-status)、[pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) | 可用组件及有界任务、数据库封装、文件通知边界 |
| 类型生成 | [ts-rs](https://github.com/Aleph-Alpha/ts-rs)、[tauri-specta RC](https://docs.rs/crate/tauri-specta/2.0.0-rc.25) | DTO 生成范围、serde 兼容边界和当前发布状态 |
