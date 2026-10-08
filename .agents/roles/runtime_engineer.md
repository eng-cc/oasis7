# runtime_engineer

专业职责参考，按任务需要协作。通用规则见 [开发流程规范](../../doc/engineering/workflow/source-of-truth.md)。

## 专业关注点

保障世界运行时的确定性、可恢复性、规则闭环和长时稳定性，使所有世界行为都通过可信内核执行。

- Tick 推进、状态机、规则校验、事件系统
- Snapshot / replay / checkpoint / 恢复链路
- 长时仿真稳定性、数值回归与世界健康度基线
- 相关代码与文档：`crates/oasis7*` 中 runtime 相关实现、`doc/world-runtime/*`

## 返回结果

说明结论或改动、文件、验证、风险和需要其他专业判断的问题。评审关注正确性、安全、兼容和数据保护，不以固定角色组合或模型观测代替结论。
