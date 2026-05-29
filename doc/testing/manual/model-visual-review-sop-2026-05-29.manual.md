# oasis7: 模型视觉评审 SOP（2026-05-29）

审计轮次: 1

## 文档定位
- 本文件是截图加模型视觉评审的 canonical `*.manual.md` 操作手册。
- 它用于替代绝大部分常规人工视觉 review：布局、遮挡、层级、可读性、响应式、截图与产品目标是否一致，默认先交给具备视觉能力的模型评审。
- 它不替代 GitHub required review、required checks、发布 owner 放行、真实玩家验证或对外承诺审批。

## 适用范围
- 适用于 Viewer Web、launcher Web、站点页面、文档封面、像素世界和其他截图可判断的 UI / visual surface。
- 适用于 PR 前本地收口、PR review 辅助、发布 evidence packet 的视觉部分。
- 不适用于仅凭截图无法判断的 runtime contract、经济参数、安全权限、链上状态、LLM provider 活性或真实玩家留存结论。

## 目标
- 把“看起来是否对”从临时人工主观 review，改成可重复的截图输入、固定 rubric、结构化 verdict 和升级边界。
- 让 routine visual review 默认由模型完成，人类只处理阻断、低置信、审美方向争议、对外 claim 和最终治理放行。
- 让模型视觉结论可追溯到截图、viewport、commit、任务目标和自动化验证结果。

## 输入要求
- 必填输入：
  - `task_uid` 或 PR / issue / project ref
  - 当前 commit SHA 或分支名
  - 变更目标，用一句话说明玩家/用户应该更容易看懂什么
  - 截图路径，至少包含目标 surface 的 desktop 视口
  - 自动化结果摘要，例如 DOM/DTO/UI tests、visual smoke、console/state probe
  - expected visual contract，例如“Agent 第一眼可见，Fragment 只是背景信息”
- 按需输入：
  - mobile 截图，建议宽度 390px 或当前产品指定断点
  - before/after 或 baseline 截图
  - DOM probe / state JSON / console log
  - 用户反馈原句或设计方向文档

## 截图采集标准
- 每轮模型视觉评审至少保留一张主截图；触达响应式或首屏时必须补 mobile 截图。
- 截图路径默认落在 `output/playwright/<topic>/<run-id>/`；若需要随文档长期归档，再复制或引用到对应 `doc/**/evidence/` 目录。
- 截图必须可复现地标明：
  - URL 或页面入口
  - viewport
  - locale
  - fixture / scenario / injected snapshot
  - commit SHA
  - 是否为 fallback / degraded mode
- 若页面可滚动，主图优先裁切目标体验区域；必要时再补 full-page 截图，避免模型把无关右栏或下方长内容当成首屏问题。

## 标准流程
1. 先跑确定性验证：
   - DOM/DTO/contract tests
   - Web visual smoke 或 browser screenshot script
   - console/state probe
2. 采集截图：
   - desktop 主视口
   - mobile 主视口（若涉及响应式或首屏）
   - before/after（若是视觉改版）
3. 组装模型视觉评审包：
   - 截图
   - 变更目标
   - must-pass visual contract
   - 自动化摘要
   - 已知 out of scope
4. 让模型按固定 rubric 输出 review card。
5. owner 根据 verdict 处理：
   - `pass`: 可替代 routine human visual review，进入常规验证/PR 主链。
   - `watch`: 可继续，但要记录 residual risk；若是 release/public surface，升级人类复核。
   - `block`: 必须修复或明确产品放弃该方向。
   - `human_escalation`: 交给对应 owner / reviewer，人类结论回写正式 sink。

