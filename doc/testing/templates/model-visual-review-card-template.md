# Model Visual Review Card Template

## Review Metadata
- task_uid:
- PR / branch:
- commit:
- reviewer_model:
- review_date:
- surface:
- locale:
- viewport_set:

## Inputs
- change_goal:
- expected_visual_contract:
- screenshots:
  - desktop:
  - mobile:
  - baseline:
- automated_evidence:
- known_out_of_scope:

## Verdict
- verdict: `pass` / `watch` / `block` / `human_escalation`
- confidence: `high` / `medium` / `low`
- human_escalation_needed: `yes` / `no`
- owner_action:

## Must-Pass Checks
| Check | Result | Evidence / Note |
| --- | --- | --- |
| First visual focus matches the task goal |  |  |
| Main subject remains readable |  |  |
| No major overlap, crop, or horizontal overflow |  |  |
| UI state is honest against supplied state / DTO evidence |  |  |
| Action feedback / blocker / next step is visible when required |  |  |
| Diagnostics or debug panels do not dominate the primary player path |  |  |
| Desktop and mobile preserve the same priority order |  |  |

## Findings
1.

## Pixel-World Addendum
- agent_readability:
- fragment_background_role:
- location_logic_anchor_role:
- action_receipt_clarity:
- no_receipt_honesty:

## What This Review Does Not Prove
-

## Residual Risk
-

## Escalation Notes
- escalation_reason:
- requested_human_owner:
- decision_needed_by:
