# GWSC State-Sync Closure Evidence Packet Template

审计轮次: 1

## Purpose
- Provide the standard evidence packet for GWSC `module_full` state-sync closure.
- Bind commit propagation, peer heads, gap/state-sync, blob closure, and observer catch-up evidence into one auditable record.
- Keep the claim boundary explicit: this packet can support `module_full`; it does not by itself claim `integration_required`, `release_full`, or public_testnet readiness.

## Usage
- Copy this template into `doc/testing/evidence/` or a task-owned evidence path before filling real values.
- If any required field remains empty, set `packet_verdict` to `blocked`.
- Do not use this template file itself as pass evidence in readiness lanes.
- Manual data/checkpoint/seed copy may be recorded as recovery context, but it cannot count as live state-sync closure.

## Claim Boundary
| Field | Value |
| --- | --- |
| Supported claim | `module_full` |
| Does not claim | `integration_required`, `release_full`, `public_testnet ready`, mainnet readiness |
| Requires follow-up for stronger claim | S10/API-viewer projection evidence and same-window real-env/public_testnet readiness lanes |

## Packet
### Meta
- `packet_id`:
- `task_uid`:
- `git_sha`:
- `branch`:
- `world_id`:
- `candidate_id`:
- `run_window_start`:
- `run_window_end`:
- `owner_role`: `qa_engineer`
- `runtime_owner`:
- `blockchain_ops_owner`:
- `packet_verdict`: `pass` / `fail` / `blocked`
- `claim_boundary`: `module_full`

### Command Evidence
| Check | Command | Result | Evidence path |
| --- | --- | --- | --- |
| GWSC module_required | `./scripts/game-world-state-sync-commit-module-required.sh` |  |  |
| Mixed topology full | `./scripts/p2p-mixed-topology-matrix.sh --tier full` |  |  |
| Triad soak | `./scripts/p2p-longrun-soak.sh --profile soak_smoke --topologies triad --duration-secs 600 --no-prewarm` |  |  |
| State-sync closure | `./scripts/p2p-verify-state-sync-closure.sh --world-dir <seed-world-dir> --execution-records-dir <seed-execution-records-dir> --store-dir <seed-store-dir> --out <report.json>` |  |  |

### Topology And Node Truth
| Node role | Node id | Endpoint / port | Data root | Status evidence |
| --- | --- | --- | --- | --- |
| sequencer |  |  |  |  |
| storage |  |  |  |  |
| validator |  |  |  |  |
| observer |  |  |  |  |

### Commit Propagation
| Field | Expected | Observed | Evidence path |
| --- | --- | --- | --- |
| `submitted_action_count` | `> 0` |  |  |
| `execution_record_count` | `>= submitted_action_count` |  |  |
| `receipt_count` | `>= submitted_action_count` |  |  |
| `committed_height_monotonic` | `true` |  |  |
| `consensus_hash_consistent` | `true` |  |  |
| `consensus_hash_mismatch_count` | `0` |  |  |

### Peer Heads And Gap Sync
| Field | Expected | Observed | Evidence path |
| --- | --- | --- | --- |
| `known_peer_heads_zero_samples` | `0` |  |  |
| `peer_heads_fresh` | `true` |  |  |
| `gap_sync_completed` | `true` |  |  |
| `replication_error_sustained` | `false` |  |  |
| `observer_catch_up_completed` | `true` |  |  |

### State-Sync Bundle And Blob Closure
| Field | Expected | Observed | Evidence path |
| --- | --- | --- | --- |
| `checkpoint_height` | `> 0` |  |  |
| `checkpoint_hash` | non-empty |  |  |
| `state_sync_bundle_ref` | real artifact path |  |  |
| `closure_report_ref` | real JSON report path |  |  |
| `missing_blob_count` | `0` |  |  |
| `manual_copy_used` | `false` |  |  |

### Failure Signatures
| Signature | Present | Evidence path | Owner | Follow-up |
| --- | --- | --- | --- | --- |
| `consensus_hash_divergence` | `no` |  | runtime_engineer |  |
| `committed_height_not_monotonic` | `no` |  | runtime_engineer |  |
| `known_peer_heads_zero_samples` | `no` |  | runtime_engineer / blockchain_ops_engineer |  |
| `missing_blob_count > 0` | `no` |  | runtime_engineer |  |
| `observer_catch_up_failed` | `no` |  | blockchain_ops_engineer |  |
| manual data/checkpoint/seed copy | `no` |  | blockchain_ops_engineer |  |

### Verdict
- `pass_conditions_met`: `yes` / `no`
- `blocked_reason`:
- `residual_risk`:
- `next_claim_level`: `integration_required` / `release_full` / `none`
- `required_follow_up_for_next_claim`:

## Minimum Review Checklist
- `packet_verdict=pass` only when all command evidence rows have real artifact paths and passing results.
- `claim_boundary` remains `module_full`.
- `missing_blob_count` is exactly `0`.
- Peer heads are non-empty and fresh in the same window as commit evidence.
- Observer catch-up is recorded separately from blob closure.
- No manual data/checkpoint/seed copy is counted as live sync evidence.
- Stronger claims cite S10/API-viewer projection and same-window real-env/public_testnet readiness evidence outside this packet.
