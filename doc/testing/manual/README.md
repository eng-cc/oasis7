# testing manual 文档入口

## 从这里开始

- 想先判断测试分层、门禁与通用执行顺序：读仓库级 [`testing-manual.md`](../../../testing-manual.md)。
- 想采样 Viewer Web 页面、保存截图 / state / console 证据：读 [`web-ui-agent-browser-closure-manual.manual.md`](web-ui-agent-browser-closure-manual.manual.md)。
- 想跑真实本地栈、真实 UI 输入与玩家流程矩阵：读 [`web-ui-playwright-closure-manual.manual.md`](web-ui-playwright-closure-manual.manual.md)。
- 想让本地入口接入 formal `public_testnet` world state，并验证 quota / provider 链路：读 [`local-public-testnet-letai-test-environment-2026-06-23.manual.md`](local-public-testnet-letai-test-environment-2026-06-23.manual.md)。
- 想用截图加模型视觉评审替代 routine 人工 review：读 [`model-visual-review-sop-2026-05-29.manual.md`](model-visual-review-sop-2026-05-29.manual.md)，并使用 [`../templates/model-visual-review-card-template.md`](../templates/model-visual-review-card-template.md)。

## 文档分工

| 需求 | 权威入口 | 不负责的内容 |
| --- | --- | --- |
| 仓库级测试层级、required/full 与通用操作 | `testing-manual.md` | 专项 Web UI 操作步骤 |
| Viewer Web 页面闭环与留证 | `web-ui-agent-browser-closure-manual.manual.md` | launcher 控制面产品动作、public testnet attach 证明 |
| 真实本地栈的 Playwright 玩家流程 | `web-ui-playwright-closure-manual.manual.md` | 通用 agent-browser 页面采样 |
| 本地入口接入 formal public testnet | `local-public-testnet-letai-test-environment-2026-06-23.manual.md` | 纯 local-only LetAI playtest |
| 截图的模型视觉评审 | `model-visual-review-sop-2026-05-29.manual.md` | runtime / security / release 放行结论 |

## 规格与追溯

- 已完成的系统测试手册工程化专题三件套已被 `testing-manual.md`、testing 根 PRD 与证据入口吸收并删除；`PRD-TESTING-MANUAL-001..003`、TMAN/DEC 历史映射保留在 `../prd.md`，实现过程从 Git history 与 GitHub task evidence 追溯。
- `web-ui-agent-browser-closure-manual.prd.md` 与 `.project.md` 分别保留 Web UI 闭环的需求 / 执行真值；对应 `*.manual.md` 承担实际步骤。
- `web-ui-playwright-closure-manual.design.md` 是该手册系列的 historical/shared design companion；真实操作从 `web-ui-playwright-closure-manual.manual.md` 进入。它仍受索引与审计留痕引用，不能当作无引用腐旧文件删除。

## 维护边界

- 新增或调整 operator 手册时，先更新本页的按问题分流；再在 `doc/testing/README.md`、`doc/testing/prd.index.md` 保持子树入口可达。
- 手册不得把 pure local playtest 与 formal public-testnet attach 证明混为一谈；具体边界以对应 runbook 为准。
- 历史设计 companion、PRD 和 project 只有在已被现行真值替代且 focused 引用审计确认无活跃调用时才能退役删除。
