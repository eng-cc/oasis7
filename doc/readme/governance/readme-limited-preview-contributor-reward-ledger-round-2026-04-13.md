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
| LTRL-PR-60 | @eng-cc | eng-cc | oc:pk:6a0701c8feff03ff02f16048c5447223708062e7e1221f79ff2410a22be1063d | PR | https://github.com/eng-cc/oasis7/pull/60 | C-03 | 40 | +10 | 50 | eligible-medium | unique | liveops_community | approved | approved eligible-medium after actual-value review | APR-LTRL-2026-04-13-01 | 100 OC |  |  | pr_title=Fix release soak sequencer restart gate; merged_at=2026-04-13T09:41:14Z; evidence_context_link=https://github.com/eng-cc/oasis7/pull/60; producer_note=actual-value review concludes this PR has real release-risk reduction value, but the delivered increment remains ordinary maintenance and gate repair rather than a rare breakthrough contribution. Downshifted from the planned high grant to a conservative medium row.; amount_decision=round_specific 100 OC after actual-value review, not a global contributor reward mapping; reviewer_note=release gate and sequencer restart recovery fix landed with soak evidence, but most value is repair/closure of expected engineering obligations rather than exceptional ecosystem leverage; intake_notes=Reward account normalized from local dev_config node public key under the current oc:pk convention. Passing short/full release soak evidence is recorded in task_de7f6b6454ba4647898ae2e3b75e4c8b execution log and .tmp soak summaries. |
| LTRL-PR-59 | @eng-cc | eng-cc | oc:pk:6a0701c8feff03ff02f16048c5447223708062e7e1221f79ff2410a22be1063d | PR | https://github.com/eng-cc/oasis7/pull/59 | C-03 | 40 | +0 | 40 | eligible-small | unique | liveops_community | approved | approved eligible-small after actual-value review | APR-LTRL-2026-04-13-02 | 50 OC |  |  | pr_title=Add AutoNAT reachability evidence to p2p auto mode; merged_at=2026-04-13T07:47:21Z; evidence_context_link=https://github.com/eng-cc/oasis7/commit/9b39fea24b92fb806d9381818eda3f9adfb93e3a; producer_note=actual-value review concludes this PR mainly improves reachability truth and operator diagnosis quality. It is useful, but the delivered increment is closer to observability and recommendation closure than a high-leverage exceptional contribution, so the planned grant is downshifted to a small row.; amount_decision=round_specific 50 OC after actual-value review, not a global contributor reward mapping; reviewer_note=AutoNAT/public-port truth closure shipped with regression coverage, but the practical increment is primarily diagnostics and recommendation accuracy rather than a direct release-blocking fix; intake_notes=P2P reachability truth closure for AutoNAT/public-port evidence; includes libp2p/node/runtime regression coverage. |

## 3. Band Summary
| Band | Row Count | Contributor Count | Status |
| --- | --- | --- | --- |
| `eligible-large` | 0 | 0 | none |
| `eligible-medium` | 1 | 1 | approved |
| `eligible-small` | 1 | 1 | approved |
| `no-token-recommendation` | 0 | 0 | none |

## 4. Approval Summary
- Producer Review Date: 2026-04-13
- Approved Rows: `LTRL-PR-60`, `LTRL-PR-59`
- Rejected Rows:
- Deferred Rows: `0`
- Approval Notes:
  - `LTRL-PR-60` 经 actual-value review 下调为 `eligible-medium`，approval id `APR-LTRL-2026-04-13-01`，round-specific amount decision=`100 OC`
  - `LTRL-PR-59` 经 actual-value review 下调为 `eligible-small`，approval id `APR-LTRL-2026-04-13-02`，round-specific amount decision=`50 OC`
  - 本轮 amount decision 只对 `ROUND-LTRL-2026-04-01_2026-04-13` 生效；原先 planned grant 高于实际增量价值，因此在执行前已先下调，且最终金额都保持在普通 merged PR `<=150 OC` ceiling 内，不把 contributor reward 全局改成固定金额映射。
  - Distribution Ref / Distribution Date / Execution Owner 仍待 execution owner 回填。

## 5. Distribution Closure
| Approval ID | Ledger ID | Contributor | Actual Amount | Distribution Ref | Distribution Date | Execution Owner | Closure Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| APR-LTRL-2026-04-13-01 | LTRL-PR-60 | @eng-cc | 100 OC |  |  |  | `pending` |
| APR-LTRL-2026-04-13-02 | LTRL-PR-59 | @eng-cc | 50 OC |  |  |  | `pending` |

## 6. Next Actions
- Rows waiting distribution: `APR-LTRL-2026-04-13-01=100 OC`, `APR-LTRL-2026-04-13-02=50 OC`
- Execution owner must fill `Distribution Ref / Distribution Date / Execution Owner`.
- After both rows are distributed, update round summary and archive this ledger.
