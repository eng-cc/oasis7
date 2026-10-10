# 开发 Agent 派出试玩 subagent：自测手册

- Owner role：qa_engineer；固定设计基线：`eng-cc/oasis7@e524debf4584e5e6f2ae5baf9d877e78c9d7bf7d`。
- 合同：[自主试玩系统设计](../governance/agent-runtime-self-playtest.design.md)；策略：[testing PRD](../prd.md)、[总测试手册](../../../testing-manual.md)。
- 使用入口：[oasis7-dev-playtest Skill](../../../skills/oasis7-dev-playtest/SKILL.md)；产品玩法仍使用实际交付的玩家 Skill。

这是实施后执行的手册，不是一次已运行记录。当前 PR 不增加 CLI runner、启动游戏或执行付费模型；缺失玩家 Skill、API、隔离环境、真实子任务/GUI 能力或授权时，应保存 blocked 及原因，不伪造测试结果。

## 最小使用方式

在开发 Codex 中显式给出：

```text
读取 skills/oasis7-dev-playtest/SKILL.md，为当前修改做一轮有界自测。
先跑适用的普通回归；使用已批准的私有测试环境和本轮预算。
创建一个独立试玩 subagent，让它读取本候选真正交付的玩家 Skill，
自行启动/接入、规划和玩游戏，不读实现代码或使用调试世界写入。
保留真实操作/回执、失败步骤及未覆盖范围；必要时增加独立 Web/native 观察任务。
汇总反馈，复现确认的问题、补普通回归、修复，并在剩余预算内用新候选复测。
缺依赖、额度或必要工具时停在该边界，不能换旧 Bridge 或另一个模型冒充通过。
```

此提示是派工意图，不意味着任何 Codex 版本都具备相同 subagent/GUI 接口；主 Agent 先核对当前工具。显式派工不自动允许新增付费服务或使用用户日常世界。

## 执行顺序

1. **范围**：读 diff 和相关 AC，选用下面的最小场景；沿用当前开发任务/PR 的授权，不创建通用流程登记。选择 `attached_codex_subagent` 或 `supervised_runtime`，记录真正受测 Runtime，避免双重启动 Codex。
2. **便宜前置**：按现有 `testing-manual.md` 执行受影响包的单元/合同测试；不跳过普通 CI，也不为了文档变更跑模型。候选准备可用当前仓库真实的 bundle/playtest 入口，必须先读其说明和实现，确认可指向本轮私有 fixture。
3. **固定候选和许可**：保留 source HEAD、未提交 patch hash（有则必记）、实际 build/Skill hash、API/driver 版本、世界信任/branch/初始条件、执行模式、总预算和 deadline；报告不能只写 main/latest。测试期间源码或 build 改变则结束当前尝试并新建 attempt。
4. **预检与环境**：确认玩家 Skill 是被测版本交付的同一份；旧 `site/skills/oasis7.md` 不自动替代新路径。验证真实 API/权限、world identity、独占进程/端口/存储、有效 executor 和最小测试账号；测试目录不是生产目录的 symlink。所有模型、GUI 与下游工具均已获准。
5. **启动场景**：`cold_entry` 由 subagent 按玩家 Skill 使用受限、预批准入口自己启动/连接；`prepared_world` 由主 Agent/setup 启动再交接。记录由谁完成每个步骤及 ready 的依据。准备好的世界不得代签冷启动体验。
6. **真实派工**：使用宿主实际派工工具创建独立子会话；按[试玩任务模板](../../../skills/oasis7-dev-playtest/references/playtester-task.md)发送最小输入，不附开发对话、答案或服务日志。验证子会话实际读取了指定 Skill。仅有独立聊天但仍能读源码时，报告隔离降级。
7. **游玩和观察**：子任务只走合法 Skill/API。主 Agent 不边看边给通关提示；需要提示时标为 assisted。UI 场景用独立 owner/observer 身份；故障注入由 setup 执行，不能赋予玩家原始存储写权限。
8. **验真**：核对原 operation、authority binding、生产/授权回执和初始/最终状态；回查 unknown/recovery_required。不信任模型的成功标签。收集已知、估计与未知费用，汇总所有子任务和受测 Runtime 的消耗。
9. **反馈与修复**：按[报告合同](../../../skills/oasis7-dev-playtest/references/report-format.md)分诊。主 Agent 复现确认缺陷，添加能在旧版本检出问题的最小普通回归，修复后新 build、新 attempt、必要时新试玩子会话；保持原失败。不能为绿色结论改判据或无限重跑。
10. **退出**：停止新调用、撤销测试委托、核对 pending、关闭自有 worker/browser/subagent、核查清理。保留脱敏 artifact，不提交私钥/模型原始思维/用户消息到 PR。最后报告执行了什么、发现什么、修了什么和哪些未证明。

## 按改动选场景

| 改动 | 默认最小组合 | 不可替代的证据 |
| --- | --- | --- |
| 文档、纯重构、无外部行为变化 | 普通静态/单元检查；必要时 ST-01 dry preflight | 不强制付费试玩 |
| Skill、接入、API schema、能力目录 | ST-01/02 + ST-03；合同负例先普通 CI | 同一发行 Skill 与实际受支持 Runtime |
| 目标、审批、fence、恢复 | ST-04 + 受影响普通故障回归 | 当前版本、授权生效、原结果与无重复副作用 |
| 玩法/生产变化 | ST-03 + ST-07；受影响协作项按需 | 当前 `production_only` 权威首产物判据 |
| Web/native 可见功能 | ST-05 + 相关 ST-03/04 | 两表面分别实操，不从 API 推导 UI |
| worker、取消、用量 | ST-06 + 受影响 ST-03/04 | 总预算、未知调用和自有进程退出 |

