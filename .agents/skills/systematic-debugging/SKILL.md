---
name: systematic-debugging
description: Use when diagnosing an oasis7 bug, failed test, or regression.
---

# systematic-debugging

通用规则见 [开发流程规范](../../../doc/engineering/workflow/source-of-truth.md)。

取得可重复失败和准确错误，区分产品、测试、环境及入口问题。缩小输入与失败边界，读相关实现和近期 diff，提出可证伪假设。

用最小实验验证关键假设，定位根因后做最小修复。不用重启、跳过测试或吞错误掩盖根因。对可自动复现行为先写或修正回归测试，观察失败再修复；测试保护真实行为，避免镜像实现。验证受影响消费者、兼容和失败路径，说明环境限制。

复杂问题按需请求领域协作，给出复现、上下文、写入范围和期望结果；不依赖流程身份。重复失败时修正假设，不堆叠猜测补丁。

## Related methods

- [executing-project-tasks](../executing-project-tasks/SKILL.md)
- [verification-before-completion](../verification-before-completion/SKILL.md)
- [requesting-repo-owned-review](../requesting-repo-owned-review/SKILL.md)

## Known Failure Modes

局部失败只阻塞依赖动作，其他已授权工作继续。目标或外部影响扩大时确认新增部分；远程结果不明先查询，不重复创建或伪造完成。不以 Task、Project、模型观测或 receipt 代替真实结果。
