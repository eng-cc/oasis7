# Limited Preview Contributor Reward Ledger Approved Pending Distribution（2026-04-13）

## Meta
- Round ID: `ROUND-LTRL-2026-04-01_2026-04-13`
- Candidate ID: `CAND-LTRL-2026-04-13-A`
- Window: 2026-04-01 -> 2026-04-13
- Claim Envelope: `limited playable technical preview`
- Owner Role: `liveops_community`
- Review Role: `producer_system_designer`
- Execution Role:
- Ledger Status: `approved`

## 1. Producer Decision Scope
- Approved rows in this round: `LTRL-PR-60`, `LTRL-PR-59`
- Excluded after scan: `PR #56`
- Exclusion note: PR #56 remains outside this round by explicit owner decision and is not approved in this candidate set.

## 2. Ledger
| Ledger ID | Contributor | Public Handle / GitHub | Reward Account | Source Type | Source Link | Contribution Type | Base Score | Quality Modifier | Total Score | Recommended Band | Duplicate Check | Reviewer | Review Status | Producer Decision | Approval ID | Actual Amount | Distribution Ref | Distribution Date | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| LTRL-PR-60 | @eng-cc | eng-cc | oc:pk:6a0701c8feff03ff02f16048c5447223708062e7e1221f79ff2410a22be1063d | PR | https://github.com/eng-cc/oasis7/pull/60 | C-03 | 40 | +20 | 60 | eligible-medium | unique | liveops_community | approved | approved eligible-medium under ordinary merged PR ceiling | APR-LTRL-2026-04-13-01 | 150 OC |  |  | pr_title=Fix release soak sequencer restart gate; merged_at=2026-04-13T09:41:14Z; evidence_context_link=https://github.com/eng-cc/oasis7/pull/60; producer_note=re-reviewed under the 2026-04-13 ordinary merged PR ceiling. This remains a high-value PR that reduces release-blocking operational risk with auditable soak evidence, but it is not being treated as a rare 1500 OC exceptional row.; amount_decision=round_specific 150 OC under ordinary merged PR ceiling, not a global contributor reward mapping; reviewer_note=release gate and sequencer restart recovery fix landed with short/full soak evidence, directly reducing release-blocking operational risk; intake_notes=Reward account normalized from local dev_config node public key under the current oc:pk convention. Passing short/full release soak evidence is recorded in task_de7f6b6454ba4647898ae2e3b75e4c8b execution log and .tmp soak summaries. |
| LTRL-PR-59 | @eng-cc | eng-cc | oc:pk:6a0701c8feff03ff02f16048c5447223708062e7e1221f79ff2410a22be1063d | PR | https://github.com/eng-cc/oasis7/pull/59 | C-03 | 40 | +10 | 50 | eligible-medium | unique | liveops_community | approved | approved eligible-medium under ordinary merged PR ceiling | APR-LTRL-2026-04-13-02 | 150 OC |  |  | pr_title=Add AutoNAT reachability evidence to p2p auto mode; merged_at=2026-04-13T07:47:21Z; evidence_context_link=https://github.com/eng-cc/oasis7/commit/9b39fea24b92fb806d9381818eda3f9adfb93e3a; producer_note=re-reviewed under the 2026-04-13 ordinary merged PR ceiling. This PR improves p2p diagnosis and operator trustworthiness, but it is still being handled as an ordinary merged PR row rather than a rare 1500 OC exceptional case.; amount_decision=round_specific 150 OC under ordinary merged PR ceiling, not a global contributor reward mapping; reviewer_note=reachability truth closure shipped with libp2p/node/runtime regression coverage and improves operator-facing p2p diagnosis quality; intake_notes=P2P reachability truth closure for AutoNAT/public-port evidence; includes libp2p/node/runtime regression coverage. |

## 3. Band Summary
| Band | Row Count | Contributor Count | Status |
| --- | --- | --- | --- |
| `eligible-large` | 0 | 0 | none |
| `eligible-medium` | 2 | 1 | approved |
| `eligible-small` | 0 | 0 | none |
| `no-token-recommendation` | 0 | 0 | none |

## 4. Approval Summary
- Producer Review Date: 2026-04-13
- Approved Rows: `LTRL-PR-60`, `LTRL-PR-59`
- Rejected Rows:
- Deferred Rows: `0`
- Approval Notes:
  - `LTRL-PR-60` re-reviewed as `eligible-medium` with approval id `APR-LTRL-2026-04-13-01` and round-specific amount decision `150 OC`
  - `LTRL-PR-59` re-reviewed as `eligible-medium` with approval id `APR-LTRL-2026-04-13-02` and round-specific amount decision `150 OC`
  - 本轮 amount decision 只对 `ROUND-LTRL-2026-04-01_2026-04-13` 生效，并按普通 merged PR 默认 ceiling 收紧到 `<=150 OC`；`1500 OC` 不再作为常规 MR 档位。
  - Distribution Ref / Distribution Date / Execution Owner 仍待 execution owner 回填。

## 5. Distribution Closure
| Approval ID | Ledger ID | Contributor | Actual Amount | Distribution Ref | Distribution Date | Execution Owner | Closure Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| APR-LTRL-2026-04-13-01 | LTRL-PR-60 | @eng-cc | 150 OC |  |  |  | `pending` |
| APR-LTRL-2026-04-13-02 | LTRL-PR-59 | @eng-cc | 150 OC |  |  |  | `pending` |

## 6. Next Actions
- Rows waiting distribution: `APR-LTRL-2026-04-13-01=150 OC`, `APR-LTRL-2026-04-13-02=150 OC`
- Execution owner must fill `Distribution Ref / Distribution Date / Execution Owner`.
- After both rows are distributed, update round summary and archive this ledger.
