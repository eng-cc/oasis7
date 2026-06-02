# Testnet Upgrade Preflight Drill Evidence

- Date: 2026-06-01 12:28 CST
- Owner role: `runtime_engineer`
- Scope: non-destructive testnet rehearsal for the new P2P smooth-upgrade preflight and recovery-plan guardrails.
- Environment: real-env triad / `shared-devnet-ecs-v1`
- Safety boundary: no ECS/testnet node state was replaced. Live-node checks were dry-run only; generated restore/rollback scripts were executed only later against local temporary fake node data with fake `systemctl`/`rsync`.

## Inputs

Canonical triad snapshot command:

```bash
env P2PARCH6_SEQ_SSH_PASSWORD=<redacted> P2PARCH6_STORAGE_SSH_PASSWORD=<redacted> \
  ./scripts/p2p-real-env-triad-snapshot.sh \
  --samples 1 \
  --interval-secs 1 \
  --ssh-timeout-secs 6 \
  --out-dir .tmp/testnet_drill_triad_snapshot_auth
```

Snapshot artifacts:

- `.tmp/testnet_drill_triad_snapshot_auth/20260601-122419/summary.md`
- `.tmp/testnet_drill_triad_snapshot_auth/20260601-122419/summary.json`
- `.tmp/testnet_drill_triad_snapshot_auth/20260601-122419/nodes/local_node/status.json`

Upgrade preflight command:

```bash
./scripts/p2p-upgrade-preflight.sh \
  --status-url http://127.0.0.1:5633/v1/chain/status \
  --status-url http://39.104.204.172:5631/v1/chain/status \
  --recovery-plan-dir .tmp/testnet_drill_preflight_20260601
```

Preflight artifacts:

- `.tmp/testnet_drill_preflight_20260601/triad-observer-local.recovery-plan.json`
- `.tmp/testnet_drill_preflight_20260601/triad-sequencer-a.recovery-plan.json`

## Observed Triad Baseline

The same-window triad snapshot returned `claim_status=blocked` and `claim_mode=transitional`.

Failure signatures:

- `cloud_pair_service_unhealthy`
- `cloud_pair_chain_not_visible`
- `sequencer_committed_height_zero`
- `storage_committed_height_zero`
- `cloud_pair_no_recent_progress_signal`
- `node_not_ready`
- `peer_head_quorum_not_ready`
- `replication_transport_unstable`
- `p2p_reachability_degraded`

Node observations:

| Node label | Status |
| --- | --- |
| `local_node` | service active, health ok, status fetch ok, `node_id=triad-observer-local`, role `sequencer`, readiness `not_ready`, sync `unknown` |
| `sequencer_ecs` | default SSH/script status path did not fetch service or chain status; public `http://39.104.204.172:5631/v1/chain/status` was reachable separately |
| `storage_ecs` | default SSH/script status path did not fetch service or chain status; public `39.104.205.67:{5631,5632,5633}/v1/chain/status` was not reachable |

The SSH credential available during the drill did not authenticate to either ECS host as `root`, `scc`, `ecs-user`, `aliyun`, `ubuntu`, or `admin`; therefore the storage-side private status endpoint could not be included in the preflight.

## Preflight Result

The dry-run preflight correctly failed before upgrade.

`triad-observer-local`:

- committed height: `4337`
- network committed height: `8216`
- replication persisted height: `4337`
- gap blocked height: `4338`
- failures:
  - `replication_gap_sync_blocked`
  - `peer_head_unavailable_for_repair`
  - `network_height_lag_exceeds_policy`

`triad-sequencer-a`:

- committed height: `8215`
- network committed height: `8216`
- replication persisted height: `4337`
- gap blocked height: `4338`
- failures:
  - `replication_gap_sync_blocked`
  - `peer_head_unavailable_for_repair`
  - `network_height_lag_exceeds_policy`

Both generated recovery plans are `dry_run_only=true`, `mode=not_required`, and contain the blocked reasons above. Because the live status payload did not yet expose a usable `state_sync_fallback_required` checkpoint boundary for this incident, no restore command plan was generated.

## Public-Testnet Three-Node Follow-Up

The first pass used the shared-devnet `563x` triad endpoints. The actual public-testnet storage endpoint is publicly reachable at `39.104.205.67:6632`, so the drill was repeated with the public-testnet `663x` endpoints and the snapshot script's public fallback path.

Snapshot command:

