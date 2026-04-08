# software_safe 主 Web 入口与 formal gameplay 证据（2026-04-07）

审计轮次: 1

## Meta
- 关联专题: `PRD-WORLD_SIMULATOR-039`
- 关联任务: `TASK-WORLD_SIMULATOR-304`
- 责任角色: `qa_engineer`
- 协作角色: `producer_system_designer`
- 当前结论: `blocked`
- 目标: 按新的主入口 contract 重跑 `software_safe` formal Web gameplay 与 `standard` visual QA 证据，确认默认 `/` 与 `render_mode=auto` 是否落到 `software_safe`，并把 release/current-entry claim 继续绑定到真实采证结果。

## 最终结论
- primary Web 入口 contract 已拿到新的 browser 证据:
  - 默认 `/` 会重定向到 `software_safe.html`
  - `?render_mode=auto` 会重定向到 `software_safe.html`
  - 显式 `?render_mode=standard` 仍停留在标准 Viewer surface，能完成最小 visual QA 取样
- `software_safe` 页面上的 canonical 文案已符合当前产品口径:
  - body 中可见 `Formal Gameplay Summary`
  - body 中可见 `Missing Action Handoff`
  - formal gameplay surface 不暴露 `main_token_transfer` 表单，资产/治理动作仍保持独立 handoff lane
- 当前 formal gameplay 证据结论不是 PASS，而是 `blocked`:
  - `step` 控制请求会被接受，但逻辑时间与事件序列都未推进
  - `gameplaySummary.blockerKind=llm_required`
  - `gameplaySummary.blockerDetail=gameplay requires a configured and reachable LLM provider: llm init failed for agent-0: llm config error: missing env variable: OASIS7_LLM_MODEL`
- QA shell 与当前 worktree 配置同时证明这是环境阻断而不是主入口 contract 回归:
  - 当前 shell 未注入任何 `OASIS7_LLM_*` / `OPENAI_*` / `ANTHROPIC_*` 环境变量
  - 当前 `config.toml` 只包含 `[node]`，没有可供 runtime live 读取的 `[llm]` / provider / model 配置
- 因此本轮可确认的真实状态是:
  - `software_safe` 已是低保真但正式可玩的主要 Web 入口的正确目标 surface
  - `standard` 继续是显式 visual QA surface
  - release claim、README 与 current-entry 口径暂时不能写成“formal gameplay 已完成 release-ready PASS”；若需要对外更新，只能写成“入口 contract 已完成，formal gameplay 仍待 LLM-enabled 环境复采”

## 执行命令
- primary entry contract:
  - `bash ./scripts/viewer-primary-web-entry-regression.sh --viewer-port 4273 --web-bind 127.0.0.1:5111 --live-bind 127.0.0.1:5123`
- software_safe formal gameplay:
  - `./scripts/viewer-software-safe-step-regression.sh --viewer-port 4373 --web-bind 127.0.0.1:5211 --live-bind 127.0.0.1:5223`
- 环境检查:
  - `env | rg '^OASIS7_LLM_|^OPENAI_|^ANTHROPIC_' || true`
  - `sed -n '1,220p' config.toml`

## 浏览器证据
### 1. primary Web 入口 PASS
- artifact: `output/playwright/viewer-primary-web-entry/viewer-primary-web-entry-20260407-235000`
- `summary.md` 结论:
  - Overall: `pass`
  - Formal gameplay entry (`/`): `software_safe`
  - Auto entry (`?render_mode=auto`): `software_safe`
  - Explicit visual QA entry (`?render_mode=standard`): `standard`
- 默认 `/` 最终 URL:
  - `http://127.0.0.1:4273/software_safe.html?...&render_mode=software_safe&software_safe_reason=primary_web_entry`
- `render_mode=auto` 最终 URL:
  - `http://127.0.0.1:4273/software_safe.html?...&render_mode=software_safe&software_safe_reason=auto_primary_web_entry`
- `render_mode=standard` 最终 URL:
  - `http://127.0.0.1:4273/?...&render_mode=standard`
- 对应截图:
  - `output/playwright/viewer-primary-web-entry/viewer-primary-web-entry-20260407-235000/default-entry.png`
  - `output/playwright/viewer-primary-web-entry/viewer-primary-web-entry-20260407-235000/auto-entry.png`
  - `output/playwright/viewer-primary-web-entry/viewer-primary-web-entry-20260407-235000/standard-entry.png`

### 2. software_safe formal gameplay BLOCKED
- artifact: `output/playwright/viewer-software-safe-step/20260407-232845`
- `software-safe-step-summary.md` 结论:
  - `failCategory=no_progress_after_step`
  - `renderMode=software_safe`
  - `stepAccepted=True`
  - `selectedAgentVisible=True`
  - `domFeedbackVisible=True`
  - `logicalTimeAdvanced=False`
  - `eventSeqAdvanced=False`
  - `feedbackStage=completed_timeout`
  - `feedbackReason=timeout_no_progress`
- `final_state.json` 明确显示:
  - `gameplaySummary.stageStatus=blocked`
  - `gameplaySummary.blockerKind=llm_required`
  - `availableActions.advance_step.disabledReason` 指向 `missing env variable: OASIS7_LLM_MODEL`
  - `assetGovernanceHandoff=Asset/governance actions remain a separate lane. software_safe exposes no main token transfer form here.`

## 风险与剩余项
- 本文档已经完成 `TASK-WORLD_SIMULATOR-304` 的 QA 责任: 新 contract 下的 PASS/FAIL 证据已重跑并正式落档。
- 当前剩余缺口不再是主入口路由或 `software_safe` 页面口径，而是 QA/runtime 环境缺少可用 LLM provider 配置，导致 formal gameplay 无法完成 release-grade PASS 复采。
- 下一轮若要把 release/current-entry claim 改成“formal Web gameplay PASS”，必须先在同一链路下提供可达的 `OASIS7_LLM_MODEL` + provider 配置，然后重跑 `viewer-software-safe-step-regression.sh` 获取成功证据。
