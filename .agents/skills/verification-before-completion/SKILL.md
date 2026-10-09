---
name: verification-before-completion
description: Use when reporting oasis7 verification or completion.
---

# verification-before-completion

通用规则见 [开发流程规范](../../../doc/engineering/workflow/source-of-truth.md)。

按实际变化和受影响消费者选择真实验证，参考 `testing-manual.md`；先局部验证，再核对实际 CI 完整必要结果。影响不明时扩大覆盖。

直接检查退出码和输出，记录命令、所测版本、结果和未覆盖部分。当前 source HEAD 的必需 CI 成功结果不会仅因 main 前进而失效；按通用规范中的冲突或明确集成风险决定是否更新分支和重测。旧版本结果不能代表新 HEAD 或新的 main 合并组合。失败、取消、缺失、未知和应执行却跳过不能算通过。

报告 ready 前检查适用评审及阻断项，报告代码完成前确认 PR 实际 merged 和提交。明确的部署、多 PR 和管理义务尚未完成时列出剩余工作。

## Related methods

- [executing-project-tasks](../executing-project-tasks/SKILL.md)
- [requesting-repo-owned-review](../requesting-repo-owned-review/SKILL.md)
- [systematic-debugging](../systematic-debugging/SKILL.md)

## Known Failure Modes

局部失败只阻塞依赖动作，其他已授权工作继续。目标或外部影响扩大时确认新增部分；远程结果不明先查询，不重复创建或伪造完成。不以 Task、Project、模型观测或 receipt 代替真实结果。
