# 文档治理维护手册

本手册只说明维护操作；组织规则、权威边界与裁定原则以 [`doc-structure-standard.design.md`](doc-structure-standard.design.md) 为准。

系统设计正文按 [`system-design-writing-standard.design.md`](system-design-writing-standard.design.md) 的十二段骨架、需求承接和验证映射维护；项目管理记录按 [`project-management-record-standard.design.md`](project-management-record-standard.design.md) 维护。生命周期、权限、任务状态和门禁仍只从 [`workflow source of truth`](../workflow/source-of-truth.md) 读取。

## 1. Intake 与分类

1. 先识别读者对象与文档职责，再选择目录和后缀。
2. 产品承诺、跨域组合和端到端成功标准，进入既有 `doc/product/` 四模块之一；实现合同、指标、测试、运维与历史证据留在专业域。
3. 不确定时暂停建档，记录候选目录、冲突的权威和需要裁定的问题，交由对应 domain owner 与 `repository_health_engineer` 复核；不要以新根级目录或双写规避判断。
4. 专业技术设计使用系统设计写作规范；任务、变更与验收事实使用项目管理记录方法，PR 可以直接承载任务，Issue/Project 按需，不在产品或设计正文复制可变状态。

### 1.1 新建与实质变更产品文档的作者步骤

适用路径是 `doc/product/**/*.prd.md` 与 `doc/product/**/*.design.md`。作者在写作前先确认所属四模块、上位 PRD、产品语义 owner、专业 authority 和是否需要同名配对 design，然后按 canonical 规范和模板填写。根 PRD 保留既有身份、SC、章节顺序与六列追踪；专题 PRD 写代表性情境、正常路径、关键选择、代价/风险、失败与恢复、REQ/AC、证据边界和未决问题；design 可以用带路径与 fragment 的 Markdown 链接承接 paired PRD 的 REQ/AC，不复制一份需求。

提交前必须用实际比较范围运行产品内容准入。`base`、`head` 和工作树路径必须显式提供，命令不得用作者标签、评论或 allowlist 跳过：

内容门禁使用固定版本的 `markdown-it-py` 解析真实 CommonMark link nodes；作者环境先运行 `python3 -m pip install -r scripts/doc-governance-requirements.txt`。CI 在 required/full test tier 前安装同一依赖，缺少解析器时门禁失败，不回退到正则伪解析。

```bash
OASIS7_PRODUCT_DOC_BASE=<base-ref> \
OASIS7_PRODUCT_DOC_HEAD=<head-ref> \
./scripts/doc-governance-check.sh

# 需要单独观察 changed-product 内容门禁时：
python3 scripts/product-doc-content-check.py \
  --repo-root <worktree-path> \
  --base <base-ref> \
  --head <head-ref> \
  --worktree
```

上面的显式参数是作者提交、CI 和 PR-prep 的准入合同。普通本地维护若完全没有这些 env 且没有 CI 事件输入，旧的 `doc-governance-check.sh` 可以为兼容性推导 `base=merge-base(HEAD, main)`、`head=HEAD`；显式参数只缺一项，或 CI 事件缺失/部分/格式错误，必须失败，不能借此回退到本地推导。

准入会对新增、复制、重命名和去除空白行/行尾空白/HTML 注释后仍有内容差异的修改运行检查；纯空白/HTML 注释差异按同一机械规则排除，行首缩进变化仍是实质内容，其他修改没有格式豁免。比较范围是 `merge-base(base, head)...head`：`base` 为集成目标、`head` 为待集成源，target-only 变化不进入源范围；`--worktree` 还纳入 staged、unstaged 和 untracked 产品文档。作者/CI/PR-prep 合同中的 base/head/event 缺失、部分存在或格式错误时必须失败，不得静默退回另一个范围。作者必须修复所有失败项：文档类 metadata 与最低内容、真实 authority 路径及 fragment、显式 REQ/AC 引用、局部 ID/HTML anchor 唯一性与可达性。fenced code block 会先从正文剔除，仅作示例，不计入声明或引用；REQ/AC 块和追踪表格的每个关联必须解析到本地声明/anchor 或带路径和 fragment 的真实跨文件 Markdown 链接，不能用其他位置偶然出现的 token 替代。通过门禁不代替语义评审；负责人按实际复杂度、风险和专业问题安排适用评审，不要求所有角色逐一签收。