未选项目标记 not_applicable 并说明与当前修改无关；已选但无法执行的项目标记 blocked/inconclusive，不可静默从分母删除。旧 L4 artifact 可以复用，但此次必须记录实际 `execution_mode`，不把固定 runner 路线改名为自主规划证据。

## 场景与判据

<a id="st-01"></a>
### ST-01：编排与独立试玩身份

- 准备：固定候选、一个 tester、可选 observer 和受限输入；另准备缺派工工具、继承源码上下文和 build 中途变化的负例。
- 执行：先 dry preflight，再在真实支持的 Codex 中创建子任务、读 Skill、返回报告；观察主 Agent 是否传入内部解法。
- 判定：真实子会话/输入/候选可核对；不具备能力或隔离时如实降级，不用主 Agent 文本代签。候选变化后旧报告只归属原版本。
- 层：确定性编排合同先普通 CI，真实 subagent 调度另行授权验证。

<a id="st-02"></a>
### ST-02：玩家 Skill 的冷启动与上手

- 准备：当前正式玩家 Skill、固定 hash、有效私有世界配置；另有 Skill 缺失、错误 major、旧 Bridge 和非法世界目标。
- 执行：新子会话仅阅读 Skill/公开帮助，使用受限入口启动或连接；分别运行 cold_entry 与 prepared_world。
- 判定：进入真实受权世界，知道任务/能力与失败下一步；缺依赖 blocked，不能创造假端点或测试专用玩家说明。记录 setup_assisted，两个启动模式不代签。
- 承接：[skill-release](external-agent-runtime-contract-validation.manual.md#skill-release)；此处新增的是独立试玩输入与冷启动体验。

<a id="st-03"></a>
### ST-03：自主首局与结果验真

- 准备：既有首局玩法/初始条件、允许的资源、实际 Runtime profile；另注入自报完成但无 receipt、仅 ambient 库存变化和结果未知等样本。
- 执行：只下达首产物高层目标，让 tester 自主决定查询/行动/等待/求助；验真器回查原请求和权威成果。
- 判定：首产物满足主责合同且与本次 Agent/任务相关。一次动作或模型报告不算；策略失败、产品故障和合法拒绝分别记录。OpenClaw 与 Codex 分别跑其真实 profile。
- 承接：[api-first-session](external-agent-runtime-contract-validation.manual.md#api-first-session)、[api-idempotency](external-agent-runtime-contract-validation.manual.md#api-idempotency)。

<a id="st-04"></a>
### ST-04：异步协作与恢复

- 准备：隔离 owner/executor、有限授权，fixture 允许受控断连/重启；预先定义 owner 会批准与拒绝哪些请求。
- 执行：离线发布目标、旧草稿竞态、问答、批准/拒绝、撤销、关 Viewer、重启 executor、事件缺口和冷启动目标恢复；故障由 setup 注入。
- 判定：读取/采纳/上下文/世界结果区分；无回复不批准；硬撤销不等已读；旧实例、旧批准和未知提交不生成第二次世界效果。scripted owner 标注为脚本协作者。
- 承接：[collab-executor](external-agent-runtime-contract-validation.manual.md#collab-executor)、[collab-recovery](external-agent-runtime-contract-validation.manual.md#collab-recovery)、[collab-approval](external-agent-runtime-contract-validation.manual.md#collab-approval)。

<a id="st-05"></a>
### ST-05：用户实际看世界和指导

- 准备：同一世界的正式 Web 和受支持 native build；分别具备可用输入/截图工具，observer 无 executor 写权限。
- 执行：世界概览、缩放/平移、定位 Agent、选择工厂、查看目标/回执/阻塞并给高层指导；记录真实输入与对应截图。
- 判定：两端各自在自身界面完成；native 仅开浏览器失败，无 GUI 则 blocked。原始截图带候选和时刻，截图可见不等于交互完成。可读性反馈保留具体步骤，不称 L5。
- 操作前读：[Viewer S6](web-ui-agent-browser-closure-manual.manual.md)；native 承接[viewer-native](external-agent-runtime-contract-validation.manual.md#viewer-native)。

<a id="st-06"></a>
### ST-06：预算、停止和隔离

- 准备：fake driver 可报告累计/重复/缺失 usage，测试 worker、browser 与一个不属于本轮的进程；禁止费用权限和生产目标作为负例。
- 执行：并行预算预留、子调用/取消/重启、超时、report 洪泛、恶意消息、越界路径、stop 后孤儿 worker；只在临时环境注入。
- 判定：所有调用共享有限授权，未知不算零；没有自动 top-up、递归派工、默认生产入口或秘密泄露；仅回收自有资源，泄漏明确报告。取消未确认不宣称上游已停止收费。
- 承接：[adapter-budget](external-agent-runtime-contract-validation.manual.md#adapter-budget)、[adapter-liveness](external-agent-runtime-contract-validation.manual.md#adapter-liveness)。

<a id="st-07"></a>
### ST-07：从试玩失败到回归与复测

- 准备：有真实失败操作/回执的旧候选、最小重现和新候选；另包含不能复现、测试框架错误、正常拒绝、预算不足样本。
- 执行：按报告分诊；确认 bug 转普通测试，验证旧版本能失败、新版本通过，然后用新 tester 对同等初始条件复测。
- 判定：原失败与新尝试均保留，候选/Skill/profile 清楚；未复现不伪造已修复，未执行不算 pass，不降低判据或改测试世界过关；达到轮数/预算上限收口反馈。
- 输出：PR 上描述定位、修复、回归命令/结果、实玩结论、费用和仍未覆盖项，不另建放行证书。
