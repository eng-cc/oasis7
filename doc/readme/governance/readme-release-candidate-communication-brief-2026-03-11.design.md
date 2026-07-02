# oasis7: 版本候选对外口径简报（2026-03-11）设计

- 对应需求文档: `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.prd.md`
- 对应项目管理文档: `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.project.md`

审计轮次: 4

## 1. 设计定位
将内部版本候选 `go/no-go` 结论翻译为可对外复用的简报层，并保持“事实、边界、风险、回滚”四段式结构。

## 2. 设计结构
- Status Layer：当前候选状态与适用范围。
- Messaging Layer：推荐表述与禁用表述。
- Risk Layer：残余风险与不夸大说明。
- Handoff Layer：`liveops_community -> producer_system_designer` 审核闭环。

## 3. 关键接口 / 入口
- `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.prd.md`
- `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.project.md`
- `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.prd.md`
- `doc/core/reviews/release-candidate-go-no-go-version-2026-03-11.md`

## 4. 约束与边界
- 只输出对外口径摘要，不替代 changelog / announcement。
- 不写超出内部证据的能力承诺。
- 异常沟通必须与内部 rollback note 保持一致。

## 5. 设计演进计划
- 先形成首份版本候选简报。
- 再视需要扩成固定 release communication 模板。
- 后续若进入真实发布节奏，再考虑同步根 `README.md` 或站点文案。
