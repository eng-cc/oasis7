# README 世界规则文档收敛整合（设计文档）

- 对应设计文档: `doc/readme/governance/readme-world-rules-consolidation.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-world-rules-consolidation.project.md`

审计轮次: 4

- 对应标准执行入口: `doc/readme/governance/readme-world-rules-consolidation.project.md`

## 目标

- 将本轮未提交的 README 文档治理任务合并为一个统一设计文档，避免多个小文档并行维护。
- 统一 README 的角色为“导航级摘要”，把规则细则集中到权威文档。
- 消除 `README.md` 与 former world-rule product topic（后来折叠进模块 PRD 并 retired/deleted）在世界设定、模拟机制、WASM 机制、开放沙盒上的重复描述。

## 范围

- `README.md`：
  - `一~六` 章节统一为“摘要 + 细则入口”结构。
  - 章节标题统一为二级（`##`），节内“摘要”统一为三级（`###`）。
  - 运行模式与系统边界保留产品导览语义，不承载规则细则。
- former world-rule product topic（后来折叠进模块 PRD 并 retired/deleted）：
  - 吸收 README `二/三/五` 的细则内容，补齐到相关章节（Agent、执行模型、治理、文明方向）。
- 文档治理：
  - 将本轮未提交的 3 组设计/项目文档收敛为 1 组统一文档。

## 接口/数据

- 文档分层入口：
  - 导航摘要层：`README.md`
  - 世界规则细则层：former world-rule product topic（后来折叠进模块 PRD 并 retired/deleted）
  - 运行与调试手册：`doc/world-simulator/viewer/viewer-manual.manual.md`
  - 系统性测试手册：`testing-manual.md`
- README 链接约定：
  - 当时的规则细则统一链接到 former world-rule product topic（后来折叠进模块 PRD 并 retired/deleted）
  - 运行/测试入口分别链接到 `./doc/world-simulator/viewer/viewer-manual.manual.md` 与 `./testing-manual.md`

## 里程碑

- M1：完成 README `一` 的摘要化并指向 former world-rule product topic（后来折叠进模块 PRD 并 retired/deleted）。
- M2：完成 README `二/三/五` 的细则迁移与摘要化。
- M3：完成 README `一~六` 导航结构统一。
- M4：完成标题层级修正（章节二级、摘要三级）。
- M5：完成文档合并、回归校验与 devlog 记录。

## 风险

- 风险 1：文档收敛后读者需要跨文档阅读，可能增加跳转成本。
  - 缓解：README 每节保留核心摘要并提供明确入口。
- 风险 2：后续改动若再次在 README 写入细则，可能复发分叉。
  - 缓解：保持 README 导航定位；当时细则变更优先落 former world-rule product topic，该专题后来折叠进模块 PRD 并 retired/deleted。
- 风险 3：文档重命名后历史引用可能失效。
  - 缓解：在 devlog 记录合并动作，并在本文件中给出统一命名。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
