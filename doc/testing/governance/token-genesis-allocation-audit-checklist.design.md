# oasis7 主链 Token 创世分配审计清单设计

- 对应需求文档: `doc/testing/governance/token-genesis-allocation-audit-checklist.prd.md`
- 对应项目管理文档: `doc/testing/governance/token-genesis-allocation-audit-checklist.project.md`

## 1. 设计定位
提供一份 `qa_engineer` 侧的创世配置门禁设计，让 Token mint 前至少经过一轮结构化 QA 审计，而不是只靠 producer/runtime 自审。

## 2. 设计结构
- 输入层：读取 `TIGR-1` 创世参数表与 producer 口径。
- 审计层：按比例、上限、流通、语义四个维度逐项判定。
- 输出层：统一输出 `pass / block / conditional_draft_only`。
- 回流层：把结论回写到 `testing` 与 `p2p token` 项目追踪。

## 3. 关键接口 / 入口
- `doc/p2p/token/mainchain-token-genesis-and-contributor-reward.project.md`
- `doc/testing/evidence/token-genesis-allocation-audit-template.md`
- `crates/oasis7/src/runtime/main_token.rs`

## 4. 约束与边界
- checklist 只做 QA 门禁，不替代 producer 的经济决策。
- `conditional_draft_only` 不是放行结果；模板存在或比例草案正确都不能替代真实证据、mint 决策或发布结论。
- 若控制主体未从逻辑名映射到真实多签/治理主体，不得给最终 `pass`。

## 5. 设计演进计划
- 先建模板和阻断规则。
- 再在真实创世冻结前跑第一次正式审计。
- 后续若 early contributor distribution fully on-chain，再扩展新审计项。
