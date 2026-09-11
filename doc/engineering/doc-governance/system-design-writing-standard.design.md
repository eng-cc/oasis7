# 系统设计写作规范

- 状态：active
- 适用范围：专业域的系统、架构、接口、状态、部署和验证设计文档
- 当前入口：[engineering/doc-governance README](README.md)
- 配套记录规范：[项目管理记录规范](project-management-record-standard.design.md)
- 上游组织规范：[文档分工与组织规范](doc-structure-standard.design.md)
- 任务生命周期权威：[workflow source of truth](../workflow/source-of-truth.md)

本文是已落位的内容规范。它规定系统设计应表达什么、如何把上游要求交给专业设计、如何把设计条款映射到验证。本文不新增 workflow 状态、任务台账、权限模型、运行服务或产品承诺。

产品 PRD 拥有产品价值、范围、玩家承诺和端到端结果；专业域 PRD 与 design 拥有该域的规则和技术合同；GitHub Issue/Project-backed task truth 拥有任务身份、范围、依赖、状态和过程证据；workflow source of truth 拥有生命周期、权限和门禁。发生冲突时，在当前 GitHub task evidence 中记录冲突和裁决，不在本文静默覆盖其他 authority。

本文中的规范关键词具有以下强度：MUST/必须表示适用时的强制要求；MUST NOT/不得表示禁止行为；SHOULD/应表示默认要求，偏离时必须记录理由、影响和残余风险；MAY/可以表示在边界内的可选做法。普通评论、模板省略或作者声明不能豁免 MUST/MUST NOT。记录的例外必须写明受影响条款、理由、范围、批准 owner、失效/复核触发，并且不能覆盖上级产品、专业域或 workflow authority，也不能绕过门禁或权限。

## 1. 问题、目标与非目标

### 1.1 问题与目标

系统设计必须让未读过聊天记录的消费者理解当前问题、目标、输入、输出、失败路径、恢复边界和验证方式。每份设计先写清：

- 设计身份：稳定标题、专题或模块、owner role、适用范围和审读基线；
- 问题、成功结果和可测目标；
- 设计承担的技术责任；
- 与产品需求、专业规则、实现和证据的链接。

需要更强追溯时，可以在人工可读的身份块记录 id、title、status、owner_role、module、upstream_refs、source_baseline、last_reviewed_at、review_evidence_ref 和 superseded_by。它们目前是写作信息，不是本次新增的机器 schema；status 只描述文档生命周期，不赋予运行时、workflow 或合同消费资格。

### 1.2 非目标

系统设计不替代产品 PRD、源码、测试报告、操作 runbook 或任务状态。它不把 UI/request accepted 写成 world effect，不把 LLM 意图写成 authority，不把 applied 写成永久持久化或跨入口同步，也不把“已合并”写成能力已发布。

仅涉及产品语义的设计继续使用现有 doc/product/ 配对 PRD/design 规范；本技术十二段骨架适用于专业技术设计，不强制产品设计改写为技术合同。

### 1.3 按风险裁剪

十二段是完整的技术设计视图。新建的长期设计和高风险变更 MUST 覆盖全部适用段落；跨域、协议/ABI、共识、存储、权限、迁移、恢复或状态模型变更 MUST 复核完整适用视图。小型、低风险且兼容的编辑 MAY 只更新受影响的条款、表格、需求承接和验证映射，不要求补写无关段落或填充空的 N/A 标题。任何省略都必须在当前 task evidence 中说明理由、未覆盖边界和 residual risk；裁剪不会豁免上级 authority、权限或 workflow gate。

## 2. 上游约束与相关角色

先列出消费者、输入 authority、专业 owner 和需要共同审读的角色。上游引用必须带路径和 fragment；固定输入还应带不可变提交、发布记录或当前 task contract 能提供的版本锚点。

### 2.1 需求承接与分配表

每一行 MUST 是一个真实的需求或专业接受关系，不能只写“支持 REQ”或类别名。路径和 fragment 要能定位到上游原文；纯工程变更没有产品 AC 时，填写专业规则/缺陷条款或用户请求的准确定位，并写明不适用产品 AC 的理由。

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| <真实上游路径#REQ/AC 或专业条款> | <本次必须承接的具体结果、前置条件和判定边界> | <本设计路径#DES 或 fragment> | <外部 authority、owner、固定输入或解除条件> | <不承诺的入口、环境、能力或未覆盖链路> |

一条需求可以映射多个设计条款，一个设计条款也可以服务多个需求；每条关系都保留上述五列，不建立一对一文档或任务要求。外部 owner/dependency 不是本设计的隐含责任，排除范围也不能用空白代替。

`path#fragment` 是被消费条款的身份，而不是可由上下文猜测的标签；完整身份还包括继承自冻结合同/发布记录的不可变 contract/publication repository（当前 canonical `eng-cc/oasis7`），并须在每个消费者匹配，不能隐式取当前仓库。固定合同、Task 或证据若跨文件消费本表关系，必须保留能够在固定内容中唯一解析的路径与 fragment；局部 `REQ-*`/`AC-*` 只有在同一文件内且无歧义时才可使用。文件或 anchor 迁移时记录旧到新的映射或明确处置，不能静默选择同名条款。

