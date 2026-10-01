# oasis7 CodeQL 全面接入：系统设计与实施方案

- 文档状态：规范设计候选；扫描实现、运行及激活尚未完成。规范 authority 仅来自 [canonical clause](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis)，本文件不自启用。
- 审查日期：2026-09-30。
- 审查基线：`d868cbd76594f843c19d797253dc73eb67a684dc`。这是本次分析的快照，不是生产扫描、合入或复用时必须追赶的固定基线。
- 仓库落点：`doc/testing/ci/codeql-integration.design.md`。
- 系统 owner：`repository_health_engineer`。
- 验收 owner：`qa_engineer`；Rust 覆盖与提取边界由 `runtime_engineer` 复核；涉及权限、发布凭据隔离时由 `blockchain_ops_engineer` 复核。
- 推荐交付：**2 个合入 PR：PR-S（规范与设计），PR-C（实现、测试、接线）。PR-C 内按工作包并行，不按语言、扫描阶段或工作包继续拆合入 PR。**
- 默认运行策略：**全面扫描、观察性接入；不增加 required check，不开启 code-scanning 合并保护，不等待 CodeQL 完成才能合入。**

> 核心原则：以 package 识别影响，以稳定分析单元保留上下文，以原生并发槽位约束资源；不以“每个 crate 一个 job”或“只扫描 diff 文件”换取表面提速。

## 1. 决策摘要

本次一次性接入 Rust、Python、JavaScript/TypeScript、GitHub Actions 四类分析。PR 使用默认查询集并按影响选择语言单元；默认分支定时更新完整默认查询基线；每日运行扩展查询。初始以四个稳定的语言级单元部署，不预设每个 Rust crate 都有独立扫描价值。

所有扫描任务与现有 Rust/required CI 平行运行。扫描工作流不依赖 task Issue 的 PR 回填、impact projection、review receipt 或 integration revalidation。扫描结果不会被纳入现有 CI 收据的身份或复用条件；CodeQL 也不得签发、替代或刷新这些凭据。

仓库级 CodeQL 重型任务并发上限初始设为 2。上限通过两个固定 job concurrency 槽位实现，不能只设置单个 workflow 的 `max-parallel`。同一 PR 新提交取消旧 workflow；不同 PR 在共享槽位排队，不能互相取消。维护扫描与 PR 扫描共用这两个槽位。

本次同时修正 harness 对观察性失败的解释边界，但不全局忽略 `UNSTABLE`，不豁免任意非 required 失败，不伪造成功，不改变已有安全审查、审批、hold、冲突、集成验证或服务器保护规则。

## 2. 当前状态与设计依据

### 2.1 已验证的仓库事实

| 事实 | 对设计的影响 | 来源 |
| --- | --- | --- |
| main 快照为本文审查基线，经典 required status 中可见 `required-gate` | 不增加第二个必需安全扫描门禁 | R1 |
| 本次读取的仓库 ruleset 仅含删除和非快进保护，没有 `code_scanning` 规则 | 当前未发现 ruleset 级 CodeQL 阻断；激活前仍需重新读回 | R2 |
| `pr-lifecycle-gate.py` 会读取 required policy，也会无条件阻断 `UNSTABLE` | “不是 required”仍可能成为事实门禁，必须修兼容性 | R3 |
| required-scope 的未知路径采用 full 回退 | 新安全路径需显式归类；不得修改未知路径的保守语义 | R4 |
| 核心代码为多 crate Rust workspace，另有 Python/JS 与大量工作流资产 | 四语言接入；分语言隔离工具安装和扫描耗时 | R5、R6 |
| 根 Cargo 使用本地 patched vendor 依赖 | 不能通配忽略 `vendor-*`，把维护中的补丁漏掉 | R5 |
| workflow 规范通常区分 system 与 code loop，允许特定明确授权的原子迁移 | 推荐标准两 PR，不把“尽量少 PR”自行当作特殊混合授权 | R7 |
| 现有 CI 包含 `cargo deny check advisories` | 保留现有依赖安全检查，不让 CodeQL 重复或替代它 | R8 |

仓库内没有 CodeQL YAML 不能证明 GitHub default setup 未开启。扫描设置、告警基线、平台生成的 CodeQL check 来源及运行耗时，必须在实施预检和首次运行中确认。

### 2.2 平台能力及限制

CodeQL 已支持四种目标语言。Rust 支持 `build-mode: none`：不要求完整构建，但 rust-analyzer 仍可能编译、执行 `build.rs` 和宏代码，因此提取阶段不是无执行的纯文本处理。Rust nightly features 不在官方支持范围内；采用 nightly 的 WASM 构建链与使用 nightly 特性的源代码需要分别核查，不能直接宣称全部覆盖。[E1、E2]

GitHub 在 2026 年公布的增量优化覆盖 Python、JS/TS 等语言，并要求相应条件，包括默认查询集；不能把这些收益直接外推到 Rust，也不能假定加入扩展查询后仍有相同优化。[E3、E4]

Code scanning 的规则保护与 required status checks 是不同机制。即使未增加 required check，启用 `code_scanning` ruleset 仍可能要求等待扫描结果。此次不启用该规则。[E9]

### 2.3 不纳入本次的工作

不重构业务 crate；不新增常驻服务或自建 runner；不新增安全任务数据库；不新建强制安全签字流程；不重写既有 CI planner；不建设自定义 CodeQL 查询平台；不把新增告警的业务修复全部捆进 CI 接入 PR；不以 CodeQL 代替共识、确定性、WASM 隔离、权限协议或运行时 E2E 验证。

## 3. 需求、约束与验收映射