```bash
env P2PARCH6_SEQ_SSH_PASSWORD=<redacted> P2PARCH6_STORAGE_SSH_PASSWORD=<redacted> \
  ./scripts/p2p-real-env-triad-snapshot.sh \
  --samples 1 \
  --interval-secs 1 \
  --ssh-timeout-secs 4 \
  --out-dir .tmp/public_testnet_drill_triad_snapshot \
  --world-id oasis7-public-testnet-parallel-20260518 \
  --local-service oasis7-testnet-observer.service \
  --local-status-url http://127.0.0.1:6633/v1/chain/status \
  --local-health-url http://127.0.0.1:6633/healthz \
  --local-env-file /opt/oasis7/p2p-testnet-local/config/node.env \
  --sequencer-service oasis7-testnet-sequencer.service \
  --sequencer-status-url http://127.0.0.1:6631/v1/chain/status \
  --sequencer-health-url http://127.0.0.1:6631/healthz \
  --sequencer-public-status-url http://39.104.204.172:6631/v1/chain/status \
  --sequencer-public-health-url http://39.104.204.172:6631/healthz \
  --sequencer-env-file /opt/oasis7/p2p-testnet/config/node.env \
  --storage-service oasis7-testnet-storage.service \
  --storage-status-url http://127.0.0.1:6632/v1/chain/status \
  --storage-health-url http://127.0.0.1:6632/healthz \
  --storage-public-status-url http://39.104.205.67:6632/v1/chain/status \
  --storage-public-health-url http://39.104.205.67:6632/healthz \
  --storage-env-file /opt/oasis7/p2p-testnet/config/node.env
```

Public-testnet snapshot artifacts:

- `.tmp/public_testnet_drill_triad_snapshot/20260601-123403/summary.md`
- `.tmp/public_testnet_drill_triad_snapshot/20260601-123403/summary.json`

Public-testnet preflight command:

```bash
./scripts/p2p-upgrade-preflight.sh \
  --status-url http://127.0.0.1:6633/v1/chain/status \
  --status-url http://39.104.204.172:6631/v1/chain/status \
  --status-url http://39.104.205.67:6632/v1/chain/status \
  --recovery-plan-dir .tmp/public_testnet_drill_preflight_20260601
```

Public-testnet preflight artifacts:

- `.tmp/public_testnet_drill_preflight_20260601_fixed/triad-testnet-local.recovery-plan.json`
- `.tmp/public_testnet_drill_preflight_20260601_fixed/triad-testnet-sequencer.recovery-plan.json`
- `.tmp/public_testnet_drill_preflight_20260601_fixed/triad-testnet-storage.recovery-plan.json`

Observed public-testnet state:

| Node | committed | network committed | replication persisted | gap blocked height | Preflight |
| --- | ---: | ---: | ---: | ---: | --- |
| `triad-testnet-local` | `9768` | `9768` | `9768` | none | pass |
| `triad-testnet-sequencer` | `16321` | `16321` | `9768` | `9769` | fail: `replication_gap_sync_blocked`, `peer_head_unavailable_for_repair`; recovery mode `blocked_missing_trusted_checkpoint` |
| `triad-testnet-storage` | `11327` | `16321` | `9768` | `9769` | fail: `replication_gap_sync_blocked`, `network_height_lag_exceeds_policy`; recovery mode `blocked_missing_trusted_checkpoint` |

The public-testnet triad snapshot returned `claim_status=blocked` and `claim_mode=three_equal_validator`. All three nodes were visible, but none were readiness-ready; peer-head quorum and reachability policy remained degraded.

Script follow-up:

- `scripts/p2p-real-env-triad-snapshot.sh` now supports `--sequencer-public-status-url`, `--sequencer-public-health-url`, `--storage-public-status-url`, and `--storage-public-health-url`.
- When SSH collection fails but a public fallback succeeds, the sample is marked with `status.fallback.txt` / `healthz.fallback.txt` and still contributes real node status to the summary.
- `scripts/p2p-real-env-triad-snapshot.test.sh` covers the SSH-fail/public-fallback path.
- `scripts/p2p-upgrade-preflight.sh` now treats `replication_gap_sync_blocked_height` as the minimum trusted-checkpoint requirement even when the live status payload does not yet set `state_sync_fallback_required`. This prevents recovery plans from reporting `mode=not_required` while a node is actually gap-blocked.

Static restore-plan drill:

- `scripts/p2p-upgrade-preflight.sh` now also accepts `--status-json <path>` so a frozen status snapshot can drive restore-plan generation without live-height races.
- Synthetic drill inputs were built under `.tmp/public_testnet_restore_plan_drill_static_20260601/`.
- Both `triad-testnet-sequencer` and `triad-testnet-storage` produced `mode=trusted_checkpoint_state_sync` plans at `required_height=16603` with restore and rollback steps.
- Generated restore/rollback scripts were written under:
  - `.tmp/public_testnet_restore_plan_drill_static_20260601/scripts/sequencer`
  - `.tmp/public_testnet_restore_plan_drill_static_20260601/scripts/storage`
- This drill stayed non-destructive. No ECS restore script was executed.

Signed-checkpoint drill:

