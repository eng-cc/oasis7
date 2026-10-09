---
name: requesting-repo-owned-review
description: Use when arranging or responding to review for an oasis7 change.
---

# requesting-repo-owned-review

通用规则见 [开发流程规范](../../../doc/engineering/workflow/source-of-truth.md)。

按复杂度和风险选择审阅对象。机械文档和格式可由负责人自检及自动检查；权限、凭据、共识、持久化兼容、不可逆迁移、安全和开发门禁必须由具备对应能力且未实施该部分的审阅者独立检查。

提供具体问题、版本、diff、验证和风险。记录版本、结论、阻断发现及处置即可，不生成 packet、epoch、handoff 或 ledger。

反馈先核对事实和影响。修复正确性、安全和数据丢失阻断项，或以可复核事实说明不适用；非阻断建议可说明理由后不改。复审关注实际变化和受影响部分；描述更新或同版本 CI 重跑不触发全量重审。CI 和评审可并行，合入前汇总实际结果。

## Related methods

- [executing-project-tasks](../executing-project-tasks/SKILL.md)
- [verification-before-completion](../verification-before-completion/SKILL.md)
- [systematic-debugging](../systematic-debugging/SKILL.md)

## Known Failure Modes

局部失败只阻塞依赖动作，其他已授权工作继续。目标或外部影响扩大时确认新增部分；远程结果不明先查询，不重复创建或伪造完成。不以 Task、Project、模型观测或 receipt 代替真实结果。
