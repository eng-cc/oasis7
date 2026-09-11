# 项目管理记录规范

- 状态：active
- 适用范围：GitHub-backed 变更记录、叶子任务、专业 slice、依赖、证据与组合验收
- 当前入口：[engineering/doc-governance README](README.md)
- 系统设计配套：[系统设计写作规范](system-design-writing-standard.design.md)
- 任务生命周期与门禁权威：[workflow source of truth](../workflow/source-of-truth.md)

本文是项目管理内容规范。它规定记录应表达什么、如何固定输入、如何保留证据和如何判断组合交付；不新增任务 schema、状态枚举、权限模型、loop activation、Project 字段、服务或调度器。生命周期、绑定、准入、权限、评审、CI、收尾和 epoch 规则始终以 workflow source of truth 及现行 helper 的实际 readback 为准。

## 1. 权威、载体与边界

GitHub Issue 是任务身份和正式证据 envelope；GitHub Project item 与字段是 active queue 和规定管理视图。`Task UID` 是稳定内部身份，Issue number 与 Project item ID 是外部对象句柄。任务标题、Issue 正文和 evidence comments 可以组织信息，但不能替代现行 binding 或 workflow gate。

生成的本地材料只用于映射、缓存和历史审计：

- `.pm/github-project-sync/tasks.json` 是可重新生成的 task-to-Issue/Project mapping cache，不是任务队列或可手写状态源；
- `.pm/github-project-sync/task-archive.jsonl` 是 repo-local 的历史 metadata/evidence archive，不是当前计划或准入依据；
- GitHub task issue evidence comments 承载正式执行、评审、验证和收尾证据；无法写入时使用现有 fallback/replay 机制，不建立新的本地台账。

同一事实只保留一个可写权威。产品文档保留产品价值、范围、玩家承诺与体验验收；专业系统设计保留技术合同、边界与验证设计；源码、测试与运行配置保留实现事实；GitHub task truth 保留本次交付的身份、范围、依赖、过程和实际证据；workflow source of truth 保留生命周期、权限和门禁。发生冲突时，在当前 Issue evidence 中记录冲突、影响、裁决 owner 与后续动作，不在本规范静默覆盖其他 authority。

## 2. 对象、角色与交付关系

项目管理记录区分长期主题、一次变更、叶子任务、PR 和组合验收：

| 对象 | 作用 | 正式维护位置 |
| --- | --- | --- |
| 产品或专业主题 | 维护稳定要求、规则和设计 | 对应文档 authority |
| 一次变更 | 约束有限范围、交付义务和验收候选 | 已授权的协调 Issue；需要时以 `change_id` 关联 |
| 叶子任务 | 一个 owner、固定输入、有限 scope 和独立完成边界 | 一个 Task UID 对应的 GitHub Issue/Project item |
| PR 或非 PR 交付 | 交付叶子任务的仓库或验证链 | GitHub PR 或现行非 PR evidence path |
| 组合验收 | 对同一候选版本和范围判断完整能力 | GitHub task evidence 与适用 QA/专业产物 |

每个叶子任务 MUST 有一个 outcome owner role、一个稳定 `Task UID` 和现行 workflow 规定的 canonical worktree/PR chain。协作 slice 可以提出发现或交付局部义务，但不改变唯一 owner。TPM 负责 task truth、dispatch、集成顺序和主链；专业角色负责其领域判断；QA 负责可验证性与发布判断的专业收口；repository health 负责组织、引用、证据边界和债务判断；涉及公开说明时由相应 LiveOps/community 角色参与。角色名或模板填写不自动产生独立评审、准入或发布权限。

## 3. 固定输入与证据身份

任务必须把“最新可读文档”“本任务批准消费的合同”和“实际测试候选”分开：

| 层次 | 必须表达 | 不得替代 |
| --- | --- | --- |
| 当前阅读版本 | 为发现当前方向而阅读的路径、版本或提交 | 在途任务的自动替换输入 |
| 任务消费版本 | 获批准且固定的产品/系统合同、发布记录、不可变内容与 consumed clauses | 浮动 `main`、分支名或 `latest` |
| 验收候选版本 | 实际测试的 source/integration identity、配置、环境和入口 | 任意历史 green、文档 active 或 Issue closed |

在有效机制支持时，复用现行 `input_contracts` 的 `contract_id`、`revision`、`contract_digest`、`publication_ref` 和 `consumed_clauses`；同时记录任务实际需要的 `acceptance_refs`。在未激活或不支持的环境，把这些语义放在可读 Issue/evidence 中，但不得用手工 JSON、front matter 或“已评审”文字伪造 adapter 绑定或消费资格。

