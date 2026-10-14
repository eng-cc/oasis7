# 项目管理记录方法

- 状态：active
- 适用范围：按需使用的 Issue、Project、复杂变更说明与组合验收
- 当前入口：[文档治理](README.md)
- 通用规则：[开发流程规范](../workflow/source-of-truth.md)

项目管理记录帮助读者理解目标、范围、依赖和结果，不构成普通开发或 PR 的准入。简单任务使用 PR 描述；复杂任务可使用已有设计文档或 Issue checklist，Project 按需展示进度。

## 内容

说明问题、预期结果、允许范围、负责人、必要依赖、验收方式和重要限制。记录已完成及剩余义务，区分代码交付与明确要求的部署、迁移或组合验收。协调 Issue 在所需义务实际完成后关闭。

用 Git 提交、PR、真实 CI 运行和评审记录定位实际版本及结果。失败只阻塞依赖动作，Project 同步失败不改变已合入事实。历史 Issue、本地资料和用户记忆保留为可读材料，不生成身份、packet、receipt 或终态证明链。

## 跨层要求关联

Trace refs: O7-REQ-276983b3-3187-405e-8453-e2785f01c5e6；本节承接[全局要求追踪](cross-layer-requirements-traceability.design.md#global-requirement-trace)的适用写作/引用约定。

涉及稳定要求的任务，按[跨层追踪规范](cross-layer-requirements-traceability.design.md)在原有说明中写 `Trace refs: <完整 ID>`，逐项链接产品或专业要求声明处与适用的系统设计条款，并说明本次实现/验证范围。产品、设计和 Task 共用要求的 ID；Task 本身不另领全局 ID。一个任务可以承接多项要求，一项要求也可以由多个任务交付。

Task 可以直接是当前 PR，Issue/Project 仍按需使用。纯工程工作链接专业要求并说明产品层不适用；简单编辑不为补齐三层而编造需求或设计。新增引用时保留已有局部 REQ/AC、真实 path#fragment 和 Git/PR/CI 证据，不生成第二份状态台账。

当前按完整 ID 搜索仓库和 GitHub 记录并人工追链；自动聚合索引尚未实现。搜索覆盖、未形成或不适用的层级和未完成义务应如实说明，找到相关 PR 不等于要求已经满足。

## 可选说明模板

- 问题与目标：
- 范围及重要限制：
- Trace refs、要求声明及适用设计链接（涉及跨层要求时）：
- 必要依赖与验收：
- 实际版本、结果及链接：
- 剩余工作和风险：

长期产品要求保留在产品文档，专业技术合同保留在对应系统设计，实现和测试承载运行事实。使用 [系统设计写作方法](system-design-writing-standard.design.md) 维护技术材料。
