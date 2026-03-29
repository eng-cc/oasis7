# Closed Beta Candidate LiveOps Runbook（2026-03-22）

审计轮次: 1

- 对应设计文档: `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.project.md`

- ## Meta
- Owner Role: `liveops_community`
- Review Role: `producer_system_designer`
- Channel Focus: `GitHub issue` + `GitHub Builder Channels` + `Moltbook`（optional fallback）
- Scope: `closed beta candidate 招募/反馈/事故回流 + technical preview 口径守护`
- Source Docs:
  - `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
  - `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.project.md`
  - `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.md`

## 目标
- 确定 `closed_beta_candidate` 招募/反馈/事故流程在 `technical preview` 口径内执行，避免过早对外升级口径。
- 让 `liveops_community` 在 unified release gate 前能提供稳定 evidence link 与 runbook。

## 范围
- 本文档覆盖 GitHub issue / Builder channel / Moltbook fallback 的招募呼叫、反馈分类与 incident 升级流程，不包含真正 release announcement。
- 不涉及时程/资源/融资的业务宣发，只聚焦 `technical preview` 体验反馈。

## 接口 / 数据
- `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
- `doc/testing/evidence/testing-quality-trend-baseline-2026-03-11.md`
- `doc/playability_test_result/templates/closed-beta-candidate-incident-templates-2026-03-22.md`
- `doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md`

## 里程碑
- M0: 2026-03-22 — 完成 runbook / template 工具链并把 evidence link 交给 producer。
- M1: 邀请 candidate builders、记录前 3 条信号。
- M2: 与 QA/runtime/liveops 共同确认 `closed_beta_candidate` gate 需求。

## 风险
- 若沟通误用了 `closed beta`/`live now`，会直接冲突 `technical preview` claim。
- 若没有统一 feedback template，会导致 signal 分散到不同文档池。
- 若 incident 处理流程不透明，release gate 可能误判 release readiness。

## 1. Runbook Intent
- 本跑道面向 `PRD-GAME-009` 中的 `closed_beta_candidate` 准入节奏。所有活动都必须默认 `limited playable technical preview` 口径，公开声明不能含 `closed beta / live now / play now`。
- 目标是把候选版本的招募、反馈、事故三个闭环沉淀成可重复模板，便于 producer 在阶段评审前对外同步一致的 evidence link 与禁语。
- 本跑道不把 primary channel 绑定到单一平台；可优先采用 GitHub issue 线程，也可在需要时复用 Moltbook 等外部 builder channel。完成招聘/反馈/事故闭环后需把信号回流给 `producer_system_designer` 决策阶段升级或维持当前技术预览声明。

## 2. Recruitment Template
### Key Messaging
1. `Controlled builder-facing callout`: “Inspect the technical preview, then huddle onto GitHub issue/PR for follow-up.”（不要说“come play now”）
2. `Candidate constraints`: 明确“limited slots, technical preview posture, hotfix-level response”。
3. `CTA`: Provide `GitHub issue template URL` or `builder inbox` (no direct `public launch` claim).

### Template Block
```
Post Title: Closed Beta Candidate Builder Call
Body:
- Status: limited playable technical preview.
- Evidence: canonical candidate release gate link: `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`.
- Goal: gather builder feedback for `closed_beta_candidate` gate.
- CTA: open a GitHub issue with `candidate-feedback` tag or provide PR/patch.
- Response promise: we monitor the primary thread/comments and will escalate issues marked `blocking`.
```
先通过上面 block 内给出 disclaimers，再在 comments/pinned mention `@liveops-community` for urgent escalations.

## 3. Feedback Intake Flow
1. `Signal surface`: comment, DM, or GitHub.
2. `Categorize`:
   - `Blocking`: actual bug/crash/perf not meeting release gate (escalate to qa_engineer + runtime_engineer).
   - `Opportunity`: small UX friction or doc gap (log to runbook, add to summary).
   - `Idea`: deferred to producer_system_designer review queue.
3. `Action`:
   - Blocking: escalate via `liveops-to-qa` template, link to output evidence + start `inspection thread`.
   - Non-blocking: respond in the primary thread with “Thanks, filing `issue-track-*` and share ETA once triaged,” keep tone `technical preview`.
4. `Log`:
   - Add to `doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md` (template below).
   - Attach `issue/PR link`, `owner`, `next action`.

## 4. Incident & Accident Escalation
| Level | Trigger | Immediate Response | Escalation |
| --- | --- | --- | --- |
| P0 | data loss, seeded builder outage, candidate gating regression | Stop new replies, send pinned comment clarifying preview posture, route to `producer_system_designer` + `qa_engineer` | Reserve emergency `call` + revert message if public claim drift occurs |
| P1 | MISLEADING claim (e.g., “it launched”), reproduced bug in candidate path | Respond with safe message (see FAQ below), log to `incident template`, escalate to runtime/qa owner | Requirement for `incident summary` within 3h, include `evidence link`+`candidate gate status` |
| P2 | high value feedback or repeated bug but not gating | Log and update liveops feed, escalate to qa/engineer if occurs 2+ times per day | Track in `doc/devlog` when aggregated |

## 5. FAQ & Disclaimers
- Q: “Can I play the closed beta now?” → “It’s still a limited playable technical preview; the closed-beta gate is still being validated. Please inspect the candidate evidence link and file a GitHub issue if you hit a blocker.”
- Q: “Is this a public release?” → “No, we continue to treat it as `closed_beta_candidate` with limited invite and require a candidate evidence bundle before broader beta talk.”
- Q: “Will there be a Moltbook integration officially?” → “The candidate is preview-only; no official integration is announced yet.”
- Q: “When will open availability happen?” → `Redirect to producer_system_designer` and reiterate “subject to release gate signals.”
- Forbidden statements: `live now`, `play now`, `public launch`, `closed beta`, `official integration announced`.
- Allowed statements: `technical preview`, `candidate access`, `builder feedback`, `GitHub issue/PR CTA`, `preview posture`, `limited builder loop`.

## 6. Templates & Evidence Links
- `Incident template`: `doc/playability_test_result/templates/closed-beta-candidate-incident-templates-2026-03-22.md` (referenced below).
- `Feedback template`: 
  ```
  Signal type: `bug / friction / idea`
  Source: `GitHub issue comment / linked issue / DM / Moltbook comment`
  Evidence link: candidate release gate doc
  Owner: `qa_engineer/runtime_engineer/liveops_community`
  Next action: e.g., `issue created, awaiting QA triage`
  ```
- Evidence to attach: `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`, `doc/testing/evidence/testing-quality-trend-baseline-2026-03-11.md`, `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.project.md`.

## 7. Daily Logging & Review
- For every outreach or signal, log `time`, `role liveops_community`, `source`, `action taken`, `owner`, `next action`.
- Weekly recap: `what posts drove candidate signals`, `which statements caused “is it live” confusion`, `converted signals to issue/PR`, `next promotional focus (world proof / builder hook / pure_api)`.

## 8. Follow-up & Next Steps
- Coordinate with `producer_system_designer` before any claim upgrade; keep `technical preview` status until the unified release gate (TASK-GAME-031) yields `pass`.
- If candidate evidence gate passes, update this document with new statements and escalate training for new closed beta wording.