跨文件消费的条款、REQ/AC 或设计义务必须记录为继承自冻结合同/发布记录的不可变 contract/publication repository（当前 canonical `eng-cc/oasis7`）加能在固定内容中唯一解析的 repository-relative `path#fragment`，并在每个消费者匹配；局部标识只在同一文件且无歧义时适用。合同 digest 固定内容身份，但不替代条款定位；移动或改名必须保留映射或正式处置，不能隐式取当前仓库。

固定输入至少应能定位到：来源路径或对象、不可变提交/发布记录、内容摘要或 digest、批准来源、消费条款、适用 owner 和失效条件。源文档仍是 proposal 时必须明确标为 proposal；本任务的 source snapshot、当前 HEAD 与 live 状态不得被历史附件中的旧 PR/分支叙述替换。

输入改变与目标分支推进是两类事件。上游语义、权限、合同资格或验收发生实质变化时，按影响范围保留旧基线、暂停并迁移、或缩小 scope，并按现行 workflow 重新绑定/验证；main 因无关变更推进时，更新 integration/CI 证据，不自动重写产品或系统要求。不能在执行中静默换合同版本。

## 4. 叶子任务最小内容

任务标题应表达可验证交付结果。正文和正式 evidence 至少覆盖以下信息；字段名仅作为与现行 binding 的语义对应，不是新 schema：

1. **结果与来源**：交付什么、依据哪个用户请求、产品 REQ/AC、专业规则或系统条款；工程缺陷可以直接引用现有合同并说明产品语义不变。
2. **身份与边界**：现行 Task UID/owner readback、canonical worktree/PR chain、In scope、允许写入范围和 Out of scope。
3. **固定输入**：产品/系统/工程输入的版本、digest、消费条款和当前消费资格；不只引用浮动路径。
4. **交付义务**：本叶子必须实际交付的文档、代码、测试、验证或说明对象，以及其完成条件。
5. **验收引用**：每项 `acceptance_refs` 对应的具体义务、前置条件、验证入口和明确未覆盖范围。
6. **依赖与风险**：上游输出、消费版本、硬依赖或咨询关系、阻断影响、解除 owner、解除证据、触发器与剩余风险。
7. **执行与协作**：有界步骤、专业 slice、写入许可、集成顺序与正式 evidence 引用。
8. **实际交付与收尾**：实际 source/PR/integration identity、命令与退出码、产物定位、未交付义务、PR/非 PR 终态以及 workflow 收尾证据。

计划按可验证结果拆分，不按“文档/代码/测试”比例估算。里程碑说明能力变化、范围、退出条件、交付义务、依赖、风险、日期类型和假设；关闭若干 Issue、合并若干 PR 或填写进度百分比都不能自动证明能力完成。一个叶子任务终态也不自动完成同一 `change_id` 的设计回写、联合验证或公开说明义务。

## 5. 依赖、slice 与交接条件

依赖分为硬依赖和咨询关系。硬依赖必须写出上游对象、具体消费内容/版本、是否阻断、解除条件、责任人和解除证据；另一个 Issue closed 不是能力存在的充分证明。独立任务是否并行由实际 write scope、合同、共享资源和集成风险决定，不设置由模板或 loop 名称推导的全局串行限制。

专业 slice 的任务、输入和写入许可必须落在当前 task scope 与有效 policy 的交集内。默认交接顺序是：产品输入明确后交给系统承接；系统合同固定后交给代码叶子；局部实现证据达到适用依赖条件后组织组合验收。代码、验证或评论完成不会自动创建、启动或授权下一个任务；等待外部条件只改变可继续性，不能触发后台 scheduler、webhook 或常驻 loop。

若上游语义不清、输入 digest 不匹配、scope 不足、依赖循环或证据身份漂移，slice 应返回缺口和停止条件，不能越权修改另一 authority。跨域反馈回到当前 GitHub Issue evidence，由 TPM 按现行 workflow 判断是否需要新任务或重新绑定。

## 6. 项目记录中的义务与条件

每项交付义务都要说明“要交付什么”和“在什么条件下可判定”。条件可包括固定产品/系统输入、权限有效、代码集成身份、配置、入口、环境、证据窗口、依赖 readback 和专业评审。把条件写在义务旁边，避免以一个笼统的“完成”吞掉失败路径、未覆盖环境或组合要求。

