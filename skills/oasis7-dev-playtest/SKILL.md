---
name: oasis7-dev-playtest
description: Coordinate a bounded oasis7 development self-test using a real playtesting subagent, the shipped player Skill, an isolated game candidate, authoritative outcome checks, and regression-driven repair. Use explicitly for agent gameplay, integration, or playability testing; never infer paid execution or production access from loading this skill.
---

# oasis7 开发自测：派出 Agent 真正玩一局

供**开发主 Agent**显式读取。不是游戏玩家 Skill，不自动安装、不默认触发、不修改仓库工作流。游戏 subagent 仍须读取本候选实际交付的玩家 Skill（目标路径 `skills/oasis7-play/SKILL.md`；若不存在，按实际批准的发行资产核对，不能自行伪造）。

## 先读和确认

1. 仓库 `AGENTS.md`、`doc/engineering/workflow/source-of-truth.md` 与 `testing-manual.md`：按当前规范开发，不因本 Skill 新增审批或必跑模型门禁。
2. [自测手册](../../doc/testing/manual/agent-runtime-self-playtest.manual.md) 与[系统设计](../../doc/testing/governance/agent-runtime-self-playtest.design.md)：选择适用场景及实际证据层。
3. 当前任务、diff、候选构建、测试环境和已有授权：不要重复向用户询问已明确的信息。

只读规划与前置检查可先做。真正调用模型/subagent、启动服务或有副作用的游戏操作，必须在用户已有授权的环境、账户、额度和资源范围内；缺必要条件说明 blocked 和缺项。读取本文件不授权付费测试或主网/生产世界操作。

## 最小执行法

### 1. 固定本轮范围

先运行相关普通测试，记录 source HEAD、未提交 patch hash（有则保留）、实际 build/Skill hash、Runtime/driver/profile、场景、私有 world/branch、表面及有界总预算。默认一名 tester；UI/协作需要时至多增一名 observer。最多两轮修复后重试，且不得超过用户总许可；无必要不派更多 Agent。

玩家 Skill、目标 API、真实 Runtime 或测试环境未就绪时，输出预检阻塞，继续可做的静态检查；不能将旧 Provider Bridge、固定动作脚本或自行生成 JSON 冒充新路径实测。检查当前 Codex 是否真有 subagent/隔离/GUI 工具，不发明通用 spawn 命令。

### 2. 准备世界而不是偷改游戏

只使用已检查且能指向本轮 local/private disposable fixture 的启动入口，确认可信 world identity、端口、存储、初始资源和权限；localhost 可能是远程代理，不能据此认定安全。预先记录自有进程/会话，禁止复用用户日常世界。

`cold_entry` 测玩家 Skill 的自助启动：给 tester 已批准、限于本轮 fixture 的启动工具；`prepared_world` 由主 Agent/setup 启动再交接。两者单独标注，setup 代办不能代签 cold_entry。现有 L4 scaffold/runner 可复用真实功能，不能调用未实现的 autonomous 参数。

### 3. 派出独立试玩 subagent

使用当前宿主真正提供的创建/发送/等待/停止能力，给出[试玩任务模板](references/playtester-task.md)。记录真实子会话，要求首先读指定玩家 Skill。不给源码、diff、开发推理、fixture 隐藏信息或固定通关答案。

默认尝试 `attached_codex_subagent`：子会话自身是已经验证支持的 attached 执行方，经 Skill/API 玩游戏；不要再无条件启动一层 Codex。要验证 OpenClaw 或 supervised Codex，则实际使用对应 worker/profile，子任务作玩家/观察者，记录两类模型费用；不能让 Codex 模仿 OpenClaw。

子会话只读玩家资料、通过受限工具操作测试世界、写本轮报告。源码不可读应由环境权限实现；只能靠指令约束时报告 isolation 降级。不要完整继承父会话，也不要默认继承 owner/model secrets。一个游戏 Agent 只能有一个当前 executor；observer 用独立 owner 测试身份。

### 4. 自主运行和回收

让 tester 自行查询、规划、行动、等待、询问并收口；不要逐步告诉它如何过关。UI observer 真实操作浏览器/原生窗口，不能用 API 写入代替用户点击。附加提示或调试帮助必须标为 assisted 并保留原失败。

主 Agent 监控总 deadline、用量、世界许可与失联状态；到上限停止新调用。对未知操作查询原 ID；收到 stop/撤销不得继续新副作用。不能让 tester 扩预算、批准自己或递归派工。

### 5. 先验真，再修复

按[报告合同](references/report-format.md)回收结果。用权威原 operation/receipt、world binding 与首局 completion 判据核对；截图、Agent 自报或进程 exit=0 不能直接判成功。缺证据标 blocked/inconclusive。

主 Agent 再读源码/日志分诊，区分 confirmed_bug、策略失败、正常拒绝、环境/框架问题和体验观察。确认缺陷转成最小普通回归，旧版本能检出问题后修复；构建新候选，用新独立 tester 复测同等条件。记录所有尝试，不靠换答案、换世界或反复运行挑一份绿结果。

### 6. 结束并交付

停止新调用、撤销测试 executor、核对 pending、关闭自有 worker/browser/subagent并实际验证；不要杀其他会话、删除未知工作区或改变线上世界。清理失败明确报告，子任务结束不意味着 worker 已退出。

最终向用户或原 PR 返回：测试候选/模式、实际执行步骤和覆盖、问题及证据、修复与普通回归、独立复测结果、消耗、未覆盖和清理状态。没有独立 subagent 就明说，没有运行就不写“已实测”；不自动建 Issue、push、合入或发布。主 Agent 仅在原任务已经授权时执行这些仓库动作。