若变更只涉及产品文档，仍应运行 `./scripts/doc-governance-check.sh`、`python3 scripts/product-doc-governance-check.test.py`、`./scripts/readme-link-check.sh` 和 `git diff --check`；PR-prep/CI 会用同一 base/head 合同再次执行，不能把本地通过外推成实现或发行结论。

### 1.1 过程信息归位

先按[流程规范的信息归属](../workflow/source-of-truth.md#information-record-placement)区分长期合同、过程、原始证据与派生摘要。具体接收/变化记录使用[项目管理记录方法](project-management-record-standard.design.md)。迁移旧标题时，兼容 fragment 应紧邻对应替代条款或历史记录入口；链接能解析不等于仍指向正确内容。同步文档时更新长期方法/边界；实际测试计数、候选/CI 等待、重试、评审修复和下一轮安排不追加到长期正文。拆分原始 evidence 时保留运行身份、原判定与真实消费者，按本手册的局部对象/组操作处理适用 hash，不用全量 sync 或改日期制造新鲜度。

## 2. 产品语义迁移

1. 逐文件清点待迁移源中的产品语义、专业语义与活跃引用。
2. 先将稳定产品语义回填到正确产品模块，再保留或更新专业域权威。
3. 修复活跃引用并运行治理检查；只有产品回填完整、专业权威未丢失时才删除已吸收的源。
4. 源仍须保留时，记录剩余语义、目标权威和删除条件；不得把同一产品承诺长期双写。
5. 每次触达式迁移都按模板的“语义迁移账目卡”把源条款、目标锚点、语义分类、未迁移部分、当前 authority、接收 owner、活跃引用、删除条件和验证结果写回当前 PR 与实际验证记录；产品目录不新增执行台账。
6. `world-rule.md` 或旧附件只能作为背景来源。未核对的旧名称、路径、资源或规则值必须标记为历史/待核对，并链接当前专业 authority，不得写成当前事实。

## 3. 顶层目录与例外

1. 新增、删除或重分类 `doc/` 一级目录时，同批更新 `doc/.governance/top-level-directory-registry.json`、`doc/README.md` 与相关 landing page。
2. registry 的 `entry` 必须是存在的根入口，并在 `doc/README.md` 可达；目录类型和 owner 要反映实际职责。
3. 例外目录必须填写进入条件、复核触发器和退出条件。短周期样本、历史摘要不得演变为未注册的长期模块。

## 4. 同步与验证

1. 先更新 canonical Design；再更新本手册、README/index、registry 与 checker/test 投影。
2. 运行 `./scripts/doc-governance-check.sh`、`bash ./scripts/doc-governance-check.test.sh`、`./scripts/readme-link-check.sh` 与 `./scripts/doc-inventory-report.sh`。
3. 任一检查失败时，不用 allowlist 或例外描述掩盖问题；先修正分类、入口或过期措辞，再复跑。

### 4.0 全量历史内容与专业设计扫描

需要查看存量缺口时，在真实仓库工作树运行：

```bash
# 保留当前变更严格检查，再独立报告历史内容缺口：
OASIS7_PRODUCT_DOC_BASE=<base-ref> \
OASIS7_PRODUCT_DOC_HEAD=<head-ref> \
./scripts/doc-governance-check.sh --full-corpus

# 仅查看专业设计追踪或产品/专业核心结构的全量历史报告：
python3 scripts/system-design-traceability-check.py --repo-root <worktree-path> --full-corpus
python3 scripts/doc-core-content-check.py --repo-root <worktree-path> --full-corpus
```

专业设计扫描覆盖当前 `doc/` 下适用的 `*.design.md`，不要求文件已经具有追踪表格，也不依赖 Git diff。产品、小说/fiction、`.governance` 目录和既有规范类 overlay 排除；其余适用专业设计的各生命周期均参与核对，不因此自动恢复旧义务或修改正文。core 全量模式同时覆盖产品文档和这些专业设计；产品身份与生命周期规则仍由产品 checker 维护。

模块根 `doc/<module>/design.md` 同样纳入 inventory，并分别报告专题合同、模块总览、采用追踪合同的模块根数量。纯模块总览按架构、组件职责、主链路、数据/状态流与专题导航核对核心章节，不强套专题 `2.1`/`11.1` 表；如果根文档显式采用任一标准追踪表标题，则核对完整追踪合同，空表或损坏表也不能豁免。所有根路径逐项可见；模块根纳入增量关系检查，基线已采用合同或当前存在标准映射标题/表格时仍按合同核对，不能删除、改名或隐藏标题后退为总览。

这两个单独的 `--full-corpus` 入口默认将历史内容缺口报告为可定位警告，且与 `--base`、`--head`、`--worktree` 互斥。聚合入口中的 base/head 环境变量只交给前置严格变更检查，历史调用不带范围参数；全量报告不能覆盖或吞掉增量失败。输入不合法、Git 范围/仓库身份错误、依赖缺失、实际读取失败及不安全路径等仍是硬失败，不可用 `|| true` 吞掉。报告为零只说明已扫描范围内未发现适用机械问题，不代表设计完备、实现完成或验收通过；只读 Trace ID 查询工具也不代替这些内容检查。

### 4.0.1 产品功能与专业设计关系的作者核对

1. 按功能/要求的适用义务核对产品声明、Trace ID、专业条款和验证映射，允许多对多；不比较两边文件数量，不为同名配对而新增专业设计文件。
2. 产品适用专题的准确上游可直接指向产品 REQ/SC/AC，或经专业条款继续链接要求声明。AC/专业中间节可用 `Trace refs` 引用已有 ID，逐跳链接其单一声明，无须每节重新声明。既有表头/单元格 Trace 字段、inline-code 标签/值及多个 refs 同样适用；被引条款之外的偶然 ID、泛首页、无关 README/锚点不能代替关系。循环或深度超限不视为成功。 无显式 refs 的专业条款仅核对自身身份和直接准确产品引用，不沿一般专业导航递归；多层间接承接引用已有同一 ID 与准确链，不为此给历史条款新发 ID 或重复声明。
3. 承接产品要求的专业中间层不写产品 N/A；只有工程-only 专题保留 `doc/` 下真实 Markdown 专业规则、手册或缺陷条款等上游 fragment，在承接行或本地条款说明产品 N/A 的具体理由与范围；这不免除专业追踪和验证合同。
4. 产品“未核实/未设计”如实报告缺口，无须伪造设计链接；“部分覆盖”列清已覆盖和剩余义务。“部分覆盖/已覆盖”的每个设计依据应能定位到唯一专业条款，并由其承接表直接或经准确 AC/专业条款链反查当前功能/要求或同一适用 Trace ID。“已覆盖”由人工核验实际设计覆盖范围，不能只看检查通过。
5. 设计与实现分别核对；`剩余范围` 写清是哪一层的缺口，设计已覆盖但实现未完成时保留实际实现缺口，不强制填“无”。模块根负责 overview 时保持架构/职责/链路/导航；已有 contract 不可删标题规避，迁移须保留真实承接定位并核对旧义务。
6. 修改或删除设计目标时，核对未改产品 PRD 的正向设计引用。core 会将 merge-base 到当前新增的无效依据/功能关系错误按本次变更严格处理；已有相同失效和未改 PRD 的旧字段缺口不因无关变化升级阻断。
7. 运行上述 scoped strict 与独立历史 full warning；只对真实变更及受影响关系作结论，不用历史缺口迫使本 PR 全仓回填。机械关系有效不证明功能枚举完整、设计语义充分或验收成功，人工核验结果与未覆盖边界记录在当前 PR。

工程-only 的适用性说明可写在实际承接行或对应本地设计条款中，例如 `产品适用性：N/A；reason=仅维护构建检查且不改变玩家承诺；scope=本设计的构建检查义务`。理由和范围必须替换为真实内容，不能填写“无”“待补”或 `-`；产品适用性 N/A 与验证方法 N/A 是两件事，后者仍按既有合同完整填写 reason、scope、owner_role、evidence_ref、re-evaluate，不能借前者省略验证处置。


### 4.1 歧义裁决记录

遇到目录归属、产品/专业权威或例外存续歧义时，在当前 PR 与实际验证记录 中记录：观察到的路径/段落、候选分类与理由、涉及的专业权威、建议裁决、裁决 owner、同步文件集、验证命令和 residual risk。领域语义由对应 domain owner 裁决；`repository_health_engineer` 裁定结构一致性；负责人整合实际结果。

裁决落定后按此顺序同步：canonical Design（若规则改变）→ registry（若一级目录或例外改变）→ landing README/index → 本手册（仅操作步骤变化时）→ checker/test。最后将实际输出和未解决风险追加到同一 PR 或已有验证记录，避免把裁决另写成新的本地台账。

### 4.2 失败签名分流

| 失败签名 | 先做什么 | 路由 |
| --- | --- | --- |
| `top-level-directory-registry` 的 directory set / required field / owner / entry / navigation / lifecycle | 对照 registry、根 README 和目录 landing page，先定位不一致的一项 | `repository_health_engineer`；涉及目录业务含义时追加相应 domain owner |
| `project.md` 或 stale project-ledger wording | 区分历史描述与当前鼓励性文案；当前文案改为 Git、PR 与实际 CI 记录 | `repository_health_engineer` |
| missing markdown path 或 README link check | 修复真实入口、引用或退役说明，不添加虚假文件 | 文档所属 domain owner；结构问题由 `repository_health_engineer` |
| product overlay contract | 回到产品模块入口与专业 authority backlink，不能通过增设第五模块规避 | `producer_system_designer`，必要时加对应专业 owner |
| inventory 的 density / age / duplication trigger | 先由 QA 判定证据是否仍有效，再制定聚合、归档或删除复核 | `qa_engineer` + `repository_health_engineer` |

### 4.3 常见错误示例

- 想表达一个短期 UI 样本池，直接新建未登记的 `doc/ui-notes/`：先记录用途和退出条件，走 registry/根导航同步，而不是让目录自行成为模块。
- 想追踪产品迁移时，误建本地任务台账或把设计文档与本地执行台账描述为固定配对：把计划、状态和证据写回 Git、PR 与实际 CI 记录，产品目录只保留产品文档。
- 为了让检查通过，把已删除专题的链接换成不存在的 placeholder：改为真实现行 authority 或明确的历史追溯说明。
- evidence 数量超过触发器后，仅在 README 增加更多链接：先发起 QA 有效性与 repository-health 结构复核，再选择后续处置。

### 4.3 Document corpus inventory v3

`doc/engineering/doc-governance/document-corpus-inventory-v3.design.md` 定义全量 `doc/**` 覆盖、静态入口、逐对象与 semantic/evidence 分片、trusted loader 和迁移技术要求，并保留原审读基线的 loop 适配背景。当前 checker 的实际能力以已合并实现及其实际接线和验证为准；现有存储校验和受信边界继续有效，设计或候选代码本身不代表启用。历史 loop/activation 包装不构成普通开发前置，通用流程只由 workflow source of truth 定义。切换完成后，使用 `scripts/document-corpus-inventory.py locate` 查看精确记录端点和组成员，使用 scoped `sync` 更新机器对象，使用 `propose` / `apply-review` 处理语义或 evidence 复核分片。`check` 与两个保留的 checker 入口执行同一模型；`export` 只输出衍生报告。不得使用 `--all` 扩大任务写范围或把自动同步解释为审批。

## 5. Evidence 生命周期

`qa_engineer` 决定证据有效性与保留语义；`repository_health_engineer` 维护密度、导航和阈值。`doc/testing/evidence/README.md` 规定 count、age 与重复触发器；触发后选择聚合、归档或删除复核，本手册不授权直接批量删除证据。

## 6. 完成检查

- 规则变更已先落入 canonical Design。
- 根目录 registry、根 README、例外生命周期和 checker 一致。
- 产品层仍只有四个模块入口，且专业权威没有被产品文本取代。
- Git、PR 与实际 CI 记录 记录任务、状态与证据；仓库未新增 project ledger。
- 验证输出已附回当前 PR 或已有验证记录，并按需路由 QA/domain review。