推荐将义务分为：

- **输入义务**：来源、版本、digest、批准/消费条款和变更影响；
- **边界义务**：owner、write scope、out-of-scope、允许入口和禁止推导；
- **实现义务**：设计、代码、测试、配置或操作材料实际交付的内容；
- **验证义务**：命令/测试/观察、候选身份、环境、阈值来源、退出码和产物；
- **组合义务**：跨组件、跨入口、失败恢复、权限一致性和每项产品 AC 的完整判断；
- **交接义务**：下游准确消费的文档/合同/证据，以及仍需产品、QA 或其他专业 owner 判断的事项。

接受、应用、持久化、发布、任务 done、PR merged 和能力可发布是不同事实。项目记录只能报告有证据支持的事实；未知、未运行、不适用和未证明范围必须保留，不以模板填满、CI 绿色或文本 `active` 互相推导。

对包含多个交付义务的有限变更，协调记录必须先冻结本次批准的必要产品/专业接受条款、设计/实现/验证义务、明确排除或接受风险、映射槽位与候选选择规则，不预先伪造实际执行引用。进入组合验收或整体完成前，必须补齐真实 Task/合同/证据身份和 aggregate candidate；缺失 execution refs 保持 pending 并阻断整体完成，但不阻止已批准叶子按顺序执行，也不允许事后删减必要集合。整体完成须逐项证明该集合；缺失、未运行或未处置的义务不能因叶子 done、PR merged 或局部 green 消失。不同叶子可以有不同 source HEAD；组合证据必须另行固定一个可复现的 integration/tested tree、配置、入口、环境与证据窗口。

## 7. 证据身份、执行与边界

正式 evidence MUST 让第三方判断声明的适用范围。最低信息为：

| 证据项 | 内容 |
| --- | --- |
| 身份 | Task UID/变更关联、记录类型、执行或审读 role、时间/证据窗口 |
| 输入与对象 | 产品/系统合同版本和消费条款、source HEAD、integration base/tested tree、配置与入口 |
| 环境 | 本地、fixture/fake-GitHub、hosted CI、真实集成、Viewer/API/Agent、平台与隔离边界 |
| 动作 | 实际命令、suite、操作、观察或评审范围 |
| 结果 | 退出码、通过/失败/未运行/不适用、关键摘要、artifact/log/URL 和 digest |
| 定位 | acceptance/REQ/DES 引用、代码或文档路径、anchor、产物定位 |
| 边界 | 未覆盖链路、mock/替身、未验证阈值、剩余风险和需要的专业判断 |

验证证据记录实际执行而不是预期行为；评审证据记录固定审读基线、发现、严重性、处置、复核和剩余风险。截图或终端摘录可以辅助，不能替代身份绑定和完整产物。结构检查只能证明结构范围；它不能单独证明产品正确、系统可实现、真实玩家体验、发布就绪或 runtime/provider 能力。

验证命令和证据对象应与候选版本同一可比较范围。不能把不同提交、环境、配置或时间窗口的局部 green 拼成一个未实际运行过的候选。必要时按现有 workflow 保存 receipt、CI artifact、review packet 或 task evidence；不要创建新的“自签通过”格式。

## 8. 三-loop 消费与交付边界

本节只是文档和记录的兼容边界。三-loop 的 policy、schema、adapter、资格与手动启用状态由 workflow source of truth 管理；候选设计、代码合并和明确 enablement 是不同事件。未激活时按现行 legacy workflow 工作，不能仅填写 `loop` 就绕过现有门禁。

| 交付类别 | 维护的权威内容 | 不得顺手修改 |
| --- | --- | --- |
| product | 产品价值、玩家承诺、玩法规则与体验验收 | 系统技术合同、实现代码 |
| system | 专业技术要求、架构、接口、状态、恢复与验证设计 | 产品承诺、实现代码 |
| code | 源码、测试、协议/配置、脚本与可执行适配面 | 正式产品和系统文档 |
| combination | 选定候选上的跨入口/跨模块证据与产品/QA判断 | 用子任务关闭或局部 mock 代签完整能力 |

沿用有效 binding 中的 `task_uid`、`change_id`、`loop`、`owner_role`、`write_scope`、`out_of_scope`、`input_contracts`、`acceptance_refs`、`dependencies`、`target_delivery`、`policy_commit` 和 `policy_digest`。`change_id` 只关联有限交付义务，不取代 Task UID；`manual_request_ref`、`request_key`、`bootstrap_epoch` 和准入资格只能由有效 harness/workflow 提供，模板不能创造它们。

