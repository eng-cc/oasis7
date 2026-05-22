# public_testnet ECS freshness audit (2026-05-22)

## Meta
- owner_role: `runtime_engineer`
- scope: current live `public_testnet` freshness and readiness recheck for `oasis7-public-testnet-parallel-20260518`
- lane_verdict:
  - `public_rpc_ready=partial`
  - `explorer_public_ready=partial`
  - `faucet_guard_ready=partial`
  - `runtime_bootstrap=block`
- aggregate_readiness_impact: `overall_readiness` remains `block`

## Reviewed inputs
- `doc/testing/evidence/public-testnet-live-candidate-endpoint-deploy-2026-05-19.md`
- `doc/testing/evidence/p2p-public-testnet-faucet-service-2026-05-19.md`
- `doc/testing/evidence/public-testnet-claims-boundary-review-2026-05-21.md`
- `doc/testing/evidence/public-testnet-live-candidate-manifest-2026-05-22.json`
- `.tmp/p2p_testnet_reality/20260522-100229/summary.md`

## Public endpoint freshness recheck
- Control-host checks at `2026-05-22 10:08:19 CST`:
  - `curl http://39.104.204.172:6631/v1/chain/status`
  - `curl http://39.104.205.67:6632/v1/chain/explorer/overview`
  - `curl http://39.104.204.172:6681/`
- Observed results:
  - RPC endpoint still returns `ok=true`, but the sequencer status now reports:
    - `last_error = node execution error: execution driver missing predecessor record for non-contiguous committed height`
    - `committed_height = null`
    - `network_committed_height = null`
    - `network_tier.bootstrap_peer_count = 2`
  - Explorer overview still returns `ok=true`, but `latest_height=12726` while the same-window triad snapshot shows no recent progress signal.
  - Faucet info endpoint still returns `ok=true`, `amount=1000000`, and `cooldown_secs=3600`, but this audit did not re-run a fresh end-to-end claim after the runtime drift was observed.

## Same-window triad snapshot
- Canonical command used:
```bash
P2PARCH6_SEQ_SSH_PASSWORD='***' \
P2PARCH6_STORAGE_SSH_PASSWORD='***' \
./scripts/p2p-real-env-triad-snapshot.sh \
  --samples 2 \
  --interval-secs 3 \
  --out-dir .tmp/p2p_testnet_reality \
  --world-id oasis7-public-testnet-parallel-20260518 \
  --local-service oasis7-testnet-observer.service \
  --local-status-url http://127.0.0.1:6633/v1/chain/status \
  --local-health-url http://127.0.0.1:6633/healthz \
  --local-env-file /opt/oasis7/p2p-testnet-local/config/node.env \
  --sequencer-target root@39.104.204.172 \
  --sequencer-service oasis7-testnet-sequencer.service \
  --sequencer-status-url http://127.0.0.1:6631/v1/chain/status \
  --sequencer-health-url http://127.0.0.1:6631/healthz \
  --sequencer-env-file /opt/oasis7/p2p-testnet/config/node.env \
  --storage-target root@39.104.205.67 \
  --storage-service oasis7-testnet-storage.service \
  --storage-status-url http://127.0.0.1:6632/v1/chain/status \
  --storage-health-url http://127.0.0.1:6632/healthz \
  --storage-env-file /opt/oasis7/p2p-testnet/config/node.env
```
- Summary:
  - `claim_status=partial_with_local_validator_blocker`
  - `claim_mode=three_equal_validator`
  - failure signatures:
    - `cloud_pair_no_recent_progress_signal`
    - `local_committed_height_zero`
    - `local_known_peer_heads_zero`
    - `local_network_committed_height_zero`
    - `local_no_recent_progress_signal`
- Node-specific observations:
  - local node:
    - `service=active`
    - `network_tier=null`
    - `last_error=fetch-commit authorization failed`
    - `committed_height=0 -> 0`
  - ECS sequencer:
    - `service=active`
    - `last_error=execution driver missing predecessor record for non-contiguous committed height`
    - `committed_height=7207 -> 7207`
    - `network_committed_height=7208 -> 7208`
  - ECS storage:
    - `service=active`
    - `last_error=null`
    - `committed_height=12729 -> 12729`
    - `network_committed_height=12729 -> 12729`

## Config drift confirmed
- Local host drift:
  - `oasis7-testnet-observer.service` is active, but `/opt/oasis7/p2p-testnet-local/bin/start-node.sh` does not pass `--network-tier-manifest`.
  - `/opt/oasis7/p2p-testnet-local/config/node.env` still pins a three-validator set:
    - `triad-testnet-local`
    - `triad-testnet-sequencer`
    - `triad-testnet-storage`
  - local `REPLICATION_REMOTE_WRITERS_CSV` still includes the local signer, which no longer matches the two-validator ECS topology.
- ECS truth:
  - both ECS nodes load `/opt/oasis7/p2p-testnet/config/network-tier-public-testnet-live-candidate.json`
  - both ECS nodes expose the same public endpoints as the live manifest mirror
  - both ECS nodes only pin the two-validator set:
    - `triad-testnet-sequencer`
    - `triad-testnet-storage`
  - bootstrap peers are currently only:
    - `39.104.204.172:6731`
    - `39.104.205.67:6732`

## Rollback material observed
- ECS backup directories are still present:
  - sequencer: `/opt/oasis7/p2p-testnet/backups/coordinated-reset-20260519-172636`
  - storage: `/opt/oasis7/p2p-testnet/backups/coordinated-reset-20260519-172637`
