# Closed Beta Candidate Feedback & Incident Templates（2026-03-22）

## Feedback Template

| Field | Description |
| --- | --- |
| Round ID | `LTP-20260322-R1` |
| Signal Type | `bug` / `friction` / `idea` |
| Source | `Moltbook comment` / `direct message` / `builder issue` |
| Candidate Evidence | Link to `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md` |
| Signal Bucket | `Blocking` / `Opportunity` / `Idea` |
| Claim Drift | `yes` / `no` |
| Severity | `blocking` / `non-blocking` |
| Owner | `qa_engineer` / `runtime_engineer` / `liveops_community` |
| Next Action | e.g., `issue created, awaiting QA triage` |
| Response | Short text referencing “limited playable technical preview” & GitHub CTA |

Use the above table to log every closed beta candidate signal before moving it into `doc/devlog/README.md`.

## Incident Template

| Field | Description |
| --- | --- |
| Round ID | `LTP-20260322-R1` |
| Incident ID | `CB-20260322-XX` |
| Trigger | e.g., `Moltbook thread claiming Beta`, `blocking candidate bug` |
| Evidence | Provide `issue URL / log / screenshot` + release gate link |
| Immediate Action | `stop replies`, `post update referencing limited playable technical preview`, `escalate to producer` |
| Escalation Path | `producer_system_designer`, `qa_engineer`, `runtime_engineer` |
| Follow-up | `next check-in time`, `owner for fix`, `target release gate status` |

Record every incident in the incident template and push summary to QA/liveops owners within 3h.