跨 loop 反馈不是自动任务创建授权。多个 code 叶子可以在共享合同固定后并行；组合验收由获授权的协调/集成任务组织相应 QA 与专业 slice，但它不是第四 loop。测试设计、代码测试、产品结论和发行判断仍归各自 owner。一个 code leaf done 不等于整个变更完成。

## 9. 组合验收

组合验收必须绑定同一候选：产品要求集合、产品/系统合同及消费条款、代码集成身份、配置、正式入口、环境和证据窗口。它逐项判断每个必要 AC、系统义务和跨入口失败路径，并保留局部证据与未证明范围。

推荐记录：

| 验收引用 | 独立义务与条件 | 实际测试/观察 | 候选版本与环境 | 结果 | 证据定位 | 未证明范围 |
| --- | --- | --- | --- | --- | --- | --- |
| `path#AC/DES` | 本次必须证明的具体部分 | 准确命令、suite 或真实入口观察 | source/integration/config/environment | 通过/失败/未运行/不适用 | Issue/artifact/receipt | 不能外推的链路 |

“不适用”必须说明范围理由并由适用 owner 审查；happy path、单元测试、fixture 或 mock 只能关闭其实际覆盖的义务。子任务都关闭、一个入口通过或一条绿色 CI 不能代签跨模块调用、共享状态、权限、恢复、产品价值或发行权限。组合结论应明确需要 product、system、runtime、Viewer、Agent、QA 或 LiveOps/community 哪些专业判断，不能由 TPM 或本文替代。

## 10. 变更影响与维护

源、合同、权限、验收或候选身份变化时，先做有界影响评审；普通排版修订不应无理由使所有下游失效。影响评审至少回答：

| 问题 | 必须记录 |
| --- | --- |
| 发生了什么 | 文件/anchor、旧/新版本、变化性质（编辑、行为、权限、合同、兼容或公开承诺） |
| 谁受影响 | 直接设计/测试/任务消费者与间接跨域/组合消费者 |
| 旧输入是否仍可用 | 新任务、在途任务、验收与发行各自的资格和限制 |
| 如何处置 | 保留旧基线、暂停、缩小 scope、按现行规则重新绑定/迁移或补充兼容 |
| 需要重验什么 | 受影响义务、入口、环境、候选 identity、证据与评审 |
| 什么不受影响 | 明确范围与理由，不做无差别全仓重跑 |
| 谁批准并实施 | 有权 owner、真实记录、实施结果；不得由候选 policy 自授权 |

稳定架构风险留在设计，动态风险和阻断留在当前 Issue；二者通过明确引用关联。风险至少包含触发条件、影响、owner、缓解/接受依据、复核触发和剩余风险。superseded 文档指向后继并退出默认首读入口；历史 evidence 和 source snapshot 保持可检索，不通过改写历史制造当前能力。

需要处置的跨 loop 反馈必须形成可回读的关系：用现有 Issue comment、artifact、path#fragment 或等价 evidence identity 稳定定位原始发现（可在本记录内使用局部标签，不要求新的全局 ID 或第二台账），并记录接收 owner、处置 authority/approval owner、处置结论及依据、已授权的任务/合同修订或明确不修改决定、对消费者的阻断影响，以及解除阻断或关闭的证据。反馈记录本身不创建或启动下一个任务；未获得授权的阻断项继续留在当前协调记录的未完成义务中。此处是记录要求，不是新增状态或自动调度器。

需要反向查询受影响消费者时，从现有合同绑定和 Task evidence 按需生成只读追踪视图，标明查询范围和未读取部分；不得维护另一份手工消费者清单或把该视图当作第二任务真值。

新增 PM/系统内容目前按人工内容规则和现有结构/link checks 维护；没有新增 metadata parser、semantic checker、Project field 或 required gate。后续只有在代表性文档和明确 owner/授权到位后，才可由 source-of-truth-first 变更实现机械校验，并补充正反例与迁移边界。

## 11. 可复用记录模板

以下模板是内容骨架，不是第二份 schema。使用时填入现行 helper/readback 提供的身份与字段；没有实际值时写明未验证或不适用，不自造 UID、状态、资格或批准。

### 11.1 有限变更协调记录

