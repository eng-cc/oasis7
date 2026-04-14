# 工程文档治理闭环设计（2026-02-27）

- 对应需求文档: `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.prd.md`
- 对应项目管理文档: `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.project.md`

## 1. 设计定位
定义工程文档治理如何从规范、台账、模块入口与门禁脚本形成闭环，避免治理结论只停留在审计层。

## 2. 设计结构
- 规范层：`doc-structure-standard.*` 作为命名、职责与互链的裁定源。
- 执行层：ROUND 台账负责扫描、分批、整改与复审。
- 模块层：各模块 `prd/design/project/index` 承接具体回写。
- 校验层：`scripts/doc-governance-check.sh` 提供结构类门禁。

## 3. 关键接口 / 入口
- 权威规范：`doc/engineering/doc-governance/doc-structure-standard.prd.md`、`doc/engineering/doc-governance/doc-structure-standard.design.md`
- 项目追踪：`doc/core/project.md`、`doc/engineering/project.md`
- 审读台账：`doc/core/reviews/consistency-review-round-*.md`

## 4. 约束与边界
- 先规范后执行：发现规范空白时先回写规范。
- 一处改名，多处回写：README、索引、项目入口必须同步。
- 问题必须落动作：不得只登记问题不做结构或内容回写。

## 5. 设计演进计划
- 从纯审计轮演进到结构治理轮，再演进到内容职责治理轮。
- 后续可继续收紧到 Design 覆盖率和 Manual/Runbook 标准化。