- A second drill used a real Ed25519 keypair, a signed checkpoint payload, and an independently computed validator-set manifest.
- Inputs were built under `.tmp/public_testnet_signed_checkpoint_drill_20260601/`.
- `--verify-trusted-checkpoint-signatures` verified the checkpoint signature.
- `--validator-set-manifest` verified `validator_set_hash`, `stake_root`, signer membership, public-key path binding, and stake threshold metadata.
- Both sequencer and storage generated `trusted_checkpoint_state_sync` restore/rollback plans with `trusted_checkpoint_signatures_verified=true` and `validator_set_proof_verified=true`.
- Generated restore plans now include execution-time toolchain and bundle re-checks, service-manager state snapshots, plus backup content and metadata verification: required tools are checked before service stop, snapshot/journal/chunk sha256 values are re-verified at restore-script execution time, chunks root is recomputed from the canonical chunk manifest, `systemctl show/status` output is captured before service stop, source and backup sha256 manifests are compared, and source and backup metadata manifests compare `path/type/size/mode/uid/gid`.
- This drill also stayed non-destructive. The signed checkpoint is synthetic and exists only to exercise the governance-input verification path.

Signed local execution drill:

- A third drill copied the same signed checkpoint, validator-set proof, and state-sync bundle into `.tmp/public_testnet_signed_execution_drill_20260601/`.
- The restore execution path was exercised only against local temporary node data with fake `systemctl` and fake `rsync` shims on `PATH`.
- The drill set `OASIS7_ALLOW_RESTORE_EXECUTION=I_UNDERSTAND_THIS_CAN_REPLACE_NODE_STATE` only for the local fake execution directory. No ECS host was modified.
- Successful restore artifacts:
  - `.tmp/public_testnet_signed_execution_drill_20260601/exec2-plans/triad-testnet-sequencer.recovery-plan.json`
  - `.tmp/public_testnet_signed_execution_drill_20260601/exec2-scripts/triad-testnet-sequencer.restore.state.json`
  - `.tmp/public_testnet_signed_execution_drill_20260601/exec2-scripts/triad-testnet-sequencer.restore.log`
  - `.tmp/public_testnet_signed_execution_drill_20260601/exec2-restore.log`
- The successful restore plan records:
  - `trusted_checkpoint_signatures_verified=true`
  - `validator_set_proof_verified=true`
  - `state_sync_bundle_semantics_verified=true`
  - `restore_execution_status=passed`
  - `restore_execution_exit_code=0`
- The local fake data dir contains restored `state-sync/snapshot` and `state-sync/journal`, and the source/backup sha256 manifests compare cleanly.
- The local fake backup also contains matching source/backup metadata manifests, so the drill covers both file content and basic ownership/permission/shape drift.
- The local fake backup includes `service-before-state-sync.systemctl-show.txt` and `service-before-state-sync.systemctl-status.txt`, proving the restore script captures service-manager state before stopping the node service.
- Tamper drills generated restore scripts from valid bundles, modified the snapshot file or a chunk file before execution, and confirmed the restore script failed on the execution-time `sha256sum -c` before issuing `systemctl stop`.
- Restore command generation rejects shell-unsafe service names and restore/bundle paths before writing scripts, preventing whitespace, command separators, command substitution, or parent traversal from entering generated shell commands.

Signed local failure and rollback drill:

- The same local fake execution path was rerun with `FAIL_RESTORE_SNAPSHOT=1` and `--auto-rollback-on-restore-failure`.
- Expected restore failure was injected at the snapshot copy step; preflight exited non-zero as intended.
- Rollback artifacts:
  - `.tmp/public_testnet_signed_execution_drill_20260601/fail2-plans/triad-testnet-sequencer.recovery-plan.json`
  - `.tmp/public_testnet_signed_execution_drill_20260601/fail2-scripts/triad-testnet-sequencer.restore.state.json`
  - `.tmp/public_testnet_signed_execution_drill_20260601/fail2-scripts/triad-testnet-sequencer.rollback.state.json`
  - `.tmp/public_testnet_signed_execution_drill_20260601/fail2-restore.log`
- The failure/rollback plan records:
  - `restore_execution_status=failed`
  - `restore_execution_exit_code=23`
  - `rollback_execution_status=passed`
  - `rollback_execution_exit_code=0`

## Verdict

The rehearsal validated the intended guardrail behavior: the new preflight blocks a smooth upgrade when testnet is already outside the safe envelope.

The follow-up covered all three public-testnet nodes through status endpoints, exercised a frozen-status restore-plan drill, exercised a signed-checkpoint + validator-set proof drill, and then executed the generated restore/rollback scripts against local fake node data only. This is still non-destructive for ECS/testnet state.

Required next operator inputs before a real ECS execution drill:

1. Verified trusted checkpoint manifest covering at least height `9769`.
2. State-sync bundle manifest and bundle dir bound to that checkpoint.
3. Explicit per-node restore allowlist, data dir, backup dir, and service name.
4. Operator access or deployment automation capable of applying the plan on the ECS hosts.
5. Separate approval to set `OASIS7_ALLOW_RESTORE_EXECUTION=I_UNDERSTAND_THIS_CAN_REPLACE_NODE_STATE`.