- This proves the current `public_testnet` stack still has concrete rollback material on the hosts.
- It does **not** upgrade shared-network `rollback_target_ready`, because that lane is defined on the `shared_devnet` track and still lacks audited `fallback_owner_ref` / `restore_steps_ref` closure.

## Lane reassessment
- `shared_devnet_pass`
  - keep `partial`
  - this audit did not change the upstream shared-network gate truth
- `public_rpc_ready`
  - downgrade to `partial`
  - the public RPC exists and is reachable, but the live status surface now reports a sequencer runtime error and no committed-height truth
- `explorer_public_ready`
  - downgrade to `partial`
  - the public explorer exists and is reachable, but the same-window chain sample shows no recent progress signal
- `faucet_guard_ready`
  - downgrade to `partial`
  - the faucet info endpoint and guard contract still exist, but this audit did not re-confirm a fresh external claim after the current runtime drift surfaced
- `reset_policy_announced`
  - keep `pass`
  - the reset-policy announcement path remains published for this network id
- `runtime_bootstrap`
  - downgrade to `block`
  - the current local observer no longer loads the formal manifest and is split onto an incompatible validator/replication contract
- `claims_boundary_review`
  - keep `pass`
  - the public wording boundary remains correctly constrained to `public_testnet/resettable/guarded faucet/non-mainnet`

## Conclusion
- The repo can now mirror the current live candidate manifest / bundle / bootstrap peers as first-class readiness inputs instead of relying on scattered dated notes.
- The current live runtime is no longer healthy enough to preserve the earlier optimistic lane readings.
- The next repair step is not “add more ECS”, but:
  - realign the local observer to the two-validator manifest contract
  - then resolve the ECS sequencer predecessor-gap runtime fault
  - only after that should the public lanes be reconsidered for promotion.

## 2026-05-22 operator addendum
- Later on 2026-05-22, the local observer was actually realigned and restarted through the repo-owned operator path:
  - `scripts/p2p-public-testnet-local-observer-sync.sh apply`
  - `scripts/p2p-public-testnet-local-observer-sync.sh reset-state`
- This cleared the earlier “local host still runs old three-validator contract” finding from the morning audit:
  - `network_tier.tier=public_testnet`
  - `network_tier.bootstrap_peer_count=2`
  - `fetch-commit authorization failed` is no longer the dominant live error
- However, aggregate readiness did not improve. The later live truth became:
  - local current runtime binary hash matches both ECS nodes: `2f836980834da470882fef4ca7ab0598c984acfc42565d574acf2cd19c474cfe`
  - mirrored bundle file `/opt/oasis7/p2p-testnet-local/config/public-testnet-live-candidate-bundle-2026-05-22.json` still declares `runtime_build.sha256=d1046485ae71a794cf0f5fb78561bd6068363ca53aee3ccac384d831829c07e8`
  - local status still fails on `gap sync height 15 execution hash validation failed`
- Updated interpretation:
  - `runtime_bootstrap` remains `block`, but no longer because the local observer is missing the formal manifest
  - the deeper blocker is now release/runtime input drift: binary parity alone does not restore execution parity
  - `public_rpc_ready` / `explorer_public_ready` / `faucet_guard_ready` should remain `partial` until the height-15 execution split is cleared and a fresh same-window external claim is revalidated
  - a later controlled reset of local `STORAGE_ROOT` also failed to clear the split, so the remaining drift is no longer explainable as “local stale CAS only”
  - even after temporarily isolating the local observer to the single healthy storage peer `39.104.205.67`, the same `height 15` execution mismatch still reproduced, so the split is not reducible to “the bad sequencer was the only corrupt upstream”
  - a controlled reset on the ECS sequencer also did not yield stable replay: it briefly came back healthy at low height, then immediately regressed to `last_applied=3 incoming=13795 predecessor=13794`

## 2026-05-22 sequencer reset addendum
- 为了验证“只要清掉坏 sequencer 本地状态就能重新追上 healthy storage”这个假设，当前会话对 `39.104.204.172` 做了受控状态重置。
- 备份目录：
  - `/opt/oasis7/p2p-testnet/backups/sequencer-state-reset-20260522-165747`
- 备份并迁出的路径：
  - `/opt/oasis7/p2p-testnet/data/execution-world`
  - `/opt/oasis7/p2p-testnet/data/execution-records`
  - `/opt/oasis7/p2p-testnet/data/storage`
  - `/opt/oasis7/p2p-testnet/output/node-distfs/triad-testnet-sequencer`
  - `/opt/oasis7/p2p-testnet/output/chain-runtime/triad-testnet-sequencer/reward-runtime-execution-bridge-state.json`
- 重启后短窗观测：
  - sequencer `/v1/chain/status` 一度恢复为 `last_error=null`
  - `committed_height=3`
  - `network_committed_height=3`
- 但不到一个采样窗口，它就重新掉回 predecessor-gap：
  - `last_error=node execution error: execution driver missing predecessor record for non-contiguous committed height: last_applied=3 incoming=13795 predecessor=13794`
  - 与此同时，storage `39.104.205.67` 仍继续健康推进到 `committed_height=13795`
- 结论：
  - 当前 live split 不是“把 sequencer 清零一次就会自愈”的简单故障
  - 更可能是历史 execution replay / checkpoint 恢复输入已经与今天的 live chain truth 不兼容
