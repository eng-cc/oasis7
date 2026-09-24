# CI 身份一致发布与并行证据复用系统设计

**设计 ID：OASIS7-CI-PARALLEL-REUSE-1**
**版本：1.0.0 · 实施设计终稿**
**状态：Proposed；尚未实施、验收或启用，不改变当前有效门禁**
**Owner：repository_health_engineer；独立验证：qa_engineer；任务协调：tpm**
**模块：engineering/workflow**
**审读日期：2026-09-24**
**固定源码基线：`eng-cc/oasis7@539fa28860a5a531a9a9d00c8167e28071377af2`**
**冻结设计输入 SHA-256：`ba335cf20a69ba6aac69de87d0860847d9b6566d76e3ad3ae4d133f06d52139b`**
**建议落位：`doc/engineering/workflow/ci-parallel-evidence-reuse.design.md`**

本文承接本次会话中“CI 秒失败排障”“PR 创建早于 Issue 编号回写”“main 高频前进不应重复工作”三组要求。实施前，TPM 必须将当前用户要求、本文固定版本和验收集合冻结到现有 coordinating Issue，并为代码叶子建立各自 Task/PR；本文不冒用已有 Issue #3871 作为本次任务，也不虚构未来 Task UID、合同 revision 或验收证据。

文中“必须／不得”表示拟采纳的技术合同；只有 `source-of-truth.md` 先更新、兼容实现通过独立验证并经现有流程授权启用后，才成为实际执行规则。现状事实与设计目标分别标注。

---

## 1. 问题、目标与非目标

### 1.1 问题定义

当前问题不是简单的 CI 重试不足，而是将不同生命周期的事实捆绑在一起：创建 PR 与回写任务绑定不是一个原子操作；源码更新与 PR body 中 projection 更新不是一个原子操作；source identity、集成目标和工作流执行身份会各自变化；源码评审是否适用，又与是否取得最新集成结果耦合。

已核对的基线行为包括：`prepare-task-pr.sh` 先创建或找回 PR，再调用 `record-pr`；`integration_ci.py` 从 PR 的 `base.sha` 获取目标，并要求运行时 SHA 与该目标完全相同；`ci_ready_receipt_identity.py` 中 `verification_affected=true` 等条件会触发严格集成路径。它们分别构成发布窗口、基线错配和过度升级的实现基础。[R1] [R2] [R3]

### 1.2 核心目标

> **同一个 PR 的 source head、相关输入和适用规则没有变化时，main 连续合入 20 个无关 PR，也不新增源码提交、不重做角色评审、不重复执行重型 CI、不回退任务阶段；仅在现有晋级／合入入口完成轻量适用性检查。**

这里的“无关”必须由可信、完整的输入范围证明，而不是由提交标题、`.md` 扩展名、作者声明或“没有修改同一个文件”推断。过期、撤销、外部依赖改变等独立失效条件不属于这个零重做承诺。

其他目标是：标准 PR 创建不依赖“先红一次再重跑”；同 head 的晚到元数据能在同一轮 CI 内完成准入；证据能够按测试单元和角色独立复用；真正相关的变化只补受影响验证；所有恢复有界、幂等、可审计。

### 1.3 非目标

不新增常驻服务、数据库、GitHub App、自动 loop、全局开发锁或后台监督器；不更换构建系统，不以本设计重构业务 crate；不关闭 required checks，不使用管理员绕过来实现提速；不把文档或代码合入视为能力已启用；不承诺普通直接合入模式具备“最终最新 main 全仓合并树已经重新实测”的原子保证。

本方案仅使用本地 Codex 客户端、现有仓库脚本、现有 GitHub Issue/Project、Actions 和 Git 对象。新的操作必须在已有授权任务内执行，长时间稳定等待后保留恢复事实并返回，不注册后台续跑。