本设计承接已写入 canonical source 的 CQ-R01–CQ-R09；未合入前仍是候选 authority。按需路由同时承接 [required contract](../../engineering/workflow/source-of-truth.md#required-gate-capability-split)，任务边界同时承接 [manual transition](../../engineering/workflow/source-of-truth.md#manual-three-loop-transition)。

### 2.1 需求承接与分配表

| 上游 requirement（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [CQ-R01](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | 全面接入四语言；第 4、7 节 | [CQ-R01 分配](codeql-integration.design.md#cq-r01) | repository_health_engineer；qa_engineer 验收；领域边界由匹配角色复核 | 不证明所有缺陷均可检出 |
| [CQ-R02](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | 不拖慢普通开发主链；第 5、6、8 节 | [CQ-R02 分配](codeql-integration.design.md#cq-r02) | repository_health_engineer；qa_engineer 验收；领域边界由匹配角色复核 | 不保证共享 hosted runner 绝无排队影响 |
| [CQ-R03](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | package 影响与分析上下文完整；第 4、5 节 | [CQ-R03 分配](codeql-integration.design.md#cq-r03) | repository_health_engineer；qa_engineer 验收；领域边界由匹配角色复核 | 不证明不受支持 nightly 路径 |
| [CQ-R04](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | 并行而不抢占无限资源；第 6 节 | [CQ-R04 分配](codeql-integration.design.md#cq-r04) | repository_health_engineer；qa_engineer 验收；领域边界由匹配角色复核 | 不保证 GitHub 提供跨 workflow 优先级 |
| [CQ-R05](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | 失败真实可见，不构造绿灯；第 8、9 节 | [CQ-R05 分配](codeql-integration.design.md#cq-r05) | repository_health_engineer；qa_engineer 验收；领域边界由匹配角色复核 | 观察态不等于安全合入担保 |
| [CQ-R06](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | 严格信任边界；第 5、8、10 节 | [CQ-R06 分配](codeql-integration.design.md#cq-r06) | repository_health_engineer；qa_engineer 验收；领域边界由匹配角色复核 | 不扩大任何既有 admin 权限 |
| [CQ-R07](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | 不因 main 无关前进重跑业务 CI；第 5、8 节 | [CQ-R07 分配](codeql-integration.design.md#cq-r07) | repository_health_engineer；qa_engineer 验收；领域边界由匹配角色复核 | 不取消既有高风险 exact integration |
| [CQ-R08](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | 尽量少 PR 且可并行实施；第 12、13 节 | [CQ-R08 分配](codeql-integration.design.md#cq-r08) | repository_health_engineer；qa_engineer 验收；领域边界由匹配角色复核 | 不授权候选实现自启用 |
| [CQ-R09](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | 能运营、排障、回退；第 9、11、14 节 | [CQ-R09 分配](codeql-integration.design.md#cq-r09) | repository_health_engineer；qa_engineer 验收；领域边界由匹配角色复核 | 不承诺当前已有健康运行数据 |

<a id="cq-r01"></a>**CQ-R01 — 全面接入四语言：** 由第 4、7 节中的完整契约与第 15–17 节验收负责，不缩减其范围。

<a id="cq-r02"></a>**CQ-R02 — 不拖慢普通开发主链：** 由第 5、6、8 节中的完整契约与第 15–17 节验收负责，不缩减其范围。

<a id="cq-r03"></a>**CQ-R03 — package 影响与分析上下文完整：** 由第 4、5 节中的完整契约与第 15–17 节验收负责，不缩减其范围。

<a id="cq-r04"></a>**CQ-R04 — 并行而不抢占无限资源：** 由第 6 节中的完整契约与第 15–17 节验收负责，不缩减其范围。

<a id="cq-r05"></a>**CQ-R05 — 失败真实可见，不构造绿灯：** 由第 8、9 节中的完整契约与第 15–17 节验收负责，不缩减其范围。

<a id="cq-r06"></a>**CQ-R06 — 严格信任边界：** 由第 5、8、10 节中的完整契约与第 15–17 节验收负责，不缩减其范围。

<a id="cq-r07"></a>**CQ-R07 — 不因 main 无关前进重跑业务 CI：** 由第 5、8 节中的完整契约与第 15–17 节验收负责，不缩减其范围。

<a id="cq-r08"></a>**CQ-R08 — 尽量少 PR 且可并行实施：** 由第 12、13 节中的完整契约与第 15–17 节验收负责，不缩减其范围。

<a id="cq-r09"></a>**CQ-R09 — 能运营、排障、回退：** 由第 9、11、14 节中的完整契约与第 15–17 节验收负责，不缩减其范围。

运行、Task UID、Issue、PR、check-run 等具体交付身份只进入现有 GitHub Project-backed task evidence，不写进长期规范、固定 allowlist 或示例生产记录。这里的 PR-S/PR-C、C1 等是文档工作包名，不是新建的生产身份系统。

## 4. 分包与分析单元

### 4.1 三层概念必须分开

**代码包**用于识别变更归属。例如 Cargo package、Python 模块集合、前端 package、workflow/local action。**分析单元**用于决定一个 CodeQL database 的稳定范围。**实施工作包**用于多人或多 Agent 并行交付。

三者不必一一对应。一个共享依赖很深的 Rust workspace 可以包含很多代码包，但初始只形成一个 Rust 分析单元；同时整个接入仍能分成多个实施工作包并行开发。

### 4.2 首版固定四个扫描单元

| unit ID | CodeQL language | 初始扫描范围 | 安装与执行边界 |
| --- | --- | --- | --- |
| `actions-repo` | `actions` | 全仓 workflow 与 local action metadata；不只看某一条 workflow | 不装 Rust，不构建业务，不运行发布脚本 |
| `python-repo` | `python` | 仓库 Python 源码，包含 scripts、PM/harness 与 tools | 不调用项目业务 bootstrap，不默认 pip install 全量依赖 |
| `javascript-repo` | `javascript-typescript` | JS/TS、相关模板和内嵌脚本 HTML，包含 viewer/site | 不跑 npm build、WASM build、Trunk 或浏览器 E2E |
| `rust-repo` | `rust` | 全仓维护中的 Rust 源码、workspace/独立 manifest 以及本地修改的 vendor 代码 | `build-mode: none`；不跑完整 cargo build/test/打包 |

CodeQL 默认检测、生成文件识别与提取覆盖并不等价于“每个文件都有效分析”。清单、提取诊断和已知控制样本必须结合验证。

纯 Python PR 初始选择一个 Python 单元；纯 JS/HTML PR选择一个 JS 单元；纯 Rust PR 选择一个 Rust 单元；跨语言与共享配置变更选相应并集。这样首先消除四语言全部触发与跨工具链安装的浪费。

### 4.3 Rust 为什么不按每 crate 拆 job

根 workspace 已包含核心 runtime、network、consensus、protocol、WASM 等相关 crate。将一个 crate 单独 checkout 或只保留它的 diff 文件，可能丢失调用方、共享类型、宏、build inputs 或跨包数据流；另一方面，给每个 crate 单独构建整个 workspace，会成倍重复提取成本。

因此首版保留 `rust-repo` 单元。包识别器仍必须列出所有 `Cargo.toml`，处理 workspace members/exclude、workspace 继承、本地 path dependency、target/build/dev dependency 与 patch，不能只读根 members 后就认定覆盖结束。

仅当实测表明 Rust 是扫描瓶颈，并且能证明两个包集合没有未建模的本地依赖、共享生成输入或跨集合调用上下文时，才在**同一 PR-C**内把真正独立的集合变成稳定单元。不要预先承诺会拆出固定数量的 Rust 单元。

### 4.4 进一步拆分的准入条件

可使用本地依赖图的无向连通分量作为保守分组起点：任一 path dependency、共享源码或生成输入构成关联边。存在 workspace 级共享设置和工具链依赖时额外建立触发边。

拆分后必须满足：

1. 所有维护中的 package 都落入一个明确单元，新增 manifest 不能静默漏扫。
2. 每个单元完整保留其需要的本地依赖与构建元数据；无法证明独立则合并单元。
3. 单元范围固定；不能同一 category 本次扫 A、下次扫 B。
4. PR 与 main 的默认档使用同一组单元和 category，便于对比。
5. 采用分片后，定时扩展档保留跨单元完整检查；明确披露 PR 分片没有证明的跨边界范围。
6. 以 `extraction_duration`、`runner_minutes` 和覆盖样本衡量收益，不以 job 数衡量并行效果。

若提取器对 manifests 的实际处理仍使多个 shard 重复加载大部分全仓，撤回分片，保留一个 Rust 数据库；不要用更多 job 掩盖重复工作。

## 5. 按需规划与输入身份

### 5.1 轻量 planner

新增小型 Python planner，不依赖 Rust、Node、CodeQL CLI、任务 Issue 或远程业务服务。它读取 Git 变更、稳定扫描策略与 manifest 数据，产生矩阵。package inventory 是运行时产物，不提交每次更新的大型文件清单。

建议入口：

```text
python3 -I scripts/security/codeql-plan.py \
  --repo-root <checkout> \
  --base <event-base-oid> \
  --head <event-head-oid> \
  --profile <default|extended> \
  --policy <trusted-policy-path> \
  --output <runner-temp>/codeql-plan.json
```

这些是待实现接口，不是现有仓库命令。

### 5.2 变更范围

普通 PR 使用 `merge-base(event.base.sha, event.head.sha) -> event.head.sha` 识别源变更；用完整 Git diff，而不是仅看 API 第一页或仅依赖 workflow 顶层路径过滤。GitHub 顶层过滤有文件数量等限制，不能作为扫描完整性的唯一依据。[E7]

采用 NUL 分隔的 `--name-status -z` 读取，处理增加、修改、删除、类型变化和改名。改名的旧路径、新路径都参与选择；删除 manifest 按 base 与 head 两侧登记结果处理。路径不得作为 shell 指令拼接；符号链接、仓库外路径、异常编码和缺失对象必须显式处理。

真正扫描的默认 PR checkout 保持 GitHub `pull_request` merge ref 语义。记录 source head、event base 和 actual checkout commit，不能把 merge commit 的结果假称为仅 source head 的扫描。[E8]

### 5.3 路由规则

| PR 的完整源差异 | 选择 |
| --- | --- |
| 纯文档，且没有被代码生成/嵌入引用的输入 | 不启动扫描矩阵；plan 报告 `not_applicable` |
| `.rs`、Cargo manifest/lock、Rust toolchain/Cargo config | Rust 单元 |
| Python 源码、Python 依赖/导入配置 | Python 单元 |
| JS/TS/HTML/模板、前端 package 与 lock | JS 单元 |
| workflow 或 local action metadata | Actions；涉及扫描/多语言工具链配置则选相关语言并集 |
| `.github/workflows/codeql.yml`、安全策略、planner、扫描配置 | 四语言及静态安全契约测试 |
| 被 Rust `include_str!`、`include_bytes!`、build script 等使用的前端/数据文件 | 文件所属语言单元与消费者单元；识别不清选保守并集 |
| 未登记源文件、无法识别的新 manifest/配置依赖 | 全部相关语言；无法确定语言则四语言 |
| diff 身份或输入不可信/不可读取 | planner 失败，记录未分析；不以空矩阵成功代替 |

普通 Rust PR 在后续仅修改描述文件时，其整体 PR diff 仍可能包含 Rust，因此可能重新触发 Rust。首版不通过自造旧报告复用来绕过这一点；由取消旧任务、观察态和官方增量机制控制代价。

Shell 不是独立受支持的 CodeQL 语言。单独 `.sh` 变更保留现有脚本契约检查；涉及 workflow 边界时选择 Actions，但不能把它描述为 Shell 全面安全扫描。

### 5.4 选择范围和分析范围的区别

可以仅选择 Python 单元，不必选择 Rust；但选中 Python 单元后扫描其稳定上下文，不只扫改动的两个 `.py`。可以不触发未受影响的独立 Rust 单元；但选中的 Rust 单元必须保留依赖闭包。

首版策略不使用按 diff 生成的极窄 `paths`。生成物、下载缓存和打包输出仅按明确目录排除；测试、fixtures、scripts、tools 与 patched vendor 不得整类忽略。

### 5.5 候选策略与信任

PR 路径的规划代码、配置和范围规则读取可信 base 上的实现；候选策略变更只作为待测试的数据。可以把可信脚本提取至 `RUNNER_TEMP`，以隔离模式运行，避免从候选 worktree 导入同名模块。

PR-C 首次引入的新 planner 在 base 尚不存在时，不得用候选内容为自身减少扫描。bootstrap 采用可信的四语言保守策略，或只运行既有 required 验证并在合入后用 baseline 模式完成扫描验收。

这种隔离不意味着可抵御候选 workflow 本身被恶意改写。观察性 CodeQL 不作为可信合入证明；扫描配置变化仍由现有审查、required 契约和合入后读回约束。

### 5.6 输出

plan 至少包含：`profile`、source/base/checkout identity、策略摘要、选择单元、触发原因、完整性状态、预期覆盖、全量回退原因和槽位分配。字段仅用于解释和诊断，不生成新的合入 authority。

零单元输出必须区分 `not_applicable` 与 `disabled`；解析失败、未知身份不得输出假 `not_applicable`。空矩阵必须通过 job-level 条件处理，不能因 `fromJSON` 或空 include 造成异常。

## 6. 调度、并发与任务取消

### 6.1 两条互不等待的执行链

```text
普通 PR
  ├─ 现有 required CI → 现有审查/集成规则 → 合入决策
  └─ CodeQL plan → 选中单元并行 → SARIF + Security + 健康摘要

定时 main
  └─ 完整默认档 / 完整扩展档 → 同一 CodeQL 扫描槽位池
```

不得在 `rust.yml` 里增加 `needs: codeql`，也不得让 CodeQL 通过 `workflow_run` 等待 required CI，再反向参与最终合入等待。

### 6.2 触发策略

| 事件 | 档位和范围 | 初始策略 |
| --- | --- | --- |
| 非 draft PR opened/reopened/synchronize/ready_for_review | default，按需单元 | 开启 |
| draft PR | 仅轻量 plan 或跳过扫描 | 不占重型槽位 |
| converted_to_draft、closed | 取消本 PR 旧扫描，不启动新重型任务 | 开启对应事件处理 |
| main 每 6 小时 | default，完整单元 | 建立/更新对比与增量基线 |
| main 每日低峰 | security-extended，完整单元 | 更广查询与覆盖诊断 |
| workflow_dispatch | default 或 extended，可信默认分支 | 诊断、初始基线及受控重跑 |
| 每一次 main push | 不额外重复扫描 | 避免高合入频率形成扫描风暴 |
| 普通无关 main 前进 | 不要求重开 PR、不重跑 required、不刷新安全收据 | 保持当前普通任务语义 |

默认档的六小时间隔是建议初值，不是扫描新鲜度保证。官方增量缓存不可用时由 CodeQL 正常分析；不得伪造基线。

可用 UTC cron 示例：`17 */6 * * *`（默认档），`43 18 * * *`（扩展档）。实施时核对仓库实际负载，避开现有重型定时任务；不在静态文档中硬编码所谓生产繁忙时段。

### 6.3 全仓扫描并发上限

`strategy.max-parallel: 2` 仅限制单次 workflow，不能阻止多个 PR 合起来产生大量扫描。建议两个固定槽位：`oasis7-codeql-scan-slot-0`、`oasis7-codeql-scan-slot-1`。

planner 为每个单元分配 slot 0 或 1；同一 run 初始任务尽量交错分布。可以用 PR 编号与稳定单元序号做有界分配，但编号只用于资源分桶，不建立扫描或任务身份。所有重型 CodeQL job 必须使用这两个槽位之一，PR、手动和定时执行不能另起无上限的池。

```yaml
# 核心配置片段，不是完整可执行 workflow。
# 工作流级只去重同一 PR；非 PR 分别按 schedule/profile 建组。
concurrency:
  group: oasis7-codeql-run-${{ github.event_name }}-${{ github.event.pull_request.number || github.event.schedule || inputs.profile || github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}

jobs:
  analyze:
    needs: plan
    if: needs.plan.outputs.has_units == 'true'
    strategy:
      fail-fast: false
      max-parallel: 2
      matrix: ${{ fromJSON(needs.plan.outputs.matrix) }}
    concurrency:
      group: oasis7-codeql-scan-slot-${{ matrix.slot }}
      queue: max
      cancel-in-progress: false
```

GitHub 原生 `queue: max` 支持同一 concurrency group 保留多个待执行项，上限为 100；不能与同一块中的 `cancel-in-progress: true` 配合。这里 PR 去重发生在 workflow 层、全仓排队发生在 job 层，两者使用不同 key，必须以真实 hosted 演练验证组合。[E5、E6]

仅设共享 group 而不设置多项队列，会使不同 PR 的 pending 互相替换，不能采用。全仓单一 workflow group 同样不能采用。

### 6.4 资源与公平性边界

重型扫描最多两个并发；plan 和摘要仍有短暂 runner 开销。两个槽位不是专属预留机器，GitHub 也没有在这里承诺 required CI 的绝对优先级。因此不能宣称真实排队时间必然零增长。

分桶可能造成一个槽位排队、另一个暂时空闲，接受这一简单方案的利用率损失，不增加自建调度服务。超出平台队列容量的取消必须进入健康报告，不能统计为完成。

可以支持经校验的 `OASIS7_CODEQL_MAX_SLOTS=1|2` 运营参数；仅允许降低/限定扫描资源，不改变 required policy。默认 2；观察到 required 排队恶化时降为 1 或退回 baseline 模式。

### 6.5 取消与超时

同一 PR 新 head 取消旧扫描；旧取消记录保留，不能清洗成成功。不同 PR 互不取消。单个语言失败不能取消同 run 的其他语言，因此 `fail-fast: false`。

不使用 runner 内 sleep 模拟优先级，不在合入 watcher 内持续轮询扫描，不为每次 main 前进重新 dispatch。

超时初值可以设轻型语言 20 分钟、Rust/扩展档 45 分钟；这是资源保护预算，不是实测耗时或完成承诺。OOM、超时要呈现为扫描失败，并用于调整提取范围和资源。

## 7. 查询、提取、缓存与扫描覆盖

### 7.1 两档查询

PR 和定时默认基线使用内置默认查询：不额外指定自定义 pack、`security-and-quality`、`security-extended` 或实验开关。尽可能保留官方对 Python、JS/TS 的默认增量路径。[E3、E4]

每日扩展档使用 `security-extended`。对当前查询包明确支持的语言，可以在扩展档验证 `local` threat model，以纳入 CLI 参数、环境和本地文件等来源；支持范围、额外告警和耗时必须单独确认，不能默认它覆盖所有项目 CLI 威胁。PR 默认档不因该扩展而改变。[E10]

不首批启用自定义查询平台或海量质量规则。扫描“全面”指四语言、维护中源码、关键上下文、定时更广查询和明确缺口，并非所有可能规则都挤进每次 PR。

### 7.2 Rust

固定 `build-mode: none`。不要为每个 crate 执行 `cargo build -p`，不要以 `ci-tests.sh required/full` 为 CodeQL 初始化步骤，不跑发行版、WASM、Trunk、GUI、GPU 或链网启动。

使用与仓库兼容的明确稳定 Rust toolchain；清理对提取无关或有干扰的 `RUSTC_WRAPPER` 等环境继承。build script 所需工具只按提取诊断补充最小集合，不照搬业务编译 apt 安装表。

对非默认 features、target cfg、宏展开、独立 manifest、build.rs 生成源、WASM/nightly 路径逐项记录覆盖。完整源码文件存在不是所有条件编译路径均获得有效语义的证明。覆盖缺口不能用移除相关目录来隐藏。

### 7.3 其他语言

Python 与 JS/TS 默认只建立静态分析所需环境，不执行项目安装脚本和构建脚本。确实依赖生成源码时，单独批准最小生成步骤，隔离凭据，并评估放到定时档而非所有 PR。

Actions 单元包含工作流与 local action metadata；它不替代所有被调用 Shell/Python 代码的语言级分析。

### 7.4 缓存

优先使用官方支持的数据库/提取增量机制，不自造旧 SARIF 上传或跨提交数据库替换。缓存 miss 只能变慢，不能改变扫描通过条件、身份和覆盖。

Rust 依赖缓存可保存包下载数据，如 Cargo registry/cache、index 和 git dependency 数据；不保存凭据、`~/.cargo/bin`、可执行 target、发布目录或全局缓存根。cache key 至少区分系统、工具链和相关 lockfile 摘要。只有受信默认分支维护执行负责写入共享依赖缓存；PR 只恢复可信缓存。不得与发布、required CI 的可执行编译产物共享缓存。

不假定 CodeQL 的内置 dependency caching 自动支持所有 Rust 场景；实现需确认当前 Action 的实际能力，不支持时采用受控的下载缓存或不用缓存。

### 7.5 度量提取而非只数绿灯

每个单元记录选中原因、预期源码/manifest 集合、提取诊断、实际执行配置、CodeQL/查询包版本、完成时间及上传状态。首次基线核查所有维护中的 manifest 是否出现于预期覆盖。

覆盖检查可以使用临时数据库查询或提取元数据辅助，但不把有文件记录等同于可用数据流。已知控制样本在临时 fixture 中验证，不能为了证明检出能力把真实可触发漏洞植入 main。

## 8. 合入门禁兼容：狭窄解释，不全局放松

### 8.1 要解决的具体问题

现有 gate 在 required checks 后仍阻断所有 `UNSTABLE`。新增观察性 CodeQL 错误或平台扫描告警 check 可能因此成为间接门禁。[R3]

修复不是删除这个分支，而是为“已确认只由观察性 CodeQL 造成的不稳定”增加明确解释。普通项目不认识的失败继续沿用原有保守处理。

### 8.2 check 分类

| 分类 | 身份与权威 | 合入影响 |
| --- | --- | --- |
| `required` | 由实时 classic protection/rulesets 指定 | 保持原要求 |
| `advisory_codeql` | 精确对应可信扫描 workflow 或验证过的平台 CodeQL provider，且当前非 required、非 code_scanning 强制规则 | 不等待其 pending，不因其单独失败阻断 |
| `other_or_unknown` | 其余或来源无法验证 | 不新增自动豁免 |

分类不能只根据 name 前缀、`[bot]`、PR body、label、候选 JSON 或 `details_url` 的字符串外观。需要验证当前 commit/ref、App 来源以及 Actions run/job 的 workflow path 或 code-scanning analysis 的可靠关联。

两类来源都要考虑：Actions 产生的 plan/analyze/summary job check；平台可能另外产生的 CodeQL 汇总 check。后者不能未经实测假定 App ID、名称或 URL 格式。

稳定规范存储 provider 类型、workflow path、category 规则；具体数字 ID 运行时读回，不把一次扫描的实例写进 allowlist。

### 8.3 UNSTABLE 解释条件

仅当实时规则读取完整、必需检查满足、来源相关的不稳定项均能解释为合法观察性 CodeQL、无其他未知或失败项且其他合入条件独立满足时，才不追加 `UNSTABLE` blocker。

必须保留原始 `mergeStateStatus` 和解释依据，不能写成 `CLEAN` 或把失败 check 改成成功。同名多 App、冲突 attempt、缺少 provenance、未分页读全、权限不足等均不能形成豁免。

`DIRTY`、`UNKNOWN`、未证明原因的 `BLOCKED`、新 required CodeQL、活动 `code_scanning` 规则、review changes requested、未解决线程、有效 hold 均继续阻断。不得为了兼容增加通用 `--admin` 旁路。

已有 approval-only 合入路径只有在其既有授权的条件集独立证明成立时才能继续使用：解释 CodeQL 失败不是新的 admin 授权；不允许把 code-scanning 规则或未知保护一起绕过。

### 8.4 避免 watcher 变慢

优先扩展现有一次性 GraphQL snapshot 的来源字段。在没有 `UNSTABLE`、没有待解释 CodeQL 异常时，不拉 CodeQL 告警历史，不下载 SARIF，不枚举所有 Actions runs。

必要时才对异常 check 做有界来源读取；按不可变 run/check 身份在一次决策中去重。沿用已有 API budget 与范围完整性保护；读取失败给出 capability-blocked 原因，不开始无限扫描或要求重跑 Rust CI。

不让 CodeQL 完成状态进入现有 CI-ready/review/integration 的 digest。CodeQL 可以在 PR 合入后完成；这是观察态的明确风险接受，不等于“合入前已验证安全”。

### 8.5 PR 会话边界

首版只使用原生 Security、check annotations 和 job summary，不自动发布 top-level PR 评论，不自动创建修复任务，以免既有 actionable-comment gate 被新的机器人摘要触发。

任何真实 review、用户安全评论或已创建的修复 hold 仍按既有规则处理。不能以“它来自 CodeQL/机器人”为由一概忽略。

### 8.6 检查所有状态消费者，不能只改一个判断

C3 必须同时审计 PR watch、最终 merge readiness、关闭任务、CI/review 状态摘要等所有消费 `statusCheckRollup`、`mergeStateStatus` 或 Actions 结论的入口。检索直接调用 `gh pr checks`、等待全部 check 完成、任何 red 一律阻断的实现；搜索无命中不能代替完整调用链验证。

把需要解释 CodeQL 状态的入口收敛到同一分类函数，不维护多份 name allowlist。保留现有 required、原生保护、review 和 hold 判断，不只是把某条 CLI 改成 `--required`。确保被选中的 required 单元计数不会混入 CodeQL matrix job，也不会因观察性取消而重新 dispatch 业务 CI。

按需单元和仅 main 执行的 extended category 可能影响平台的扫描完整性提示。验收需记录真实显示与来源；不能上传空报告压掉提示。观察态可以如实显示“该 PR 未运行某单元”，但不得把它宣称为全仓扫描完成，未来强制模式必须重新验证这一边界。

## 9. 结果与告警治理

### 9.1 稳定 category

建议格式：`/language:<language>/unit:<stable-unit>/profile:<default|extended>`。

PR 与 main default 的 category 一致；extended 使用另一 category。同一 category 不包含 PR 编号、commit、run ID 或动态路径集合，不随事件变化范围。不得修改 CodeQL 工具名来区分子项目。[E10]

未选中的单元不上传空 SARIF；失败单元不上传伪造“零告警”；旧 SHA 结果不重新标注为新 SHA。未来单元拆并要执行明确 category 迁移，防止错误关闭/重复悬挂历史告警。

### 9.2 三种状态分开记录

`execution_status` 表示扫描是否真正执行成功；`upload_status` 表示结果是否被平台接受；`finding_status` 表示告警结果。planner 的 `not_applicable`/`disabled` 另列。

扫描成功不等于零漏洞；上传失败不等于成功发布；取消、超时和队列溢出不等于完成。全部真实状态保留在 Actions/summary 和只读健康报告中。

### 9.3 修复流程

告警事实以原生 Security 为源。先核实新告警与历史告警，再分派到 runtime/network/WASM、harness、viewer/site 或 CI owner。源码修复按既有 package 与任务边界执行，不为消除初始噪音把它们一并塞进接入 PR。

误报或风险接受使用平台 dismissal 原因与已有任务证据。默认不新增全仓 ignore 清单、不整类关闭 CWE、不忽略整个测试或脚本目录。

确认存在严重漏洞时使用现有安全事件/hold/发布控制，不以观察态为由无限期放置。但是否将 CodeQL 升为自动强制门禁属于后续独立决策。

### 9.4 只读健康报告

`scripts/security/codeql-health.py` 读取允许的仓库分析和 run 数据，报告最近成功覆盖、失败单元、队列情况、缺失 manifest、耗时与告警趋势。默认只输出本地 JSON/Markdown 或 Actions summary，不写 Issue、不设置 branch protection、不触发重跑。

建议完整 main 覆盖 24 小时内有成功记录；超期标记不健康而不是继续显示旧绿灯。权限不足或 API 不可用标成 unknown，不认定没有告警。

## 10. 执行安全与供应链

使用 ephemeral GitHub-hosted Ubuntu runner；不访问用户本机、链网、云账户或发布 environment。默认 `permissions: contents: read`；仅扫描上传相关 job 授予 `security-events: write`，必要时给予只读 Actions/PR 权限。plan/summary 不给写权限。

使用 `pull_request`，不为了 fork 上传或读取私有数据改成 `pull_request_target`。不使用 `secrets: inherit`、PAT、云密钥、签名密钥、业务 API key；checkout 使用 `persist-credentials: false`。[E8]

同仓 PR、fork、Dependabot 的权限差异分别验收。只读 token 不妨碍采用 GitHub 官方的 PR code-scanning 上传支持；遇到不支持或策略阻止的情况记录真实结果，不通过高权限重跑规避限制。

build.rs/宏可能执行不可信代码，最小权限仍不等于无风险。不能把仓库发布权限或业务 secrets 放入扫描 job；扫描结果亦不能被视为防恶意 workflow 的独立证明。

将官方 Actions 固定到经核验的完整 commit SHA，并保留版本注释。示例中 `v4` 只表示当前 CodeQL Action 大版本；正式合入前解析、验证并锁定源码，不能编造 SHA。只更新本次新增扫描链使用的 Actions，不顺带重写所有发布流水线。

## 11. 静态契约与现有 CI 接线

新增扫描本身不是 required；但**扫描配置和 gate 兼容实现的回归测试**属于现有 workflow governance 范畴，需要按相关路径运行。

复用 `workflow_governance` 能力，不新增 capability、selector、receipt schema 或第二套 required planner。精确归类 `.github/workflows/codeql.yml`、`scripts/security/**` 和相关 gate 测试。

修改 shared planner、`ci-tests.sh`、required-scope policy 或 `rust.yml` 本身时仍走当前保守 required-full。本次首次接入的 PR-C 可能因此较重，这是对门禁变更的验证成本，不转嫁给以后每个普通 PR。

新增测试必须登记到现有 capability test inventory，并保留其应有的 full/full-core/full-support 覆盖，不只在新 workflow 中孤立执行。

### 11.1 验证映射表

| 上游 requirement（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 验证方法、test/manual source、scenario/layer | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [CQ-R01](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | [CQ-R01](codeql-integration.design.md#cq-r01) | 全面接入四语言 | N/A: reason=PR-S 仅冻结规范且测试实现尚未交付; scope=CQ-R01 的 CodeQL implementation 与 hosted acceptance; owner_role=qa_engineer; evidence_ref=codeql-integration.design.md#codeql-test-matrix; re-evaluate=PR-C 实施时逐项绑定第 15 节真实 test source 并在 baseline/observe 前复验 | 现有 GitHub task evidence：fixture 与 hosted、tested tree、运行环境及原始输出分列；本次仅 docs 检查 | 不证明所有缺陷均可检出；本次不证明扫描已运行 |
| [CQ-R02](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | [CQ-R02](codeql-integration.design.md#cq-r02) | 不拖慢普通开发主链 | N/A: reason=PR-S 仅冻结规范且测试实现尚未交付; scope=CQ-R02 的 CodeQL implementation 与 hosted acceptance; owner_role=qa_engineer; evidence_ref=codeql-integration.design.md#codeql-test-matrix; re-evaluate=PR-C 实施时逐项绑定第 15 节真实 test source 并在 baseline/observe 前复验 | 现有 GitHub task evidence：fixture 与 hosted、tested tree、运行环境及原始输出分列；本次仅 docs 检查 | 不保证共享 hosted runner 绝无排队影响；本次不证明扫描已运行 |
| [CQ-R03](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | [CQ-R03](codeql-integration.design.md#cq-r03) | package 影响与分析上下文完整 | N/A: reason=PR-S 仅冻结规范且测试实现尚未交付; scope=CQ-R03 的 CodeQL implementation 与 hosted acceptance; owner_role=qa_engineer; evidence_ref=codeql-integration.design.md#codeql-test-matrix; re-evaluate=PR-C 实施时逐项绑定第 15 节真实 test source 并在 baseline/observe 前复验 | 现有 GitHub task evidence：fixture 与 hosted、tested tree、运行环境及原始输出分列；本次仅 docs 检查 | 不证明不受支持 nightly 路径；本次不证明扫描已运行 |
| [CQ-R04](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | [CQ-R04](codeql-integration.design.md#cq-r04) | 并行而不抢占无限资源 | N/A: reason=PR-S 仅冻结规范且测试实现尚未交付; scope=CQ-R04 的 CodeQL implementation 与 hosted acceptance; owner_role=qa_engineer; evidence_ref=codeql-integration.design.md#codeql-test-matrix; re-evaluate=PR-C 实施时逐项绑定第 15 节真实 test source 并在 baseline/observe 前复验 | 现有 GitHub task evidence：fixture 与 hosted、tested tree、运行环境及原始输出分列；本次仅 docs 检查 | 不保证 GitHub 提供跨 workflow 优先级；本次不证明扫描已运行 |
| [CQ-R05](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | [CQ-R05](codeql-integration.design.md#cq-r05) | 失败真实可见，不构造绿灯 | N/A: reason=PR-S 仅冻结规范且测试实现尚未交付; scope=CQ-R05 的 CodeQL implementation 与 hosted acceptance; owner_role=qa_engineer; evidence_ref=codeql-integration.design.md#codeql-test-matrix; re-evaluate=PR-C 实施时逐项绑定第 15 节真实 test source 并在 baseline/observe 前复验 | 现有 GitHub task evidence：fixture 与 hosted、tested tree、运行环境及原始输出分列；本次仅 docs 检查 | 观察态不等于安全合入担保；本次不证明扫描已运行 |
| [CQ-R06](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | [CQ-R06](codeql-integration.design.md#cq-r06) | 严格信任边界 | N/A: reason=PR-S 仅冻结规范且测试实现尚未交付; scope=CQ-R06 的 CodeQL implementation 与 hosted acceptance; owner_role=qa_engineer; evidence_ref=codeql-integration.design.md#codeql-test-matrix; re-evaluate=PR-C 实施时逐项绑定第 15 节真实 test source 并在 baseline/observe 前复验 | 现有 GitHub task evidence：fixture 与 hosted、tested tree、运行环境及原始输出分列；本次仅 docs 检查 | 不扩大任何既有 admin 权限；本次不证明扫描已运行 |
| [CQ-R07](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | [CQ-R07](codeql-integration.design.md#cq-r07) | 不因 main 无关前进重跑业务 CI | N/A: reason=PR-S 仅冻结规范且测试实现尚未交付; scope=CQ-R07 的 CodeQL implementation 与 hosted acceptance; owner_role=qa_engineer; evidence_ref=codeql-integration.design.md#codeql-test-matrix; re-evaluate=PR-C 实施时逐项绑定第 15 节真实 test source 并在 baseline/observe 前复验 | 现有 GitHub task evidence：fixture 与 hosted、tested tree、运行环境及原始输出分列；本次仅 docs 检查 | 不取消既有高风险 exact integration；本次不证明扫描已运行 |
| [CQ-R08](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | [CQ-R08](codeql-integration.design.md#cq-r08) | 尽量少 PR 且可并行实施 | N/A: reason=PR-S 仅冻结规范且测试实现尚未交付; scope=CQ-R08 的 CodeQL implementation 与 hosted acceptance; owner_role=qa_engineer; evidence_ref=codeql-integration.design.md#codeql-test-matrix; re-evaluate=PR-C 实施时逐项绑定第 15 节真实 test source 并在 baseline/observe 前复验 | 现有 GitHub task evidence：fixture 与 hosted、tested tree、运行环境及原始输出分列；本次仅 docs 检查 | 不授权候选实现自启用；本次不证明扫描已运行 |
| [CQ-R09](../../engineering/workflow/source-of-truth.md#codeql-advisory-analysis) | [CQ-R09](codeql-integration.design.md#cq-r09) | 能运营、排障、回退 | N/A: reason=PR-S 仅冻结规范且测试实现尚未交付; scope=CQ-R09 的 CodeQL implementation 与 hosted acceptance; owner_role=qa_engineer; evidence_ref=codeql-integration.design.md#codeql-test-matrix; re-evaluate=PR-C 实施时逐项绑定第 15 节真实 test source 并在 baseline/observe 前复验 | 现有 GitHub task evidence：fixture 与 hosted、tested tree、运行环境及原始输出分列；本次仅 docs 检查 | 不承诺当前已有健康运行数据；本次不证明扫描已运行 |

## 12. 最少 PR 的交付划分

### 12.1 PR-S：规范与设计

只做 system 文档：

- 在 workflow source of truth 增加观察性扫描、来源分类、资源限制与非自启用规范。
- 新增本设计文档，承接需求、实施、验收、排障与回退；不另建一套产品承诺。
- 现有 CI 专题文档只添加简短边界/导航，避免重复规范。
- 按现有文档登记机制登记新设计；只改必要条目，不整体重写 document inventory，不捆绑其他文档拆分迁移。

PR-S 独立审查并合入后，PR-C 绑定该不可变规范提交。PR-S 不声称扫描已启用、不修改生产门禁。

### 12.2 PR-C：一次完整实现

一次交付四语言、planner、工作流、全仓双槽位、gate 兼容、静态回归、只读健康报告与模式切换。默认关闭重型扫描，合入后通过运维配置完成 baseline → observe，不为开启语言和档位再分别开 PR。

若在 PR-C 合入前实测能够安全分离独立 Rust 单元，可在 PR-C 内完成；不能证明则保持 Rust 全仓单元，不为追求分包数量另开重构 PR。

后续业务漏洞修复不计作“接入的第三 PR”，也不应为了两 PR 指标而省略必要修复。强制安全门禁不属于本次默认交付。

## 13. PR-C 内部分工和并行图

### 13.1 一个 canonical code task，多份有边界工作包

每个合入 PR 对应既有 canonical task/branch/worktree。C1–C5 是同一 code task 下的有界工作包/子 Agent packet，不自行创建多个 leaf task、Issue、Project item 或 PR，也不共享写 git index。

可以由多个执行者并行修改互不重叠的文件或交付 patch；只有集成者负责 canonical git index、提交、最终推送和共享热点文件。不让多个 Agent 在同一 worktree 并发 rebase、checkout 或 cherry-pick。

### 13.2 工作包与文件所有权

| 工作包 | 交付 | 独占路径/输入 | 依赖 |
| --- | --- | --- | --- |
| C1 范围与分包 | policy、planner、路径/manifest/改名测试 | `scripts/security/codeql-policy.json`、`codeql-plan.py`、对应测试 | PR-S 规范和冻结接口 |
| C2 扫描执行与结果 | CodeQL workflow、双槽位、profile、读回健康工具及测试 | `.github/workflows/codeql.yml`、`codeql-health.py`、workflow/health 测试 | 可按 C1 固定输出 fixture 并行开发 |
| C3 门禁兼容 | 精确 advisory 分类、惰性来源读取、UNSTABLE 解释及负例 | `scripts/pm/pr-lifecycle-gate.py` 与相关专用测试；必要的小 helper | 与 C1/C2 可并行，仅依赖稳定 check/category 合约 |
| C4 CI 接线 | scope 精确路由、测试清单、dispatcher 接线 | `scripts/ci-required-scope.v2.json`、`scripts/ci-tests.sh`、capability inventory | 先准备；单一集成者最后落热点文件 |
| C5 QA 与验收 | 独立负例、联合树测试、真实 hosted 覆盖/队列演练 | 测试数据与现有任务 evidence；不重复编辑 C1–C4 源文件 | 可先写用例，联合验收在集成后 |

### 13.3 依赖顺序

```text
PR-S 规范与接口冻结
          │
          ├─ C1 planner/policy ─────┐
          ├─ C2 workflow/health ────┤
          ├─ C3 gate compatibility ─┼─ C4 单点接线 → 联合树回归 → PR-C 合入
          └─ C5 独立验收用例 ──────┘                           │
                                                   baseline → observe
```

C2 不必等 C1 全部实现结束：共享一个冻结的 matrix/config 输出 fixture 即可开发。C3 不必等待首次扫描成功才实现来源 resolver；可先用隔离 fixture，再由真实读回确认平台 check 形态。

规范与代码 PR 串行，工作包开发和审查可并行；无需“一个语言一个 PR”“gate 修复一个 PR、激活另一个 PR”。

### 13.4 实施顺序

先固定接口与 category；然后 C1/C2/C3 并行；C5 同步准备负例；C4 最后统一更新热点接线；对同一精确 combined tree 运行审查和既有 required 验证。基线扫描的真实结果只能在获得相应可信执行条件后验收，不能用 fixture 代替生产扫描事实。

## 14. 激活、预检和回退

### 14.1 模式

`OASIS7_CODEQL_MODE` 支持 `off`、`baseline`、`observe`。缺失按 off；非法值报配置错误。

- off：不启动重型扫描；报告为 disabled，不冒充通过。
- baseline：只允许可信 main 的定时/手动扫描，用于确认覆盖、版本、平台来源和输出。
- observe：保留 baseline，并开启非 draft PR 按需扫描。

这是扫描运行模式，不是 required policy。变量、候选 policy、label 或 PR body 都不能减弱现有 branch/ruleset/Project authority。

### 14.2 实施预检

使用本地已授权 gh 会话读取 repository metadata、default setup、现有分析、classic protection、完整有效 rulesets 和 Actions 配置。下面仅是操作接口示例，本次未执行写操作：

```bash
REPO=eng-cc/oasis7

gh api "repos/$REPO/code-scanning/default-setup"
gh api --paginate "repos/$REPO/code-scanning/analyses?per_page=100"
gh api "repos/$REPO/branches/main/protection"
gh api --paginate "repos/$REPO/rulesets?per_page=100"
gh variable list --repo "$REPO"
```

逐个扩展有效 ruleset 的详情与适用范围，不能只看列表。404/403 和连接器能力不足不能一概解释为未启用。若 default setup 已配置，先明确迁移并关闭重复来源；不得边开 advanced 边保留另一套相同 CodeQL 上传。

### 14.3 合入后激活

PR-C 合入后读回默认分支上预期文件与模式。先启用 baseline，分别运行 default 与 extended，核查四语言、CodeQL 工具/查询包版本、manifest 覆盖、提取诊断和平台上传。

```bash
# 仅在实施获得授权、PR-C 已合入且预检通过后执行。
gh variable set OASIS7_CODEQL_MODE --repo "$REPO" --body baseline
gh workflow run codeql.yml --repo "$REPO" --ref main -f profile=default
gh workflow run codeql.yml --repo "$REPO" --ref main -f profile=extended
```

通过已有候选或适当的隔离验证路径确认 check provenance、观察性失败解释及原生队列配置；不为每种故障都创建一个要合入的 PR。fixture、真实扫描与真实 gate 决策证据分开记录。

验证满足后切换 observe：

```bash
gh variable set OASIS7_CODEQL_MODE --repo "$REPO" --body observe
```

该步骤不需要第三个源码 PR，但必须写回现有授权 task evidence，并核实 required check/rulesets 没有被意外修改。首次状态错误时留在 baseline，不宣布全面上线。

### 14.4 回退

资源占用异常：先把扫描槽位降为 1；PR 兼容性异常：退回 baseline；更严重故障：off 或仅停用 CodeQL workflow。不得关闭 required-gate、跳过 review 或把扫描错误吞掉。

回退后保留已有 Security 告警与运行记录。若平台中已被另行加入强制 code-scanning 规则，不能只关 workflow：那会引入等待/未配置阻断，应按既有授权流程处理规则变化。

<a id="codeql-test-matrix"></a>
## 15. 测试矩阵

以下 ID 是测试用例标识，不是生产交付身份。所有测试需注明 fixture/实际 hosted、tested tree、环境和证据位置。

| ID | 场景 | 预期 |
| --- | --- | --- |
| CQ-T01 | 纯文档 PR | 零重型扫描，明确 not_applicable |
| CQ-T02 | Python-only | 只选 Python，不装 Rust/Node 业务链 |
| CQ-T03 | JS/HTML-only | 只选 JS；模板与内嵌脚本不漏 |
| CQ-T04 | Rust-only | 只选 Rust，完整稳定上下文 |
| CQ-T05 | workflow/local action | Actions；共享输入按规则扩展 |
| CQ-T06 | CodeQL policy/workflow/planner 变化 | 四语言保守选择与治理契约 |
| CQ-T07 | Cargo.lock、target/build/dev/path dependency、patch | Rust 受影响范围完整 |
| CQ-T08 | 新增/删除独立 manifest | 不静默漏包；base/head 双侧处理 |
| CQ-T09 | 文件跨目录改名、空格/换行/特殊字符路径 | 两端参与选择，不 shell 注入 |
| CQ-T10 | diff 对象缺失、范围不完整 | planner error，不是空矩阵绿灯 |
| CQ-T11 | 超过平台原生路径过滤上限的源差异 | planner 仍用完整 Git diff |
| CQ-T12 | candidate 策略试图把自己归为文档 | 不使用其减扫结论 |
| CQ-T13 | frontend/data 被 Rust 嵌入引用 | 消费者单元加入；不清楚时保守扩大 |
| CQ-T14 | patched vendor 源修改 | Rust 范围包含该代码，不被全局忽略 |
| CQ-T15 | default / extended | 查询与 category 分离，默认档不偷偷加入自定义规则 |
| CQ-T16 | 四语言真实基线 | 都有真实执行/上传证据；无源或未提取明确失败/缺口 |
| CQ-T17 | Rust 宏/build.rs/feature/target | 诊断可见，不声称超出验证的覆盖 |
| CQ-T18 | 同一 PR 连续推送 | 旧 workflow 取消，新提交任务保留 |
| CQ-T19 | 两个以上 PR 同时执行 | 不相互取消；重型峰值不超过配置槽位数 |
| CQ-T20 | shared slot 多个 pending | queue:max 保留任务，非默认替换行为 |
| CQ-T21 | queue 容量限制或取消 | 健康报告不统计为扫描完成 |
| CQ-T22 | 单语言错误或超时 | 其他语言继续，失败真实保留 |
| CQ-T23 | CodeQL pending + required 成功 | 不等待扫描；其他保护仍须满足 |
| CQ-T24 | 已证明 advisory CodeQL 失败 + UNSTABLE | 仅免除此解释得出的 blocker，不伪造 CLEAN |
| CQ-T25 | 非 CodeQL 未知失败 + UNSTABLE | 继续阻断 |
| CQ-T26 | 伪装同名 check / 错 App / 错 workflow / 错 commit | 不归为 advisory |
| CQ-T27 | required check 失败或缺失 | 保持阻断 |
| CQ-T28 | CodeQL 被实时加入 required 或活动 code_scanning 规则 | 不按观察态绕过 |
| CQ-T29 | DIRTY/UNKNOWN/未解释 BLOCKED/review/hold | 保持现有保护 |
| CQ-T30 | 来源读回失败/分页溢出/重复冲突身份 | 能力不足可见，不自动降级或无限请求 |
| CQ-T31 | UNSTABLE 伴随 REVIEW_REQUIRED | 不扩大 admin 权限，不假称审批已满足 |
| CQ-T32 | 无关 main 前进 | 不强制刷新 CodeQL 或业务收据；高风险规则不变 |
| CQ-T33 | SARIF 上传失败 | 与扫描执行成功分开，不标安全完成 |
| CQ-T34 | 未选单元、空计划、旧 SHA SARIF | 不上传空报告关闭告警，不冒用旧扫描 |
| CQ-T35 | fork/Dependabot | 最小权限，无业务 secret，不改为特权触发 |
| CQ-T36 | 依赖缓存 miss/不可信 cache | 只影响速度，不影响身份与判定 |
| CQ-T37 | off/baseline/observe/非法模式 | 模式准确；候选配置不能改变 required |
| CQ-T38 | watcher/merge/closeout 等所有状态消费者的健康路径与观察性失败 | 共用分类，不等待全部 CodeQL；健康路径不新增告警历史/全仓 run 枚举 |
| CQ-T39 | 静态测试路径接线与 full 覆盖 | 纳入现有 capability，不丢历史 suite |
| CQ-T40 | 退回 baseline/off | 不触碰 required policy、保留告警事实 |

对于真实检测控制样本，优先采用临时、不可提升为生产凭据的 fixture。它们的结果不能当作仓库安全扫描的替代，也不上传到 main 对应 category 污染真实告警。

## 16. 性能预算与验收

### 16.1 强制结构性指标

- required DAG 新增 CodeQL 等待边数：0。
- CodeQL 参与普通 CI-ready/review/integration digest 的字段数：0。
- 纯文档 PR 重型扫描任务数：0。
- 默认扫描中主动调用完整业务 build/test/packaging 的次数：0。
- 选中单一语言的 PR 安装其他业务语言工具链的次数：0。
- CodeQL 重型任务仓库级并发峰值：不超过配置的 1 或 2。
- 已确认观察性 CodeQL 状态单独造成的误阻塞：0。
- 由于不同 PR 共用去重 group 造成的任务丢失：0。

### 16.2 需实测的指标

采集 planner 计算耗时（不混 runner 排队）、扫描排队、setup/dependency、extraction、query、upload、runner-minutes、cache hit、latest-head 完成率、完整 main 覆盖年龄。

同时观察原 required CI 的排队、运行及合入延迟，按文档、harness、Rust 包等类型匹配样本比较，不能用全部 PR 的混合均值掩盖回归。

建议 planner 本体 P95 不超过 5 秒。required 排队回归报警阈值可暂定为相对增加 10% 或绝对增加 30 秒中较大者。上述数值只是调优和回退预算，不是已达成指标；样本不足不能宣布性能验收通过。

不承诺 Rust 扫描能在固定几分钟内结束，不承诺共享 GitHub-hosted runner 上零资源竞争。若耗时不达标，先排查重复提取、依赖下载、无效矩阵与环境安装，再考虑真正独立的 package 单元；不以降低覆盖或伪造结果解决。

## 17. 完成定义与实施禁区

完成本次接入必须同时有：规范合入；四语言实现与静态/负例测试合入；实际默认与扩展基线；扫描来源可验证；观察态已启用并真实读回；两个槽位/取消行为验证；coverage gap 明示；原 required 保护不变；运营 owner 和回退路径明确。

**“完整接入”不等于“强制门禁上线”。** 后续启用漏洞阈值保护会引入等待/覆盖要求，必须重新评估按需跳过、所有 category 的基线、fork、平台规则与 gate 行为。不能简单把四个 matrix job 名称勾为 required。

禁止事项：为每个语言/阶段开独立合入 PR；修改业务 crate 只为适配扫描；用 GitHub Issue/PR 回填身份阻挡扫描启动；全局忽略 UNSTABLE；任意 bot 白名单；共享单 pending group 丢任务；以 continue-on-error 或空 SARIF 伪造成功；每次 main push 重跑四语言全量；在仓库提交高频生成的扫描状态总表；为扫描增加常驻服务。

## 18. 参考来源

### 仓库快照

- R1：main 分支与可见 required status。`https://api.github.com/repos/eng-cc/oasis7/branches/main`
- R2：本次读取的仓库有效 ruleset 详情。具体实例定位保留在本次审查记录，长期规范通过 rulesets API 动态发现。`https://api.github.com/repos/eng-cc/oasis7/rulesets`
- R3：`scripts/pm/pr-lifecycle-gate.py`，审查基线，尤其 `decision` 与 policy discovery。
- R4：`scripts/ci-required-scope.v2.json`，审查基线。
- R5：根 `Cargo.toml` 与 `crates/oasis7/Cargo.toml`，审查基线。
- R6：`.github/workflows/rust.yml` 与现有 capabilities，审查基线。
- R7：`doc/engineering/workflow/source-of-truth.md`，审查基线。
- R8：`scripts/ci-tests.sh` 的 `run_rustsec_advisory_check`，审查基线。

仓库源码引用统一按审查基线读回；落仓时不要把本次具体任务/PR/run 身份写成固定生产规则。

### 官方平台资料（本次核对）

- E1：CodeQL build options，Rust none 与 build.rs/宏执行。`https://docs.github.com/en/code-security/reference/code-scanning/codeql/build-options-for-compiled-languages`
- E2：支持语言、Rust edition 与 nightly 限制。`https://codeql.github.com/docs/codeql-overview/supported-languages-and-frameworks/`
- E3：2026-03-24 增量分析更新。`https://github.blog/changelog/2026-03-24-faster-incremental-analysis-with-codeql-in-pull-requests/`
- E4：2026-06-10 增量分析扩展。`https://github.blog/changelog/2026-06-10-incremental-analysis-for-go-c-c-and-codeql-cli/`
- E5：workflow/job concurrency、queue 与容量。`https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency`
- E6：2026-05-07 queue 扩展公告。`https://github.blog/changelog/2026-05-07-github-actions-concurrency-groups-now-allow-larger-queues/`
- E7：workflow syntax 与路径过滤限制。`https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax`
- E8：PR 触发安全、merge checkout 与特权触发风险。`https://docs.github.com/en/actions/reference/security/securely-using-pull_request_target`
- E9：code scanning merge protection。`https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/manage-your-configuration/set-merge-protection`
- E10：CodeQL 配置、category、paths 与查询/threat model。`https://docs.github.com/en/code-security/reference/code-scanning/workflow-configuration-options`
- E11：CodeQL Action init 输入。`https://raw.githubusercontent.com/github/codeql-action/v4/init/action.yml`
- E12：CodeQL Action analyze 输入。`https://raw.githubusercontent.com/github/codeql-action/v4/analyze/action.yml`

本文是待审查的规范设计；扫描实现与激活仍待分别验证，不是扫描已通过或生产已激活的证明。
