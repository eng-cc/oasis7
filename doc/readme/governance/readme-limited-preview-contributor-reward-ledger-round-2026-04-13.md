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
| LTRL-PR-60 | @eng-cc | eng-cc | oc:pk:6a0701c8feff03ff02f16048c5447223708062e7e1221f79ff2410a22be1063d | PR | https://github.com/eng-cc/oasis7/pull/60 | C-03 | 80 | +20 | 100 | eligible-large | unique | liveops_community | approved | approved eligible-large | APR-LTRL-2026-04-13-01 | 1500 OC |  |  | pr_title=Fix release soak sequencer restart gate; merged_at=2026-04-13T09:41:14Z; evidence_context_link=https://github.com/eng-cc/oasis7/pull/60; producer_note=approved as a high-value merged PR that directly reduces release-blocking operational risk with auditable soak evidence; amount_decision=round_specific 1500 OC for this eligible-large row, not a global contributor reward mapping; reviewer_note=release gate and sequencer restart recovery fix landed with short/full soak evidence, directly reducing release-blocking operational risk; intake_notes=Reward account normalized from local dev_config node public key under the current oc:pk convention. Passing short/full release soak evidence is recorded in task_de7f6b6454ba4647898ae2e3b75e4c8b execution log and .tmp soak summaries. |
| LTRL-PR-59 | @eng-cc | eng-cc | oc:pk:6a0701c8feff03ff02f16048c5447223708062e7e1221f79ff2410a22be1063d | PR | https://github.com/eng-cc/oasis7/pull/59 | C-03 | 80 | +10 | 90 | eligible-large | unique | liveops_community | approved | approved eligible-large | APR-LTRL-2026-04-13-02 | 1500 OC |  |  | pr_title=Add AutoNAT reachability evidence to p2p auto mode; merged_at=2026-04-13T07:47:21Z; evidence_context_link=https://github.com/eng-cc/oasis7/commit/9b39fea24b92fb806d9381818eda3f9adfb93e3a; producer_note=approved as a high-value infrastructure truth-closure PR that improves p2p diagnosis and operator trustworthiness; amount_decision=round_specific 1500 OC for this eligible-large row, not a global contributor reward mapping; reviewer_note=reachability truth closure shipped with libp2p/node/runtime regression coverage and improves operator-facing p2p diagnosis quality; intake_notes=P2P reachability truth closure for AutoNAT/public-port evidence; includes libp2p/node/runtime regression coverage. |

## 3. Band Summary
| Band | Row Count | Contributor Count | Status |
| --- | --- | --- | --- |
| `eligible-large` | 2 | 1 | approved |
| `eligible-medium` | 0 | 0 | none |
| `eligible-small` | 0 | 0 | none |
| `no-token-recommendation` | 0 | 0 | none |

## 4. Approval Summary
- Producer Review Date: 2026-04-13
- Approved Rows: `LTRL-PR-60`, `LTRL-PR-59`
- Rejected Rows:
- Deferred Rows: `0`
- Approval Notes:
  - `LTRL-PR-60` approved as `eligible-large` with approval id `APR-LTRL-2026-04-13-01` and round-specific amount decision `1500 OC`
  - `LTRL-PR-59` approved as `eligible-large` with approval id `APR-LTRL-2026-04-13-02` and round-specific amount decision `1500 OC`
  - 本轮 amount decision 只对 `ROUND-LTRL-2026-04-01_2026-04-13` 生效，不把 contributor reward 全局改成固定 `eligible-large=1500 OC` 映射。
  - Producer 同步冻结当前 `main_token_config.initial_supply = 10,000,000,000 OC`；据此，`early_contributor_reward_reserve = 1,500,000,000 OC`。
  - `1500 OC` 单笔发放占总发行量 `0.000015%`，占 early contributor reward reserve `0.0001%`。
  - 本轮两笔合计 `3000 OC`，占总发行量 `0.00003%`，占 early contributor reward reserve `0.0002%`。
  - Distribution Ref / Distribution Date / Execution Owner 仍待 execution owner 回填。

## 5. Distribution Closure
| Approval ID | Ledger ID | Contributor | Actual Amount | Distribution Ref | Distribution Date | Execution Owner | Closure Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| APR-LTRL-2026-04-13-01 | LTRL-PR-60 | @eng-cc | 1500 OC |  |  |  | `pending` |
| APR-LTRL-2026-04-13-02 | LTRL-PR-59 | @eng-cc | 1500 OC |  |  |  | `pending` |

## 6. Next Actions
- Rows waiting distribution: `APR-LTRL-2026-04-13-01=1500 OC`, `APR-LTRL-2026-04-13-02=1500 OC`
- Execution owner must fill `Distribution Ref / Distribution Date / Execution Owner`.
- After both rows are distributed, update round summary and archive this ledger.
