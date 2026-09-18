# oasis7: CI 与提交钩子测试分级设计

- 对应需求文档: `doc/testing/ci/ci-tiered-execution.prd.md`
- 可变任务状态与历史: GitHub task issue evidence comments

## 1. 设计定位
定义 CI 与测试门禁专题设计，统一流水线分层、门禁策略、产物校验与失败保护。

## 2. 设计结构
- 流水线分层：按 `commit` / `required` / `full`、runner、target 或专题阶段划分执行链路。
- required-scope 规划层：在保持 `required-gate` 单一上下文不变的前提下，先按 changed paths 规划 `minimal / targeted / full`，再决定哪些重型组件实际执行。
- 门禁策略层：定义通过条件、阻断条件与 required check 保护。
- 校验执行层：收敛构建、测试、hash/determinism 等自动校验入口。
- 回归治理层：沉淀失败签名、发布影响与后续演进。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [Cargo package scope and impact-scoped integration verification](../../engineering/workflow/source-of-truth.md#cargo-package-scope-and-impact-scoped-verification) | 将 package identity、`H/B/T`、`B -> T` impact selection 与 high-risk escalation 投影到 CI 分级设计；仅在 trusted analysis 下允许 impact-scoped `integration_revalidation`。 | [包身份与 exact integration 目标语义](ci-tiered-execution.design.md#package-aware-exact-integration-target) | `producer_system_designer` 定义语义；runtime/QA/CI 实现与验证另行负责。 | 本次不实现 planner、workflow、Cargo metadata 解析或 enforcement；当前 path-based 行为保持现状。 |

<a id="package-aware-exact-integration-target"></a>
### 2.2 包身份与 exact integration 目标语义（批准目标；当前实现未强制）

- 规范来源是 [workflow source of truth 的 canonical clause](../../engineering/workflow/source-of-truth.md#cargo-package-scope-and-impact-scoped-verification)，不是本设计的第二套 authority。
- V0 activation boundary：write/migration boundaries、high-risk escalation 与 fail-closed behavior 立即按 canonical clause 生效；reduced package-aware `integration_revalidation` 在 trusted merged producer、receipt、gate implementation 与 explicit activation 具备前保持 inactive，当前 conservative/full behavior 保持权威。
- 批准目标以 Cargo manifest 与解析后的 `cargo metadata` 图确定 package identity；普通 Rust code PR 保持单一 package 边界，CI/harness PR 不改变 business package，product/system-document PR 不含 code。
- exact integration 绑定 `H`（source head）、`B`（target commit）与 `T`（tested tree），选择真实 `B -> T` impact，不把 `B..H` 当作 integration impact；planner 还要冻结 package、rules、profiles、commands/results、toolchain、targets/features 与 policy version。
- `integration_revalidation` 仅在 trusted analysis 证明 impact-scoped 足够时可用；unknown impact 与 high-risk API/default/feature/dependency、ABI/signature、state-root/persistence/consensus profiles 进入 `full_escalation`。取消、超时、缺失或 unexpected skip fail closed。
- 当前 planner 仍以 changed paths/config rules 计算 capability；`scope=full` 只是 required tier 内的覆盖扩张。本设计只记录批准目标与当前差距，不声称 package-aware `H/B/T` enforcement 已存在。

## 3. 关键接口 / 入口
- `pre-commit` legacy hook 静默 no-op 入口；`commit` tier 仅供显式调用
- CI workflow / check 入口
- `scripts/plan-rust-required-scope.sh`
- 门禁/required check 配置
- runner/target/产物校验点
- CI 回归与失败签名

## 4. 约束与边界
- 门禁变更必须可审计、可回放。
- 本地默认提交路径与显式 required / full 门禁需边界清晰。
- changed-path 剪裁只能作用于 CI `required-gate`，不得改变本地显式 `./scripts/ci-tests.sh required` 的语义。
- 当前 path-based planner 命中共享 CI / gate 输入、diff base 不可解析或路径未分类时，必须在 required tier 内回退 full；这不是批准目标的 `full` tier 选择，也不替代基于 package identity 与 `H/B/T` 的 exact integration。
- 不在本专题重构整个平台 CI 基础设施。

## 11. 验证设计与可追溯性

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [Cargo package scope and impact-scoped integration verification](../../engineering/workflow/source-of-truth.md#cargo-package-scope-and-impact-scoped-verification) + `PRD-TESTING-CI-TIERED-004` | [ci-tiered-execution.design.md#package-aware-exact-integration-target](ci-tiered-execution.design.md#package-aware-exact-integration-target) | 文档必须把批准目标的 package identity、`H/B/T` 与 `B -> T` impact selection 映射到 CI 分级；立即 normative 的 write/migration、high-risk、fail-closed obligations 必须保留；当前 path-based planner 的非 package-aware status 与 reduced-route inactive boundary 必须成为 negative assertion。 | [ci-required-scope-audit-contract.test.sh](../../../scripts/ci-required-scope-audit-contract.test.sh) 的 required-tier planner/current-policy contract；`PRD-TESTING-CI-TIERED-004` 的 V0 文档/negative-boundary review；本次还运行 `./scripts/doc-governance-check.sh` 与 `git diff --check`。Future activation requires trusted Cargo metadata, `H/B/T` impact, producer/receipt/gate identity, and high-risk/full fail-closed receipt evidence. | 当前 task evidence 与 V0 negative assertion；future trusted CI receipt plus explicit activation record before reduced `integration_revalidation`. | 本次不证明 planner 已执行 package-aware selection、`H/B/T` binding、trusted reduced integration revalidation 或 full-escalation activation。 |

## 5. 设计演进计划
- 先冻结门禁与执行分层。
- 再补 `required-gate` 的 changed-path planner 与保护策略。
- 后续独立实现阶段再把 Cargo metadata/package identity 与 `H/B/T` exact integration 接入 planner；在实现与 trusted evidence 具备前，不得把本设计目标当作当前 enforcement。
- 最后固化失败签名与回归。