## 3. 当前状态、目标状态与差距

必须把当前基线、目标、差距和假设分开。推荐使用下表；当前状态没有证据时写 unknown 或 未验证，不能以目标描述代替现状。

| 对象/能力 | 当前状态（基线） | 目标状态 | 差距/假设 | 证据或 owner |
| --- | --- | --- | --- | --- |
|  |  |  |  |  |

当前基线至少包含适用的提交、版本、配置、数据/快照、消费者和环境。若设计描述 partial、current、target、proven，必须说明它们分别代表文档、实现、测试还是发布事实。

输入改变、消费者切换或基线失效时，不在正文中改写历史结论；通过当前 task 的变更记录和重新绑定处理，具体 eligibility/epoch 规则引用 workflow authority。

## 4. 边界与结构

说明组件、依赖、读写边界、authority、信任边界和不属于本设计的部分。至少回答：

- 谁产生、拥有、读取和持久化每个事实；
- 哪些输入是 advisory/intent，谁有权接受或拒绝；
- 组件之间的同步、异步、重放或幂等边界；
- 外部系统、WASM、LLM、节点、Viewer 和资源系统各自能证明什么；
- 禁止哪些跨域修改或隐式回退。

边界图、关系图或文字均可，图必须带图例、方向、版本/身份和失败语义。若产品输入包含控制权、draft、accepted、applied、stale 或 control-lost，技术设计要引用产品条款并保留这些状态的语义边界，不自行增加 UI 或协议字段。

## 5. 关键运行流程

用顺序图、状态转移或编号步骤表达成功、拒绝、超时、重试、并发冲突、恢复和取消。每条主流程要说明：

1. 触发与前置条件；
2. 输入身份、版本和幂等键；
3. authority 作出的判定；
4. 可观察结果和 receipt；
5. 失败后是否重试、回滚、人工处理或终止。

请求被接受不等于世界状态已应用；世界效果必须由 authoritative state、receipt 或明确的完成边界证明。没有该证据时，流程只可声明 request/validation outcome。

## 6. 接口与数据合同

每个跨组件接口说明 producer、consumer、身份、版本、顺序、兼容、错误、超时、重试和大小/资源边界。表格可采用：

| 接口/条款 | producer → consumer | identity/version | ordering/idempotency | success/error | compatibility |
| --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |

任务输入合同沿用当前 binding 提供的 input_contracts、固定提交/发布记录和 acceptance references；本文不宣布这些字段已经由新的 parser 或 adapter 执行。不得只写不断变化的 main 路径作为 in-flight contract。

接口字段、schema、ABI 或协议变化必须说明消费者兼容、版本切换、旧数据/快照处理和验证入口。缺少实现或运行证据时写明未验证范围，不用“设计上支持”替代事实。

## 7. 状态、事务与持久化

在完整适用的设计视图中，本节 MUST 单独出现；小型兼容编辑按 §1.3 只在受影响时更新本节相关条款和验证映射，不创建空的 N/A 标题。若状态、事务或持久化对该设计确实不适用，仍须写明理由以及负责的 authority。至少说明：

- 状态集合、转移触发、拒绝条件和终态；
- 事务原子性、提交点、并发冲突、重放和幂等；
- 数据、快照、receipt、审计记录的 owner、生命周期和恢复方式；
- accepted、applied、persisted、published 等词各自的证明条件。

任务或文档生命周期不是运行时状态机。不要把 workflow done、文档 active、能力 proven 或产品 accepted 互相推导；它们分别回链自己的 authority。

## 8. 部署、安全与运行约束

写明部署拓扑、权限、信任边界、密钥/凭据使用、资源预算、超时、外部依赖、失败隔离、降级/禁用边界和环境差异。权限应说明“谁能在什么条件下做什么”，而不是只列角色名。

运行、发布和恢复步骤留在相应 manual/runbook；本文只记录这些操作所依赖的设计约束和可验证接口。没有 hosted、真实网络、跨节点或浏览器证据时，不得从本地 fixture、同进程 demo 或可编译推断生产能力。

## 9. 质量与容量

每个质量目标采用“环境 / 输入或刺激 / 预期响应 / 判定指标 / 验证入口”格式。

| 场景 | 环境、规模与资源 | 预期响应 | 指标/阈值来源 | 验证入口 | 当前范围 |
| --- | --- | --- | --- | --- | --- |
|  |  |  |  |  | 未验证/已验证 |

性能目标要给出数据规模、并发或节点数、硬件/资源边界、样本窗口、分位数和阈值来源。确定性、重放、故障恢复、权限和跨平台要求要注明是否覆盖。单条通路、fixture、feature 编译或文档检查只能证明相应范围。

## 10. 兼容、迁移与回滚

说明旧消费者、旧数据/快照、版本切换、启用/禁用、迁移顺序、双读/双写（如有）和回滚限制。回滚必须指出回到哪个已知基线、哪些副作用无法撤销、何时需要人工处置。

