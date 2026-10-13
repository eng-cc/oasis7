# 指挥中心文档树系统设计

- 文档 ID：`DESIGN-ENGINEERING-CC-DOCUMENT-TREE`
- 状态：`draft`；目标方案，本次不交付程序或桌面验收
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
| 文档索引 | [index.rs](../../../tools/oasis7-command-center/core/src/index.rs)按 tracked 路径读取并生成章节，单文件/总量/数量有界 | 复用章节及生命周期解析；补文件级清单、明确遗漏原因、固定基线读取和树投影 |
| 来源身份 | 现有 Project 含 worktree、HEAD，章节含内容摘要 | 文件节点须绑定同一快照；工作树并发编辑检测不能只检查 HEAD |
| 导航与关系 | 仓库入口和配对链接存在；没有据此证明专用树 UI | 增加明确导航关系、配对关系和可见未归属集合 |
| 桌面验证 | 本次仅源码与文档核对 | 未执行新 UI、读屏、性能或模型验收；不得以设计表认定已通过 |

## 2. 上游承接

### 2.1 需求承接与分配表

| 上游条款 | 独立义务与适用条件 | 本设计条款 | 外部 owner / dependency | 明确排除或未覆盖 |
| --- | --- | --- | --- | --- |
| [REQ-CC-016](command-center.prd.md#REQ-CC-016) | 三种层级、确定关系与来源 | [DES-CC-TREE-001](#DES-CC-TREE-001) | 文档 owner；产品 README、专业入口及显式配对 | 不推断语义主责或设计完成率 |
| [REQ-CC-016](command-center.prd.md#REQ-CC-016) | 版本、覆盖、局部失败和只读边界 | [DES-CC-TREE-002](#DES-CC-TREE-002) | Git / Rust adapter、既有索引 | 不扫描私有库，不访问任意远端 |
| [AC-CC-016](command-center.prd.md#AC-CC-016) | 定位、往返、恢复与可访问操作 | [DES-CC-TREE-003](#DES-CC-TREE-003) | Solid UI / Tauri 宿主 / QA | 设计交付不证明真实桌面通过 |

<a id="DES-CC-TREE-001"></a>

## 3. DES-CC-TREE-001：文件、关系与树投影

Rust 构建文件级 `DocumentNode`，至少含 worktree/project 身份、仓库相对路径、标题、kind、声明 ID、原始生命周期声明及独立规范化类别/解析状态、内容摘要、snapshot ID、dirty 状态。文件键使用项目/工作树身份与规范化路径；文档 ID 只作为检索字段，重复 ID 产生诊断，不合并文件。章节沿既有章节定位，关系的源/目标 fragment 保留完整路径及源位置。

`DocumentEdge` 保存 from/to、kind（navigation / paired_design / upstream_requirement / reference / successor）、声明位置、源摘要及解析结果（resolved / missing / ambiguous / outside_scope / unread）。只有入口的模块/专题导航、明确配对字段及设计上游/承接表产生相应 typed edge；其他 Markdown 链接为 reference。解析支持相对路径、显式锚点和 Markdown 标题锚点，沿仓库治理解析规则保持一致；重复锚点不静默选第一项。链接不存在与目标超出本次读取范围分别记录。远端链接仅作为引用显示，刷新不跟随网络抓取。

产品投影固定从产品 README 的四域入口读取模块与专题声明；异常额外模块作为诊断，不自动新增第五域。专业系统投影从专业域入口与模块 design 导航展开，同域工程 PRD 可作为需求关联入口；专题归属来自明确导航，无法定位的文件进入“未归属/待核对”。目录投影按路径组织所有候选 PRD/design，README 仅作为结构入口；类型以根 prd/design 与专题后缀为默认，显式声明冲突保留诊断。

结构节点与文件节点分开，树 occurrence 只引用文件键；一个文件被多个父项引用时显示多个引用入口，详情与计数按文件去重。树展开时维护当前祖先集合，遇到循环展示“循环引用”链接并停止该路径展开；交叉域和多对多配对在详情列为关系，不强行放入唯一父子树。排序沿源导航顺序；无序集合按路径稳定排序。

计数标注快照范围、生命周期筛选和去重口径。配对计数只统计已解析 paired_design，不将普通引用或 upstream_requirement 自动提升为配对；无配对、未读取、明确缺失分别计数。生命周期原样保留源声明（包括 `target-contract` 等专业值），另存规范化的已知类别 proposed/draft/active/superseded/retired；其他已声明值归 other，可按原值筛选，只有缺声明才显示 unknown。默认隐藏明确历史类别的叶项，但被当前条款引用时显示历史引用占位。生命周期不决定运行或发布状态。

<a id="DES-CC-TREE-002"></a>

## 4. DES-CC-TREE-002：读取、快照与失败恢复

数据流为：用户选基线 → Git 文档清单 → 有界源读取 → Markdown/生命周期解析 → 关系校验 → 三种树投影 → SQLite 派生缓存 → 有限 DTO → UI。Rust application 组织快照，index adapter 读取源，domain 保持关系语义，UI 不自行扫描文件或推断权威。沿既有 core、Tauri bridge 和生成 DTO 扩展，不新增常驻服务或依赖。

已提交模式以固定 commit OID 的树列举和 blob 读取，切换分支不改变快照。工作树预览以选定根目录读取 tracked 文件和用户显式纳入的 doc 范围内 untracked Markdown；排除 ignored 文件、私有库、会话及所有符号链接（包括路径祖先中的链接）；固定提交中的 symlink blob 也只标记跳过，不解析为正文。每个候选记录 read/skipped/deleted/error 与原因，先列举再读取，不能沿用现有静默跳过来声称完整覆盖。

沿现有 3000 文件、1 MiB 单文件、16 MiB 总量限制作为初始预算；入口优先读取，超限生成可见覆盖说明，未读节点仍可定位路径。解析与树深度有界，循环不递归读取新文件。工作树扫描前后复核候选清单、HEAD 及已读文件摘要；发现改变时丢弃候选，保留旧快照并提示重试，不无限自动重扫。读权限失败只标记相关节点，部分成功快照明确为 partial。

快照含基线模式/OID、工作树身份、清单摘要、生成时间、每文件摘要和覆盖结果。完整成功候选以事务替换当前快照；错误或部分候选单独保存为本次尝试，最后成功快照仍作为默认阅读快照并标明 stale、原版本及本次失败/遗漏范围，用户可显式查看部分尝试。首次无成功快照时可以展示 partial，并明确没有完整基线。节点与边不可混用两个快照；同项目较早异步请求不能覆盖较新选择。缓存过期时保留旧来源版本及原因，重建只替换派生数据，不删除个人关联、阅读偏好或未决操作。

本功能不执行 fetch、checkout、模型调用或文件写入。源 Markdown 视为不可信展示数据，复用安全渲染和受控源打开；拒绝目录逃逸、任意 scheme 与原文内脚本/命令执行。不存在的本地目标只有在完整列举范围内核实后标 missing，外部/未读目标标 outside_scope/unread。

<a id="DES-CC-TREE-003"></a>

## 5. DES-CC-TREE-003：查询接口与交互恢复

目标接口是有限的树查询与文件详情读取：tree query 接受项目/工作树、snapshot、view、parent occurrence、cursor、搜索与筛选；返回当前级节点、计数口径、next cursor 和 coverage。详情接受同一 snapshot 下的文件键/fragment，返回章节与明确关系及依据。失效 snapshot 返回可修复错误，不静默跳到当前不同版本。接口名称与生成 DTO 在实施时落位，不宣称已有命令。

首次只展开模块层；按需取子节点并虚拟化长列表，分页不裁掉搜索命中的祖先。搜索复用本地标题、路径、文档/条款 ID 索引，保留祖先与命中标记；隐藏生命周期的命中须提供显式展开入口。配对跳转在详情切换目标并保存返回栈，不修改源文件或确认关系。

个人偏好按工作树、基线模式和视图保存展开 occurrence、筛选、选中文件/章节及滚动位置。新快照以路径和明确章节定位重投影；节点删除时保留旧位置提示和最近可用祖先，不按相同标题跳到另一文件。偏好丢失可回到根，不影响源或个人决定。

UI 提供 tree/treeitem 语义、展开状态与层级；方向键导航与折叠、Enter 打开详情，IME 组合输入不触发跳转。窄窗口以树/详情两页和返回入口保持阅读上下文；加载、空结果、部分结果与错误分别呈现。数量和预算采用实现后代表性语料验证，不预写性能达标结论。

## 11. 验证映射与实施顺序

### 11.1 验证映射表

| 上游条款 | 本设计条款 | 独立义务与适用条件 | 验证方法与当前入口 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [REQ-CC-016](command-center.prd.md#REQ-CC-016) | [DES-CC-TREE-001](#DES-CC-TREE-001) | 四域/专业树、配对、历史与歧义 | 扩展 [foundation tests](../../../tools/oasis7-command-center/core/tests/foundation.rs)，并补生命周期反例：多父、多配对、重复 ID/锚点、循环、任意原始状态值、无配对和断链 fixture | 实现 PR 的 source HEAD 与实际测试结果 | 当前仅设计，无新增程序回归 |
| [REQ-CC-016](command-center.prd.md#REQ-CC-016) | [DES-CC-TREE-002](#DES-CC-TREE-002) | 有界读取、快照一致性和隔离 | 扩展 [foundation tests](../../../tools/oasis7-command-center/core/tests/foundation.rs) 的故障回放覆盖删除/权限/超限/并发修改/旧请求晚到及部分刷新保留成功快照；覆盖 doc 链接到仓库内 `.env`、目录外目标及祖先目录链接的拒绝反例；检查无外部读取、模型调用和源写入 | 实现 PR 的候选及失败/恢复结果 | 未验证工作树并发与完整 corpus 性能 |
| [AC-CC-016](command-center.prd.md#AC-CC-016) | [DES-CC-TREE-003](#DES-CC-TREE-003) | 搜索祖先、往返和个人位置 | 扩展 [UI tests](../../../tools/oasis7-command-center/ui/src/App.test.tsx) 覆盖筛选/分页/恢复；真实 macOS App 完成三分钟定位、键盘/IME/读屏和窄窗口任务 | 实现 PR 的 App build、语料规模、耗时与具体卡点 | 浏览器/DOM 测试不能代签原生体验 |

先实现文件清单与覆盖反馈，再实现关系和快照投影，随后接入树/详情及恢复，最后验证真实 App。只到前两步时可以交付索引切片，但不宣称 AC-CC-016 已通过。纯设计不改变游戏 Agent Skill/API/runtime 行为，开发自主试玩不适用；功能实现仍需上述真实桌面验证。

## 12. 风险与未决项

历史 Markdown 导航不完全统一，初版只解析声明过的有限关系，未识别模式保留诊断并可从目录查漏。不得以标题相似度补关系。完整文档集可能超出既有预算，实施时先测实际规模和交互成本，再调整分页/预算并保持覆盖透明。真实 App 的读屏组件适配、未跟踪预览选择和基线选择入口仍需实施验证；本设计不新增付费、部署或生产访问要求。
