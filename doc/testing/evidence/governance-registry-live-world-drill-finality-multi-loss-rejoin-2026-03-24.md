# Governance Registry Live-World Drill Evidence: `governance.finality.v1` multi-signer loss / rejoin (`signer01 + signer02` loss, restore baseline) (2026-03-24)

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
- 目标: 补上 finality `multi-signer loss / rejoin` 覆盖，验证 `governance.finality.v1` 在“两把 signer 同时不可用”时会被更早的 policy gate 拦住，并确认 restore 可把默认 world 带回 baseline。

## 执行范围
- source world:
  - `output/chain-runtime/viewer-live-node/reward-runtime-execution-world`
- baseline manifest:
  - operator-local `public_manifest.json`
  - batch id: `oasis7-governance-batch-20260323-01`
- preflight clone-world bundle:
  - `output/governance-drills/20260324-finality-multi-loss-clone-world-signer05/`
- live drill bundle:
  - `output/governance-drills/20260324-finality-multi-loss-live-world-signer05/`
- target slot:
  - `governance.finality.v1`
- pass case replacement:
  - compromised / rotated signer: `signer02`
  - replacement signer: `signer05`
  - replacement public key: `e60a52c7df39e19bbabc2b499a8d8546941fae8fd2a53f56116d7bcc6374088a`
- multi-loss block case:
  - removed signer ids:
    - `signer01`
    - `signer02`
  - expected remaining finality signer count: `1`

## 关键运行规则
1. finality slot 的 single-signer loss 可通过“同次 replacement -> 保持 2-of-3”恢复；这条已由 pass case 覆盖。
2. finality slot 的 multi-signer loss 若导致 `signer_count < threshold`，不会等到 audit 再判定，而会在 import 阶段直接命中 policy reject。
3. 对当前 `threshold=2` 的 finality registry，`1 signer left` 是不合法 registry，不允许写入 world-state truth。
4. rejoin / 恢复当前用 restore baseline manifest 表达；本轮证明默认 world 可以从被拒绝的 multi-loss 尝试中恢复回原始 baseline。

## 执行步骤
1. 先用 clone-world 跑 preflight，确认：
   - `signer02 -> signer05` pass case 仍返回 `ready_for_ops_drill`
   - `signer01 + signer02` dual-loss block case 在 import 阶段被 policy reject
2. 对默认 world 做 baseline 审计，确认起始状态仍为 `ready_for_ops_drill`。
3. 导入 pass manifest，完成一轮合法 finality replacement，并审计通过。
4. 导入 multi-loss block manifest，模拟两把 finality signer 同时不可用，观察 import 直接失败。
5. 再对 block manifest 做审计，确认由于 world 未接受非法 registry，结果表现为 `manifest_mismatch`。
6. 导回 baseline manifest，并做 restore audit，确认默认 world 已恢复回 baseline truth。

## 执行命令
- clone-world preflight:
  - `./scripts/governance-registry-drill.sh --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --baseline-manifest <operator-local-public-manifest.json> --slot-id governance.finality.v1 --replace-signer-id signer02 --replacement-signer-id signer05 --block-remove-signer-id signer01 --block-remove-signer-id signer02 --replacement-public-key <replacement_public_key_hex> --out-dir output/governance-drills/20260324-finality-multi-loss-clone-world-signer05`
- live-world drill:
  - `./scripts/governance-registry-live-drill.sh --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --baseline-manifest <operator-local-public-manifest.json> --slot-id governance.finality.v1 --replace-signer-id signer02 --replacement-signer-id signer05 --block-remove-signer-id signer01 --block-remove-signer-id signer02 --replacement-public-key <replacement_public_key_hex> --out-dir output/governance-drills/20260324-finality-multi-loss-live-world-signer05`

## 关键产物
- clone-world:
  - `output/governance-drills/20260324-finality-multi-loss-clone-world-signer05/summary.json`
  - `output/governance-drills/20260324-finality-multi-loss-clone-world-signer05/summary.md`
- live-world:
  - `output/governance-drills/20260324-finality-multi-loss-live-world-signer05/run_config.json`
  - `output/governance-drills/20260324-finality-multi-loss-live-world-signer05/summary.json`
  - `output/governance-drills/20260324-finality-multi-loss-live-world-signer05/summary.md`
  - `output/governance-drills/20260324-finality-multi-loss-live-world-signer05/world-backup-pre-drill/`
  - `output/governance-drills/20260324-finality-multi-loss-live-world-signer05/manifests/*.json`
  - `output/governance-drills/20260324-finality-multi-loss-live-world-signer05/logs/*`

## 结果摘要
- clone-world preflight:
  - `block_enforcement_stage=import_policy_reject`
  - pass case `ready_for_ops_drill`
  - block case:
    - `block_import.rc=1`
    - `block_audit.overall_status=manifest_mismatch`
    - `expectation_met=true`
- live baseline pre:
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
- live pass case:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
- live multi-loss block case:
  - `block_enforcement_stage=import_policy_reject`
  - `import_rc=1`
  - `audit_rc=2`
  - `audit overall_status=manifest_mismatch`
  - import failure signature:
    - `GovernancePolicyInvalid`
    - `finality signer registry threshold exceeds signer count`
- live restore:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
  - 默认 world 已恢复回 baseline truth

## QA 结论
- 这轮补齐了 finality `multi-signer loss / rejoin` 的第一条正式证据。
- 结论可以正式记为：
  - finality slot 的 dual-loss (`signer01 + signer02`) 不会进入 audit failover gate，而是更早被 import policy reject
  - 这说明 finality 对 “`signer_count < threshold` 的非法 registry” 具备更强的写入门禁
  - restore baseline 后，默认 world 能稳定回到 `ready_for_ops_drill`

## 边界与遗留
1. 本轮证明了 multi-signer loss 会被 import policy 拦住，但还没有覆盖 “部分失效但仍可写入 registry 的复杂 failover” 或 signer rejoin 的非-baseline 变体。
2. 当前仍缺 shared network / release train，以及 `MAINNET-3` 的 genesis binding / ceremony / QA pass。
3. 当前对外口径仍保持 `limited playable technical preview` + `crypto-hardened preview`，不因此升级。
