# 文档治理维护手册

本手册只说明维护操作；组织规则、权威边界与裁定原则以 [`doc-structure-standard.design.md`](doc-structure-standard.design.md) 为准。

## 1. Intake 与分类

1. 先识别读者对象与文档职责，再选择目录和后缀。
2. 产品承诺、跨域组合和端到端成功标准，进入既有 `doc/product/` 四模块之一；实现合同、指标、测试、运维与历史证据留在专业域。
3. 不确定时暂停建档，记录候选目录、冲突的权威和需要裁定的问题，交由对应 domain owner 与 `repository_health_engineer` 复核；不要以新根级目录或双写规避判断。

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

准入会对新增、复制、重命名和去除空白行/行尾空白/HTML 注释后仍有内容差异的修改运行检查；纯空白/HTML 注释差异按同一机械规则排除，行首缩进变化仍是实质内容，其他修改没有格式豁免。比较范围是 `merge-base(base, head)...head`：`base` 为集成目标、`head` 为待集成源，target-only 变化不进入源范围；`--worktree` 还纳入 staged、unstaged 和 untracked 产品文档。作者/CI/PR-prep 合同中的 base/head/event 缺失、部分存在或格式错误时必须失败，不得静默退回另一个范围。作者必须修复所有失败项：文档类 metadata 与最低内容、真实 authority 路径及 fragment、显式 REQ/AC 引用、局部 ID/HTML anchor 唯一性与可达性。fenced code block 会先从正文剔除，仅作示例，不计入声明或引用；REQ/AC 块和追踪表格的每个关联必须解析到本地声明/anchor 或带路径和 fragment 的真实跨文件 Markdown 链接，不能用其他位置偶然出现的 token 替代。通过门禁后仍需按对应专业 role 完成产品、规则、交互和证据评审。

若变更只涉及产品文档，仍应运行 `./scripts/doc-governance-check.sh`、`python3 scripts/product-doc-governance-check.test.py`、`./scripts/readme-link-check.sh` 和 `git diff --check`；PR-prep/CI 会用同一 base/head 合同再次执行，不能把本地通过外推成实现或发行结论。

## 2. 产品语义迁移

1. 逐文件清点待迁移源中的产品语义、专业语义与活跃引用。
2. 先将稳定产品语义回填到正确产品模块，再保留或更新专业域权威。
3. 修复活跃引用并运行治理检查；只有产品回填完整、专业权威未丢失时才删除已吸收的源。
4. 源仍须保留时，记录剩余语义、目标权威和删除条件；不得把同一产品承诺长期双写。
5. 每次触达式迁移都按模板的“语义迁移账目卡”把源条款、目标锚点、语义分类、未迁移部分、当前 authority、接收 owner、活跃引用、删除条件和验证结果写回当前 GitHub task evidence；产品目录不新增执行台账。
6. `world-rule.md` 或旧附件只能作为背景来源。未核对的旧名称、路径、资源或规则值必须标记为历史/待核对，并链接当前专业 authority，不得写成当前事实。

## 3. 顶层目录与例外

1. 新增、删除或重分类 `doc/` 一级目录时，同批更新 `doc/.governance/top-level-directory-registry.json`、`doc/README.md` 与相关 landing page。
2. registry 的 `entry` 必须是存在的根入口，并在 `doc/README.md` 可达；目录类型和 owner 要反映实际职责。
3. 例外目录必须填写进入条件、复核触发器和退出条件。短周期样本、历史摘要不得演变为未注册的长期模块。

## 4. 同步与验证

1. 先更新 canonical Design；再更新本手册、README/index、registry 与 checker/test 投影。
2. 运行 `./scripts/doc-governance-check.sh`、`bash ./scripts/doc-governance-check.test.sh`、`./scripts/readme-link-check.sh` 与 `./scripts/doc-inventory-report.sh`。
3. 任一检查失败时，不用 allowlist 或例外描述掩盖问题；先修正分类、入口或过期措辞，再复跑。

### 4.1 歧义裁决记录

遇到目录归属、产品/专业权威或例外存续歧义时，在当前 GitHub task evidence 中记录：`task_uid`、观察到的路径/段落、候选分类与理由、涉及的专业权威、建议裁决、裁决 owner、同步文件集、验证命令和 residual risk。领域语义由对应 domain owner 裁决；`repository_health_engineer` 裁定结构一致性；TPM 记录 task truth 并安排合流。

裁决落定后按此顺序同步：canonical Design（若规则改变）→ registry（若一级目录或例外改变）→ landing README/index → 本手册（仅操作步骤变化时）→ checker/test。最后将实际输出和未解决风险追加到同一 task evidence，避免把裁决另写成新的本地台账。

### 4.2 失败签名分流

| 失败签名 | 先做什么 | 路由 |
| --- | --- | --- |
| `top-level-directory-registry` 的 directory set / required field / owner / entry / navigation / lifecycle | 对照 registry、根 README 和目录 landing page，先定位不一致的一项 | `repository_health_engineer`；涉及目录业务含义时追加相应 domain owner |
| `project.md` 或 stale project-ledger wording | 区分历史描述与当前鼓励性文案；当前文案改为 GitHub task truth | `repository_health_engineer` |
| missing markdown path 或 README link check | 修复真实入口、引用或退役说明，不添加虚假文件 | 文档所属 domain owner；结构问题由 `repository_health_engineer` |
| product overlay contract | 回到产品模块入口与专业 authority backlink，不能通过增设第五模块规避 | `producer_system_designer`，必要时加对应专业 owner |
| inventory 的 density / age / duplication trigger | 先由 QA 判定证据是否仍有效，再制定聚合、归档或删除复核 | `qa_engineer` + `repository_health_engineer` |

### 4.3 常见错误示例

- 想表达一个短期 UI 样本池，直接新建未登记的 `doc/ui-notes/`：先记录用途和退出条件，走 registry/根导航同步，而不是让目录自行成为模块。
- 想追踪产品迁移时，误建本地任务台账或把设计文档与本地执行台账描述为固定配对：把计划、状态和证据写回 GitHub task truth，产品目录只保留产品文档。
- 为了让检查通过，把已删除专题的链接换成不存在的 placeholder：改为真实现行 authority 或明确的历史追溯说明。
- evidence 数量超过触发器后，仅在 README 增加更多链接：先发起 QA 有效性与 repository-health 结构复核，再选择后续处置。

## 5. Evidence 生命周期

`qa_engineer` 决定证据有效性与保留语义；`repository_health_engineer` 维护密度、导航和阈值。`doc/testing/evidence/README.md` 规定 count、age 与重复触发器；触发后选择聚合、归档或删除复核，本手册不授权直接批量删除证据。

## 6. 完成检查

- 规则变更已先落入 canonical Design。
- 根目录 registry、根 README、例外生命周期和 checker 一致。
- 产品层仍只有四个模块入口，且专业权威没有被产品文本取代。
- GitHub task truth 记录任务、状态与证据；仓库未新增 project ledger。
- 验证输出已附回当前 task evidence，并按需路由 QA/domain review。
