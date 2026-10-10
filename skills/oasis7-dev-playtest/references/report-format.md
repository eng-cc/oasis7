# 自测报告：事实、判定和修复分开

这是本轮测试 artifact 的字段约定，不是仓库任务准入、进度数据库或新的 PR 放行证书。复用已有 L4 输出目录及当前 PR，格式可用 JSON 或 Markdown；必须包含相同事实，不强制为普通任务另建报告文件链。

## 运行身份与覆盖

- run/attempt 与原失败或复测引用；source HEAD、未提交 patch hash（有则记录）、实际 build/Skill hash、Runtime/driver/profile/API 版本。
- 实际子会话引用、是否真的创建独立 subagent、`attached_codex_subagent / supervised_runtime / scripted_probe / main_agent_manual`。
- `cold_entry / prepared_world`、`autonomous / regression_replay`、`assisted=true/false`、`isolation=enforced / instruction_only / source_visible`，以及为何降级。
- 获准 world/branch/Agent/task、初始条件和信任来源、目标与范围、有效授权引用；不要保存 secret。
- 实际表面 `game_api / web / native`、平台/build、选中和未选中的场景、开始/结束/停止原因。

## 每个场景

| 字段 | 要求 |
| --- | --- |
| case_id / requirement | 复用已有场景与对应产品/专业要求 |
| verdict | pass、fail、blocked、inconclusive、not_applicable |
| expected / observed | 预先确定的目标和实际发现；不临时降低判据 |
| steps | 按真实发生的顺序记录，区分玩家操作、Agent 提案和 setup 故障注入 |
| evidence | 本轮脱敏文件或受权回执引用，记录采集者/时刻/对象身份；没有证据不写通过 |
| oracle | 谁用哪个权威结果判定；Tester 自报只能是 reported |
| unmet / next | 未完成的范围、原因与下一步，不用“暂无问题”代替未测试 |

只有已执行且 oracle 证明要求满足才是 pass；已执行且违反要求是 fail；前置缺失是 blocked；预算到限/模型策略未成功/证据不足通常是 inconclusive。合法拒绝若正是负例目标，可以是该负例通过，不代表正常首局已通过。

## 问题条目

每个 finding 包含 `category、severity、scenario、目标版本、复现步骤、预期/实际、发生次数/尝试数、operation/receipt/截图、影响范围、诊断置信度、修复建议`。

category 使用 confirmed_bug、suspected_bug、agent_strategy_failure、expected_rejection、environment_blocker、harness_fault、usability_observation。tester 可以给 suspected_bug；confirmed_bug 需要主 Agent 或核验器的复现/事实依据。模型对“好玩/难懂”的评价保留原场景观察，不转换为玩家留存指标。

业务完成的证据必须和原 world/Agent/task、source/build、请求/回执关联。不能提交其他任务 receipt、手工改造成功 JSON、从本机日志伪造世界状态或用旧截图充新版本。

## 消耗、修复和清理

保留总授权预算、已报告 token/费用及单位、估计/预留与未知部分、预算可强制范围，以及子任务和实际受测 Runtime 的分项。缺失计量不当作零，不跨货币隐式汇总，不重复累加父子调用或累计事件。

修复记录指向原 finding、普通回归源/命令/旧失败与新通过、新 source/build 和复测 attempt；不能用重跑一次成功删除原失败。暂不能复现或未复测分别说明，不能标已闭合。

清理记录区分测试 executor 撤销、原 pending 对账、worker/浏览器/subagent/私有世界的实际退出。失败保留明确资源引用，不记录秘密；不能只因为子线程返回就宣称后台进程已停。

## 总结给开发负责人

```text
候选和受测 Runtime/profile：
实际派工、隔离与启动模式：
本轮真实执行及范围内 verdict：
已确认问题与证据：
策略/环境/框架问题及体验观察：
修复、普通回归和独立复测：
消耗（已知/估计/未知）和停止原因：
未执行、未证明或需要其他 surface 验证的部分：
自有资源清理结果：
```

汇总 scope pass 要求所有选中场景都有完成证据且无 blocked/inconclusive；未选项不算通过。API、不同 Runtime、Web/native 和 L4B/L5 各保留自身边界，不用总分消除缺项。
