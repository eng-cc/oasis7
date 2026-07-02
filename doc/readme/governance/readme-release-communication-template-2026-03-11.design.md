# oasis7: 对外口径简报模板（2026-03-11）设计

- 对应需求文档: `doc/readme/governance/readme-release-communication-template-2026-03-11.prd.md`
- 对应项目管理文档: `doc/readme/governance/readme-release-communication-template-2026-03-11.project.md`

审计轮次: 4

## 1. 设计定位
将首份版本候选对外口径简报抽象成固定模板，供后续候选直接实例化。

## 2. 设计结构
- Header：候选 ID、内部来源、owner、状态。
- Messaging：可对外表述、禁用表述与替代表述。
- Risk / Rollback：残余风险、回滚口径、升级入口。
- Approval：LiveOps 起草、Producer 审核。

## 3. 关键接口 / 入口
- `doc/readme/governance/readme-release-communication-template-2026-03-11.prd.md`
- `doc/readme/governance/readme-release-communication-template-2026-03-11.project.md`
- `doc/readme/governance/readme-release-communication-template-2026-03-11.md`
- `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.prd.md`

## 4. 约束与边界
- 模板不直接包含具体候选结论。
- 所有实例必须回填内部 evidence link。
- 模板服务口径稳定，不替代正式公告文体。

## 5. 设计演进计划
- 先冻结模板结构。
- 再在下一候选中复用验证。
- 后续视需要联动更正式的发布文案模板。
