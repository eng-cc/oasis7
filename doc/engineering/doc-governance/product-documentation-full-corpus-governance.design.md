# 产品文档全量治理设计

- 上游内容规范：[`product-documentation-standard.design.md`](product-documentation-standard.design.md)
- 产品树入口：[`doc/product/README.md`](../../product/README.md)
- Owner role：`producer_system_designer`
- 治理协作：`repository_health_engineer`
- 状态：`active`
- Last reviewed：2026-09-13

本文定义如何把已经采纳的产品文档内容规范应用到全部存量产品文档，并在不破坏专业 authority 的前提下收口外部产品语义。它补充全量治理机制，不改变四模块产品树、专业职责或 GitHub task truth。

## 1. 目标与完成边界

全量治理完成必须同时满足：四个模块根 PRD 保持唯一入口和汇总职责；每个 active 专题达到现行内容合同；复杂专题在适用时拥有配对 design；外部产品语义已分类并迁移或明确保留为专业 authority；全量检查与 changed-range 检查均通过。

`superseded` 与 `retired` 文档不要求恢复为 active 内容模板，但必须具备可判定的接收 authority、剩余语义、稳定引用与删除条件。历史文档不能继续承担 active 路线图、产品承诺或实现输入。

完成不表示实现、试玩、发行或公开能力已经成立。产品、专业实现、体验证据与公开 claim 继续由各自 authority 判定。

## 2. 治理单元与顺序

治理以单个产品专题或单个外部来源文件为最小单元，按以下顺序执行：

1. 建立全量、可重复的机械审计入口，列出每个文件的具体缺口。
2. 按四模块分别治理 active 根 PRD、专题 PRD 和配对 design。
3. 分类 `doc/product/` 外的产品语义，迁移产品承诺并保留专业合同。
4. 复核 retired/superseded 文档的迁移闭合与删除条件。
5. 冻结最终 HEAD，执行全量验证、涉及角色审查与 PR 流程。

每个治理单元必须在同一修改中完成内容、导航、回链、authority、REQ/AC 和必要 design；不得留下只能靠后续读者猜测的半迁移状态。

### 2.1 多义务 traceability 绑定

全量治理是一个有限多义务 change。协调 GitHub Issue 必须在任何治理单元开始前冻结：被消费的产品与专业条款、内容/检查器/验证义务、明确排除项或接受风险、每项义务的 mapping slot 与 owner，以及 aggregate candidate 的选择规则。每个治理单元只消费已声明义务，并返回自己的真实结果，不能以局部完成关闭整体义务。

整体完成前，协调记录必须绑定实际 Task、contract 与 evidence identity，以及同一棵可比较的 source、integration 和 tested tree。每个 required obligation 必须恰好映射到一个 owner-role 结果；任何缺失、重复、owner 不匹配或仍有 blocking feedback 的映射都使整体保持未完成。

执行步骤按现行 workflow 在 GitHub task evidence 中记录 `Plan-Gap Evidence`，包括 acceptance refs、依赖、验证命令与实际证据、写范围、排除范围和 required role slices。具体任务状态、mapping 与 evidence 不复制到本文或产品目录。当前 workflow 中尚未显式启用的 traceability producer、leaf-result schema、approval map 或 live-readback gate 不因本文获得 activation；实施继续使用当时有效的仓库入口和 GitHub task truth。

## 3. Active 专题内容合同

每个 active 专题 PRD 必须明确包含或等价表达：

- 所属模块、上位 PRD、生命周期、owner、专业 authority 与复核日期；
- 不依赖实现术语的玩家或使用者情境；
- 范围、Non-Goals、正常路径、核心选择、成本与风险；
- 主要失败、拒绝、失效、恢复和安全停止边界；
- 目标要求、已验证事实、设计假设与公开承诺的区分；
- 可独立失败且可观察的稳定 `REQ-*`；
- 以给定/当/则或等价方式表达的稳定 `AC-*`；
- `REQ -> AC -> product/design/professional authority -> evidence tier` 追踪；
- 证据能够证明和不能证明的范围；
- 未决问题的影响条款、决策 role、所需信息、触发条件和临时排除范围。

根 PRD 只保留模块承诺、组合关系、稳定 SC 与六列追踪。细节达到独立生命周期、状态机、资源取舍、恢复或多角色协作规模时，必须下沉到专题。

## 4. 配对 design 判定

专题存在下列任一条件时，应提供同名 `*.design.md`：非平凡状态转换；跨入口或跨角色体验；授权、成本或不可逆提交；失败恢复交互；信息分层；多个真实策略选项。

配对 design 说明信息阶段、选择理由、成本反馈、状态优先级、异常恢复、可访问性和失败信号。它通过带 fragment 的链接承接 PRD 的 REQ/AC，不复制需求，也不引入 API、schema、组件或算法 authority。

如果专业角色判断专题足够简单而不需要 design，必须在 PRD 中保留简短设计说明，并在当前 GitHub task evidence 记录不适用理由。产品树不保存长期例外台账。

## 5. 外部语义迁移

每个 `doc/product/` 外候选文件按段落分成四类：产品承诺、专业合同、历史过程、未迁移语义。