## 模型评审 Rubric
| 维度 | 必答问题 | `block` 信号 |
| --- | --- | --- |
| 第一视觉焦点 | 用户第一眼是否看到本轮最重要的主体或任务？ | 主体被侧栏、诊断、装饰或噪声压住 |
| 层级与可读性 | 文字、按钮、状态、实体是否能快速扫读？ | 文本过密、字号失衡、标签互相竞争 |
| 遮挡与溢出 | 是否有重叠、裁切、横向溢出、按钮文本挤压？ | 主要操作或状态被遮挡/截断 |
| 数据诚实性 | 截图表达是否和真实 state/DTO 一致？ | UI 暗示了未发生的进展、因果或 agent 行动 |
| 行动反馈 | 用户动作是否有明确 receipt / next step / blocker？ | 只看到世界在动，看不到玩家导致了什么 |
| 响应式 | desktop/mobile 是否保持同一套主次逻辑？ | 移动端首屏主路径消失或需要横滚 |
| 商业化观感 | 是否像面向玩家的游戏界面，而不是内部调试台？ | diagnostics、原始数据或工程面板抢主视野 |
| 回归风险 | 与目标/基线相比是否明显退化？ | 新图比 baseline 更难读或更像 placeholder |

## 像素世界补充 Rubric
- Agent 必须比 Fragment 背景更容易被第一眼识别。
- Fragment block 默认是背景信息，不能抢过 Agent、目标或行动回执。
- Location 若只是逻辑锚点，不能以主实体方式支配画面。
- Action Receipt 必须回答“玩家做了什么，世界因此反馈了什么”。
- No-receipt 状态必须诚实，不得暗示 active agent 已造成进展。
- Renderer diagnostics 默认不得比 World Command Board 或 Action Receipt 更显眼。

## 输出格式
- 使用模板：`doc/testing/templates/model-visual-review-card-template.md`
- 每张评审卡必须包含：
  - verdict: `pass` / `watch` / `block` / `human_escalation`
  - confidence: `high` / `medium` / `low`
  - screenshots reviewed
  - must-pass checks
  - findings
  - what this review does not prove
  - human escalation needed: `yes/no`
  - owner action

## 替代人工 Review 规则
- 可由模型视觉评审替代的人类 review：
  - routine screenshot inspection
  - 布局/遮挡/溢出检查
  - 首屏主次关系判断
  - 响应式截图扫查
  - UI 是否误导玩家因果/状态
  - 与已冻结视觉方向的一致性检查
- 不能完全替代的人类 review：
  - 全新品牌/美术方向定调
  - 对外宣传、社区承诺、事故复盘口径
  - 法务、安全、经济、权限、隐私或财务风险
  - L5 真实玩家 / 真实外部信号
  - GitHub branch protection 要求的 review / approval
  - 模型 confidence 为 `low` 或截图证据缺关键视口

## 升级条件
- 任一 must-pass check 为 `block`。
- 模型指出截图缺失、状态不明或无法判断真实交互。
- 模型与自动化 state / DOM probe 结论冲突。
- 评审结论会影响公开 claim、release note、limited preview 或用户承诺。
- 用户或 owner 明确要求人工审美判断。
- 同一问题连续两轮模型评审仍为 `watch`。

## 正式 Sink
- 当前 task：回写 `.pm/tasks/<TASK-UID>.execution.md`。
- PR：在 PR 描述或评论里附 screenshot path、review card 摘要和 residual risk。
- 长期规则：回写 `doc/testing/**`、`doc/world-simulator/**` 或对应模块 PRD/project。
- 只保留短期样本：可写入 `doc/ui_review_result/**`，但必须在结论稳定后迁回正式模块文档。

## 最小完成定义
- 至少一组截图已采集并可打开。
- 模型视觉 review card 已按模板填写。
- verdict 与 human escalation 状态明确。
- 若 verdict 为 `pass`，自动化验证也必须已通过或明确列出未跑原因。
- 若 verdict 为 `watch/block/human_escalation`，下一步 owner action 已写清。

## 推荐命令入口
- Viewer Web 闭环：`doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
- Pixel-world visual smoke：
```bash
npm --prefix crates/oasis7_viewer run test:pixel-world:visual
```
- Viewer UI deterministic tests：
```bash
npm --prefix crates/oasis7_viewer run test:ui
```

## 决策记录
- DEC-MVR-001: 模型视觉评审是 routine visual review 的默认替代路径，但不是发布治理或 GitHub required review 的替代品。
- DEC-MVR-002: 模型视觉评审必须以截图证据为输入，不能只读 DOM 或口头描述后给视觉 verdict。
- DEC-MVR-003: `pass` 只有在截图、自动化摘要与任务目标一致时才能替代人工视觉 review；低置信或 claim 影响对外口径时必须升级人类。
