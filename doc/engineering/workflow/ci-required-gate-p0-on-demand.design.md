# 合入前门禁 P0：公共基线瘦身、能力独立执行与影响闭包解耦

**设计 ID：OASIS7-CI-P0-SCOPE-1**<br>
**版本：1.0.0**<br>
**状态：S0 候选规范已写入 canonical source，等待 review/merge；实施、验证与启用待完成**<br>
**Owner：repository_health_engineer；独立验证：qa_engineer；协调：tpm**<br>
**模块：engineering/workflow、testing/ci**<br>
**审读日期：2026-09-24（Asia/Singapore）**<br>
**固定源码基线：`eng-cc/oasis7@9d404f6ea863e0ec11aed0e8c6beb759189ce5fe`**<br>
**诊断样本基线：`55e59b0b68a2843b873ab6c0a626ab2e0a88394f` 及相应 PR source head**<br>
**S0 bootstrap HEAD：`a7bdf5ff5e934e04e5261e2a9518297ba475fb03`（前序规范 v1.20.0；代码事实仍固定于上方基线）**<br>
**建议落位：`doc/engineering/workflow/ci-required-gate-p0-on-demand.design.md`**

本文承接用户要求“把 P0 项，做成设计文档给我”，其中 P0 是上一轮诊断明确提出的三项：拆除公共基线中的无条件重自测；拆开 workflow、packaging、operational 三类能力的执行开关；分离影响范围闭包与 CI/复审就绪状态。本文不宣称这些改动已经发生，也不创建、绑定或修改任何 GitHub Task/PR。

P0-R1–R5 的规范性验收以 [required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) 为准；本设计展开其接口、迁移和验证映射，不构成第二套规范。代码实现仍须消费独立合入的 S0 规范版本，并经过现有任务、评审、CI 与授权启用流程；本文不表示当前 runner 已达到目标。

---

## 1. 问题、目标与非目标

### 1.1 问题

当前规划层与执行层不一致。`scripts/ci-tests.sh` 的 `run_required_gate_checks()` 直接执行文档校验器自测、测试网 clean-room／identity 工具自测、Cargo worktree 自测等；部分调用在 capability 开关之前。另一方面，planner 把 `workflow_governance`、`packaging_contracts` 和 `operational_contracts` 都映射成 `run_operational_contracts`。[S2][S3]

所以，`scope=minimal` 并不代表只执行轻量普适检查；`scope=targeted` 也不代表只执行命中的专业域。PR #3955 的 Run `35979151984`、attempt 2 中，`required-gate` 为 21 分 26 秒，其中 `Run required test tier` 为 21 分 01 秒；工具链、Node、Trunk 和系统依赖的安装步骤都被跳过。这个样本主要慢在实际执行的测试集合，而不是安装或排队。[E1]

另一个问题是：`closure_status` 被部分输入用于表达“CI/复审还在等待”，而投影构建器会将未完成的闭包升级为 full。这里需要纠正输入语义及其生产链路，而不能直接删除安全兜底。[S4]

### 1.2 成功结果

本次交付只解决以下问题：

| 要求 | 必须得到的结果 |
| --- | --- |
| P0-R1：基线瘦身 | 普通文档变更不运行测试网／身份／打包／Cargo fixture 等无关自测；保留现行规范要求的实际文档校验和普适准入检查。 |
| P0-R2：能力分离 | 纯 workflow 变更不因开关别名而运行 packaging 或 operational；多域变更执行显式并集。 |
| P0-R3：状态解耦 | 已查清影响范围但 CI／复审仍 pending 时，测试选择不因此扩大；真实范围未知时仍保守扩大或阻断。 |
| P0-R4：覆盖不丢失 | 被移出公共基线的测试仍在自身实现、规则、fixture 或已登记共享依赖变化时运行，并保留在适用 full 覆盖中。 |
| P0-R5：所有入口一致 | 本地已规划 required、PR CI、可信 integration_revalidation 对同一有效计划执行一致的能力集合；旧入口不因缺新字段而少跑。 |

### 1.3 非目标

不实现按单测试项的通用执行清单／动态 DAG，不实现跨 run 或跨 main 前进的证据复用，不新增服务、数据库、GitHub App、全局开发锁、自动 loop 或后台任务；不迁移构建系统、不拆业务 crate、不优化测试网套件内部算法。

不取消 `required-gate`，不修改 required check 名称、身份绑定和既有角色评审，不放宽严格集成的触发条件，不通过把 `verification_affected` 改为 false 来提速。`main` 前进导致的重复工作和 CI 发布竞态仍由各自方案处理。

本方案不改变游戏规则、链上规则、实际测试网部署或密钥管理。涉及 live/provider/hosted-account 的显式授权边界保持不变。

## 2. 上游约束与相关角色

<a id="p0-requirements"></a>
### 2.1 需求承接与分配表

用户请求及上一轮 P0 定义是本设计的工程需求输入，不是已经发布的机器合同。实施协调任务必须原文冻结该输入、本设计固定版本、下列必要集合及候选选择规则，再建立真实的 Task/PR/evidence 引用；本文不虚构 Issue 号、Task UID 或合同 revision。

