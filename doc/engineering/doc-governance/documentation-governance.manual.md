# 文档治理维护手册

本手册只说明维护操作；组织规则、权威边界与裁定原则以 [`doc-structure-standard.design.md`](doc-structure-standard.design.md) 为准。

## 1. Intake 与分类

1. 先识别读者对象与文档职责，再选择目录和后缀。
2. 产品承诺、跨域组合和端到端成功标准，进入既有 `doc/product/` 四模块之一；实现合同、指标、测试、运维与历史证据留在专业域。
3. 不确定时暂停建档，记录候选目录、冲突的权威和需要裁定的问题，交由对应 domain owner 与 `repository_health_engineer` 复核；不要以新根级目录或双写规避判断。

## 2. 产品语义迁移

1. 逐文件清点待迁移源中的产品语义、专业语义与活跃引用。
2. 先将稳定产品语义回填到正确产品模块，再保留或更新专业域权威。
3. 修复活跃引用并运行治理检查；只有产品回填完整、专业权威未丢失时才删除已吸收的源。
4. 源仍须保留时，记录剩余语义、目标权威和删除条件；不得把同一产品承诺长期双写。

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
