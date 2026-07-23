# Governance Registry Live-World Drill Evidence: `governance.finality.v1` revocation recovery (`signer02 -> signer05`) (2026-03-24)

审计轮次: 1

## Meta
- 关联专题:
  - `PRD-P2P-GOVSIGN-003`
  - `PRD-P2P-BENCH-002/003`
  - `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.prd.md`
- 关联任务:
  - `GOVSIGN execution workstream`
  - `BENCH-G1`
- 责任角色: `qa_engineer`
- 协作角色: `runtime_engineer`
- 当前结论: `pass_for_default_live_world`
- 目标: 补一轮 additional finality revocation coverage，验证 `governance.finality.v1` 在“单 signer compromise -> 同次替换恢复”的路径上，不依赖首个 `signer03 -> signer04` 样本。

## 执行范围
- source world:
  - `output/chain-runtime/viewer-live-node/reward-runtime-execution-world`
- baseline manifest:
  - operator-local `public_manifest.json`
  - batch id: `oasis7-governance-batch-20260323-01`
- preflight clone-world bundle:
  - `output/governance-drills/20260324-finality-revocation-clone-world-signer05/`
- live drill bundle:
  - `output/governance-drills/20260324-finality-revocation-live-world-signer05/`
- target slot:
  - `governance.finality.v1`
- compromised signer:
  - `signer02`
- replacement signer:
  - `signer05`
- replacement public key:
  - `e60a52c7df39e19bbabc2b499a8d8546941fae8fd2a53f56116d7bcc6374088a`

## 关键运行规则
1. finality revocation 不是“先删 signer 再观察”，而是同一次导入内完成 compromised signer replacement，并保持 `2-of-3`。
2. 对 `governance.finality.v1` 做 revocation recovery 时，replacement signer 仍必须是新的 signer node id。
3. 当前第二条已验证可通过的 recovery 语义是：`signer02 -> signer05`。

## 执行步骤
1. 先用 clone-world 跑 preflight，确认 `signer02 -> signer05` 的 pass/block manifests 能稳定返回 `ready_for_ops_drill / failover_blocked`。
2. 对默认 world 做 baseline 审计，确认起始状态仍为 `ready_for_ops_drill`。
3. 导入 pass manifest，把 finality signer 从 `signer02` 替换到 `signer05`，模拟“单 signer compromise 后的同次恢复”，并做真实审计。
4. 导入 negative manifest，模拟“revoked signer 被移除但未同次补位”，确认 hard gate 阻断。
5. 导回 baseline manifest，并做 restore audit，确认默认 world 已恢复回 baseline truth。

## 执行命令
- clone-world preflight:
  - `./scripts/governance-registry-drill.sh --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --baseline-manifest <operator-local-public-manifest.json> --slot-id governance.finality.v1 --replace-signer-id signer02 --replacement-signer-id signer05 --replacement-public-key <replacement_public_key_hex> --out-dir output/governance-drills/20260324-finality-revocation-clone-world-signer05`
- live-world drill:
  - `./scripts/governance-registry-live-drill.sh --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --baseline-manifest <operator-local-public-manifest.json> --slot-id governance.finality.v1 --replace-signer-id signer02 --replacement-signer-id signer05 --replacement-public-key <replacement_public_key_hex> --out-dir output/governance-drills/20260324-finality-revocation-live-world-signer05`

## 关键产物
- clone-world:
  - `output/governance-drills/20260324-finality-revocation-clone-world-signer05/summary.json`
  - `output/governance-drills/20260324-finality-revocation-clone-world-signer05/summary.md`
- live-world:
  - `output/governance-drills/20260324-finality-revocation-live-world-signer05/summary.json`
  - `output/governance-drills/20260324-finality-revocation-live-world-signer05/summary.md`
  - `output/governance-drills/20260324-finality-revocation-live-world-signer05/world-backup-pre-drill/`
  - `output/governance-drills/20260324-finality-revocation-live-world-signer05/manifests/*.json`
  - `output/governance-drills/20260324-finality-revocation-live-world-signer05/logs/*`

## 结果摘要
- clone-world preflight:
  - baseline `ready_for_ops_drill`
  - pass case `ready_for_ops_drill`
  - negative block case `failover_blocked`
- live baseline pre:
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
- live pass case:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
  - `governance.finality.v1`: `threshold=2`、`signer_count=3`、`tolerated_failures=1`
- live negative block case:
  - `import_rc=0`
  - `audit_rc=2`
  - `overall_status=failover_blocked`
  - `governance.finality.v1`: `threshold=2`、`signer_count=2`、`tolerated_failures=0`
  - 失败签名:
    - `single_failure_blocks_slot`
    - stderr: `governance registry audit failed: at least one slot cannot tolerate a single signer failure`
- live restore:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
  - 默认 world 已恢复回 baseline truth

## QA 结论
- 这不是首个 finality drill 的重复，而是第二条独立的 finality revocation recovery 样本。
- 结论可以正式记为：
  - finality slot 的“单 signer compromise -> 同次 replacement”路径已至少有两条独立样本可通过：`signer03 -> signer04` 与 `signer02 -> signer05`
  - finality slot 的“移除 revoked signer 但不补位”路径会被 hard gate 正常阻断
  - 默认 world 在真实 revocation drill 后能够恢复回 baseline

## 边界与遗留
1. 本轮仍然只覆盖单 signer compromise / replacement recovery，不代表双 signer 丢失、长时间 failover、signer rejoin 或 ceremony 已完成。
2. `MAINNET-2` 的 finality revocation gate 证据已进一步增强，但 `MAINNET-3` 的 genesis binding / ceremony / QA pass 仍是后续主 blocker。
3. 当前对外口径仍保持 `limited playable technical preview` + `crypto-hardened preview`，不因此升级。
