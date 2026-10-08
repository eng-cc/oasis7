---
name: executing-project-tasks
description: Use when implementing and finishing an authorized oasis7 change.
---

# executing-project-tasks

通用规则见 [开发流程规范](../../../doc/engineering/workflow/source-of-truth.md)。

明确问题、范围、验收和用户限制。简单修改写入 PR，复杂任务使用已有设计或 Issue checklist。需要比较方案时列出少量选项及取舍。

在独立分支连续分析、实现、局部验证和修复。subagent 按需使用，说明必要上下文、写入范围和期望结果，避免重叠编辑。保留他人改动。可复现行为缺陷增加有用回归测试，先观察失败再做最小修复；机械修改无需重复测试。同步相关技术和使用文档。

使用验证和评审方法。同一目的继续更新原 PR，说明问题、改动、验证和风险。合入前检查当前 HEAD、required checks、评审及阻断项、冲突、权限和 hold；正常保护合入绑定预期 HEAD。实际 merged 才完成代码交付，明确部署或整体验收继续执行。

清理独立进行，保留 dirty、未推送、使用中和身份不明资源。

## Related methods

- [verification-before-completion](../verification-before-completion/SKILL.md)
- [requesting-repo-owned-review](../requesting-repo-owned-review/SKILL.md)
- [systematic-debugging](../systematic-debugging/SKILL.md)

## Known Failure Modes

局部失败只阻塞依赖动作，其他已授权工作继续。目标或外部影响扩大时确认新增部分；远程结果不明先查询，不重复创建或伪造完成。不以 Task、Project、模型观测或 receipt 代替真实结果。

PR 标题和 commit message 使用英文。
