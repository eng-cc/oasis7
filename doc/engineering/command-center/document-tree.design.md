# 指挥中心文档树系统设计

- 文档 ID：`DESIGN-ENGINEERING-CC-DOCUMENT-TREE`
- 状态：`draft`；树目标方案尚未完整实现，已有章节索引基础见 §4；未验收树桌面交互
- owner_role：`runtime_engineer`
- module：`engineering`
- 上游：[REQ-CC-016](command-center.prd.md#REQ-CC-016)、[AC-CC-016](command-center.prd.md#AC-CC-016)
- 总体设计：[指挥客户端系统设计](command-center.design.md)
- last_reviewed_at：2026-10-13
- 核对基线：`main@b7e21b23bf0edfe86f302bb7186aab77e57b26c3`

## 1. 目标、现状与边界

为规范与决策视图提供可追溯文档全局投影，以结构树定位产品与专业设计，以关系详情解释需求如何被设计承接。源 Markdown 拥有条款，客户端只拥有派生索引与个人阅读偏好。遵循[产品四域边界](../../product/README.md)和[专业文档组织规范](../doc-governance/doc-structure-standard.design.md)，不迁移文档、不自动修复正文、不增加文档登记或发布门禁。

| 对象 | 基线事实 | 目标与差距 |
| --- | --- | --- |
| 文档索引 | [index.rs](../../../tools/oasis7-command-center/core/src/index.rs)按 tracked 路径读取并生成章节，单文件/总量/数量有界 | 复用章节及生命周期解析；已有工作树文件清单及逐项覆盖；固定基线读取和树投影待实现 |
| 来源身份 | 现有 Project 含 worktree、HEAD，章节含内容摘要 | 已有章节索引复核 HEAD、tracked 清单、dirty 路径和已读内容摘要，变化时拒绝候选；已有文件覆盖记录；完整文件节点关系与独立树快照仍待实现 |
| 导航与关系 | 仓库入口和配对链接存在；没有据此证明专用树 UI | 增加明确导航关系、配对关系和可见未归属集合 |
| 桌面验证 | 本次仅源码与文档核对 | 未执行新 UI、读屏、性能或模型验收；不得以设计表认定已通过 |

## 2. 上游承接

### 2.1 需求承接与分配表

| 上游条款 | 独立义务与适用条件 | 本设计条款 | 外部 owner / dependency | 明确排除或未覆盖 |
| --- | --- | --- | --- | --- |
| [REQ-CC-016](command-center.prd.md#REQ-CC-016) | 三种层级、确定关系与来源；产品适用性：N/A；reason=只读开发文档导航、索引与个人阅读体验，不定义游戏规则或玩家验收；scope=三种层级、确定关系与来源 | [DES-CC-TREE-001](#DES-CC-TREE-001) | 文档 owner；产品 README、专业入口及显式配对 | 不推断语义主责或设计完成率 |
| [REQ-CC-016](command-center.prd.md#REQ-CC-016) | 版本、覆盖、局部失败和只读边界；产品适用性：N/A；reason=只读开发文档导航、索引与个人阅读体验，不定义游戏规则或玩家验收；scope=版本、覆盖、局部失败和只读边界 | [DES-CC-TREE-002](#DES-CC-TREE-002) | Git / Rust adapter、既有索引 | 不扫描私有库，不访问任意远端 |
| [REQ-CC-016](command-center.prd.md#REQ-CC-016) | AC-CC-016：定位、往返、恢复与可访问操作；产品适用性：N/A；reason=只读开发文档导航、索引与个人阅读体验，不定义游戏规则或玩家验收；scope=定位、往返、恢复与可访问操作 | [DES-CC-TREE-003](#DES-CC-TREE-003) | Solid UI / Tauri 宿主 / QA | 设计交付不证明真实桌面通过 |

<a id="DES-CC-TREE-001"></a>

## 3. DES-CC-TREE-001：文件、关系与树投影

Rust 构建文件级 `DocumentNode`，至少含 worktree/project 身份、仓库相对路径、标题、kind、声明 ID、原始生命周期声明及独立规范化类别/解析状态、内容摘要、snapshot ID、dirty 状态。文件键使用项目/工作树身份与规范化路径；文档 ID 只作为检索字段，重复 ID 产生诊断，不合并文件。章节沿既有章节定位，关系的源/目标 fragment 保留完整路径及源位置。

`DocumentEdge` 保存 from/to、kind（navigation / paired_design / upstream_requirement / reference / successor）、声明位置、源摘要及解析结果（resolved / missing / ambiguous / outside_scope / unread）。只有入口的模块/专题导航、明确配对字段及设计上游/承接表产生相应 typed edge；其他 Markdown 链接为 reference。解析支持相对路径、显式锚点和 Markdown 标题锚点，路径沿仓库治理范围规则；导航按下述大小写/标题合同定位，不继承治理校验的宽松 fragment 大小写匹配；重复锚点不静默选第一项。链接不存在与目标超出本次读取范围分别记录。远端链接仅作为引用显示，刷新不跟随网络抓取。

产品投影固定从产品 README 的四域入口读取模块与专题声明；异常额外模块作为诊断，不自动新增第五域。专业系统投影从专业域入口与模块 design 导航展开，同域工程 PRD 可作为需求关联入口；专题归属来自明确导航，无法定位的文件进入“未归属/待核对”。目录投影按路径组织所有候选 PRD/design，README 仅作为结构入口；类型以根 prd/design 与专题后缀为默认，显式声明冲突保留诊断。

结构节点与文件节点分开，树 occurrence 只引用文件键；一个文件被多个父项引用时显示多个引用入口，详情与计数按文件去重。树展开时维护当前祖先集合，遇到循环展示“循环引用”链接并停止该路径展开；交叉域和多对多配对在详情列为关系，不强行放入唯一父子树。排序沿源导航顺序；无序集合按路径稳定排序。

计数标注快照范围、生命周期筛选和去重口径。配对计数只统计已解析 paired_design，不将普通引用或 upstream_requirement 自动提升为配对；无配对、未读取、明确缺失分别计数。生命周期原样保留源声明（包括 `target-contract` 等专业值），另存规范化的已知类别 proposed/draft/active/superseded/retired；其他已声明值归 other，可按原值筛选，只有缺声明才显示 unknown。规范化只提取状态字段首个完整 token（移除包裹反引号并按大小写归一），首项若为 Markdown code span，只取该完整 code span 的内容；否则以空白、中文/英文分号、逗号及左右圆括号（含全角括号）作为 token 终止符；已知 token 后的说明保留在 raw，不影响类别，例如 draft 后附目标说明归 draft，active 后附 owner 信息归 active，target-contract 保持 other。声明为空或格式歧义时单独标记解析诊断，不猜状态。默认隐藏明确历史类别的叶项，但被当前条款引用时显示历史引用占位。生命周期不决定运行或发布状态。

<a id="DES-CC-TREE-002"></a>

## 4. DES-CC-TREE-002：读取、快照与失败恢复

数据流为：用户选基线 → Git 文档清单 → 有界源读取 → Markdown/生命周期解析 → 关系校验 → 三种树投影 → SQLite 派生缓存 → 有限 DTO → UI。Rust application 组织快照，index adapter 读取源，domain 保持关系语义，UI 不自行扫描文件或推断权威。沿既有 core、Tauri bridge 和生成 DTO 扩展，不新增常驻服务或依赖。

已提交模式以固定 commit OID 的树列举和 blob 读取，切换分支不改变快照。工作树预览以选定根目录读取 tracked 文件和用户显式纳入的 doc 范围内 untracked Markdown；排除 ignored 文件、私有库、会话及所有符号链接（包括路径祖先中的链接）；固定提交中的 symlink blob 也只标记跳过，不解析为正文。每个候选记录 read/skipped/deleted/error 与原因，先列举再读取，不能沿用现有静默跳过来声称完整覆盖。

沿现有 3000 文件、1 MiB 单文件、16 MiB 总量限制作为初始预算；入口优先读取，超限生成可见覆盖说明，未读节点仍可定位路径。解析与树深度有界，循环不递归读取新文件。工作树扫描前后复核候选清单、HEAD 及已读文件摘要；发现改变时丢弃候选，保留旧快照并提示重试，不无限自动重扫。读权限失败只标记相关节点，部分成功快照明确为 partial。

快照含基线模式/OID、工作树身份、清单摘要、生成时间、每文件摘要和覆盖结果。完整成功候选以事务替换当前快照；错误或部分候选单独保存为本次尝试，最后成功快照仍作为默认阅读快照并标明 stale、原版本及本次失败/遗漏范围，用户可显式查看部分尝试。首次无成功快照时可以展示 partial，并明确没有完整基线。节点与边不可混用两个快照；同项目较早异步请求不能覆盖较新选择。缓存过期时保留旧来源版本及原因，重建只替换派生数据，不删除个人关联、阅读偏好或未决操作。

升级采用独立版本化的 document_tree_v1 派生缓存键/表，不向当前持久 Snapshot 的反序列化模型新增必填字段。旧缓存仍可供既有视图读取；树缓存缺失、版本不兼容或损坏时只重建树派生对象，不清空个人表和原 Snapshot。节点、关系、覆盖与部分尝试可重建，个人阅读偏好使用独立版本化记录和可选默认值；失败只回退树入口，不阻止原项目打开。缓存 schema 初始化及写入采用事务，失败保留旧 schema；旧 App 忽略新表/键，回退不删除个人或未决数据。生成 DTO 与 bridge/UI 随同包原子更新，tree 查询携带 schemaVersion，未知版本返回明确 unsupported，旧客户端不消费新 DTO；既有查询响应保持原 shape。实施回归须覆盖旧数据库打开、损坏树缓存、迁移失败和旧版本回读，确认个人关联/草稿/未决操作保留。

本功能不执行 fetch、checkout、模型调用或源文档写入。源 Markdown 视为不可信展示数据，复用安全渲染和受控源打开；拒绝目录逃逸、任意 scheme 与原文内脚本/命令执行。不存在的本地目标只有在完整列举范围内核实后标 missing，外部/未读目标标 outside_scope/unread。

现有章节索引已承接 §4 的扫描前后复核基础：复核 tracked 清单、dirty 路径、HEAD 与预算内已读文件的内容摘要；发现变化返回可重试的读取错误，由既有刷新路径保留旧索引和来源版本。校验是有界的二次观察，不提供文件系统锁或固定提交 blob 快照；逐文件覆盖由下述清单切片承接；独立树 attempt/generation、树缓存与树 UI 合同仍待实现。确定性并发反例见 `core/src/index_consistency_tests.rs`，实际运行结果归实现 PR。

<a id="DES-CC-TREE-003"></a>

## 5. DES-CC-TREE-003：查询接口与交互恢复

目标接口是有限的树查询与文件详情读取：tree query 接受项目/工作树、snapshot、view、parent occurrence、cursor、搜索与筛选；返回当前级节点、计数口径、next cursor 和 coverage。详情接受同一 snapshot 下的文件键/fragment，返回章节与明确关系及依据。失效 snapshot 返回可修复错误，不静默跳到当前不同版本。接口名称与生成 DTO 在实施时落位，不宣称已有命令。

首次只展开模块层；按需取子节点并虚拟化长列表，分页不裁掉搜索命中的祖先。搜索复用本地标题、路径、文档/条款 ID 索引，保留祖先与命中标记；隐藏生命周期的命中须提供显式展开入口。配对跳转在详情切换目标并保存返回栈，不修改源文件或确认关系。

个人偏好按工作树、基线模式和视图保存展开 occurrence、筛选、选中文件/章节及滚动位置。新快照以路径和明确章节定位重投影；节点删除时保留旧位置提示和最近可用祖先，不按相同标题跳到另一文件。偏好丢失可回到根，不影响源或个人决定。

UI 提供 tree/treeitem 语义、展开状态与层级；方向键导航与折叠、Enter 打开详情，IME 组合输入不触发跳转。窄窗口以树/详情两页和返回入口保持阅读上下文；加载、空结果、部分结果与错误分别呈现。数量和预算采用实现后代表性语料验证，不预写性能达标结论。

## 6. 接口合同与关键运行流程

| 接口 | producer → consumer | 身份与版本 | 顺序/重放 | 成功与错误 | 兼容与预算 |
| --- | --- | --- | --- | --- | --- |
| 刷新 | UI → Rust application → Git adapter | project/worktree、基线模式/OID、单调 request generation | 相同 generation 重复请求复用进行中尝试；新 generation 使旧尝试不得发布 | 返回 attempt 状态、当前成功 snapshot 和 coverage；取消/超时不发布候选 | 每项目一个刷新；初始 30 秒总期限，超时保留旧值；预算见 §4，实施时验证 |
| 树查询 | Rust → 同包 UI | schemaVersion、snapshot、view、parent、cursor、filter | 只读重复调用无副作用；cursor 绑定快照和过滤摘要 | 返回分页、coverage；snapshot/cursor 失效要求重新定位，版本不支持报 unsupported | 每页最多 100 个节点；搜索每页最多 100 个命中及祖先，深度最多 64；完整计数不受分页影响 |
| 详情与打开源 | Rust → UI / 有限宿主入口 | 文件键、snapshot、fragment | 原快照原文优先，打开现行工作树另标版本 | read_error、missing、ambiguous、outside_scope 分别返回；不执行原文命令 | 内容受单文件预算；源打开复用受控 scheme 和路径校验 |

用户刷新先选择基线并取得 generation，Rust 列举候选、读取解析并校验，再按 §7 提交；UI 只在发布成功后切换当前树。失败/取消返回尝试状态及最后成功快照。查询超时不自动重复刷新，用户可重试只读查询；两次选择不同项目不能交叉展示结果。所有期限和资源上限是目标合同，尚未性能验收。

## 7. 状态、事务与持久化

快照状态与源文档生命周期分别存储。Rust application 拥有 attempt 状态机，SQLite 拥有提交事实；UI 接受请求不等于快照已发布。

| 对象/状态 | 触发与转移 | 拒绝/终态及阅读结果 |
| --- | --- | --- |
| attempt queued → reading → validating | 用户明确刷新，身份/基线有效，获取该项目刷新槽位 | 无效路径/基线/schema 拒绝，不建立成功快照 |
| validating → complete | 候选全部完成且前后校验一致，仍是当前 generation | 原子持久化节点/边/coverage 并切换 current pointer 后才为 published；数据库提交失败转 failed |
| reading/validating → partial | 权限、读取或预算产生已知遗漏 | 独立终态 attempt，保存可用候选与缺口；旧成功 snapshot 保留并标 stale；无旧成功值时显示 partial |
| queued/reading/validating → failed/cancelled | 超时、用户取消、并发变更或不可恢复解析/存储错误 | 终态、不改 current pointer；记录原因，不自动重扫或重放 |
| current → stale → current | 当前刷新失败/partial 标 stale；后续 complete 成功可切换 current | stale 是最后成功数据的新鲜度，节点仍绑定原 snapshot；切换项目不取消源事实 |

只有最新 generation 可提交 current pointer；旧请求完成或 UI 已取消时拒绝发布，保留取消原因。整个快照与 pointer 在一个 SQLite 事务提交，进程中断要么保留旧 pointer，要么看到完整新快照，不能出现一半节点和另一版关系。attempt ID 与快照摘要构成幂等键，重复读取结果可复用派生对象但不修改源；异常重开将非终态 attempt 标 failed，reason 为 abandoned_after_restart，按终态保留规则计入最近 20 条尝试，并保留最后成功 pointer，用户选择刷新才新建 generation。已提交终态不重放。

源文件归仓库 owner，节点/关系/coverage/attempt 归可重建派生缓存，偏好及已有个人数据归用户存储。每项目保留 current、最近一次 partial 和最近 20 条终态尝试元信息，清理先确认未被当前读取引用；个人偏好不随缓存清理删除。被淘汰的 snapshot 查询返回 expired，原视图可按原路径修复定位。快照元信息只含文档路径、摘要、覆盖和错误码，不持久化凭据或私有会话。persisted 以事务成功为据，published 以 pointer 绑定为据，不代表文档语义正确或产品功能已实现。

## 8. 部署、安全与运行约束

沿既有 macOS App 的普通用户 Rust Core 和静态 UI 部署，无新进程、后台调度、凭据或网络来源。用户选择项目与预览范围后只能读取该范围；刷新不授予 Git 写入或执行权限，源文件含命令与链接仍是展示数据。固定提交模式拒绝 symlink blob；工作树模式拒绝文件及祖先 symlink，并在打开/读取时复核对象身份，替换成链接或范围改变即拒绝该候选。路径越界或权限失败不扩大读取范围，其他视图继续使用原数据。

## 9. 质量与容量

| 场景 | 环境与输入 | 预期响应 | 指标/阈值来源 | 验证入口 | 当前范围 |
| --- | --- | --- | --- | --- | --- |
| 定位设计 | 真实 macOS App、四域与 engineering 代表语料 | 找到两个专题及明确配对、正确返回 | AC-CC-016 的三分钟任务；记录机器、规模、实际耗时 | §11.1 UI/真实桌面任务 | 未验证 |
| 预算边界 | 3000 文件、1 MiB 单文件、16 MiB 总量的前后边界及 64 层输入 | 有界读取/分页，显示遗漏与原快照 | 既有预算及 §6 深度/页数，拒绝无限递归 | §11.1 foundation 故障回放 | 未验证 |
| 异常恢复 | 权限/超时/并发编辑/中途退出/数据库事务失败 | 不发布混合快照，不丢个人数据 | §7 原子性与拒绝条件；重开前后核对 pointer/个人数据 | §11.1 foundation 回放 | 未验证 |

不承诺未测的启动时间、帧率或内存分位数；实施记录样本环境和测量窗口后再提出性能目标。跨平台 Core fixture 只证明逻辑，macOS 读屏/窄窗口必须真实验证。

## 10. 兼容、迁移与回滚

§4 的独立 document_tree_v1 缓存先初始化，再生成新 DTO，成功后开放树入口；旧 Snapshot 和既有视图继续原格式，不双写旧模型。新缓存构建失败禁用树增量并展示原因。回退到本文核对基线的旧 App 时忽略新增表/键，源文件与既有个人记录没有不可逆变更；仅新增树阅读偏好暂不可见。旧库 schema 被未知版本修改或无法打开时保留数据库并报告错误，禁止删库恢复。实施前用实际旧库副本验证旧/新 App 读取及事务失败恢复，不把此设计当作兼容已证明。

## 11. 验证映射与实施顺序

### 11.1 验证映射表

| 上游条款 | 本设计条款 | 独立义务与适用条件 | 验证方法与当前入口 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [REQ-CC-016](command-center.prd.md#REQ-CC-016) | [DES-CC-TREE-001](#DES-CC-TREE-001) | 四域/专业树、配对、历史与歧义 | 扩展 [foundation tests](../../../tools/oasis7-command-center/core/tests/foundation.rs)，并补生命周期反例：多父、多配对、重复 ID/锚点、循环、任意原始状态值、已知状态附注及紧邻全角括号 token 归一、无配对和断链 fixture | 实现 PR 的 source HEAD 与实际测试结果 | 生命周期基础已有程序回归；树关系反例待实现 |
| [REQ-CC-016](command-center.prd.md#REQ-CC-016) | [DES-CC-TREE-002](#DES-CC-TREE-002) | 有界读取、快照一致性和隔离 | 扩展 [foundation tests](../../../tools/oasis7-command-center/core/tests/foundation.rs) 的故障回放覆盖删除/权限/超限/并发修改/旧请求晚到及部分刷新保留成功快照和旧库/缓存版本升级回退；覆盖 doc 链接到仓库内 `.env`、目录外目标及祖先目录链接的拒绝反例；检查无外部读取、模型调用和源写入 | 实现 PR 的候选及失败/恢复结果 | 章节索引并发反例已有程序回归；独立树快照、迁移与完整 corpus 性能未验证 |
| [REQ-CC-016](command-center.prd.md#REQ-CC-016) | [DES-CC-TREE-003](#DES-CC-TREE-003) | AC-CC-016：搜索祖先、往返和个人位置 | 扩展 [UI tests](../../../tools/oasis7-command-center/ui/src/App.test.tsx) 覆盖筛选/分页/恢复；真实 macOS App 完成三分钟定位、键盘/IME/读屏和窄窗口任务 | 实现 PR 的 App build、语料规模、耗时与具体卡点 | 浏览器/DOM 测试不能代签原生体验 |

生命周期解析基础已由 [lifecycle_metadata.rs](../../../tools/oasis7-command-center/core/src/lifecycle_metadata.rs) 和现有来源详情承接：原始声明与规范化类别分开，新增可选字段兼容旧阅读记录；[生命周期回归](../../../tools/oasis7-command-center/core/tests/document_lifecycle.rs)覆盖附注、全角分隔符、未知原值和错误声明。这不提供文件级树、筛选或真实桌面验收。

现有章节索引按 core PRD、工作流主责、结构入口（`AGENTS.md`、文档目录 `README.md`、项目地图）、产品正文/其他 `prd.md`、其余正文的顺序消费文件数量和字节预算；同级按路径稳定排序，入口也受相同单文件及总预算约束，不保证所有入口必然可读。入口优先回归见 `core/src/index_consistency_tests.rs`。

现有章节索引的覆盖反馈由 [index_coverage.rs](../../../tools/oasis7-command-center/core/src/index_coverage.rs) 承接：来源状态显示候选/已读/遗漏数量、实际已读字节、完整遗漏原因计数与最多 20 条路径样本（标明样本/总数）；单文件实际读取受文件和剩余总量预算限制，空文件计为已读，全量遗漏不判为空结果。这是现有工作树章节索引的局部反馈，不提供逐文件持久覆盖、独立树快照/attempt、固定提交或树 UI；本节表内尚待实现的义务仍保留。

文件清单切片由 [document_inventory.rs](../../../tools/oasis7-command-center/core/src/document_inventory.rs) 和 [DocumentInventory.tsx](../../../tools/oasis7-command-center/ui/src/DocumentInventory.tsx) 承接：现有 tracked 工作树扫描保留所有候选路径，记录 read/skipped/deleted/error、具体原因、可得的标题/类型/生命周期、摘要、已读字节和 dirty；空文件保留记录，未读文件不虚构正文元数据。文件键按项目/路径生成，读取前去重，清单按路径稳定排序；不将文件类型推定为明确配对或主责关系。

清单与章节/搜索在同一 SQLite 事务发布，使用独立 `document_inventory_v1:<project>` 派生键，不修改原持久 Snapshot shape 或数据库 schema 版本。查询 `query_document_inventory` 携带 schemaVersion=1、projectId、可选 snapshotId、标题/路径查询、读取结果筛选及游标；最多返回 100 项，计数不随分页裁剪，游标绑定清单摘要和筛选。来源变化后旧游标明确失效；失败保留原清单并显示 stale，首次缺清单、损坏或未来版本只使清单查询失败，用户刷新可重建，个人记录与未决操作不随清单删除。旧客户端忽略新键。清单仍沿现有章节索引的 partial 发布行为，不提供本设计 §7 独立树成功 pointer/partial attempt 的恢复合同。

规范视图按文件查询读取结果，可查看遗漏、空文件与同摘要章节，项目切换或刷新使旧响应无效。此清单不宣称已经提供三种树投影、显式文档 ID/条款搜索、配对边、固定提交/untracked 基线、独立树快照、树偏好或完整 AC-CC-016。程序回归见 [文件清单测试](../../../tools/oasis7-command-center/core/tests/document_inventory.rs) 与 [界面测试](../../../tools/oasis7-command-center/ui/src/DocumentInventory.test.tsx)；真实界面与宿主验证的候选、环境和限制归实现 PR。

显式配对前置切片在同一文件清单中保留 `pairings`：只识别首个非 H1 标题之前、顶层列表的精确“配对系统设计”字段内 Markdown 链接，普通引用、代码块、引用块、嵌套列表和正文不推断配对；每条保留声明行、原链接、规范化目标和文件级诊断。每文件最多 32 条，字段最多 4096 字节，跨行字段仅提取首行链接并显示解析不完整，超过预算或无法识别链接也显示解析不完整，不声称无配对。声明链接按源顺序保留，不合并多条声明。

目标仅与本次候选清单比对，不沿链接读文件或访问网络：已读取且没有 fragment 的文件目标标 resolved，带 fragment 的目标仅在同次已读内容中定位；唯一显式 HTML `id` 或 Markdown 标题锚点标 resolved，未找到标 missing_fragment，重复显式/标题锚点及二者冲突标 ambiguous，不自动选择第一项或添加编号后缀。已列举但删除的目标标 missing，其他读取遗漏标 unread；未纳入清单、外部、越界或不安全目标标 outside_scope，不把未跟踪文件猜为不存在。详情展示诊断及源行，内容摘要沿所属文件绑定。已解析目标附可选 targetSection/targetRevision，UI 仅在目标路径、章节 ID 和内容摘要均匹配当前章节时开放跳转；无 fragment 时进入文件首个索引章节，空文件无章节可跳。显式锚点紧邻标题时定位该标题，正文/表格内锚点定位所属章节。

锚点解析使用 CommonMark 事件，排除代码和 HTML 注释；标题按可见文本小写，仅保留 Unicode 字母/数字和连字符，移除格式符及包括中文标点在内的其他字符，并以连字符连接空白，支持 Setext 标题。显式 HTML `id` 按原始大小写精确匹配（Contract 与 contract 分开），生成的标题锚点使用小写值；HTML `id` 支持单/双引号及其他属性前置，raw-text HTML 块不提取内部伪锚点。旧清单缺少可选 `pairings` 时显示未解析并要求刷新，不将缺字段当作无配对；旧 App 忽略新增派生字段，原 Snapshot 不变。旧配对清单若仍有 fragment_unchecked 或没有章节定位字段，保留诊断并提示刷新，不按标题猜位置。跳转复用来源详情，关闭后回到原配对按钮并保留当前筛选、分页和文件选择；项目/源代次变化使原清单失效。此切片尚不解析正文上游承接表、导航、successor 边或显式文档 ID，不提供三种树投影。程序回归沿文件清单和界面测试入口，实际桌面验证归实现 PR。

设计文件头的显式上游切片复用上述有界链接投影与同版本章节定位：仅对路径类型为 design 的已读文件，识别首个顶层非 H1 标题之前、顶层列表的精确“上游”字段；保留源行和链接顺序。普通正文、引用、代码、嵌套示例与其他字段不推断上游。目标 targetKind 来自同次候选清单的路径类型，prd 显示需求文档，design 显示设计引用，不能把上游设计引用提升为需求或配对，也不证明承接完整或实现完成。32 目标/4096 字节及跨行不完整提示分别作用于配对和上游字段组。

派生文件记录新增可选 upstream，复用同一次扫描、摘要、事务及查询；没有新源读取或 IPC。旧清单缺字段显示未解析并提示刷新，未读取文件不产生虚构上游。已解析上游目标仅按同路径/章节 ID/内容摘要打开，关闭后返回原上游按钮，保留筛选、分页和文件选择。配对与上游在详情分组展示，解析不完整、缺失、歧义、未读取和范围外沿同一诊断合同；当前仍不解析“上游需求”等别名、文档身份二级章节或正文承接表，不提供完整双向关系图与树投影。回归入口为 [上游声明测试](../../../tools/oasis7-command-center/core/tests/document_upstream.rs)及既有文件清单界面测试。

先实现文件清单与覆盖反馈，再实现关系和快照投影，随后接入树/详情及恢复，最后验证真实 App。只到前两步时可以交付索引切片，但不宣称 AC-CC-016 已通过。纯设计不改变游戏 Agent Skill/API/runtime 行为，开发自主试玩不适用；功能实现仍需上述真实桌面验证。

## 12. 风险与未决项

历史 Markdown 导航不完全统一，初版只解析声明过的有限关系，未识别模式保留诊断并可从目录查漏。不得以标题相似度补关系。完整文档集可能超出既有预算，实施时先测实际规模和交互成本，再调整分页/预算并保持覆盖透明。真实 App 的读屏组件适配、未跟踪预览选择和基线选择入口仍需实施验证；本设计不新增付费、部署或生产访问要求。