```markdown
## 本次变更目标
<一个可验证且有限的结果>

## 身份与范围
- Change ID：<有效关联标识，若适用>
- 产品/专业范围：<REQ/AC/规则/设计引用>
- 工程范围：<实际模块与允许写入范围>
- 不在本次范围：<明确排除>
- 用户授权与约束：<真实请求、hold 与允许动作>

## 固定输入与关键依赖
<来源、revision/digest、consumed clauses、解除条件和 evidence>

## 必要交付义务
| 交付义务 | 真实 Task/Issue | 交付对象 | 满足条件 |
| --- | --- | --- | --- |
| <义务> | <身份> | <文档/代码/验证> | <可判定条件> |

## 组合验收
- 组合候选、入口、环境、配置：<实际 integration/tested tree 与证据窗口；各叶子保留自己的 source HEAD，不要求彼此相同>
- 必需 AC/系统义务：<精确集合>
- 组织者与专业判断：<真实 owner>
- 证据位置：<Issue/artifact/receipt>
- 完成条件与未证明范围：<逐项结果>
```

### 11.2 叶子任务内容卡

```markdown
## 交付目标与来源
<可验证结果；用户请求、REQ/AC、规则或 DES 引用>

## 固定输入
<产品/系统/工程来源、revision/digest、消费条款与 readback>

## 写入边界
- owner/scope：<现行绑定提供的值>
- In scope：<允许路径与义务>
- Out of scope：<排除项与不可修改 authority>

## 依赖与条件
<上游对象、硬/咨询关系、解除证据、阻断 owner>

## 验收范围
| 验收引用 | 本叶子证明的义务 | 明确未覆盖 | 验证入口 |
| --- | --- | --- | --- |
| <path#AC/DES> | <具体部分> | <组合/入口/环境限制> | <命令/测试/观察> |

## 执行、交付与收尾
<原子步骤、slice、集成顺序、真实产物、source/PR identity、workflow evidence>

## 异常处理
<输入失效、语义不清或 scope 不足时的停止条件与回流 owner>
```

### 11.3 专业 slice 交接卡

```text
任务：真实 Task UID、Issue、worktree、HEAD/epoch（如由有效 workflow 提供）
专业角色：实际 role
固定输入：产品/系统合同、revision/digest 与具体消费条款
目标：本 slice 必须返回的判断或有界实现
写入许可：task scope 与有效 policy 的交集
允许读取：理解合同和边界所需的上下游资料
返回内容：结果、路径、条款承接、命令/证据、缺口和未证明范围
集成前置：必须等谁的哪些结果
跨域发现：回到当前 Issue evidence；不得越权修改另一 authority
```

### 11.4 验证执行证据卡

```markdown
## 验证对象
- Task UID/变更关联：<真实身份>
- 产品/系统合同：<版本、digest、consumed clauses>
- Source HEAD 与 integration base/tested tree：<实际值>
- 入口、环境、配置与证据窗口：<实际值>

## 执行事实
- 方法/命令：<实际执行；未运行写未运行>
- 退出码与结果：<实际值>
- 产物：<路径/URL、摘要、digest>
- 替身或模拟：<mock 与真实入口的边界>

## 逐项判定
| 验收引用 | 独立义务/前置 | 测试/观察 | 结果 | 证据定位 | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| <AC/DES> | <条件> | <准确测试> | <通过/失败/未运行/不适用> | <定位> | <限制> |

## 结论边界
<支持哪个叶子义务；哪些组合、产品、运行或发行判断仍需专业 owner>
```

### 11.5 阶段目标与交付验收记录

```markdown
# <阶段目标或能力增量>

目标：<产生什么可验证变化>
范围与非目标：<边界>
负责角色：<协调责任与专业交付责任>
目标日期：<日期/未设定；假设及是否对外承诺>

## 必要交付义务
| 义务 | 验收条件 | 真实 Task/evidence 引用 |
| --- | --- | --- |
| <义务> | <可判定条件> | <定位> |

## 硬依赖与风险
<解除条件、owner、复核触发和范围变更记录>

## 验收结论
核对时间：<带时区时间>
已确认结果：<附证据>
未完成义务：<不可因叶子结束而省略>
剩余风险与接受依据：<正式记录>
整体结论：<完成/部分/未验证；不替换 task 状态枚举>
```

### 11.6 跨 loop 合同差异反馈记录

