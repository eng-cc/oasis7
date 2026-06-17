# oasis7：系统性应用测试手册工程化收口（历史决策记录）

- 当前 canonical 手册: `testing-manual.md`
- 对应设计文档: `doc/testing/manual/systematic-application-testing-manual.design.md`
- 对应项目管理文档: `doc/testing/manual/systematic-application-testing-manual.project.md`

审计轮次: 5

## 状态
- 当前状态: completed / historical
- 默认阅读面: 不作为当前测试执行入口；当前测试层级、套件矩阵、Web 闭环与证据口径以 `testing-manual.md` 为准。
- 保留原因: 记录 2026-02-26 手册工程化收口的需求决策，维持 PRD/project traceability。

## 历史决策摘要
- Problem Statement: 测试分层模型、触发矩阵与证据标准若分散在多处文档/脚本，容易出现执行口径漂移，导致“通过门禁但风险未覆盖”。
- Selected Solution: 以 `testing-manual.md` 作为统一入口，配套 Web 闭环分册与脚本入口，固化 Human/AI 共用的可审计测试流程。
- Success Criteria:
  - `testing-manual.md` 稳定承载分层模型与套件映射。
  - 手册、脚本入口与 CI 门禁口径一致。
  - Web 闭环分册与主手册引用稳定，执行路径唯一。
  - 改动路径到必跑套件映射可复用，发布前可直接判定 required/full 组合。
  - 文档迁移后命名统一为 `.prd.md/.project.md`，并通过文档治理检查。

## Traceability
| PRD-ID | 历史目标 | 当前权威入口 |
| --- | --- | --- |
| PRD-TESTING-MANUAL-001 | one canonical manual | `testing-manual.md` |
| PRD-TESTING-MANUAL-002 | clear suite mapping | `testing-manual.md` 的 L0-L5 / S0-S10 与 required/full 章节 |
| PRD-TESTING-MANUAL-003 | auditable test evidence | `testing-manual.md`、`doc/testing/templates/release-evidence-bundle-template.md`、`doc/testing/evidence/README.md` |

## 目标
- 历史目标已完成：统一分层测试模型、触发矩阵与证据标准，并把当前执行入口收口到 `testing-manual.md`。
- 本文件现在只保留历史决策和 traceability，不再重复当前手册正文。

## 范围
- In scope: 保留手册工程化收口的历史需求、决策、PRD-ID 映射和 canonical redirect。
- Out of scope: 不新增测试框架、不重写当前 `testing-manual.md`、不替代 Web UI 闭环分册。

## 接口 / 数据
- 当前手册入口: `testing-manual.md`
- Web UI 闭环分册: `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
- 发布证据模板: `doc/testing/templates/release-evidence-bundle-template.md`
- evidence 分流入口: `doc/testing/evidence/README.md`

## 里程碑
- TMAN-1: 完成手册迁移与主入口命名统一。
- TMAN-2: 收口分层模型与套件矩阵。
- TMAN-3: 完成 Web 闭环分册拆分并建立主手册引用入口。
- TMAN-4: 持续补齐 fail-fast、GPU/headed 门禁与运行约束。
- TMAN-5: 专题文档人工迁移到 strict schema 并统一命名。

## 风险
- 当前主要风险是历史规划文档被误读为当前执行入口；本文件通过 `completed / historical` 状态和 canonical 手册 redirect 降低该风险。
- 若 `testing-manual.md` 与脚本/CI 入口漂移，应更新当前 canonical 手册和对应 task evidence，而不是在本历史 PRD 中复制新规则。

## Historical Decisions
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-TMAN-001 | 主手册 + 分册双层结构 | 所有细节堆叠在单文档 | 保持可读性并降低维护冲突。 |
| DEC-TMAN-002 | required/full 分层执行策略 | 每次全量执行 | 兼顾执行效率与发布覆盖。 |
| DEC-TMAN-003 | 证据包作为发布门禁硬要求 | 仅口头确认结果 | 无法支撑审计与追溯。 |
| DEC-TMAN-004 | legacy 文档逐篇人工迁移 | 脚本批量改写 | 保证内容语义与约束完整保真。 |
