# software_safe 主 Web 入口与 formal gameplay 证据（2026-04-07）

审计轮次: 1

## Meta
- 关联专题: `PRD-WORLD_SIMULATOR-039`
- 关联任务: `TASK-WORLD_SIMULATOR-304`
- 责任角色: `qa_engineer`
- 协作角色: `producer_system_designer`
- 当前结论: `pass`
- 目标: 按新的主入口 contract 重跑 `software_safe` formal Web gameplay 证据，确认默认 `/` 与 `render_mode=auto` 是否落到 `software_safe`，并把 release/current-entry claim 继续绑定到真实采证结果。

## 最终结论
- primary Web 入口 contract 已拿到新的 browser 证据:
  - 默认 `/` 会重定向到 `software_safe.html`
  - `?render_mode=auto` 会重定向到 `software_safe.html`
- `software_safe` 页面上的 canonical 文案已符合当前产品口径:
  - body 中可见 `Formal Gameplay Summary`
  - body 中可见 `Missing Action Handoff`
  - formal gameplay surface 不暴露 `main_token_transfer` 表单，资产/治理动作仍保持独立 handoff lane
- 2026-04-08 addendum：formal gameplay 已在 LLM-enabled 环境中补跑 PASS：
  - 复采链路使用 `config.toml` 中的 `base_url=https://api.letai.run/v1` 与 `model=gpt-5.4-mini`
  - 首次复采先暴露出 provider 兼容性缺口：该兼容层要求 list-shaped `Responses API input`、必须走 stream，而且完整 function call 只出现在 `response.output_item.done`，`response.completed.response.output` 会保持空数组
  - 对 `crates/oasis7/src/simulator/llm_agent/openai_payload.rs` 与 `crates/oasis7/src/simulator/llm_agent.rs` 补齐 stream/output-item 聚合后，再次复采已通过
- 因此当前可确认的真实状态是:
  - `software_safe` 已是低保真但正式可玩的主要 Web 入口的正确目标 surface
  - `software_safe` formal gameplay 已取得 release-grade PASS 证据；后续剩余工作主要是同步 README/current-entry/release claim 口径，而不是继续补主入口或 LLM provider 可达性

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
- 默认 `/` 最终 URL:
  - `http://127.0.0.1:4273/software_safe.html?...&render_mode=software_safe&software_safe_reason=primary_web_entry`
- `render_mode=auto` 最终 URL:
  - `http://127.0.0.1:4273/software_safe.html?...&render_mode=software_safe&software_safe_reason=auto_primary_web_entry`
- 对应截图:
  - `output/playwright/viewer-primary-web-entry/viewer-primary-web-entry-20260407-235000/default-entry.png`
  - `output/playwright/viewer-primary-web-entry/viewer-primary-web-entry-20260407-235000/auto-entry.png`

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

### 3. software_safe formal gameplay PASS（2026-04-08 follow-up）
- artifact: `output/playwright/viewer-software-safe-step/20260408-133532`
- 复采命令:
  - `./scripts/viewer-software-safe-step-regression.sh --url 'http://127.0.0.1:4373/?ws=ws://127.0.0.1:5211&test_api=1'`
- `software-safe-step-summary.md` 结论:
  - `ok=True`
  - `failCategory=None`
  - `renderMode=software_safe`
  - `stepAccepted=True`
  - `selectedAgentVisible=True`
  - `domFeedbackVisible=True`
  - `logicalTimeAdvanced=True`
  - `feedbackStage=completed_advanced`
- 该 PASS 之前的真实 blocker 不是主入口 contract，而是 provider 兼容层缺少以下处理：
  - `Responses API` 顶层 `input` 需为 list-shaped message items
  - 请求需改走 stream path
  - 需从 `response.output_item.done` 回补 function call output item，不能只信 `response.completed.response.output`

## 风险与剩余项
- 本文档已经完成 `TASK-WORLD_SIMULATOR-304` 的 QA 责任: 新 contract 下的 PASS/FAIL 证据已重跑并正式落档。
- 当前剩余缺口不再是主入口路由、`software_safe` 页面口径或 LLM provider 可达性；formal gameplay PASS 已拿到，剩余工作是把该 PASS 与 `PRD-WORLD_SIMULATOR-040 completed but keep experimental default-enable gate` 的边界一起同步到 README/current-entry/release claim。
- `PRD-WORLD_SIMULATOR-037/038` 仍需继续推进 parity/latency experimental 收口，因此 Local Provider 默认启用门禁并未因本次 PASS 自动解除。