## 2. 上游约束与相关角色

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [source-of-truth.md#proposed-ci-input-scoped-evidence-reuse-contract](source-of-truth.md#proposed-ci-input-scoped-evidence-reuse-contract) | 新能力默认禁用；仅在完整实现、独立验证和授权后按可信输入适用性复用 | [DES-CIR-01](ci-parallel-evidence-reuse.design.md#des-cir-01) | TPM 冻结用户请求和 v1.0.0 acceptance；QA 验证 20 次无关前进 | 设计和静态文档检查不能证明运行能力 |
| [source-of-truth.md#split-source-review-integration-contract](source-of-truth.md#split-source-review-integration-contract) | 保留不可变 source/review/test/run 身份，并阻止新失败遮蔽旧证据 | [DES-CIR-06](ci-parallel-evidence-reuse.design.md#des-cir-06) | receipt 与 lifecycle owner；QA 审 attempt 隔离 | 不将 reused result 写成本次已执行 |
| [ci-projection-publication.design.md#des-cip-01](ci-projection-publication.design.md#des-cip-01) | 新建及更新 PR 时按固定候选顺序发布并严格绑定 projection | [DES-CIR-02](ci-parallel-evidence-reuse.design.md#des-cir-02) | C1 publisher/resolver 实施 | 不承诺跨 GitHub API 原子事务 |
| [ci-projection-publication.design.md#des-cip-04](ci-projection-publication.design.md#des-cip-04) | 每个 receipt 只消费对应 run/attempt 的闭合证据 | [DES-CIR-06](ci-parallel-evidence-reuse.design.md#des-cir-06) | C4 receipt consumers；QA hosted 验证 | artifact 名称或存在本身不是通过证明 |
| [../doc-governance/system-design-writing-standard.design.md#11-验证设计与可追溯性](../doc-governance/system-design-writing-standard.design.md#11-验证设计与可追溯性) | 准确映射设计义务、验证入口、候选和未证明范围 | [DES-CIR-10](ci-parallel-evidence-reuse.design.md#des-cir-10) | QA 独立验证；TPM 回写实际证据 | 本表不能代替组合或 hosted 证据 |

### 2.2 Authority 与现有设计关系

`doc/engineering/workflow/source-of-truth.md` 是唯一规范性工作流来源。source v1.20.0 已记录“首次验证强度”与“后续证据适用性”的目标区分，并明确 `input-scope-reuse/v1` 仍 Proposed / disabled；能力通过独立验证并获授权启用前，现行普通/严格路径仍控制所有任务。本文细化未来能力合同，不单独改变当前 gate。[R4]

既有 `ci-projection-publication.design.md` 的状态是 Proposed，已经定义了 publication v1、受限 resolver、同 run 恢复、attempt-bound envelope v2 和先消费者后生产者的迁移顺序。本文复用并扩展该设计，不另建一套 publication、journal 或 artifact authority。既有 DES-CIP 发布／恢复细节仍由该文档维护；本文维护并行复用和跨组件组合规则，两份文档有冲突时必须在规范更新任务中显式裁定。[R5]

### 2.3 Supplementary requirement context

完整条款身份均属于 canonical repository `eng-cc/oasis7`，下表路径在落位后相对 engineering/workflow 解析。

| 上游 requirement／professional acceptance | 具体义务与适用条件 | 本设计条款 | 外部 owner／依赖 | 排除或未证明范围 |
| --- | --- | --- | --- | --- |
| [Issue #3951 冻结的 professional acceptance](https://github.com/eng-cc/oasis7/issues/3951#issuecomment-5810526692) | 无关目标变化不触发源码、评审和重型测试重做 | 本文 `#des-cir-01`、`#des-cir-04` | TPM 冻结用户要求；QA 20 次前进验证 | Issue evidence 是任务验收定位；不将聊天记录冒充已发布机器合同 |
| [Issue #3951 冻结的 professional acceptance](https://github.com/eng-cc/oasis7/issues/3951#issuecomment-5810526692) | 保留严格 source／Task／run／attempt 身份和 required-gate 的 fail-closed 判定 | 本文 `#des-cir-01`、`#des-cir-06`、`#des-cir-09` | repository health 定义身份合同；QA 验证错身份与坏证据负例 | 新能力仍为 Proposed；设计映射不授权放宽当前门禁 |
| [Issue #3951 冻结的 professional acceptance](https://github.com/eng-cc/oasis7/issues/3951#issuecomment-5810526692) | 真实输入或义务变化时，只补受影响测试单元和专业角色 | 本文 `#des-cir-04`、`#des-cir-05` | 可信 planner 和 QA 完整性验证 | 未证明输入闭包时保守扩大范围，不声称细粒度复用 |
| [Issue #3951 冻结的 professional acceptance](https://github.com/eng-cc/oasis7/issues/3951#issuecomment-5810526692) | publication／binding 恢复有界、幂等，等待或失败不得伪装为通过 | 本文 `#des-cir-02`、`#des-cir-03`、`#des-cir-08` | publisher、resolver 与 QA 乱序／丢响应验证 | GitHub API 不提供跨调用原子事务 |
| [Issue #3951 冻结的 professional acceptance](https://github.com/eng-cc/oasis7/issues/3951#issuecomment-5810526692) | 启用前取得独立 QA、hosted 行为和平台 required-check 证据 | 本文 `#des-cir-09`、`#des-cir-10` | QA 独立组合验收；TPM 保留协调任务 | S0 文档门禁通过不等于代码能力或 hosted 验收通过 |
| `source-of-truth.md#manual-three-loop-transition` | 本地手动、单 Task/PR 主链、保留准入与权限边界 | 本文 `#des-cir-02`、`#des-cir-09` | TPM、现有 PM adapter | 不增加自动任务或第四个 loop |
| `source-of-truth.md#split-source-review-integration-contract` | 严格区分 source、review、integration、run/attempt 身份 | 本文 `#des-cir-01`、`#des-cir-05`、`#des-cir-06` | receipt 与 lifecycle consumers | 新能力未启用前仍按当前高风险 current-target 规则执行 |
| `ci-projection-publication.design.md#des-cip-01`、`#des-cip-02` | 首次创建和源码更新不暴露可被错误消费的半发布状态 | 本文 `#des-cir-02`、`#des-cir-03` | 既有 publisher/resolver 设计 | 不是跨 GitHub API 原子事务 |
| `ci-projection-publication.design.md#des-cip-04`、`#des-cip-05` | attempt 隔离；只对可信的前测试发布失败恢复 | 本文 `#des-cir-06`、`#des-cir-08` | 全部 artifact／receipt readers | 不混用 A1/A2，不掩盖真实测试失败 |
| `../doc-governance/system-design-writing-standard.design.md#11-验证设计与可追溯性` | 需求、设计、测试入口、候选和证据边界可追踪 | 本文 `#des-cir-10` | QA 独立验证；TPM 回写实际事实 | 本文通过不等于 hosted 行为通过 |

这是纯工程流程变更，产品 requirement 拟标为 `not_applicable`：不改变玩家承诺、玩法规则或产品 AC；范围为 engineering/workflow，applicability owner 为 repository_health_engineer，复核触发为改动扩展到产品／发行承诺。该 N/A 的真实 review/evidence locator 必须在实施任务中补齐；补齐前不得声称其机器追踪合同已经完整。

角色分工：repository health 负责身份、安全与工作流契约；QA 独立执行故障矩阵与并行验证；TPM 管理上游冻结、交付依赖和现有生命周期；各专业角色只复审影响到本专业的输入。CI 成功不代替本地 live Project／权限／hold 准入。

## 3. 当前状态、目标状态与差距

| 对象 | 当前基线事实 | 目标状态 | 需要改变的部分 |
| --- | --- | --- | --- |
| PR 编号绑定 | PR 创建／复用后才执行 `record-pr` | 缺号但合法创建中的请求有界等待；错号立即阻断 | publisher 创建意图与 `loop-ci.py` 准入协调 |
| Projection 发布 | 既有设计记录事件 body 与新 H 可错配，修复仍为 Proposed | 有序发布；事件优先；严格同 H 回读 | 落地既有 DES-CIP，不降低 source digest 校验 |
| 集成基线 | PR `base.sha`、运行 SHA、workflow SHA 被要求相等 | B 固定被测目标，W 记录实际可信执行版本；无需追 main | 重构 dispatch/prepare/discovery/verified_run 全链路 |
| 复用决策 | 高风险条件可能直接否决 ordinary reuse | 首次强度、测试复用、角色复用分别判定 | 用结构化决策替代单一布尔结果 |
| 文档基线门禁 | 规范要求 product full-corpus，system design 为 changed-scope | 不丢完整覆盖，按文档与关系单元组合有效结果 | 必须配套修改规范、checker、完整性聚合器 |
| GitHub 保护 | 已有 required-gate；完整规则配置本次未重新取得 | required checks 保留，日常不要求追平 main | 启用前读取分支保护及叠加 rulesets，独立验收 |

前三项来自固定源码，文档门禁来自规范正文。已有函数中“评审适用性”和集成 provenance 的部分分离应保留，不应推倒重写。[R1] [R2] [R3] [R4]

## 4. 边界与结构

<a id="des-cir-01"></a>

### DES-CIR-01：证据事实与适用性分离

定义以下身份：

| 符号／字段 | 含义 | main 无关前进时 |
| --- | --- | --- |
| `H / source_head_oid` | 当前候选源码提交 | 不变 |
| `S / source_scope_oid` | 本代源码评审比较起点；首次冻结并核验 ancestry | 不因查询 main 自动替换 |
| `B / integration_base_oid` | 某次真实测试采用的目标快照 | 历史记录不变 |
| `M / tested_commit_oid`、`T / tested_tree_oid` | 实际执行测试的 commit/tree | 历史记录不变 |
| `W / workflow_sha` | 真实运行的工作流定义身份 | 保留原值，不冒充最新 main |
| `P / effective_policy_identity` | 已合入、获准适用的策略／executor 合同 | 新草稿或无关 main commit 不自动替换 |
| `Q / assessed_target_oid` | 最近一次已完成适用性判断的目标快照 | 轻量判断成功后前移 |
| `R/A` | GitHub run ID／attempt | 已有执行记录不变 |

`S` 必须是合法 source 祖先，并满足当前任务的范围合同；不能为了让 projection 通过而随意改成最新 B。若 target 已吸收部分 source、历史重写或 source 比较关系异常，执行显式范围核验，不能静默重定义任务。

架构由五部分组成：现有本地 publisher 负责发布；CI 的可信 admission/resolver 负责冻结输入；现有 planner/executor 负责实际验证；拟新增纯函数适用性模块负责按单元判定复用；现有 receipt/lifecycle/closeout 聚合准入、评审、测试和合入条件。

```text
本地 canonical task + 冻结源码 H/S
       │ 有序发布，原有 Task evidence + publication journal
       ▼
Draft PR ──► PR required-gate
                │ 可信准入与输入冻结
                ▼
          planner → 实际测试 → R/A 独立 artifact
                                  │
角色评审证据 ─────────────────────┤
main 的固定 Q ──► 输入适用性判定 ─┤
                                  ▼
                    现有 ci-ready/lifecycle gate
                      │ 复用 / 增量验证 / 阻断
                      ▼
                      授权合入与事后回读
```

图中箭头表示当前任务内的数据消费，不表示新增监听器或后台调度。GitHub Issue/Project 保持任务事实来源；本地 journal/cache 只用于恢复和加速；PR body 是不可信运输载体；源码候选、作者自报 SHA-256、普通评论不能自行授予测试成功或策略资格。

## 5. 关键运行流程

<a id="des-cir-02"></a>

### DES-CIR-02：首次创建，先意图、后编号、有界准入

创建前固定 repository ID、Task UID、epoch、canonical branch、H/S、projection digest 和有效策略身份，完成可在创建前执行的校验。在现有 Task evidence 中发布 creation/publication intent 并回读，随后推送精确 H，创建携带该 projection 的 Draft PR。

PR 返回后，调用现有 `record-pr` 回写编号，再回读 canonical Issue、PR 和 publication binding，确认双向绑定一致。创建前的 publication ID 不依赖尚不存在的 PR 编号；编号通过独立的 reciprocal binding observation 关联同一个 publication，不能在拿到编号后偷偷改写原 publication 的身份。

CI 启动时，只在存在匹配仓库／Task／分支／H 的可信创建意图、且唯一缺项确实是尚未写入的 PR 编号时进入 `binding_pending`。重读必须有界，补齐后在同一 run 内继续。绑定另一个 PR、UID 重复、仓库错误、缺乏可信创建意图，均不能套用宽限。

绑定与 projection 共享一个 admission 等待预算，初始采用既有设计的 45 秒、最多三轮双读、单次读取 5 秒和预算内退避；不叠加成多个 45 秒。阈值是待验证设计值。超时令 required-gate 保持不通过并产生明确诊断，不伪装成成功或跳过。

create／record-pr 的响应丢失后先按精确身份回读；找不到且无法证明未创建，不盲目再次 POST。客户端退出后只恢复原 journal 的动作，不创建第二条 PR 主链。

<a id="des-cir-03"></a>

### DES-CIR-03：已有 PR 更新，先发布新 projection，再精确推送

在同一分支级独占发布事务内固定新 H1，先生成并完整验证 P(H1)，保留 PR 人工正文，只替换唯一机器块，PATCH 后回读；再用显式 H1 refspec 和针对远端 H0 的 lease 推送，并按现行政策额外限制 fast-forward。不得用可继续变化的本地分支名代替已冻结 H1。

这段时间 PR body 可能暂时指向未来 H1，而远端仍为 H0；因此 reader 不能任取“最新 body”。事件中匹配的 projection 优先；仅对合法但 stale/missing 的输入执行受限 live 回读，且 live head 必须等于本次事件 H。坏 digest、重复 key／marker、同 H 错 scope／config、未知 schema、错误 Task 立即阻断，不能靠拉取新 body 覆盖。

admission 成功后，将本次 H/S/D/P、publication 和任务绑定原子写入 runner 临时目录。后续 planner、测试和 receipt 只读该冻结输入。已准入的 H0 运行可成为 H0 历史证据，但不能在后续 H1 的准入中使用；尚未准入且 live H 已改变时返回 `SOURCE_SUPERSEDED`。

本条是既有 DES-CIP 有序发布和严格 resolver 的实现承接，不能再新增 `edited` 自动跑全量、空提交、日常 close/reopen 或靠删 marker 退回 legacy 的恢复路径。[R5]

<a id="des-cir-04"></a>

### DES-CIR-04：main 前进，按证据单元增量判断

**默认不响应每一个 main push 主动派发任务。** 只在当前任务 resume、promotion、merge 入口，读取并固定真实 default-branch ref 为 Q；已有成功判断覆盖到 Q0 时，先对 Q0→Q 做增量筛选。Q0 必须连同 H、策略、证据集合和完整闭包一起验证；本地 cache 丢失或不可信时从原证据重算，不能将 Q0 当作已测试基线。

每一验证单元 u 和角色 r 分别定义输入集合与指纹：

```text
F_test(u, T) = SHA256(canonical(
  unit_contract + obligation_set + command/checker_bytes
  + selected_input_bytes_and_modes(T) + membership_and_dependency_edges(T)
  + applicable_policy + environment_contract
))

F_review(r, H, Q) = SHA256(canonical(
  source_change_identity + role_contract + review_rules
  + consumed_requirements_and_context + affected_consumer_contracts
))
```

输入指纹不得包含仅用于审计的“整个 main commit SHA”“整个 workflow commit SHA”“全仓 tree ID”或观察时间；但必须包含实际影响测试语义的 workflow 命令、脚本、工具链／action 版本和环境条件。不能把实际全仓读取的测试伪装为单 crate 输入范围。

判定顺序为：先核验来源、身份、成功状态、有效资格和最新适用请求屏障；再用 target 增量筛选；命中时构造只读的当前集成候选，比较实际输入指纹及义务集合。所有单元和角色均未改变，则复用既有证据，仅刷新轻量 applicability observation。测试输入改变而评审输入未变，只补测试；相关验收／契约改变，才补相应角色；H 改变时本版不做跨 H 自动复用。

路径不相交只是优化，不是完整性证明。依赖图使用旧图与新图的并集处理新增边、删除／重命名两端、目录成员变化、共享消费者和新增测试义务。无法映射的 consumer／contract 必须在首次计划阶段解析到有限模块／契约范围；仍不完整时扩大到已证明完整的专业域，必要时全域验证，但不得直接清空所有角色评审。影响判定只读 manifest，不执行候选 build.rs 或候选自带发现脚本。

共识、WASM ABI、序列化、状态根、回放、依赖配置、全局安全策略是真实相关输入；不能为了命中复用目标而排除。网络服务、fleet health、时间敏感安全检查等另有环境／新鲜度合同，不能仅凭 Git 输入未变永久复用。

### 5.4.1 两种验证维度，避免高风险任务追 main

`initial_validation_level = standard | elevated` 决定首次需要哪些测试、是否需要默认分支可信 executor、哪些独立角色。高风险代码可以选择 elevated，但不因此在每次 main 前进后重跑。

`applicability_mode = input_scoped | snapshot_exact` 决定后续结果能否复用。日常 PR 包括高风险工程 PR，默认 input_scoped；snapshot_exact 只对明确冻结的发行／验收候选使用，必须实际测试该 T*。main 继续前进不自动移动 T*。确实要“最新 main 的最终组合精确实测”的任务单独使用服务器端串行集成边界，见 §8，不伪装成普通复用模式。

### 5.4.2 全仓门禁不得被漏掉，也不能成为全局重跑开关

现行规范要求 product full-corpus；因此仅缩小 changed paths 不足以实现本目标。[R4] 必须将“完整覆盖义务”和“每次完整重执行”拆开：完整性聚合器枚举本次树的全部产品文档及关系，按文档、相关链接／anchor 和必要全局索引分成验证单元。原始成功结果仍绑定原 tree；对当前树只复用输入完全一致的单元，变化的单元与跨文档关系补验；新增文档不能遗漏，删除及改名必须更新成员集合。

允许复用已可信核验的 main CI 单元结果，但必须验证当前合并候选中的该单元输入与其完全相同；“在 main 上通过”本身不是组合正确的证明。没有可用结果时，只补对应轻量文档／关系校验，不带着其他 PR 重编译 Rust。新组合协议启用前仍执行现行 full-corpus，不用 opt-out 偷渡。

Cargo.lock、workspace manifest、文档 inventory 等共享文件，只有在解析器能证明相关条目与全局不变量边界时才做条目级投影；未完成这项验证前使用保守域级输入。真正的全局不变量保留全局检查，不能任意分片。

<a id="des-cir-05"></a>

### DES-CIR-05：有必要才派发集成，B 与 W 独立

集成请求从真实默认分支 ref 获取 B，不再把 PR `base.sha` 当作实时 main。请求冻结 H、B、publication、需执行的 unit IDs、输入合同／指纹、有效 executor 合同和 request key；使用现有 main `workflow_dispatch`，不引入临时可信分支／标签。

runner 从实际 GitHub workflow/run 身份获取 W，并显式 checkout 固定 W 来提取可信 helper；随后在独立 worktree 构造 merge(B,H) 并测试。W 与 B 不相等本身不失败。接受前必须证明运行确实来自 canonical 默认分支路径、实际 executor 协议与被批准合同兼容、相关策略仍有资格，且没有借 candidate 的新规则授权自己。W 的相关执行内容改变而无法证明兼容时，返回 `EXECUTOR_CONTRACT_CHANGED`，仅刷新受影响的执行计划；不得通过删除校验来接受任意旧／新 W。

原 B 在默认分支的 ancestry、精确 H 及 source scope 均必须验证；main 前进时，仍允许完成已冻结 B 的执行并保留结果。其后对新 Q 按 DES-CIR-04 判断适用性，不自动取消运行、重新派发或 rebase。真正相关输入变动才产生新的请求。

request key 不以“最新 main SHA”单独区分日常重复工作，可由 Task/PR/H/epoch、待执行 unit 集合、各输入指纹、executor 合同和 purpose 形成。第一次请求选定的 B 一经持久化不可变；相同 key 的重试先找回原请求。若选取更大测试快照导致某个实际输入变化，该指纹必须改变，不能沿用旧 key。snapshot_exact 的 key 额外包含 T*。

dispatch 返回不表示 CI 成功，响应丢失必须通过 run/请求 readback 对账；确认范围不完整时不得再次盲发。单次手动任务内有界恢复，不创建轮询服务。

<a id="des-cir-06"></a>

### DES-CIR-06：证据粒度、attempt 隔离与失败屏障

保留 GitHub 原始记录：每份 execution evidence 明确标注 `executed_at=(H,B,M,T,W,R,A)`，复用项明确标注 `reused_from=(R,A,artifact_id,unit_id)` 和本次等价判定。不得将来自 A1 的测试写成“A2 本次已执行”；不得把较早绿灯当作较新失败 attempt 的成功。

沿用 proposed `oasis7-required-plan-v2-<R>-a<A>`，固定 archive 成员和 `overwrite=false`；每个结果包亦使用 R/A 或唯一 artifact ID，完整列出义务、结果、输入指纹、来源和 disposition。计划 artifact 仅证明计划，诊断 artifact 仅证明诊断，二者都不证明测试通过。[R5]

消费者先识别**当前适用的请求**，再读取其最新 attempt；不能按成功结果过滤后挑一个绿灯。对同 H、同义务、同输入和同资格的新必需请求，pending、真实测试失败、未知 provenance 都阻断消费旧绿灯。旧 target 的纯身份／发布失败不能跨请求污染别的候选；取消、过期或被替代的请求只能按精确身份和既有授权规则处置，不能根据自由文本随意清除。

没有新的必需请求、也没有适用失效条件时，main 前进不能自行制造“必须取得一个新 run”的 freshness 要求。真实失败修复后的同输入重试仍需真实通过；代码变化则进入新 H 的证据集合。

<a id="des-cir-07"></a>

### DES-CIR-07：合入前轻量检查，不在评审和 CI 期间持锁

现有 lifecycle/closeout 入口读取 live Task/Project、holds、权限、PR OPEN／draft、精确 H 和目标分支；读取 Q，确认 required-gate 为平台可接受状态，执行适用性聚合与 mergeability 检查。没有受影响义务则进入已有授权合入路径，不创建新 CI。

发出合入前再次读取 target ref。Q 变化时只比较新增区间并重新判断受影响单元；不退回源码开发、角色评审或全量 CI。source H 变化、权限／hold 变化则停止。真正的 scope／input 变化按其最小义务集合处理。

合入 API 必须携带预期 source head；但不能把它当成 target-base CAS。当前公开 REST merge 参数中的 `sha` 约束的是 PR head，没有对应的 expected base 参数。[G5] 本机可以用极短的共同目录提交锁协调本机 executor，但不得声称它能锁住 GitHub UI、其他机器或外部合入。

因此普通模式是带适用性检查的乐观合入：完成后回读实际 merge/squash commit 和真实父基线，确认与最终 observation 的关系；若最后窗口仍有变化，补轻量对账。若出现此前未见的相关变化，明确记录“已合入，存在待补验证”，阻断发行／最终完成，执行授权范围内的针对性验证；不伪造“未合入”，不自动 reset main 或批量取消其他任务。失去客户端连接时由下次显式 resume 回读恢复，现有 main CI 可提供集成缺陷信号，但不是无人监督保证。

首版只启用已验收的实际合入方式适配；rebase merge 等不能可靠还原真实候选的方式需要独立验收，不从末端 SHA 猜测基线。

## 6. 接口与数据合同

### 6.1 复用已有 schema，不产生平行 authority

source projection 继续为 `oasis7-workflow-impact-projection/v2`；publication 继续扩展既有 proposed `oasis7-ci-publication/v1`；外层执行 envelope 沿用 proposed `oasis7-required-plan-v2`，在该未启用协议中明确声明必需 capability `input-scope-reuse/v1`。reader 必须识别该 capability，不能由忽略未知字段的旧 reader 误判通过。若落地前发现同版本已经发布，必须按真实版本协商另行升级，不原地改已生效 schema 语义。

以下为字段合同，不是可直接提交的完整实例：

| 对象 | 必需身份／数据 | producer → consumer | 不变性与兼容 |
| --- | --- | --- | --- |
| Publication | repository ID、Task/epoch、源／目标 ref、H/S/D、固定 planner／policy、publication_id、projection_required | 本地 publisher → CI resolver | 创建前可无 PR number；后置 reciprocal binding 不改 pub ID；同 H 不同 D 必须显式重新冻结 |
| Execution evidence | unit ID、obligation set、input manifest/digest、environment、H/B/M/T/W/R/A、check/app/artifact IDs、结果 | 可信执行路径 → receipt reader | 原始 provenance 不可重写；旧 schema 不臆造 input closure |
| Review evidence | role ID、H/S、source change、实际读入契约／上下文、review rules、applicability digest、返回与处置 | 既有角色流程 → review consumer | 相关上下文变更才失效；角色晚返回按原身份归档 |
| Applicability decision | H、Q、prior assessed target、policy、原 evidence locators、全部 required units、逐项 reuse/revalidate/blocked、原因 | 可信本地评估器 → lifecycle／closeout | 是可重算 observation，不是 CI 通过证明，不把 Q 写成 tested base |
| Validation request | request key、H、固定 B、unit IDs、输入指纹、executor contract、purpose、创建事实 | 本地 adapter → integration runner/readback | 重试复用 key，B 不可悄悄前移；run ID 只是 locator |

### 6.2 结构化决策接口

拟新增纯函数模块 `scripts/pm/ci_evidence_applicability.py`，由现有 receipt/lifecycle 入口调用，而不新增调度器。

```python
def evaluate_evidence_applicability(
    source_plan, evidence_set, target_snapshot, effective_policy
) -> ApplicabilityDecision:
    """只读、确定性；不触发模型、push、dispatch 或 merge。"""
```

`ApplicabilityDecision` 至少分别给出 `source_review`、`test_evidence`、`merge_readiness` 三组结果，以及 `reused_units`、`required_test_units`、`required_review_roles`、`blockers`。不可继续用一个 `False` 混合表示“要补测试”“评审不适用”和“权限未知”。

保留旧布尔函数作为未迁移消费者的兼容入口；新 capability 任务必须使用完整决策，旧入口遇到该任务应报协议不支持，不能忽略新字段继续放行。CLI 是现有 `ci-ready-receipt.py`、`prepare-task-pr.sh`、`pr-lifecycle-gate.py` 的参数扩展，具体 flags 在代码叶子冻结，本文不宣称它们已可运行。

### 6.3 Input manifest 最低要求

每个单元记录规范化路径、文件类型／mode、内容 digest、目录成员集合、相关依赖边、命令／checker digest、工具链与适用配置。删除、rename 两端、symlink target、submodule OID、feature／target／环境变量必须按该单元真实语义纳入。网络／时间／远程数据源必须有版本或新鲜度合同，无法固定时标为非纯输入复用单元。

运行前后核验受测输入；工作区被测试修改时隔离源输入与输出，不能只事后计算一个已经被修改的 input digest。单元的义务集合需由可信 planner 完整枚举，缺一个新增义务也不能通过聚合。

## 7. 状态、事务与持久化

<a id="des-cir-08"></a>

### DES-CIR-08：有界等待和可恢复状态

publication journal 沿用 `<git-common-dir>/oasis7/pr-publication/...`，记录副作用前 intent 与回读后的 observation。补充首次创建的 `CREATE_INTENT_PUBLISHED → PR_OBSERVED → RECIPROCAL_BINDING_CONFIRMED → ADMISSION_READY`；源码更新沿用既有 `PREPARED → METADATA_CONFIRMED → HEAD_CONFIRMED → RUN_OBSERVED`。这些是内部恢复阶段，不是新增 GitHub Project 状态。

证据记录保持事实不变；`applicable / revalidate_subset / blocked / superseded` 是相对于当前候选的消费判断。main OID 变化本身不转移任务阶段。Q 只有在完整判定成功后才能记录为新的覆盖点，不能在 fetch 后先前移。

| 诊断 code | 处置与放行语义 |
| --- | --- |
| `BINDING_PUBLICATION_PENDING`、`PROJECTION_PUBLICATION_PENDING` | admission 内有界等待；不是成功 |
| `BINDING_NOT_READY_TIMEOUT` | 前测试阻断；身份补齐后可限定同 run 整轮恢复 |
| `TASK_IDENTITY_CONFLICT`、`EVENT_PROJECTION_INVALID` | 立即阻断；不能用最新 body 修饰通过 |
| `SOURCE_SUPERSEDED` | 停止旧候选准入，不回退新 H 的任务状态 |
| `TARGET_ADVANCE_UNRELATED` | 输出可复用，不创建新执行请求 |
| `INPUT_CHANGED`、`REVIEW_INPUT_CHANGED` | 只要求受影响测试／角色 |
| `EXECUTOR_CONTRACT_CHANGED`、`EVIDENCE_REVOKED` | 阻断受影响消费者，检查有效资格，不自动改 H |
| `RUN_ATTEMPT_MISMATCH`、`UNSUPPORTED_RUN_PROTOCOL` | 不取旧 artifact，不进行无效重跑 |
| `NETWORK_UNCERTAIN`、`PUBLICATION_WRITE_CONFLICT` | 保留 intent，回读后恢复；无盲目重复写 |

journal 采用当前用户权限、临时文件 fsync/rename 和现有 OS 锁；不得存 token。锁覆盖真实子进程寿命，不能因 TTL 到期自行夺锁。单机同 Git common-dir 可以协调，跨主机并发不作 CAS 承诺；PR body PATCH 没有本方案可依赖的原子条件更新，因此采用单 canonical publisher、精确回读与冲突阻断。[R5]

恢复只在已知支持协议的 W、可信结构化诊断证明测试尚未开始、H 未变化且 publication 修复已回读时，最多发起一次自动的同 run 整轮新 attempt。rerun 请求丢响应后先查询 attempt，不多发。真实测试失败不属于元数据恢复；超过自动预算保持可恢复事实，不能无限刷红灯。GitHub rerun 保留原 `GITHUB_SHA/GITHUB_REF`，所以它不能更新集成目标。[G3]

## 8. 部署、安全与运行约束

<a id="des-cir-09"></a>

### DES-CIR-09：可信执行与 GitHub 平台边界

本地 publisher 使用现有授权的 Git／gh 会话；hosted admission 使用仓库范围只读 token，不增 Project token、不把本地用户凭据注入 CI。默认不引入 `pull_request_target`、新 App 或写权限 CI。候选代码不得决定自身测试义务、放宽 planner 或选择通过证据；helper 及其 import 依赖从验证过的固定 tool root 提取，不能只拷入口文件却从 candidate import 模块。

W、app ID、run/attempt、check 和 artifact 来源必须通过 GitHub 回读验证。SHA-256 只证明内容一致，不证明发布者有权授权；Task／策略／证据资格仍依赖真实作者、现行权限和批准记录。工作流自身被 PR 修改的 elevated 路径，必须消费可信默认分支 executor 的独立证据，不能仅信候选自报的 PR artifact。

平台必要约束：[G1] [G2] [G4]

- 保留 PR 事件产生的 `required-gate` 作为平台 required check；缺绑定、缺证据不能将这个 job skip，或将 neutral/success 当作等待。
- `workflow_dispatch` 提供额外集成证据，不直接替代 PR required check。真实 GitHub 对 workflow job checks 的事件资格有限制，必须验证平台确实承认当前 PR 的检查。
- 不把 `GITHUB_SHA` 当作统一 source/check SHA。PR 事件可能使用 synthetic merge commit；通过精确 R/A 的 job/check provenance 定位，分别验证事件 H、M 和真实 check head。
- 日常 required checks 采用不强制源分支追平 main 的规则；启用前管理员核对 classic protection 与叠加 rulesets。配置未核实即不宣告启用；禁止借管理员 bypass 实现“通过”。

PR admission 只证明冻结候选的输入一致，不能让已经通过的 check 随 mutable body 自动代表新输入。义务、projection D 或评审规则发生实质修改而 H 不变时，必须显式重新冻结 generation，并重新执行受影响 gate；必要的轻量 gate 重跑是输入变化的代价，不属于“main 无关前进”的重做。

**最终合入的并发边界。** 普通直接合入没有 target-base CAS，本方案保留明确的最后窗口残余风险。若要求所有合入都在服务器端原子排除该窗口，需要经过验证的 GitHub merge queue／merge_group 等服务器串行集成路径；不能用本地双读证明原子性，也不能声称启用 queue 后绝无额外集成检查。该路径不作为日常默认 prerequisite。snapshot_exact 发行模式则冻结实际候选 T*，发布同一候选产物，不追逐新的 main。

并发取消仅按同一 PR 的源码代际／已明确 superseded 的请求进行；不同 PR、main push、文档提交不得取消其他 PR 的有效运行。开发、评审、测试期间不持有全局合入锁；真正合入的短暂串行不等于串行开发。

## 9. 质量与容量

以下均为验收目标，不是实测数据。

核心试验固定 H、相关输入、有效策略与可用证据，连续合入 20 个在完整闭包外的 main 变更。要求额外 source commits=0、角色重派发=0、重型 CI 执行=0、任务阶段回退=0；只在最终 resume／merge 入口产生一次累计增量判断，不按 20 次 push 创建 20 次任务。

首次创建／源码更新各执行至少 30 组 fake 乱序／崩溃案例，以及覆盖正常、晚绑定、晚 projection、同 run 恢复和负例的 hosted 组合。admission 预算内发布完成时首轮通过准入率要求 100%；超过预算可以有界失败，但不得假绿或无限自动重试。

轻量适用性评估目标：在 8 核、16 GiB、SSD 的参考本地环境，Git 对象已具备、约 5 万 tracked paths、目标增量不超过 200 paths、最多 100 个 required units，30 次 warm 样本 P95 不超过 5 秒；不包含 GitHub API 和首次 fetch。超出规模按预算报告，不将性能目标当正确性豁免。实际仓库规模和冷／热基线由 V1 测量。

建议从原诊断／receipt 生成指标：pretest failure 按 binding/projection/authority 分类；无关 main 前进导致的重型重跑数；每 H 的重型 attempt 数；按单元的 reuse 与 revalidate 比例；被迫同步 main 次数；错身份／坏证据拒绝率。取消、超时、跳过不得混入“通过率”。复用 cache 的规模和淘汰策略不影响正确性，删除 cache 后必须能从原始来源重建。

## 10. 兼容、迁移与回滚

### 10.1 实施文件与改动责任

| 现有／拟新增路径 | 本轮责任 |
| --- | --- |
| `doc/engineering/workflow/source-of-truth.md` | 先修订首次风险与后续复验的区分、准入等待、证据适用性、产品完整覆盖组合规则 |
| 既有 `ci-projection-publication.design.md` | 合并首次编号等待、publication binding 兼容和恢复细节；不重复 authority |
| `scripts/pm/projection_publication_contract.py`、`pr_projection_publication.py`、`pr_projection_resolver.py`（既有设计拟新增） | publication / intent / strict resolver 的唯一实现 |
| `scripts/prepare-task-pr.sh`、`scripts/pm/github-project-task.py`、`scripts/pm/loop-ci.py` | 发布事务、record-pr 回读、bounded binding admission |
| `scripts/pm/ci_evidence_applicability.py`（拟新增） | 可信输入范围、单位指纹、增量复用和三维决策；纯只读 |
| `scripts/pm/ci_ready_receipt_identity.py`、`scripts/pm/ci-ready-receipt.py` | 原始 provenance 与适用性分离；capability/version；失败屏障 |
| `scripts/pm/integration_ci.py` | B/W 分离、执行合同验证、请求幂等、独立集成 worktree、精确 R/A |
| `.github/workflows/rust.yml`、`scripts/plan-rust-required-scope.py`、现有 test driver | 统一 admission、unit-level plan/result、artifact 隔离、required-gate 聚合 |
| `scripts/pm/pr-lifecycle-gate.py`、`scripts/pm/task-closeout.sh`、`scripts/pm/workflow-next.py` | 消费相同决策；不因无关 target 前进退状态；next 保持只读 |
| 现有文档 checker 与文档完整性聚合入口 | 保留所有 product 文件与关系的覆盖；按输入组合结果，禁止漏项 |

### 10.2 交付叶子与依赖

下表是交付拆分，不是已创建的任务编号。每个叶子一个独立 Task/PR；只有既有接口冻结后才能并行，不能让几个 PR 同时编辑 rust.yml 或同一 receipt 入口。

| 叶子 | 内容 | 依赖／退出条件 |
| --- | --- | --- |
| S0 · system | source-of-truth、本文、DES-CIP 对齐、professional acceptance | 规范与设计独立 review；冻结实际 source / publication |
| C0 · code | schema/capability、publication／evidence 解析、三维决策接口，默认禁用 | S0；兼容和负例通过 |
| C1 · code | 首次编号等待、源码／projection 有序发布、journal 恢复 | C0；fake 时序与丢响应矩阵通过 |
| C2 · code | 输入闭包、unit 指纹、全量义务完整性与轻量文档组合 | C0；可与 C1 并行，边界不交叉 |
| C3 · code | B/W 解耦、executor 合同、幂等请求与独立 merge tree | C0；可与 C1/C2 并行，不直接改最终 workflow 接线 |
| C4 · code | CI 接线、attempt artifacts、receipt/lifecycle 全消费者迁移 | C1/C2/C3；单独收口共享文件；consumer first |
| V1 · verification | fake + hosted + 本地 Codex + 20 次无关前进 + 平台门禁验证 | 固定组合候选，不以叶子各自通过代替组合通过 |
| S1 · system | manual／运行说明与验收结果边界更新 | V1 真实事实；可执行 skill／角色卡另属 code 叶子 |

TPM 在 coordinating Issue 冻结必要交付集合与可并行边界，实际 Task/PR/commit/run/evidence 在执行时回写。旧 #3871 交付若被复用，必须验证已合入内容和实际能力，不根据设计存在就认定依赖完成。

### 10.3 启用顺序

先实现 reader、validator、诊断和兼容入口；shadow 阶段仅比较新旧判定，沿用旧放行，不产生新的重型验证或额外模型评审。随后在明确选择的任务 cohort 中启用，先验证普通代码／文档，再验证工作流高风险路径。所有策略变化先合入可信 main，candidate 不能自我启用。

旧证据没有 input manifest 时不得事后虚构闭包。选择性迁移在途任务：能从可信固定 planner 和真实执行范围完整重建者，记录迁移证据；不能证明者仅补缺失范围。迁移可能需要一次真实验证，但不能对每次 main 前进都重复。legacy 任务维持其原资格，未经明确迁移不静默改 epoch 或降低原严格要求。

旧 run 的 W 不支持新 admission/protocol 时，不反复 rerun 期待新行为；标记 `UNSUPPORTED_RUN_PROTOCOL`。产生新有效事件的过渡操作必须经现有授权流程，不能成为日常空提交／close-reopen 机制。

### 10.4 回滚

立即可用的 kill switch 是在可信策略中禁用 `input-scope-reuse/v1` 的新消费，而不是删 marker 或清掉失败记录。退回保守的受影响验证；保留有序发布、身份严格性、v2 readers、journal 和历史 artifact，避免回滚再次引入首轮竞态。任何旧 reader 看不懂新 envelope 时必须停止，不能 fallback 成假绿。

回滚不修改 H、不自动 rebase、不删除已发生远端副作用。策略撤销的范围、owner、原因和解除条件写入 Task evidence；在途 run 仍按自己的 W/B 完成，但消费资格由当前有效策略核验。

## 11. 验证设计与可追溯性

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [source-of-truth.md#proposed-ci-input-scoped-evidence-reuse-contract](source-of-truth.md#proposed-ci-input-scoped-evidence-reuse-contract) | [DES-CIR-01](ci-parallel-evidence-reuse.design.md#des-cir-01) | 复用策略仍为 Proposed/disabled，旧门禁继续控制至启用门槛全部满足 | [system-design traceability test](../../../scripts/system-design-traceability-check.test.py)：本地结构/精确引用检查；不执行 CI 能力 | `./scripts/doc-governance-check.sh` 输出、冻结设计 SHA-256 | 静态检查不证明 runtime 或 hosted 行为 |
| [source-of-truth.md#split-source-review-integration-contract](source-of-truth.md#split-source-review-integration-contract) | [DES-CIR-06](ci-parallel-evidence-reuse.design.md#des-cir-06) | 同 run/attempt 失败屏障不可由较早成功结果覆盖 | [receipt identity test](../../../scripts/pm/ci-ready-receipt.test.py)：本地 receipt 身份回归；扩展场景保留给 C4/V1 | C4 receipt 输出与 V1 R/A evidence | 现有入口不验证 proposed unit-level reuse |
| [ci-projection-publication.design.md#des-cip-01](ci-projection-publication.design.md#des-cip-01) | [DES-CIR-02](ci-parallel-evidence-reuse.design.md#des-cir-02) | PR 创建/更新发布先后顺序与候选身份必须闭合 | [prepare helper review-risk test](../../../scripts/pm/prepare-task-pr-review-risk.test.py)：本地 helper 风险回归；publisher 时序扩展属 C1 | C1 fake 时序记录和 V1 hosted 新建/续更证据 | 当前回归不证明新时序或真实 GitHub 原子性 |
| [ci-projection-publication.design.md#des-cip-04](ci-projection-publication.design.md#des-cip-04) | [DES-CIR-06](ci-parallel-evidence-reuse.design.md#des-cir-06) | attempt 证据闭合，禁止跨 attempt 混用 | [receipt identity test](../../../scripts/pm/ci-ready-receipt.test.py)：本地 receipt 身份回归；hosted R/A 对账属 C4/V1 | C4 same-attempt artifact/check receipt 和 V1 hosted run | 本地测试不证明平台 event/check 归属 |
| [../doc-governance/system-design-writing-standard.design.md#11-验证设计与可追溯性](../doc-governance/system-design-writing-standard.design.md#11-验证设计与可追溯性) | [DES-CIR-10](ci-parallel-evidence-reuse.design.md#des-cir-10) | 设计条款对需求映射且每项验证边界可追踪 | [doc governance test](../../../scripts/doc-governance-check.test.sh)：checker 负例/正例回归；本地文档命令实跑 | doc-governance output 与 QA independent review | 不替代 runtime、20-advance 或 hosted acceptance |

<a id="des-cir-10"></a>

### 11.2 DES-CIR-10：以乱序、并发、负例和 hosted 证明收口

每个测试的完整证据必须记录候选 H、实际 B/M/T/W、策略、R/A、命令、退出码、artifact 和未覆盖范围。fake 只证明状态机和身份逻辑，hosted 才证明真实事件、check 归属和 artifact 行为；source review 需由独立角色完成。

| 上游与设计映射 | 测试 ID／准确入口（新增部分均为 Proposed） | 场景与候选环境 | 预期与 evidence target | 未证明范围 |
| --- | --- | --- | --- | --- |
| DES-CIP-01 → CIR-02 | `pr-projection-publication.test.py::create_binding_delayed` | fake；CI 早于 record-pr，预算内完成 | 同 R 首轮继续；记录每次 API 读写和冻结身份 | fake 不证明 GitHub 时序 |
| DES-CIP-07 → CIR-02/08 | 同文件 `create_response_lost`、`record_pr_crash` | macOS/Linux publisher；各副作用后中断 | 同一 PR 恢复；无重复 POST，无伪完成 | 不证明跨主机锁 |
| DES-CIP-02 → CIR-03 | `pr-projection-resolver.test.py::stale_event_same_head` | event H1/body H0，live P(H1) 已发布 | 严格同 H/S/D 恢复；坏输入不能 fallback | 不证明新 main 已测试 |
| DES-CIP-02/06 → CIR-02/03 | 同文件 `wrong_task_or_pr`、`invalid_projection` | 错 UID/编号、重复 key、非法 base64、同 H 错 S | toolchain 前拒绝；无假绿 | 不替代专业语义审读 |
| 用户并行要求 → CIR-01/04 | `ci-evidence-applicability.test.py::twenty_unrelated_advances` | 固定 H／输入／策略，20 个无关 main commits | H、review dispatch、heavy run、task phase 均不变 | 不涵盖真实外部服务输入改变 |
| 同上 → CIR-04/05 | 同文件 `elevated_pr_unrelated_docs` | 高风险工程 PR 首次验证成功，main 修改无关文档 | 不因 elevated 重新派发严格集成 | 不对真实全局输入强行复用 |
| source identity → CIR-04 | 同文件 `dependency_edge_added`、`shared_consumer` | 新增依赖、共同消费者、两端改名、增加测试 | 旧新图并集识别相关；只补对应 units／roles | 不证明未注册单元可细粒度复用 |
| full-corpus 规范 → CIR-04 | 同文件 `corpus_completeness`、现有 doc checker suite 扩展 | 新增／删除文档、broken anchor、inventory 并发变化 | 覆盖集合完整；坏新增文档拒绝；不重编译无关 Rust | 不改变产品语义验收 |
| run identity → CIR-05 | `integration-selection-regression.test.py` 扩展 `workflow_base_diverge` | B 固定，dispatch 前 main 推进，W!=B | 兼容可信 executor 测 B/H；不改 H、不因 SHA 本身失败 | 不能接受被撤销 executor |
| manual entry → CIR-05/08 | 同文件 `dispatch_response_lost`、`same_input_request_dedup` | 丢 dispatch 响应、多次本地 resume | 同 request key 回读，无无限派发 | 不新增后台续跑 |
| attempt contract → CIR-06 | `ci-ready-receipt.test.py` 扩展 `attempt_mixing`、`latest_failure_barrier` | A1 成功，A2 pending/失败/产物缺失 | 不偷取 A1 假装 A2，通过资格精确按请求判断 | 不把 artifact 本身当结果 |
| review distinction → CIR-04/06 | `ci-evidence-applicability.test.py::late_role_return` | main 无关前进；另例 H1→H2 后 H1 review 晚到 | 第一例接受原角色结果；第二例不放行 H2 | 首版无跨 H 等价复用 |
| GitHub required checks → CIR-09 | `ci-parallel-reuse.integration.test.py` hosted profile | 草稿、同 H rerun、dispatch 补验、skipped 负例 | 平台 required-gate 真正阻断/允许；R/A/check SHA 可回读 | 本地伪 check 不作 hosted 证据 |
| merge boundary → CIR-07/09 | `ci-parallel-reuse.integration.test.py::target_moves_at_merge` | 合入前与 API 调用间 target 再变化 | 不宣称 CAS；回读真实合入基线并分类补验 | 普通直接合入仍有最后窗口风险 |
| policy authority → CIR-09 | `ci-evidence-applicability.test.py::candidate_self_authority`、`revoked_policy` | 恶意同名模块、候选降低 scope、撤销策略 | 候选不能授权自己；撤销只阻断受影响资格 | 不是完整沙箱安全证明 |
| compatibility → CIR-10 | `ci-parallel-reuse.integration.test.py::legacy_and_rollback` | legacy 无 manifest、旧 W、kill switch | 不虚构证据、不静默迁移、回滚不恢复竞态 | 不保证旧 run 能使用新代码 |

新增测试名称是实施合同，当前不存在的测试不得写成已通过。实施时将它们落位到表中指定文件或记录显式路径映射；不能仅以“相关测试通过”作为证据。QA 必须在真实候选上核验相互组合，而不仅是 mock 单元通过。

### 11.1 总体验收清单

只有在以下结果均可回读时，coordinating task 才能声明本次改造完成：正常创建首轮无需人为重跑；新 head 与 projection 严格匹配；20 次无关前进零重型重做；高风险 PR 也遵守输入复用；相关变更精确触发补验；所有负例无假绿；旧／新协议及回滚已测试；平台 required check 和权限配置已核实；真实合入回读与最终完成边界可恢复。

源代码合入、文档 checker 通过、单个 PR 绿灯，都不能替代上述组合验收。

## 12. 决策、长期风险与未决问题

### 12.1 关键决策

**ADR-1：默认输入范围复用，而不是追逐最新 SHA。** 收益是并行 PR 不互相重启；代价是必须维护可信输入闭包，不能随意声明不相关。Owner：repository health；失效触发：闭包不完整或单元语义变化。

**ADR-2：沿用已有 publication／receipt／Task truth。** 只增加版本化字段、pure evaluator 和内部恢复阶段，避免第二台账。Owner：TPM 与 repository health；失效触发：既有 parser 无法保留新字段，必须先修兼容，不另开旁路。

**ADR-3：高风险不等于每次 main 前进都全量验证。** elevated 控制首次强度，input_scoped 控制后续适用性；真实协议／全局策略输入变化仍需补验。Owner：专业 owner 与 QA。

**ADR-4：普通直接合入采用显式乐观边界。** 不为减少重复 CI 而承诺不存在的 target CAS；精确最终快照走单独的候选冻结或服务器串行集成。Owner：repository health／发行 owner。

### 12.2 启用阻断项与解除条件

| 风险／未核实点 | Owner | 解除条件／运行期处理 |
| --- | --- | --- |
| 分支保护／rulesets 的 strict 与合入方式未完整核实 | 仓库管理员 + repository health | live 配置回读及 hosted 检查；否则不启用普通模式 |
| 消费者／依赖图／全仓验证范围不完整 | planner owner + QA | 完整性测试；未知单元保守扩大，不按假闭包复用 |
| 老 artifact 没有输入清单或已经不可用 | receipt owner | 原始内容可验证地重建或一次补验；不能自填字段 |
| PR body 跨机器并发、人工同时修改 | publisher owner | 单 canonical writer；冲突有界停机回读，不承诺无损多写 |
| executor 或安全策略紧急撤销 | 对应 authority owner | live 资格核验，保留审计，阻断对应范围；不由旧 hash 绕过 |
| 合入最后窗口出现相关 target 变化 | repository health／发行 owner | 事后对账与针对性验证，阻断发行／最终完成；需要原子保证时采用另行验收的服务器路径 |
| 远程服务／动态依赖改变 | 各验证单元 owner | 独立环境／新鲜度合同，不能以 Git 未变代表稳定 |

### 12.3 最终边界

本设计不把正常等待伪装成测试通过，也不把无关 main 前进伪装成源码失败。它保留严格的身份、义务完整性、执行来源和权限检查，只改变失效的粒度和触发条件。

> **优化的目标不是更快地重复工作，而是让无关变化不再制造重复工作；真正相关的变化，只支付对应范围的验证成本。**

---

## 附录：审读来源与证据定位

下列仓库链接固定在本设计审读基线。代码显示实现事实，Proposed 设计显示待实施契约，二者不能互相替代。GitHub 文档于 2026-09-24 核对；上线前按实际平台版本复核。

仓库来源：[R1] PR 发布脚本；[R2] 集成执行脚本；[R3] 证据身份与复用逻辑；[R4] workflow source of truth；[R5] CI projection publication 设计；[R6] 系统设计写作规范。

平台来源：[G1] required check 排障与事件资格；[G2] check 状态；[G3] rerun 语义；[G4] strict/loose 分支保护；[G5] merge API 参数。

[R1]: https://github.com/eng-cc/oasis7/blob/539fa28860a5a531a9a9d00c8167e28071377af2/scripts/prepare-task-pr.sh#L2070-L2134 "PR 创建与 record-pr 时序"
[R2]: https://github.com/eng-cc/oasis7/blob/539fa28860a5a531a9a9d00c8167e28071377af2/scripts/pm/integration_ci.py#L85-L160 "集成身份、dispatch 与 verified_run"
[R3]: https://github.com/eng-cc/oasis7/blob/539fa28860a5a531a9a9d00c8167e28071377af2/scripts/pm/ci_ready_receipt_identity.py "source review / integration 身份及复用条件"
[R4]: https://github.com/eng-cc/oasis7/blob/539fa28860a5a531a9a9d00c8167e28071377af2/doc/engineering/workflow/source-of-truth.md "唯一规范性 workflow source"
[R5]: https://github.com/eng-cc/oasis7/blob/539fa28860a5a531a9a9d00c8167e28071377af2/doc/engineering/workflow/ci-projection-publication.design.md "既有 Proposed publication 设计"
[R6]: https://github.com/eng-cc/oasis7/blob/539fa28860a5a531a9a9d00c8167e28071377af2/doc/engineering/doc-governance/system-design-writing-standard.design.md "十二段系统设计规范"
[G1]: https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks "required check 事件资格与 head/merge SHA"
[G2]: https://docs.github.com/en/pull-requests/reference/status-checks "skip/neutral 的平台语义"
[G3]: https://docs.github.com/en/actions/how-tos/manage-workflow-runs/re-run-workflows-and-jobs "rerun 沿用原 SHA/ref"
[G4]: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches "strict 与 loose required checks"
[G5]: https://docs.github.com/en/rest/pulls/pulls#merge-a-pull-request "merge API 的 source-head sha 条件"