| 上游 requirement／professional acceptance | 具体义务与适用条件 | 本设计条款 | 外部 owner／dependency | 排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) | P0-R1：保留实际文档检查与普适准入，把文档 checker 自测移入按需能力 | [公共基线](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-01) | repository health、QA；依赖完整调用 inventory | 不把产品 full-corpus 移到 nightly。 |
| [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) | P0-R2：workflow、packaging、operational 与新工具自测能力独立选择；共享依赖走显式并集 | [能力拆分](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-02) | planner、runner、workflow、receipt；operational domain review | 不提供 test-case 级 DAG。 |
| [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) | P0-R3：范围闭包只表达影响分析，不吸收 CI/review readiness | [影响闭包](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-03) | projection 输入生产者、PM readiness consumer | 不豁免 CI、复审或高风险集成。 |
| [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) | P0-R5：新旧版本、字段完整性、资源需求和 receipt identity 一致 | [接口与版本](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-04) | CI receipt 与本地/可信入口 | 不改 projection v2 或历史 receipt。 |
| [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) | P0-R1、R4：真实 dispatcher、full superset、资源隔离及变异负例证明选择行为 | [执行行为验证](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-05) | QA；完整搬移 inventory | 不以 selector 文本或单个 fixture 代替 hosted 验收。 |
| [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) | P0-R4、R5：先 consumer、兼容 legacy，再由同树组合验证受控启用 | [迁移与回滚](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-06) | repository health、QA、TPM；按序交付 | 不声明当前实现已满足目标。 |
| [Manual three-loop transition](./source-of-truth.md#manual-three-loop-transition) | 保留 source、scope、integration、run/attempt 身份和手动授权边界 | [接口与版本](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-04) | CI receipt、PR lifecycle、现行 effective authority | 不启用尚未启用的 loop／复用能力。 |
| [System-design writing standard](../doc-governance/system-design-writing-standard.design.md#11-验证设计与可追溯性) | 每项设计义务映射到准确测试入口、候选、环境及 evidence target | [执行行为验证](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-05) | QA；真实 Task/PR 实施证据 | 文档检查通过不等于实现或 hosted 验收通过。 |

以上仓库路径的 canonical repository 均为 `eng-cc/oasis7`。这是纯工程变更，不改变玩家承诺或产品 AC；实施 trace 应使用 `professional_acceptance`。正式 N/A 处置、review locator 和责任人由实际任务按现行规范记录，不能由本文中的说明代替。[S1][S7]

### 2.2 角色与权限

`repository_health_engineer` 负责规范投影、planner／runner 合同和兼容性；`qa_engineer` 独立验证没有漏跑、越域运行和假绿；`blockchain_ops_engineer` 审读运维、身份工具及 live 边界的迁移归属；`tpm` 冻结需求与组合验收集合，维护既有任务链。角色名称不自行授予生产操作权限。

## 3. 当前状态、目标状态与差距

| 对象 | 当前基线事实 | 目标状态 | 主要差距 |
| --- | --- | --- | --- |
| 公共基线 | 包含实际校验，也包含无条件工具自测。[S2] | 只保留实际普适校验和准入；自测归入相关能力。 | 未区分“使用校验器”与“回归测试校验器”。 |
| 三类非 Rust 能力 | 共用一个 planner output；operational 函数内部嵌套 packaging 及 PM 测试。[S3][S5] | 独立输出、独立函数、独立 workflow 条件。 | 分类在执行层重新合并。 |
| 文档校验 | 规范要求产品 full-corpus 与系统设计 changed-scope。[S1] | 原语义保留。 | 不能靠取消必要文档校验达到时间目标。 |
| 闭包 | 未 complete 或证据不足会触发 full；v2 输入／投影采用严格字段检查。[S4] | `closure_status` 仅表达影响范围是否完成分析。 | 输入、模板、指令和就绪状态需要解耦；不是只改 planner。 |
| receipt | `RUN_FIELDS` 固定列出当前开关并纳入 canonical planner。[S6] | 新增开关及新解释版本完整参与验收和摘要。 | 只改 YAML 会留下证据丢字段风险。 |
| 资源准备 | 当前 Rust 需求推导包含“capabilities 减非 Rust 集合”的逻辑。[S3] | 新的文档自测能力不得误触发 Rust；Cargo fixture 依赖必须显式。 | 新增 capability 会影响工具链判定。 |

现状与目标不能互相替代。代码事实仍固定读取 `9d404f6ea863e0ec11aed0e8c6beb759189ce5fe`；#3967 在 `a7bdf5ff5e934e04e5261e2a9518297ba475fb03` 上启动，source-of-truth 为 v1.20.0。对 planner、runner、workflow、local renderer 与 receipt/projection consumer 的路径级比较未发现这些代码文件在两提交间变化；后续代码任务仍须按其冻结 source head 重新核对并绑定 S0 合入后的规范 commit，不能把旧设计快照当作默示实施基线。

## 4. 边界与结构

目标沿用现有链路：

```text
可信源码范围 + 有效规则 + 独立影响范围分析
    -> 现有 planner / workflow impact projection
    -> 显式 capability outputs + 解释版本
    -> 普适校验 + 命中的能力函数
    -> 现有 required-gate / artifact / CI receipt

CI 与角色复审的实时状态
    -> 现有 readiness / promotion / merge gate
    （不反向写成“影响范围未知”）
```

PR body、任务作者输入、环境变量均不能独立成为减跑 authority。减跑必须由当前有效的可信 planner、规则和输入身份共同决定；候选规则修改不能自批准。[S1]

<a id="des-ci-p0-01"></a>
### 4.1 DES-CI-P0-01：公共基线只保留普适实际校验

继续保留 `run_product_doc_governance_check()` 的有效行为：产品 changed-range、产品 full-corpus、系统设计 changed-scope。继续执行 `lint-skills.sh`、`check-windows-paths.sh`、`check-script-executable-bits.sh`、`check-rust-file-size.sh` 等普适实际校验，以及有效投影消费验证、可信 Cargo 范围准入和已激活 profile 的完成检查。

“只保留实际校验”不表示省略 checker-stage 的强制 admission；checker-stage 被选中时，其前置测试和 receipt 顺序仍须保留。只是把通用 checker 自测从每个文档 PR 的无条件路径移走。

公共基线不得无条件调用测试网／身份工具自测、Cargo fixture、打包自测或整个 PM 回归包。搬移清单必须逐项记录原入口、原调用、归属能力、触发依赖和 full 归属；未完成归属的调用不得直接删除。

### 4.2 测试归属

| 新归属／沿用能力 | 迁入或保留的代表性执行内容 | 选择原则 |
| --- | --- | --- |
| `required_gate_baseline` | 上述实际文档与普适校验、身份与准入检查；不放领域回归包 | 普通 required 恒定执行。 |
| 新增 `doc_checker_contracts` | `product-doc-governance-check.test.py`、`product-doc-content-check.test.py`、`system-design-traceability-check.test.py`、`product-doc-content-callers.test.sh`、`doc-governance-check.test.sh` | 校验器、规则、Markdown parser、相关 fixture、调用接口或影响其行为的规范改变时运行。 |
| 新增 `cargo_tooling_contracts` | `cargo-dev-windows-toolchain.test.sh`、`cargo-dev-worktree-isolation.test.sh`、`pm/new-task-worktree-cargo-cache-migration.test.sh`、`cargo-dev-lib.test.sh`、相关 Rust 工具检查器自测 | Cargo 开发包装器、worktree 缓存、相关 toolchain／配置／fixture 变化时运行；不是每个文档或 Rust 源文件变更都运行。 |
| `workflow_governance` | 从 operational 提取现有 PM／loop／CI receipt 回归；迁入 planner、projection、checker-stage、CI 调用合同、skills 治理等相关自测 | PM 代码、执行指令、scope 规则、对应规范／fixture 变化。 |
| `packaging_contracts` | 现有 `run_packaging_contract_tests()`、发布 Trunk 缓存合同、归属于产物交付的其他自测 | 包结构、打包／交付脚本及其声明依赖变化。 |
| `operational_contracts` | clean-room、clean-room-adapter、identity signing、CLI bridge、evidence aggregate、peer registry；现有部署／升级／回滚／离线 provider 合同测试 | 运维、身份／信任格式及其真实共享输入变化。 |
| 既有 viewer／compile metrics 等能力 | 保持相应业务和工具合同；将其无条件调用放回所属能力 | 不扩大本次 P0 为全部套件重新分类。 |

表中脚本名是当前基线已有入口，不表示新 capability 已存在。完整搬移 inventory 是代码叶子的必交付测试数据，不是另一套生产 registry。

真实产品文档只是被检查对象时，不因“检查器会读取它”就触发检查器完整自测；专门作为测试 fixture、规则定义或解析契约输入时则必须触发。无法判明的依赖不得靠扩展名放行。

<a id="des-ci-p0-02"></a>
### 4.3 DES-CI-P0-02：能力输出一对一，不再共用别名

| Capability | planner output | `ci-tests.sh` 输入 | 执行函数 |
| --- | --- | --- | --- |
| `workflow_governance` | 新增 `run_workflow_governance_contracts` | 新增 `OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS` | `run_workflow_governance_contract_tests` |
| `packaging_contracts` | 新增 `run_packaging_contracts` | 新增 `OASIS7_CI_RUN_PACKAGING_CONTRACTS` | 现有 `run_packaging_contract_tests` |
| `operational_contracts` | 保留 `run_operational_contracts`，新模式下仅代表运维域 | `OASIS7_CI_RUN_OPERATIONAL_CONTRACTS` | 拆分后的 `run_operational_contract_tests` |
| `doc_checker_contracts` | 新增 `run_doc_checker_contracts` | 新增 `OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS` | `run_doc_checker_contract_tests` |
| `cargo_tooling_contracts` | 新增 `run_cargo_tooling_contracts` | 新增 `OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS` | `run_cargo_tooling_contract_tests` |

拆分后的 operational 函数不得再嵌套 packaging 或 PM 回归函数。每个能力仅由一次顶层 dispatcher 调用；共享依赖通过规则选择多个能力，而不是在能力函数中隐式调用整个相邻域。

新增 capability 必须同步 `CAPABILITIES`、`FIELDS`、`selector_ownership`、资源需求推导、workflow env／output、artifact 和 receipt 消费。不能只增加 YAML 中的 `if`。

### 4.4 规则优先级和依赖处理

继续使用 `scripts/ci-required-scope.v2.json` 作为选择规则来源。新模式规则优先级为：明确全范围触发 > 显式能力并集 > 普通 minimal 分类；未知路径沿用 full 兜底，身份错误不靠 full 掩盖。

当前实现中 `minimal` 命中会过滤 `workflow_governance`。因此，新增 `.agents/**` 或专业规范的精确治理规则时，必须同步修复这个抑制逻辑：宽泛 `*.md`／`doc/**`／`.agents/**` 的 minimal 规则不得覆盖更精确的执行合同或共享依赖命中。[S3]

P0 不把全部治理文档升级成 workflow 回归。仅将真正定义 executable workflow、checker 行为、信任规则的精确路径及其声明依赖登记为相应能力。普通产品文本仍走实际文档校验。

`rust.yml`、中央 dispatcher、planner、规则配置等现有明确 shared-required/full 触发，不在本次为提速而降级；它们涉及本次改造的安全边界。新模块、未知共享依赖和已知风险升级仍可扩大范围。

## 5. 关键运行流程

### 5.1 普通已规划 required

有效入口先完成 Task/PR/source/scope 与 projection 身份校验，再由可信 planner 生成全部开关和资源需求；在安装工具或执行测试之前校验输出完整性。随后执行普适基线和命中的能力函数，任何必需项失败都使 `required-gate` 失败，receipt 仅在原有完整成功条件满足时签发。

对文档样本，预期是 `required_gate_baseline`，不运行 doc checker 自测、P2P/identity、packaging 或 Cargo fixture。对纯 PM 实现样本，预期是普适基线加 `workflow_governance`；不连带执行相邻专业域。若变更命中共享依赖，则明确执行能力并集并打印各自原因。

### 5.2 本地与可信集成入口

`prepare-task-pr.sh` 的本地计划渲染必须传递全部新字段；`ci-tests.sh required --impact-projection` 不能被误当成“参数存在就自动按需”，必须使用可信 planner 结果或由统一入口完成规划与显式传参。

可信 `integration_revalidation` 继续使用可信 default workflow 和已冻结的 driver／planner，而不是候选脚本自选范围。保留 source-scope 与 integration-base 的区分；存在相关 target-only 变化时，集成执行可以是源范围能力的安全超集。P0 不用单纯 source diff 替代当前集成语义，也不承诺 main 前进后必然复用。

### 5.3 子 Job 路由

保留单个 `required-gate` 的名称及现行 required-check 关系；本次不拆全新的并行 DAG。已经存在的子 Job 改为消费真正对应的能力输出：fleet-health 与 Windows rollout behavior 归 operational；macOS package contract 归 packaging。若某个具体检查同时验证包与部署合同，须登记两域依赖并显式 OR，不能继续引用历史总开关。

子 Job 是否属于实际放行所需集合，仍由现有 receipt/lifecycle authority 负责。迁移验收必须验证应选 Job 失败／取消／缺失时不能签发成功；本设计不把父 Job 的成功冒充完整证据。

<a id="des-ci-p0-03"></a>
### 5.4 DES-CI-P0-03：影响范围与验证就绪分开

`closure_status` 在新生产契约中仅回答“影响范围是否完成分析并有可核对依据”，不回答“本次 CI 或专业复审是否成功”。

| 范围分析 | CI／角色复审 | 测试选择 | 晋级／合入 |
| --- | --- | --- | --- |
| complete 且依据有效 | pending／running | 按实际路径、合同及风险选择；不因等待扩大 | 等待已有就绪门禁，不能通过。 |
| complete 且依据有效 | failed／changes requested | 不因结果失败改写范围；按已有授权修复／重跑 | 阻断。 |
| complete 且依据有效 | 全部必要项通过 | 按原范围 | 仍检查其他 live admission、hold 和严格集成条件。 |
| pending／unknown，真实范围尚未查清 | 任意 | 保留 full 兜底；严重不确定性可以阻断 | 不因旧绿灯而通过。 |
| 任意 | 身份错、证据摘要错或 authority 无法确认 | 阻断；不把错误转换成“全跑就算可信” | 阻断。 |

**不新增 projection v3。** `oasis7-workflow-impact-projection/v2` 保留现有字段集和 `closure_status={status,reason,evidence}` 结构；CI 与复审状态继续由现有 PM／receipt／lifecycle 记录和实时回读提供，不塞进投影的新字段。[S4]

新生产者应使用独立的 scope-analysis 构造路径：仅根据冻结 source 范围、消费合同、受影响消费者、共享输入、对应验证义务和有效依据生成 `closure_status`。该路径不得读取 CI conclusion 或 review pending 来生成 status。`reason` 是解释文本，不能通过关键字匹配来推断 complete。

现有 `build_projection()` 对真实未闭合范围的 full 兜底保留。主要修复对象是所有投影输入的生产模板、任务指令、生成入口和错误耦合的调用；只改 planner 的 `if not closure_verified` 不算完成。

complete 需要同时满足：变更路径已由 Git 核对；消费合同、相关消费者与已知共享输入已有明确归属；验证义务能够被所选能力承接；依据可回读且绑定冻结源码或有效合同。非空文件 hash 只证明引用内容一致，不能单独证明依赖分析完整，也不能证明测试通过。

无需为闭包证明新增一个会违反单 loop 范围的证据文件。可以引用现有合同／测试源码等输入，实际分析与独立确认留在既有任务 evidence；无有效依据时仍保持 pending／unknown。

### 5.5 老错误输入的处理

历史投影中 `reason="Exact-head CI and role review pending"` 不能被字符串替换或批量重写成 complete。对受影响任务先补做真实范围分析，再经现有授权流程发布新的完整投影；重新计算摘要并更新对应绑定。旧投影和旧证据保留原样。

仅 readiness 从 pending 变成 passed、源范围与范围依据未变时，不重新生成投影。`verification_affected`、角色列表和高风险标记不因本次提速而弱化。

## 6. 接口与数据合同

<a id="des-ci-p0-04"></a>
### 6.1 DES-CI-P0-04：显式版本、完整布尔量与证据字段

由于 `run_operational_contracts` 的语义从“混合总包”变为“运维域”，不得无版本地重解释历史结果。在既有规则配置和 planner 输出增加 `execution_contract="required-domain-split/v1"`；该值必须来自有效可信配置，而不是用户传入的 opt-out 开关。

projection v2 结构不变。既有 `oasis7-required-plan-v1` artifact 的 planner 元数据增加该解释版本及四个新布尔输出；这是有显式版本的增量合同。receipt 的 canonicalization 在新版本中纳入全部新字段和解释版本，旧版本保留原摘要规则，不重写历史 receipt。

| 接口 | producer → consumer | 关键新增／变化 | 错误与兼容 |
| --- | --- | --- | --- |
| Scope 配置 | 有效规则 → planner | 扩展 capability 列表、selector ownership；新解释版本 | 旧配置按旧能力集合解读；未知解释版本拒绝。 |
| Planner outputs | planner → local renderer／workflow／runner | 四个新布尔量；operational 新语义；解释版本 | 新模式缺任何必需值、非 true/false、重复或矛盾字段都在测试前拒绝。 |
| 资源需求 | planner → tool setup | doc checker 为 Python／Markdown；Cargo tooling 显式需要 Rust；其他保留真实需求 | 新 capability 不得落入“未知就装 Rust”的负集陷阱。 |
| Plan artifact | workflow → receipt | 完整新 outputs 和解释版本，连同既有身份、config digest | 不能仅依赖 `selected_capabilities` 文字。 |
| Canonical receipt | CI reader → readiness | 版本化 RUN_FIELDS；新字段完整纳入 authority digest | 不把缺失字段当 false，不用旧绿证明新合同。 |
| Impact input | 独立范围分析 → projection | closure_status 只表示范围，wire shape 保持 v2 | live 验证状态不进入范围函数输入。 |

新契约下 `should_run_ci_required_component()` 不得利用空字符串默认开启来掩盖漏传；在 dispatcher 之前一次性校验所有 planner-owned fields。manual-only 项保持既有显式授权和默认关闭规则，不能因 full 自动启用。

### 6.2 Legacy／planned／full 三类模式

| 模式 | 行为 |
| --- | --- |
| 新 `required-domain-split/v1` 已规划模式 | 所有必需开关显式、完整；执行独立能力函数。 |
| 无新解释版本的旧 required 入口 | 保留旧语义和原固定覆盖，必要时走旧覆盖组合；不得因搬移而静默漏掉原来必跑的测试。日志标明 legacy，而不是宣称 minimal。 |
| `full`／`full-core` 等既有显式模式 | 按各模式原职责保留 full superset；不得继承残留环境 false 而漏掉迁出的必要套件。manual-only 仍不默认开启。 |

为了避免“直接执行 required 反而少跑”，dispatcher 必须先确定模式，再应用开关；不能通过初始化四个新变量为 false 来兼容所有调用者。`commit` 和分片模式同样需要回归，虽然本次不以改变它们的覆盖为目标。

### 6.3 资源、信任和比较范围

source head、source scope base、integration base、tested tree、workflow ref/SHA、run/attempt、check app/id 保持独立并按原规则核对。新解释版本不是替代这些身份的授权凭证。

普通文档已规划路径不得通过 fixture 隐式触发 rustup 安装或 Rust 构建。实际普适准入所需资源也必须纳入资源审计；no-toolchain fixture 未通过时不能宣称该路径已实现无需 Rust，不能用 runner 预装工具掩盖依赖泄漏。

## 7. 状态、事务与持久化

范围分析状态与执行状态使用两个独立维度。范围分析可以在测试未开始前 complete；测试成功不能反推范围已查清。源码、相关合同或范围依据改变时，按现有身份合同重新分析；仅 CI／复审状态变化不得改写范围记录。

本次不创建 PM 新生命周期、不创建数据库。计划和 receipt 继续走现有 GitHub artifact／任务证据链。新 producer 写本地产物时先完成全部校验，再临时文件写入并原子替换；写失败不得留下可被误读的半份计划。远程发布沿用现有 pending intent／readback，不宣称跨 API 原子性。

取消或失败后的恢复保持原 run/attempt 语义。日志或部分成功不能形成通过 receipt；不得选择较早绿色结果掩盖适用范围内的新失败或未知状态。本次不新增自动重试、后台续跑或任意证据复用。

## 8. 部署、安全与运行约束

实现只使用现有本地客户端、仓库 Python/Bash、GitHub Actions、Issue/Project 和本地已授权 gh。不得新增 token、Secret、远程服务或生产网络调用；测试网工具自测继续使用其原有隔离 fixture，真实部署不因重新分类获得授权。

可信集成复制的 driver 必须带齐依赖。P0 优先把拆出的函数保留在现有 `ci-tests.sh` 中，避免增加复制遗漏面；后续若确需抽脚本文件，必须同时更新 trusted staging、本地打包和负例测试，不能退回候选工作树随意 source。

新模式的基线和 dispatcher 边界必须可审计。日志至少输出解释版本、scope、selected capabilities、每组 selected/skipped 原因和开始／结束耗时；这只是现有日志的有界增强，不是 P1 的通用测试结果平台。

P0 验证不依赖生产网络，不以重新设置 `GH_TOKEN`、放宽 check app 身份或管理员绕过换取绿色结果。新配置的普通代码变更仍经过当前可信门禁；首次改造 PR 不允许使用自己的新减跑规则证明自己。

## 9. 质量与容量

以下为拟定验收目标，不是已经达到的测量值；S0 需由 owner 与 QA 冻结。

| 场景 | 环境／刺激 | 预期响应 | 指标与判定 | 验证入口 |
| --- | --- | --- | --- | --- |
| 普通文档 minimal | 固定候选；记录 tracked 文件数、产品文档数、runner image；无相关 checker／共享输入变化 | 保留实际文档与普适检查，不运行领域重自测 | P2P／identity／packaging／Cargo fixture 次数为 0；至少 10 次成功 hosted 样本，Job P50 ≤5 分钟、P95 ≤8 分钟；可比样本中位数较旧链下降至少 70% | 测试日志与 GitHub Job 时间；PR 与集成分别统计。 |
| 纯 PM targeted | 已闭合范围，只改 PM 实现／相应 tests，不触及 shared full 触发 | workflow 回归被选中，非相关域不运行 | packaging、operational、Cargo fixture 无关调用为 0；域内耗时单独报告，不预先承诺未经测量的分钟上限 | planner + 真实 dispatcher 捕获 + hosted。 |
| 运维或打包变更 | 各自实现及真实共享依赖样本 | 所需域不能漏跑 | 选择集合符合冻结 dependency matrix；新失败必须阻断 | 正反例及适用 OS Job。 |
| 未知依赖／坏输入 | 未分类路径、坏 digest、缺 selector | 不出现假绿 | 未分类走安全 full；身份／证据错误明确失败；不能按 minimal 继续 | planner、runner、receipt 负例。 |

时间统计分离 queue、setup、actual checks、自测、cleanup；失败／取消样本单独报告，不能删除失败记录来满足时延目标。若必要 full-corpus 成为剩余瓶颈，另立授权优化，不以减少覆盖应付本目标。

## 10. 兼容、迁移与回滚

<a id="des-ci-p0-06"></a>
### 10.1 DES-CI-P0-06：先消费者、后生产者、最后启用

当前 `ci-ready-receipt.py` 会固定读取 RUN_FIELDS；投影还严格校验字段与 planner config digest。迁移必须覆盖整条链，而不能期待字段被静默忽略就是兼容。[S4][S6]

| 交付 | 范围 | 可并行性与放行条件 |
| --- | --- | --- |
| S0：规范及设计 | 先更新 source of truth；同步本设计、测试覆盖／执行文档和准确上游锚点 | 单独 system 文档任务。冻结验收集合；不改变运行行为。 |
| C1：兼容读取与合同测试 | planner config 解释版本、artifact/receipt canonicalization、旧数据 fixture、所有必需消费者 | 生产默认仍是 legacy；新解释版本未发布。 |
| C2：执行层拆包 | `ci-tests.sh` 函数归属、dispatcher、旧模式覆盖组合、workflow 新字段接线与子 Job 路由 | 消费 C1 固定合同；可与 C3 在文件边界清楚时并行。 |
| C3：规划与生产者修复 | planner、规则、完整本地参数渲染、独立 scope-analysis 生产链及全部模板／执行指令 | 新规则先在 fixture 验证，不自动改变默认 authority；与 C2 共同进入组合候选。 |
| C4：组合验证及受控启用 | 在同一 integration/tested-tree 上验证 C1–C3；通过后以正常授权改动发布可信配置中的新解释版本 | 不增加全仓暂停；按既有 effective authority／任务资格启用，记录真实候选和证据。 |

C2／C3 的文件修改归属必须在协调记录中确定，例如 `prepare-task-pr.sh` 归 C3，`rust.yml` 归 C2；遇到共享文件就显式协调，不用更多小 PR 掩盖接口未一致。

最终代码叶子不得混入未经规范采纳的行为修改；代码叶子消费已独立审核并合入的规范版本，遵守既有 code-only projection 例外条件。[S1]

### 10.2 在途任务与旧投影

新 reader 兼容“读取旧证据”不等于旧证据能证明新执行合同。旧 artifact 无新解释版本且无四个新字段时，按旧规则验证；新版本缺字段、旧格式夹带部分新字段、未知版本都拒绝。

旧投影的 planner config digest 不得被改写或跳过校验。仍有资格使用旧可信执行链的在途任务可继续旧链；进入新链需要现有流程下的显式定向迁移和新投影。当前入口不支持旧链或身份不匹配时，返回明确的 migration-required 阻断及恢复入口，不做“猜测补字段”。

不承诺这次合同升级使所有在途任务零重跑。迁移只针对受影响合同／任务，不批量 rebase 源码、不把普通 main 前进当成再次升级；更细的无重跑证据复用属于独立方案。

### 10.3 回滚

未启用时回滚 C2／C3 不影响 legacy 语义；已启用时优先在可信配置中停止对后续任务采用新版本，并使用兼容 reader 完成精确的旧／新证据处置。不得把新 receipt 降格成旧 receipt 或抹去已经执行的结果。

确认存在漏跑、领域开关串线、错误 complete 或假绿时，阻断受影响新合同，恢复已知旧覆盖（基线快照为本文固定源码基线或其经核对的完整可信版本），重新验证受影响候选。回滚代码本身不撤销已经发生的合入，相关影响由实际任务 evidence 和正常修复流程跟踪。

## 11. 验证设计与可追溯性

<a id="des-ci-p0-05"></a>
### 11.1 验证映射表

| 上游 requirement／professional acceptance | 本设计条款 | 独立义务与适用条件 | 准确验证 source 与边界 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) | [公共基线](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-01) | 普通文档仍只运行实际文档检查与普适准入；checker 自测按相关输入触发。 | [ci-tests-full-superset-contract.test.sh](../../../scripts/ci-tests-full-superset-contract.test.sh)：扩展为 P0-T01/T02，覆盖普通文档与 checker inputs。 | S0 记录规范/文档验证；C2 保存调用 inventory、dispatcher 日志与 negative cases。 | 当前测试未证明新 runner 选择行为或 hosted timing。 |
| [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) | [能力拆分](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-02) | 选择集合须一对一传过 planner、workflow、runner、子 Job 和 receipt。 | [plan-rust-required-scope.test.sh](../../../scripts/plan-rust-required-scope.test.sh)：扩展为 P0-T03/T05；真 dispatcher 与 hosted route 按 P0-T16 验证。 | planner selection matrix、command-capture logs、Job status and receipt identity。 | 当前测试未证明拆分后的能力，也不替代 operational review。 |
| [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) | [影响闭包](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-03) | scope complete 不随 CI/review readiness 变化；真 unknown 保留 full。 | [workflow-impact-projection.test.py](../../../scripts/pm/workflow-impact-projection.test.py)：扩展为 P0-T10/T11；producer fixture 按 P0-T12 建立。 | projection digests、scope analysis inputs、readiness consumer negative cases。 | 当前 producer inventory 与 new closure cases remain pending。 |
| [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) | [接口与版本](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-04) | 新合同字段在测试前拒绝缺失/混合输入；旧 receipt 保持旧摘要语义。 | [ci-ready-receipt.test.py](../../../scripts/pm/ci-ready-receipt.test.py)：扩展为 P0-T07/T09/T14。 | old/new/mixed fixture results and canonical digest evidence. | 当前 reader 未实现 new execution contract. |
| [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) | [执行行为验证](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-05) | 实际 dispatcher、工具需求、full superset 和变异行为须有可观察证明。 | [ci-tests-argument-contract.test.sh](../../../scripts/ci-tests-argument-contract.test.sh)：扩展为 P0-T07/T08；新 capture suites 为 P0-T01/T03/T17。 | selected/skipped reason, invoked commands, exit status, no-network/toolchain evidence. | S0 不实现或运行新增的 dispatcher fixtures。 |
| [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split) | [迁移与回滚](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-06) | 同一 tested tree 验证全部交付，legacy 和新版不得混合解释或自启用。 | [integration-ci.test.py](../../../scripts/pm/integration-ci.test.py)：扩展为 P0-T15；hosted samples 按 P0-T20。 | source/base/tested-tree/run/attempt identities, hosted matrix logs, old/new coverage comparison. | 文档交付不证明 code compatibility, hosted outcomes, performance, or activation. |
| [Manual three-loop transition](./source-of-truth.md#manual-three-loop-transition) | [接口与版本](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-04) | 保留现有 source/integration/run identity 与 readiness 独立边界。 | [ci-ready-receipt.test.py](../../../scripts/pm/ci-ready-receipt.test.py) 是既有 receipt identity regression；P0-T13/T14 增补新字段情形。 | 当前 readiness 条件不被 closure 状态替代；后续记录 legacy/new receipt fixture。 | S0 不验证新 receipt reader、实时 GitHub readback 或 hosted result。 |
| [System-design writing standard](../doc-governance/system-design-writing-standard.design.md#11-验证设计与可追溯性) | [执行行为验证](./ci-required-gate-p0-on-demand.design.md#des-ci-p0-05) | 每个 P0 obligation 都必须有明确 test source、scenario 与 evidence target。 | [system-design-traceability-check.test.py](../../../scripts/system-design-traceability-check.test.py) 与本任务 `./scripts/doc-governance-check.sh`。 | 本设计的 demand/validation relations 与 checker output。 | 文档结构通过不证明业务语义、runner 或 hosted 行为。 |

<a id="des-ci-p0-05-execution"></a>
### DES-CI-P0-05：验收执行行为，而不是只验 selector 文本

P0 不实施 P1 的生产测试项 registry，但必须补两类验收：公共基线的有界调用清单，以及执行真实 dispatcher 的命令捕获 fixture。fixture 使用临时目录、替身命令和禁网环境，不能只 mock planner 的返回字符串；例如把 cargo／rustup／npm 设为记录并失败的替身，验证文档路径不会隐式调用它们。

针对无条件重测试的回归，必须加入变异负例：向公共基线重新插入 clean-room-adapter 自测，或者重新让 workflow 映射到 operational，测试必须失败。只通过 grep 检查变量名、只验证新字段存在，不构成 P0 完成。

下面列出的新增测试文件与用例 ID 是拟实施入口，尚不存在或尚未验证；不得把路径存在声明当作通过证据。

| 验证 ID | 上游／设计义务 | 精确入口与场景 | 判定／证据目标 |
| --- | --- | --- | --- |
| P0-T01 | P0-R1；DES-CI-P0-01 | 新增 `scripts/ci-required-baseline-routing.test.sh`：普通产品文档、设计文档、删除／重命名样本 | 实际文档校验保留；五个领域自测组均不运行。 |
| P0-T02 | P0-R1、R4；DES-CI-P0-01 | 同上：校验器代码、Markdown helper、requirements、fixture 变化 | doc checker 自测被执行；相关检查器失败确实使门禁失败。 |
| P0-T03 | P0-R2；DES-CI-P0-02 | 新增 `scripts/ci-required-domain-isolation.test.sh`：纯 PM 样本 | workflow 运行；packaging／operational 不运行。 |
| P0-T04 | P0-R2、R4；DES-CI-P0-02 | 同上：包结构、运维、身份、信任格式及共享脚本样本 | 独立域和显式并集正确；不靠相邻函数嵌套。 |
| P0-T05 | P0-R2；DES-CI-P0-02 | 扩展 `scripts/plan-rust-required-scope.test.sh`：宽泛 minimal 与精确治理规则同时命中 | 显式域能力不被 minimal 抑制；未知路径 full。 |
| P0-T06 | P0-R1、R5；DES-CI-P0-04 | domain fixture：doc checker 与 cargo tooling 各自资源输入；文档 no-toolchain 模式 | doc 不触发 Rust 安装／构建；Cargo fixture 需要的工具明确准备。 |
| P0-T07 | P0-R5；DES-CI-P0-04 | 扩展 `scripts/ci-tests-argument-contract.test.sh`：遗漏新字段、空值、拼写错误、非法布尔、版本错 | 在任何重测试之前明确失败。 |
| P0-T08 | P0-R4、R5；DES-CI-P0-04 | 扩展 `scripts/ci-tests-full-superset-contract.test.sh`：required/full/full-core/full-support 与残留 false 环境 | 每个迁移测试仍在适用 full 集合；不减少历史入口覆盖。 |
| P0-T09 | P0-R5；DES-CI-P0-06 | domain fixture：legacy 无新字段、新版完整字段、半新半旧字段 | legacy 保守兼容；新版严格；半新半旧拒绝。 |
| P0-T10 | P0-R3；DES-CI-P0-03 | 扩展 `scripts/pm/workflow-impact-projection.test.py`：scope complete；独立 CI／review 状态 pending、running、passed | 投影及 scope digest 不因 readiness 变化而变化。 |
| P0-T11 | P0-R3；DES-CI-P0-03 | 同上：scope 真 unknown／pending；空 evidence、错误 hash、错 source | 真未知保留 full；无效身份／依据拒绝；不能换文本伪造 complete。 |
| P0-T12 | P0-R3；DES-CI-P0-03 | 扩展 `scripts/pm/workflow-impact-consumers.test.py`；对实际 producer 输入模板／入口建立 fixture | 所有生产路径只用 scope analysis 构造 closure；不再输入“等待 CI”的 closure。 |
| P0-T13 | P0-R3、R5；DES-CI-P0-03/04 | 扩展 `scripts/pm/ci-ready-receipt.test.py`、`scripts/pm/pr-lifecycle-gate.test.sh`、`scripts/pm/pr-lifecycle-trust.test.sh` | scope complete 但 CI／review pending 或 failed，仍不能 ready／merge；高风险规则不降级。 |
| P0-T14 | P0-R5；DES-CI-P0-04/06 | receipt fixture：新四字段逐个篡改／删除、旧摘要、新旧格式混用 | 新字段进入摘要和身份检查；旧摘要不重写；缺字段不默认为 false。 |
| P0-T15 | P0-R5；DES-CI-P0-04 | 扩展 `scripts/pm/integration-ci.test.py`：本地、PR、trusted integration 消费一致输出；source/target-only 场景 | trusted driver 生效；源能力不遗漏；相关集成超集不被强行裁掉。 |
| P0-T16 | P0-R2、R5；DES-CI-P0-02/04 | domain fixture + hosted：fleet-health、Windows rollout、macOS package 子 Job 的正反选择 | 纯 PM 不启动这些 Job；应该运行时失败／取消／缺失阻断相应放行。 |
| P0-T17 | P0-R4；DES-CI-P0-05 | 新增 `scripts/ci-required-baseline-routing.test.sh` 变异 fixture | 插回无条件 clean-room-adapter 或恢复能力别名时测试变红。 |
| P0-T18 | P0-R4；DES-CI-P0-01/05 | 冻结的旧／新搬移 inventory 对照 | 每项旧调用有新归属和正向选择用例；不靠删除测试满足时延。 |
| P0-T19 | P0-R1、R5；DES-CI-P0-04 | no-network fixture；显式 full 和普通 required | manual-only provider live／hosted-account 不被自动开启。 |
| P0-T20 | P0-R1、R2；第9节 | 同一组合候选上的 hosted 文档／PM／运维样本 | 实际 Job／命令／工具链与计划一致，达到可比统计条件与性能目标。 |

P0-T13 的既有 lifecycle 入口已由当前仓库 `workflow-behavior-eval.sh` 的调用清单核对。[S8] 新增场景仍需实际编写并运行，入口存在不等于上述负例已经覆盖。

### 11.2 组合候选与证据

C2 和 C3 可以有不同 leaf source heads，但整体验收必须选定同一个能包含全部改造的 integration/tested tree，绑定同一可信配置版本、解释版本、工具入口和环境。先按既有身份规则验证该组合，不把多份不同候选的绿色结果拼成“整体已通过”。

实际任务 evidence 必须保存：各 Task/PR、源与集成／tested-tree 身份、workflow/run/attempt/check 身份、选择原因、完整命令日志、退出码、失败／取消样本、旧新覆盖对照及性能原始数据。文档保留验证计划，不随每次代码迭代编造通过记录。

P0 完成条件是 P0-R1–R5 均有独立验证，且必要用例、hosted 组合与规范／实现／消费者一致性完成。仅提交设计、合入代码、输出 `scope=minimal` 或单个本地 fixture 通过都不算启用成功。[S7]

## 12. 决策、长期风险与未决问题

### 12.1 核心决策

本次选择“现有 capability 级拆分”，不引入单测试项调度平台；选择“保留实际产品 full-corpus，移动校验器自测”，不把有效规范降级；选择“修复闭包生产者语义”，不删除 unknown→full；选择“显式解释版本和先读后写”，不依赖环境变量缺省值。

这些决策优先降低小改动的固定成本，并控制兼容范围。代价是：同一个专业域内部仍可能跑较多测试，治理／中央工作流修改仍可能 full，可信集成仍可能重复执行。这些是明确的 P1／后续边界，不是本次未识别的漏洞。

### 12.2 长期风险与解除条件

| 风险 | 责任角色 | 解除／重新评估触发 |
| --- | --- | --- |
| 测试真正依赖共享配置、规则或动态调用，路径表漏登 | repository health + 对应域 owner + QA | 完整搬移 inventory 和正向／负向依赖用例通过后启用；新增依赖须更新规则。 |
| 新 capability 意外触发 Rust 或其他隐藏资源 | repository health + QA | no-toolchain／禁网 dispatcher fixture 和 hosted 日志确认。 |
| 把 evidence 文件存在当作范围完整或 CI 成功 | QA + 专业复审 owner | 对 scope、input digest、执行成功三个事实分别验证；发现错误即阻断相关任务。 |
| 在途旧投影进入新配置产生不兼容 | TPM + receipt owner | 逐任务确认原有效链或显式迁移；不跳过 digest。 |
| 为缩短时间而把中央 CI 修改降为 minimal | repository health + QA | 共享执行控制面保持既有 full／风险复验边界，独立测试新减跑规则。 |
| 必要文档 full-corpus 成为剩余成本 | 文档门禁 owner | 用新样本量化后单独授权优化，不在本次删除强制覆盖。 |

**最终交付边界：P0 解决“不该跑的也跑、不同能力被合并、等待状态误触发扩大”。不声称解决全部 CI 慢、测试项级最小化或 main 前进后的证据复用。**

---

## 附录 A：文件改造清单

| 文件／区域 | P0 改造责任 |
| --- | --- |
| `doc/engineering/workflow/source-of-truth.md` | 先冻结 baseline 边界、能力独立选择、closure 语义、兼容／启用条款，并为代码消费增加稳定锚点。 |
| `testing-manual.md`、相关 `doc/testing/ci/` 文档 | 与实际迁移后的覆盖及入口一致；保留 required/full 语义和证据边界。 |
| `scripts/ci-tests.sh` | 五组能力 dispatcher、baseline 瘦身、legacy 组合、必需字段验证、有界执行日志。 |
| `scripts/ci-required-scope.v2.json` | 两个新 capability、四个新 selector、精确触发依赖和新解释版本；不新增旁路 registry。 |
| `scripts/plan-rust-required-scope.py` | 独立 FIELDS、版本化输出集合、规则优先级、资源依赖和 projection 一致性检查。 |
| `.github/workflows/rust.yml` | env／outputs／artifact 字段完整接线、真实能力对应子 Job、trusted staging 完整性。 |
| `scripts/prepare-task-pr.sh` | 本地 planned-required 命令完整渲染和输入契约校验；保持身份／准入。 |
| `scripts/pm/workflow-impact-projection.py` | 明确 scope-only closure 生产契约和测试；保留 projection v2 严格校验及真未知 full 兜底。 |
| `scripts/pm/ci-ready-receipt.py`、`ci_ready_receipt_identity.py` 与相关消费链 | 版本化 canonical planner、RUN_FIELDS／digest、旧格式兼容和缺字段拒绝。 |
| `scripts/pm/pr-lifecycle-gate.py` 与对应复审／claim consumers | 保持 readiness 独立、风险判定和旧证据边界；发现实际耦合时修复，不预先删除原门禁。 |
| 现有 scope／projection 输入模板及 executable task instructions | 完整搜索并在实施 evidence 列出实际生产者；避免只修测试 fixture 却遗漏真实生产链。 |
| 第11节列出的 existing/new tests | 覆盖选择、执行、资源、证据、legacy、full、变异及 hosted 组合。 |

候选 `.agents` 指令属于执行资产，应按当前 ownership 由 code 叶子维护，不因后缀是 Markdown 混入 system 文档叶子。[S1]

## 附录 B：来源与证据边界

代码来源均固定到本文审读基线。它们支持“现状事实”；目标接口、版本名、新增能力、测试 ID 和性能门槛是本设计提出的改造，不能反向引用源码当作已经存在。

- [S0] [main 基线对应提交](https://github.com/eng-cc/oasis7/commit/9d404f6ea863e0ec11aed0e8c6beb759189ce5fe)。
- [S1] [Engineering Workflow Source of Truth（S0 前序版本 v1.20.0）](https://github.com/eng-cc/oasis7/blob/a7bdf5ff5e934e04e5261e2a9518297ba475fb03/doc/engineering/workflow/source-of-truth.md)：规范先行、文档实际校验、手动 loop／身份／授权边界及 ownership 分类。代码任务必须改为绑定 S0 合入后的准确 source commit。
- [S2] [ci-tests.sh](https://github.com/eng-cc/oasis7/blob/9d404f6ea863e0ec11aed0e8c6beb759189ce5fe/scripts/ci-tests.sh)：`run_required_gate_checks()`、`run_product_doc_governance_check()` 和各 tier 的调用。
- [S3] [plan-rust-required-scope.py](https://github.com/eng-cc/oasis7/blob/9d404f6ea863e0ec11aed0e8c6beb759189ce5fe/scripts/plan-rust-required-scope.py)：三能力同一 FIELDS 输出、selector inventory、minimal 抑制、Rust 需求推导和闭包 full 验证。
- [S4] [workflow-impact-projection.py](https://github.com/eng-cc/oasis7/blob/9d404f6ea863e0ec11aed0e8c6beb759189ce5fe/scripts/pm/workflow-impact-projection.py)：v2 严格字段、closure evidence、scope planner 和 `build_projection()` 的升级行为。
- [S5] [ci-tests.sh 的 operational／packaging 入口](https://github.com/eng-cc/oasis7/blob/9d404f6ea863e0ec11aed0e8c6beb759189ce5fe/scripts/ci-tests.sh)：operational 内部嵌套 packaging 及 PM／loop 回归。
- [S6] [ci-ready-receipt.py](https://github.com/eng-cc/oasis7/blob/9d404f6ea863e0ec11aed0e8c6beb759189ce5fe/scripts/pm/ci-ready-receipt.py) 和 [ci_ready_receipt_identity.py](https://github.com/eng-cc/oasis7/blob/9d404f6ea863e0ec11aed0e8c6beb759189ce5fe/scripts/pm/ci_ready_receipt_identity.py)：planner 字段、摘要和 source／integration 身份。
- [S7] [系统设计写作规范](https://github.com/eng-cc/oasis7/blob/9d404f6ea863e0ec11aed0e8c6beb759189ce5fe/doc/engineering/doc-governance/system-design-writing-standard.design.md)：十二段结构、需求承接、候选／证据及兼容迁移要求。
- [S8] [workflow-behavior-eval.sh](https://github.com/eng-cc/oasis7/blob/9d404f6ea863e0ec11aed0e8c6beb759189ce5fe/scripts/pm/workflow-behavior-eval.sh)：既有 `pr-lifecycle-gate.test.sh` 和 `pr-lifecycle-trust.test.sh` 回归入口。
- [E1] [PR #3955 的 required-gate（Run 35979151984，attempt 2，Job 107568561707）](https://github.com/eng-cc/oasis7/actions/runs/35979151984/job/107568561707)：运行时间与日志样本。该样本是历史诊断依据，不是 P0 改造后的验收证据。
- [S9] [Required-gate capability-selection contract](./source-of-truth.md#required-gate-capability-split)：本 S0 更新的规范锚点。Issue #3967 的 S0 交付通过后，代码任务必须从合入后的 source commit 读取该 clause；候选 source 或本设计本身不授权运行行为。

本文没有运行仓库测试、没有执行 hosted 对照实验、没有验证所有动态依赖，也没有修改远端仓库。实际实现、精确生产者 inventory、独立复审及第11节运行证据必须由后续授权任务完成。