每个跨文件消费条款必须记录 canonical repository `eng-cc/oasis7`、repository-relative path 和从冻结 contract/publication 继承的 stable fragment，并在接收文档中匹配该身份。裸 token、可变 URL、本地摘要或未带 fragment 的泛化路径不能替代 consumed-clause identity。

- 产品承诺迁入对应模块根 PRD 或稳定专题，并建立旧条款到新 REQ/SC 的映射。
- 专业合同留在原专业域；产品文档只说明它支持或限制的产品结果并链接准确 fragment。
- 历史过程从 active 阅读链移除，由 Git history 与 GitHub task evidence 追溯。
- 未迁移语义必须记录接收 owner、阻塞原因、当前 authority 和可验证删除条件。

只有产品语义完整接收、专业 authority 仍可达、所有活跃引用已修复后，才删除旧来源。混合文件不得为了目录整齐整体搬迁或整体删除。

## 6. 全量检查模型

现有 changed-range 检查继续约束新增和实质修改的产品文档。另提供显式 full-corpus 模式，对当前工作树中的全部根 PRD、专题 PRD 和 design 执行同一机械内容合同。

full-corpus 模式必须：

- 输出确定的文件顺序、检查数量和稳定诊断；
- 检查身份字段、生命周期、authority、复核日期、最低内容、REQ/AC、anchor、跨文件 fragment、回链和追踪关系；
- 对 active、superseded、retired 应用各自生命周期合同；
- 不使用 allowlist、作者标签或任务评论绕过内容要求；
- 明确说明通过只证明机械合同，不证明产品质量、实现或发行状态。

canonical invocation 为：

```bash
python3 scripts/product-doc-content-check.py --repo-root <canonical-worktree> --full-corpus
```

`--full-corpus` 与 `--base`、`--head`、`--worktree` 互斥；它读取所选 canonical worktree 的全部 `doc/product/**/*.prd.md` 与 `doc/product/**/*.design.md`，按 repository-relative path 排序。成功输出稳定的 mode、checked 数量和零 error 结果并返回 0；内容缺口逐行输出稳定 error code、路径与 detail 并返回 1；参数或身份错误返回 2。checker 回归必须覆盖选择互斥、文件排序、生命周期合同、空树、失败码和诊断稳定性。

现有 changed-range 路径及其 trusted base/head、merge-base 和 worktree overlay 语义保持不变。`doc-governance-check.sh` 在当前全量治理任务及其 required CI/PR-prep 范围中显式调用 full-corpus 模式；治理合入后，仓库的持续验证必须保留全量回归，不能退回只验证一次性基线或用 full-corpus 替换作者变更范围身份。

aggregate completion 只能从全部治理单元、required obligations 和 Plan-Gap 状态推导。任一 `unknown`、`pending`、失败诊断、未关闭 gap、缺失 authority/traceability、未清 blocking feedback 或未映射 required obligation 都使结果为 incomplete/blocked；checker 通过、文件计数完成或某个模块局部 green 不能单独产生“全部治理完成”结论。

## 7. 角色与集成

- `producer_system_designer` 裁定产品承诺、模块归属、组合结果与最终内容完整性。
- `gameplay_designer` 裁定玩法循环、选择、成本与规则引用。
- `game_visual_interaction_designer` 裁定信息、反馈、异常交互和可访问性。
- 相关 runtime、WASM、Agent、Viewer、blockchain ops 角色确认专业 authority 与实现边界。
- `repository_health_engineer` 负责结构、重复、链接、checker 与迁移完整性。
- `qa_engineer` 复核 AC 可判定性、证据层级和全量验证结果。
- `liveops_community` 复核公开承诺、发行范围和玩家沟通边界。
- `tpm` 维护单一 task/worktree/PR 主链并按互斥写范围集成。

专业 slice 不创建第二任务、工作树或 PR，不在共享文件上并发写入。根入口、checker、治理规范和跨模块索引由 TPM 串行集成。

## 8. 验证与失败处理

每个模块批次先运行其全量内容检查和链接检查；跨模块集成后运行完整文档治理、checker 回归、README 链接、workflow lint 与 `git diff --check`。最终冻结 HEAD 必须由涉及角色审查。

检查失败按稳定诊断定位到文件、条款和缺失关系。语义归属冲突返回产品 owner 与专业 owner 共同裁定；无法证明专业 authority 的内容不得写成当前事实。无法形成安全恢复或可判定 AC 时，专题保持未完成，不通过治理验收。

需要行动的跨 role finding 必须保留现有 Issue comment、artifact 或 `path#fragment` 等稳定 source locator，并沿 `receiving owner -> disposition authority -> decision and basis -> authorized revision 或 explicit no-change -> affected consumer/blocking effect -> clearance evidence` 闭合。未解决的 blocking feedback 继续作为 outstanding obligation；finding 本身不授权自动派发或扩大写范围。

## 9. 非目标

- 不借文档治理改变玩法数值、runtime、WASM、Agent、Viewer 或链实现。
- 不把模板标题数量当作内容质量。
- 不为每个小主题机械创建 design。
- 不把目标态产品要求表述为当前实现或公开发行能力。
- 不在产品目录创建任务、迁移或 review 台账。
