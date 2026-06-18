# README 缺口 3 收口：模块安装目标语义（自身 / 基础设施）（项目管理文档）

- 对应设计文档: `doc/readme/gap/readme-gap3-install-target-infrastructure.design.md`
- 对应需求文档: `doc/readme/gap/readme-gap3-install-target-infrastructure.prd.md`

审计轮次: 4

## 治理状态（2026-06-18）
- 本文是已完成的 README gap3 模块安装目标语义增量专题，保留原址用于历史追溯。
- 当前主入口统一指向 `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.prd.md`、`doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.design.md` 与 `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.project.md`。
- 本专题无当前下一步动作。

## 审计备注
- 主项目入口统一指向 `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.project.md`，本文仅维护增量任务。

## 任务拆解
- [x] T0：输出设计文档（`doc/readme/gap/readme-gap3-install-target-infrastructure.prd.md`）
- [x] T0：输出项目管理文档（本文件）
- [x] T1：扩展数据结构（`ModuleInstallTarget` + action/event/state）并保持反序列化兼容
- [x] T2：实现 simulator/runtime 安装目标处理与 LLM 动作入口
- [x] T3：补齐测试并执行回归（`cargo check` + 定向 tests）后回写文档/devlog

## 依赖
- T2 依赖 T1 的类型冻结。
- T3 依赖 T1/T2 完成后统一执行。

## 状态
- 当前阶段：T0/T1/T2/T3 完成。
- 阻塞项：无。
- 下一步：无（本缺口收口完成）。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