输入合同、协议、schema 或权限资格发生实质变化时，必须在当前 task evidence 建立新基线并重新评估下游；新 draft 不自动使旧的有效合同失效。迁移示范不等于已完成全量迁移。

## 11. 验证设计与可追溯性

设计维护稳定的“要求—设计—验证方法”关系；任务维护本次实际实现、基线和证据。推荐追踪链：

~~~
产品成功标准（适用时）
  -> 专业域 PRD-ID / 规则条款
  -> 系统设计条款（路径 + fragment）
  -> GitHub Task UID
  -> PR + 固定源提交 / 实际测试基线
  -> 测试、评审与交付证据
~~~

### 11.1 验证映射表

该表是持久的设计验证计划，每行 MUST 把 AC 或专业接受条款、设计义务和准确的测试/手册入口连起来。表中的 candidate/environment 要求或选择规则表达适用的候选选择规则、环境要求与能力边界；协调记录先冻结批准的必要集合、映射槽位和选择规则，不预先伪造未来 Task/contract/evidence identity；实际 candidate、source/integration/tested tree、通过/失败、退出码和产物必须在组合前写入当前 task evidence，缺失引用保持 pending 并阻断整体完成，不阻止已批准叶子执行，也不要求长期设计随每次代码迭代回写实际提交。只有描述已由证据证明的历史基线时，才保留明确标注的历史 candidate，并链接其对应 evidence。

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| <真实上游路径#REQ/AC 或专业条款> | <本设计路径#DES 或 fragment> | <本次要证明的具体义务和前置条件> | <准确命令或观察、测试/手册路径与 ID、场景/层、版本/环境> | <Issue evidence、CI、artifact、receipt 或评审定位> | <未覆盖入口、失败路径、平台或阈值> |

证据必须区分本地运行、fixture/fake-GitHub、hosted CI、真实集成和发布验证。结构检查通过只证明结构；不证明产品正确、专业语义、运行行为、独立审读或 release readiness。

## 12. 决策、长期风险与未决问题

记录替代方案、约束、选择理由、正负后果、适用/失效条件、残余风险、决策 owner 和触发器。未决问题必须能定位到条款、owner 和解除触发；活跃合同中不要留下没有责任人和边界的裸 TODO/TBD。日常进度、排期和任务状态留在 GitHub task evidence。

重要模块边界、协议/ABI、存储/一致性、部署、安全权限、确定性或难以回退的迁移 SHOULD 有 ADR。实质反转用新的 ADR 替代并保留双向追溯，不伪造历史。

### 12.1 交付边界

本规范当前交付：专业技术设计的十二段骨架、需求承接表、当前/目标/差距表、接口/质量/验证映射、固定输入和证据边界，以及附录模板。

本规范当前不激活：机器可执行的 oasis7.doc/v1 metadata schema、metadata/lifecycle checker、新的 PM 或 loop 状态、新 Project 字段、scheduler/service、自动下游任务和代表性技术 pilot。本文的引用、验证计划和记录边界是内容契约，不等于 checker/schema 已实现或 loop 已启用；它们只有在各自 authority、adapter、检查范围和验证证据具备后才能单独采纳。这里的列举不是待办台账，也不改变当前 workflow。

---

## 附录 A：系统设计模板

完整设计复制后保留适用的十二段，并用 N/A（理由）表示确实不适用。小型兼容编辑使用下方轻量修改卡，不要求创建空的十二段标题：

~~~text
# <topic> system design

适用范围 / owner / 固定基线：
关联 PRD、规则、任务和消费者：

## 1. 问题、目标与非目标
## 2. 上游约束与相关角色
## 3. 当前状态、目标状态与差距
## 4. 边界与结构
## 5. 关键运行流程
## 6. 接口与数据合同
## 7. 状态、事务与持久化
## 8. 部署、安全与运行约束
## 9. 质量与容量
## 10. 兼容、迁移与回滚
## 11. 验证设计与可追溯性
## 12. 决策、长期风险与未决问题

附录：需求承接、接口/质量/验证映射、ADR、证据边界
~~~

### A.1 小型兼容编辑卡

~~~text
受影响条款/表格：
上游 requirement 或专业 acceptance（path#fragment）：
具体 obligation 与适用条件：
本设计条款（path#anchor）：
准确 test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则：
实际 candidate/source/integration/tested tree、环境、结果与 artifact：<当前 task evidence；普通代码迭代不回写本长期设计>
外部 owner/dependency：
明确排除、未覆盖范围与 residual risk：
~~~

## 附录 B：ADR 模板

~~~text
# ADR-<id>: <decision>

状态 / 日期 / owner：
问题与背景：
约束：
候选方案：
选择及理由：
正面与负面后果：
适用与失效条件：
替代关系与当前设计引用：
验证与复核触发：
~~~

## 附录 C：设计审读与证据卡

~~~text
Task UID：
记录类型：system-design-review
角色与时间：
固定 source/head、输入合同和环境：
审读条款（path#fragment）：
发现与严重性：
处置与复核：
验证命令/套件、退出码、artifact：
结论：
未覆盖边界与 residual risk：
~~~