```markdown
## 合同差异/跨 loop 反馈
当前 Task UID 与交付类别：<正式绑定>
消费合同：<准确 publication/revision/digest/条款>
发现：<可复核事实>
差异分类：<实现违反既有合同/需要修改合同/文档陈旧/未决>
受影响范围：<接口、产品结果、测试、兼容、交付义务>
建议：<保持现合同修复实现/由有权任务修订合同>
当前可继续部分：<有界范围>
当前阻断部分：<解除条件>
来源 finding 定位：<现有 Issue comment/artifact/path#fragment 或 evidence identity；可加本记录局部标签，不新增全局 ID>
接收 owner：<负责回读和处置的真实角色>
处置 authority/approval owner：<批准修订或明确不修改的真实 authority>
处置结论与依据：<保持合同修复实现／授权修订合同／明确不修改>
关联任务／合同修订：<已授权记录；没有则写无>
解除或关闭证据：<准确证据定位；未解除则写未完成>
权限说明：此反馈不创建或启动下一个任务。
```

### 11.7 组合验收记录

```markdown
## 本次候选
<产品要求、产品/系统合同、代码集成身份、入口、配置、环境、窗口；叶子 source HEAD 可以不同，但组合必须固定可复现的 integration/tested tree>

## 必需交付对象
<真实任务与证据；不以 closed 代替交付>

## 必要集合与处置
<本次冻结的完整 AC/义务集合、映射槽位与候选选择规则；组合前补齐真实映射、结果或有权处置；遗漏项保持未完成>

## AC/义务汇总
| 验收引用 | 必需义务与入口 | 同一候选证据 | 结论 | 缺口/阻塞 |
| --- | --- | --- | --- | --- |
| <AC/DES> | <范围> | <证据> | <实际判断> | <仍缺什么> |

## 集成与产品边界
<跨模块调用、权限、状态、失败恢复、mock 限制；产品/QA/发行判断的 owner>

## 下一步
<当前授权内的有限动作；新的 task 或继续须由相应 authority 授权>
```

### 11.8 变更影响评审卡

```text
发生变化的源：文件、anchor、旧/新版本、digest
变化性质：纯编辑／行为／权限或损失／技术合同／兼容／公开承诺
直接消费者：设计、测试、任务及其消费条款
间接消费者：跨域调用、组合验收与发行说明
旧合同资格：新任务、在途任务、验收、发行分别如何处理
处置：保留旧基线／暂停／缩小 scope／按 workflow 重新绑定或迁移
需要重验：受影响义务、入口、环境、候选身份和证据
不受影响范围：依据与理由
需要处置的反馈：稳定来源定位、接收 owner、处置 authority、结论依据、关联授权与解除/关闭证据
批准与实施：有权 owner、真实记录与复核结果
```

## 12. 采用与维护检查

在审阅或收尾前核对：

- 同一任务事实只有一个 GitHub/Project authority，`.pm` 只作生成映射或历史归档；
- Task UID、owner、scope、固定输入、依赖、验收引用和实际证据能够互相定位；
- 每个条件、失败路径和未证明范围都被单独表达；局部完成没有冒充组合完成；
- 记录使用的状态和门禁来自现行 workflow，没有复制或发明枚举；
- 文档、实现、测试、产品、QA、发布和公开说明的结论没有越过各自 owner；
- 变更影响、历史证据和 superseded/retired 跳转保持可追溯；
- 结构/link 检查的结果只被声明为结构证据；新的语义 checker 或 activation 未实现时明确写未实现。

代表性 PM/系统文档迁移、第一批新字段需求、真实组合验收规模扩大或现有 checks 无法诊断固定输入/引用漂移时，应重新建立有界 owner task，先更新唯一 workflow/source authority，再实现兼容 checker 与正反例。这个触发器不创建自动 backlog，也不授权后台继续执行。

## 参考与来源处置

- 当前工作流：[`doc/engineering/workflow/source-of-truth.md`](../workflow/source-of-truth.md)。
- 配套系统设计内容标准：[`system-design-writing-standard.design.md`](system-design-writing-standard.design.md)。
- 当前 task/project 映射与归档路径：见 [`.pm/README.md`](../../../.pm/README.md) 及 workflow source 的 GitHub Project-backed PM contract。
- 本规范吸收的系统设计/PM v1、贯通方案和模板均为本任务固定 proposal inputs；原文对照、哈希、历史 PR 状态与未采纳项保留在 GitHub Issue #3662 evidence 和任务 scratch，不把附件 metadata 当作当前 task truth。
- 本规范采用其 PM sections 8–9、Appendices C–F、integration sections 5–8 和 templates B–F/H 的可复用内容语义；具体 lifecycle、schema、adapter、activation 与 release gate 继续由现行 workflow/专业 owner 管理。
