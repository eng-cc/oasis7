# Limited Preview Contributor Reward Ledger（2026-03-22）

审计轮次: 1

## Meta
- Round ID:
- Candidate ID:
- Window:
- Claim Envelope: `limited playable technical preview`
- Owner Role: `liveops_community`
- Review Role: `producer_system_designer`
- Execution Role:
- Ledger Status: `draft / under_review / approved / partially_distributed / archived`

## 1. Intake Rules
- Only record contribution-based rows.
- Use the scoring rules from `readme-limited-preview-contributor-reward-pack-2026-03-22.md`.
- Use `Contributor` and `Public Handle / GitHub` for review identity; keep `Reward Account` as the execution field.
- If `Source Type=PR`, first read the optional reward intake block from `.github/pull_request_template.md`.
- Do not include signup, login, casual play, online time, AFK, or vague praise as rewardable rows.
- Missing key evidence means default `deferred` or `no-token-recommendation`.

## 2. Ledger
| Ledger ID | Contributor | Public Handle / GitHub | Reward Account | Source Type | Source Link | Contribution Type | Base Score | Quality Modifier | Total Score | Recommended Band | Duplicate Check | Reviewer | Review Status | Producer Decision | Approval ID | Actual Amount | Distribution Ref | Distribution Date | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| LTRL-001 |  |  |  | `issue / PR / DM / content / session summary` |  | `C-01..C-07` |  |  |  | `no-token-recommendation / eligible-small / eligible-medium / eligible-large` | `unique / duplicate / superseded` |  | `draft / reviewed / approved / rejected / deferred / distributed` |  |  |  |  |  |  |

## 3. Evidence Checklist
Each non-rejected row should include:
- `proof_link`
- `issue_or_pr_link` if available
- `build_id / env`
- `repro_steps` or `session summary`
- `why_this_matters`
- `duplicate_check`
- `reviewer_note`

If a contributor only provides raw account derivation material, normalize it to `Reward Account` before filling the ledger.
If a PR source has no reward intake block, treat it as "no reward review requested". Only create or advance a reward row after `Reward Account` is backfilled from an approved follow-up channel.

## 3.1 Scripted Import
Use the repo script before copying PR-sourced rows into the ledger:

```bash
./scripts/readme-reward-pr-intake-import.py --pr 123
./scripts/readme-reward-pr-intake-import.py \
  --body-file /tmp/pr-body.md \
  --source-link https://github.com/<owner>/<repo>/pull/123 \
  --public-handle builder01 \
  --contributor @builder01 \
  --ledger-id LTRL-PR-001 \
  --format ledger-md
```

Recommended handling:
- `ready`: import the emitted row as `draft`.
- `deferred`: keep the row deferred until claimant fields are completed.
- `no_reward_review_requested`: do not create a ledger row.
- `invalid_intake`: ask the PR author to fix or delete the intake block before import.

## 4. Band Summary
| Band | Row Count | Contributor Count | Status |
| --- | --- | --- | --- |
| `eligible-large` |  |  |  |
| `eligible-medium` |  |  |  |
| `eligible-small` |  |  |  |
| `no-token-recommendation` |  |  |  |

## 5. Approval Summary
- Producer Review Date:
- Approved Rows:
- Rejected Rows:
- Deferred Rows:
- Approval Notes:

## 6. Distribution Closure
| Approval ID | Ledger ID | Contributor | Actual Amount | Distribution Ref | Distribution Date | Execution Owner | Closure Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |  | `pending / distributed / failed / retried` |

## 7. Safe Copy Check
- Public wording reviewed: `yes / no`
- Forbidden phrase hits: `0`
- Notes:

Forbidden phrasing still includes:
- `play-to-earn`
- `login reward`
- `time played = token`
- `come play to earn`
- `airdrop for players`
- `just try the game and get token`

## 8. Next Actions
- Unresolved items:
- Missing accounts:
- Rows waiting producer review:
- Rows waiting distribution:
- Archive note:
