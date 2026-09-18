# 跨层需求追踪闭环设计

- 状态：proposed
- Owner role：`repository_health_engineer`
- 产品/系统语义联审：`producer_system_designer`
- QA 证据联审：`qa_engineer`
- 任务真值：GitHub Issue `#3706` / `task_53191b31a5a9498cb7745fec8ebc0bc8`
- 上游审计：GitHub Issue `#3704`
- 生命周期权威：[workflow source of truth](../workflow/source-of-truth.md#traceability-record-contract)
- 配套规范：[系统设计写作规范](system-design-writing-standard.design.md)、[项目管理记录规范](project-management-record-standard.design.md)、[产品文档规范](product-documentation-standard.design.md)

本文定义产品或专业要求、系统设计义务、GitHub Task 与实际证据之间的最小闭环。它复用现有 `oasis7.loop-change/v1` 协调记录，不创建第二台账、第四 loop、新 Project taxonomy 或新的 Product PRD-ID。本文的 YAML/JSON 片段若标为“字段节选”只用于解释字段关系，不能作为完整记录或 evidence；标为“完整样例”时必须可由 C1 的 canonical fixture 自动校验。S1 只冻结这一区分与消费边界，不预先引用尚不存在的 fixture。

<a id="traceability-goals"></a>
## 1. 问题与目标

当前仓库分别具备产品追踪表、系统设计追踪写作规范、GitHub-backed PM 真值和可选 W2 traceability 校验，但普通任务没有统一、强制且可往返保存的跨层关系。结果是各局部门禁可以同时为绿色，而产品要求、系统设计条款、Task UID 和实际 evidence 仍可能无法互相定位。

本设计目标是：

1. 用一个规范化 obligation 关系表达上游要求、系统设计、本次任务和证据；
2. 对不适用关系使用显式、可审查的处置，不从缺失字段推导 N/A；
3. 让 Issue、Project 投影和 repo-local cache 往返时不丢失追踪或终态字段；
4. 对新增或实质修改内容启用强门禁，同时避免一次性迁移未触达的遗留文档与任务；
5. 保留产品、系统设计、任务证据和 workflow 的既有 authority 边界。

## 2. 非目标

- 不建立全仓常驻依赖图、第二任务台账或手工消费者清单。
- 不把 GitHub Project 字段提升为合同或证据 authority。
- 不要求每个纯工程任务虚构产品需求，也不允许用空引用冒充“不适用”。
- 不在本变更中迁移全部历史文档、Issue 或旧 traceability record。
- 不改变现有 workflow 状态、completion mode、loop、角色权限或 PR/终态门禁。
- 不以结构检查代替产品正确性、系统可实现性、运行行为或 QA 放行判断。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [system-design writing contract](system-design-writing-standard.design.md#2-上游约束与相关角色) | 跨层记录必须把上游要求、系统设计义务和验证入口分别绑定，且保留各自 authority 边界。 | [traceability goals](#traceability-goals) | `repository_health_engineer`; product/system/QA review roles | 不证明产品语义、运行行为或发布就绪。 |

## 3. Authority 与冲突边界

| 事实 | 唯一可写 authority | 本闭环中的作用 |
| --- | --- | --- |
| 产品价值、范围、玩家承诺、产品 AC | workflow 当前声明的 product authority；新内容使用 `doc/product/` 四模块树，未迁移 legacy 仍使用其现行路径 | 提供稳定上游 requirement；不因未迁移路径否定旧批准输入 |
| 专业规则与接受条件 | 专业域 PRD/规则文档 | 为纯工程或专业变更提供稳定上游 acceptance |
| 技术合同、边界、状态、迁移与验证设计 | 专业系统设计 | 提供稳定 system-design obligation |
| Task UID、scope、执行、实际候选与 evidence | GitHub Issue evidence | 绑定本次交付事实 |
| active queue 与 cockpit 字段 | GitHub Project | 只作投影，不覆盖 Issue 或 receipt 的细粒度事实 |
| 生命周期、权限和门禁 | workflow source of truth | 决定准入与终态 |

若上游产品承诺与系统可实现性冲突，当前协调 Issue 必须记录冲突、裁决 owner、影响和处置；任一消费者不得静默改写另一 authority。

### 3.1 实际 authority 与有限消费

引用身份和内容资格是两项独立检查。`repository + path + fragment` 是语义定位；冻结合同的 `contract_id + revision + contract_digest + publication_ref` 是合同版本身份；`source_commit`、`source_head_oid` 或等价 OID 只定位实际源码/文档快照。`revision` 遵循所消费 schema 的类型（例如合同版本为正整数），不得把 Git SHA 静默写入要求整数的 task input `revision`；OID 也不能替代条款 fragment。

实际 authority 必须由 canonical repository 的冻结发布内容和可回读的 Issue/comment 身份共同证明。当前工作树文本、`latest`、浮动分支、可变 URL、Project/cache 字段、作者自报 digest 或本地 JSON 都只是线索，不能授予消费资格。消费者只解析本记录声明的 `trace`/`consumed_clause_refs` 关系闭包，逐条确认 path/fragment、合同发布身份和适用 owner；缺失、重复、歧义、越界或无法回读的引用阻断该消费，但不要求扫描或迁移无关 legacy。

## 4. Canonical Trace Relation

唯一协调记录继续使用 `oasis7.loop-change/v1`。每个 `required_obligations[]` 元素表示一个可独立判定的 obligation，并增加下列语义：

```yaml
# 字段节选（不是完整 oasis7.loop-change/v1 记录；不能直接作为 C1 通过样例）
obligation_id: stable-local-id
required: true
applicability: required # required | not_applicable
trace:
  upstream_refs:
    - kind: professional_acceptance # product_requirement 适用于产品义务
      applicability: required
      repository: eng-cc/oasis7
      path: doc/engineering/doc-governance/project-management-record-standard.design.md
      fragment: 3-固定输入与证据身份
      contract_id: engineering-project-management-record-standard
      revision: 1 # 合同 schema revision；不是 Git commit OID
      contract_digest: sha256:af924078c204d31d8ae63ceddaa6920db723fe8b2dd557353ec6bd823381df11
      publication_ref:
        issue_number: 3706
        comment_id: 5679917357
  system_design:
    applicability: required # required | not_applicable
    repository: eng-cc/oasis7
    path: doc/engineering/doc-governance/system-design-writing-standard.design.md
    fragment: 11-验证设计与可追溯性
    reason: <仅在 not_applicable 分支填写；此字段节选未展开>
    owner_role: repository_health_engineer
    evidence_ref: <仅在 not_applicable 分支填写；此字段节选未展开>
mapping_slot: stable-slot
owner_loop: code
owner_role: repository_health_engineer
acceptance_refs:
  - doc/engineering/doc-governance/project-management-record-standard.design.md#3-固定输入与证据身份
```

`required=true` 是 `applicability=required` 的兼容别名。新建或实质修改的记录必须同时生成两者且值一致；既有记录缺少 `applicability` 时标记为 `legacy-unclassified`，不能静默解释成 required 或 N/A。

上述 typed upstream 保持当前 `oasis7.loop-change/v1` 字段位置：`applicability` 在每个 upstream ref 上，合同版本由 `contract_id + revision + contract_digest + publication_ref` 绑定；不在 typed upstream ref 中新增 `source_commit`。协调记录自身的源快照 OID 继续使用现行 `coordination_ref.source_commit`，文档条款快照使用现行 contract/publication identity。C1 必须依此验证版本与 OID 分离，不得自行发明第三个字段位置；若未来需要在 typed upstream ref 上增加 OID，必须以新 schema revision 和单独兼容评审引入。

跨文件引用必须同时包含 canonical repository `eng-cc/oasis7`、repository-relative path、稳定 fragment，以及适用的冻结 contract/publication identity。裸 `REQ-*`、`AC-*`、`DES-*`、浮动分支或只有文件路径的引用不能满足跨文件关系。

完整样例必须包含 schema/marker、Task UID、协调记录 authority、每项 obligation 的 typed upstream/system trace、`applicability` 与兼容 `required`、映射槽位、候选选择规则和反馈链；其中所有必填引用均须有可回读身份，不能用 `null`、占位符或节选字段冒充完整记录。未来 C1 的完整 fixture 是唯一可执行样例来源，文档片段不得另维护一套 digest 或字段默认值。

## 5. Applicability 与显式 N/A

### 5.1 上游要求

- 变更产品价值、玩家承诺、产品范围或产品 AC 时，至少一个 `product_requirement` 必须为 required。
- 纯工程、治理或专业合同变更必须引用 `professional_acceptance`；此时产品 requirement 可以显式 N/A。
- 声称存在交付工作的 obligation 不得同时把产品和专业上游都标成 N/A。

### 5.2 系统设计

消费或改变技术合同、跨组件行为、状态、接口、迁移、恢复、安全边界或实现 obligation 时，system design 必须为 required，并引用准确 `path#fragment`。

只有变更不消费上述技术义务时，system design 才能标记 N/A。

### 5.3 N/A disposition

任何 N/A 都必须包含：

- 非空理由；
- 有界适用范围；
- applicability owner role；
- 可回读的 review/evidence locator；
- 重新评估触发器。

省略、`null`、空数组、`required=false`、`unknown` 或 `pending` 均不等于 N/A。N/A 不产生 leaf execution row，也不能关闭 required obligation；未解析引用、未清阻断反馈或未知 applicability 必须阻断 aggregate completion。

## 6. Task 与 Aggregate Evidence 绑定

仅当变更已绑定冻结 coordinating record 并进入 aggregate candidate 时，该记录中的每个 required obligation 必须恰好对应一个 aggregate evidence row。该组合语境下二者必须匹配：

- `obligation_id` 与 `mapping_slot`；
- `owner_loop` 与 `owner_role`；
- leaf `Task UID`；
- GitHub Issue/comment 的结构化 locator 与精确正文 digest；
- source HEAD、integration/tested tree、配置、入口、环境和 evidence window；
- acceptance 结果、失败或未覆盖范围。

对该 aggregate candidate，重复 slot、缺行、多行竞争同一 required obligation、owner 不一致或 evidence identity 不完整都必须失败。多个 leaf 可以有不同 source HEAD，但 aggregate 结论只能绑定一个实际组合验证过的 candidate。

普通单叶子是一个 owner、一个 Task UID、一个 scope 和自己的交付证据，只沿自己的 Issue/PR 链判定，不因为没有多义务记录而被迫新建协调 Issue、aggregate row、`oasis7.loop-leaf-result/v1`、等价审批或 candidate tuple；它不能声明组合完成。组合变更必须在首个叶子执行前绑定一个冻结协调记录，完整必需集合、mapping slot、候选选择规则和阻断反馈不可通过删除字段、空 `delivery_obligations` 或关闭某个叶子来降级；每个叶子只关闭自己的 obligation，aggregate 仍须逐项回读。

## 7. PM Projection 与 Round Trip

GitHub Issue 正文和 evidence comments 保存细粒度任务事实；Project 只投影可管理字段；`.pm/github-project-sync/tasks.json` 是可刷新 cache。序列化、解析和 refresh 必须保留：

- `workflow_phase`；
- `completion_mode`；
- `non_pr_completion_evidence` 及其 canonical file/digest binding；
- `doc_refs`；
- `related_prd`。

Project 的粗粒度 `done` 不得覆盖 Issue/cache 中的 `task_done`、`closed_without_merge` 或 `post_merge_done`。当 live Issue 未包含某个可选投影字段、但 identity-bound cache 已有受信值时，refresh 必须按字段 authority 决定保留或 fail closed，不能静默清空。

对于 `non_pr_task`，`completion_mode`、分类 evidence、verified `task_complete`、`task_done` 和 terminal receipt 必须共同成立。默认 worktree refresh 后必须仍能消费 canonical task-worktree evidence；任一字段丢失时终态写入失败关闭。

## 8. Changed-Scope Admission

准入范围由可信的 `base`、`head` 和 worktree overlay 计算：

- 新增或实质修改的产品/系统设计及新 trace record 使用本强合同；
- target-only 变化不归因于 source；
- staged、unstaged 和 untracked 的适用文件纳入 worktree 检查；
- 仅空白、行尾空格和 HTML comment 变化可按现有规则排除；
- 未触达的 legacy 文档与任务不强制迁移。

`legacy-unclassified` 记录一旦被实质修改、重新发布或纳入新的 aggregate，必须升级到强合同，或取得明确 owner、理由、范围、证据和过期触发器的迁移 disposition。

## 9. 失败处理与诊断

Checker 必须输出稳定、可定位的诊断，至少区分：

- `trace-upstream-missing`；
- `trace-system-design-missing`；
- `trace-na-incomplete`；
- `trace-ref-unresolved`；
- `trace-required-alias-mismatch`；
- `trace-slot-cardinality`；
- `trace-owner-mismatch`；
- `trace-evidence-identity`；
- `trace-projection-loss`；
- `trace-legacy-upgrade-required`。

诊断必须包含 record/文件路径、obligation 或字段、失败关系和修复方向；不得以无上下文异常、默认值或忽略字段继续执行。

## 10. 实施边界

实施顺序必须 source-of-truth-first：

1. 更新 workflow traceability contract 及产品、系统设计、PM 记录规范；
2. 扩展现有 W2 schema/validator，不创建平行格式；
3. 修复 task Issue serializer/parser 与 refresh authority whitelist；
4. 把 changed-scope checker 接入现有文档治理与适用 workflow 入口；
5. 增加 focused regression 与默认 worktree non-PR integration test；
6. 由 repository health、producer/system 与 QA 按冻结 HEAD 审查。

本次 S1 为后续实现冻结以下只读交接：C1 只消费本四份规范的冻结提交、`oasis7.loop-change/v1`/`oasis7.loop-task/v1` 当前字段及本节完整/节选定义，验收身份/版本分离、typed applicability、完整样例与负向诊断且不重解释 legacy；C2 只消费 C1 已合入的引用解析/校验接口，验收实际技术权威、声明消费闭包、changed-scope 与 bounded legacy 关系，不能以改后缀逃逸；C3 只消费 C1/C2 的固定接口、当前 task binding/policy/CI identity，验收创建、resume、发布、promotion、closeout 与 CI 使用同一 binding、保留 round-trip/aggregate 阻断且不自授权。三者都必须在各自兼容实现、负向测试和真实 readback 完成后，另经现行 upgrade/enablement 流程才可启用。

## 11. 验收与验证

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [system-design writing contract](system-design-writing-standard.design.md#2-上游约束与相关角色) | [traceability goals](#traceability-goals) | 验证 changed-scope 只准入可解析关系，并对缺失、歧义、N/A 不完整与目标侧专有变更 fail closed。 | [system-design traceability regression](../../../scripts/system-design-traceability-check.test.py) | GitHub task evidence and required CI | 不证明专业语义正确性或真实运行环境。 |

### test_tier_required

1. 合法的 product/professional → system/N/A → Task UID → evidence 关系通过。
2. 缺失、歧义、不可解析、alias 冲突、slot/owner/cardinality 错误或未处置关系失败关闭。
3. 显式 N/A 完整时通过；省略、空值或 `required=false` 冒充 N/A 时失败。
4. Issue → Project/cache refresh round trip 保留五类 PM 字段和细粒度终态。
5. 默认 worktree 的 non-PR terminal closeout 在字段完整时成功，在 lossy refresh 时失败。
6. changed-range 正反例证明 target-only 与未触达 legacy 不被误纳入。

### test_tier_full

- `./scripts/doc-governance-check.sh --full-corpus`
- `python3 scripts/pm/loop-traceability.test.py`
- `./scripts/pm/lint.sh`
- `./scripts/pm/workflow-lint.sh --task-uid task_53191b31a5a9498cb7745fec8ebc0bc8 --phase current`
- non-PR default-worktree refresh/finalizer integration regression
- `git diff --check "$COMPARISON_REF...$SOURCE_HEAD"`

结构测试只证明结构和投影合同；最终 evidence sufficiency 与发布判断由 `qa_engineer` 收口。

## 12. 风险、回滚与未决边界

- **过度门禁风险**：用 changed-scope、声明消费闭包和显式 applicability 限制影响，不全仓迁移。
- **双重 authority 风险**：schema 只表达引用关系，不复制产品、系统或 evidence 正文。
- **Project 粗粒度覆盖风险**：refresh 按字段 authority 合并，细粒度终态只能由 canonical receipt writer 更新。
- **旧记录歧义风险**：缺失 applicability 保持 `legacy-unclassified`，触达时升级，不静默默认。
- **误报语义正确风险**：自动检查只验证身份、路径、关系和完整性；专业语义仍由匹配角色审查。

- **迁移/启用混淆风险**：迁移是经授权的旧条款到新权威的逐条映射、消费者修复和历史快照保留；启用是兼容 helper/checker 已通过负向测试、真实 readback 和限定范围回退审查后，按现行升级流程允许新任务消费。S1 的规范合入、C1/C2/C3 的实现合入或一份绿色文档检查都不自动执行迁移或启用。legacy 在途任务继续消费其已批准不可变输入，除非有新 epoch 与明确迁移处置。

若新增字段或 changed-scope gate 导致现有合法工作流无法启动，回滚仅撤销本变更新增的 admission wiring，并保留 Issue/cache round-trip 数据修复；不得恢复会丢失终态或 evidence 的旧 refresh 行为。
