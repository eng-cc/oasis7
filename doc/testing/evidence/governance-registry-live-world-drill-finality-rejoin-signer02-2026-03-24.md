# Governance Registry Live-World Drill Evidence: `governance.finality.v1` non-baseline rejoin (`signer02 -> signer05`) (2026-03-24)

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
- 目标: 补上 finality `non-baseline rejoin / partial degraded-but-importable failover` 覆盖，验证 `governance.finality.v1` 在先降到 `2-of-2` 后，能够通过 rejoin 再回到合法 `2-of-3`。

## 执行范围
- source world:
  - `output/chain-runtime/viewer-live-node/reward-runtime-execution-world`
- baseline manifest:
  - operator-local `public_manifest.json`
  - batch id: `oasis7-governance-batch-20260323-01`
- preflight clone-world bundle:
  - `output/governance-drills/20260324-finality-rejoin-clone-world-signer05/`
- live drill bundle:
  - `output/governance-drills/20260324-finality-rejoin-live-world-signer05/`
- target slot:
  - `governance.finality.v1`
- degraded failover path:
  - remove signer: `signer02`
  - degraded state after block import: `2-of-2`
  - degraded audit result: `failover_blocked`
- rejoin path:
  - replacement signer: `signer05`
  - replacement public key: `e60a52c7df39e19bbabc2b499a8d8546941fae8fd2a53f56116d7bcc6374088a`

## 关键运行规则
1. 当 finality slot 仍满足 `signer_count == threshold == 2` 时，registry 仍可写入 world-state，但 audit 必须阻断，因为已经失去单 signer 故障容忍。
2. 这种 `2-of-2` degraded state 不应被视为终态，只能作为 failover 中间态。
3. 合法 rejoin 路径是：在 degraded world 上重新导入合法的 `2-of-3` manifest，并再次通过 audit。
4. 当前已验证的 non-baseline rejoin 语义是：`remove signer02 -> failover_blocked -> rejoin signer05 -> ready_for_ops_drill`。

## 执行步骤
1. 先用 clone-world 跑 preflight，确认 `block -> rejoin` 链条可复现：
   - block case 进入 `failover_blocked`
   - rejoin case 回到 `ready_for_ops_drill`
2. 对默认 world 做 baseline 审计，确认起始状态仍为 `ready_for_ops_drill`。
3. 导入 pass manifest，确认 `signer02 -> signer05` 的合法 replacement 仍能通过。
4. 导入 block manifest，只移除 `signer02`，把 finality slot 降到 `2-of-2`，并做 audit。
5. 在未 restore baseline 的前提下，直接重新导入 pass manifest，把 degraded world rejoin 回 `2-of-3`，再做 audit。
6. 最后导回 baseline manifest，并做 restore audit，确认默认 world 已回到原始 baseline。

## 执行命令
- clone-world preflight:
  - `./scripts/governance-registry-drill.sh --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --baseline-manifest <operator-local-public-manifest.json> --slot-id governance.finality.v1 --replace-signer-id signer02 --replacement-signer-id signer05 --replacement-public-key <replacement_public_key_hex> --out-dir output/governance-drills/20260324-finality-rejoin-clone-world-signer05`
- live-world drill:
  - `./scripts/governance-registry-live-drill.sh --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --baseline-manifest <operator-local-public-manifest.json> --slot-id governance.finality.v1 --replace-signer-id signer02 --replacement-signer-id signer05 --replacement-public-key <replacement_public_key_hex> --out-dir output/governance-drills/20260324-finality-rejoin-live-world-signer05`

## 关键产物
- clone-world:
  - `output/governance-drills/20260324-finality-rejoin-clone-world-signer05/summary.json`
  - `output/governance-drills/20260324-finality-rejoin-clone-world-signer05/summary.md`
- live-world:
  - `output/governance-drills/20260324-finality-rejoin-live-world-signer05/run_config.json`
  - `output/governance-drills/20260324-finality-rejoin-live-world-signer05/summary.json`
  - `output/governance-drills/20260324-finality-rejoin-live-world-signer05/summary.md`
  - `output/governance-drills/20260324-finality-rejoin-live-world-signer05/world-backup-pre-drill/`
  - `output/governance-drills/20260324-finality-rejoin-live-world-signer05/manifests/*.json`
  - `output/governance-drills/20260324-finality-rejoin-live-world-signer05/logs/*`

## 结果摘要
- clone-world preflight:
  - `block_enforcement_stage=audit_failover_gate`
  - block case `failover_blocked`
  - rejoin case `ready_for_ops_drill`
- live baseline pre:
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
- live pass case:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
- live degraded block case:
  - `block_enforcement_stage=audit_failover_gate`
  - `import_rc=0`
  - `audit_rc=2`
  - `overall_status=failover_blocked`
  - degraded finality state:
    - `threshold=2`
    - `signer_count=2`
    - `tolerated_failures=0`
- live rejoin case:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
  - finality slot recovered to:
    - `threshold=2`
    - `signer_count=3`
    - `tolerated_failures=1`
- live restore:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
  - 默认 world 已恢复回 baseline truth

## QA 结论
- 这轮补齐了 finality `non-baseline rejoin` 的第一条正式证据。
- 结论可以正式记为：
  - finality slot 的 `2-of-2` degraded state 可以被写入，但会被 audit 明确阻断
  - 在 degraded state 上重新导入合法 `2-of-3` manifest 后，可恢复回 `ready_for_ops_drill`
  - 这条样本证明 finality 不仅有 single-signer replacement，也有一条明确的 failover-to-rejoin 操作链

## 边界与遗留
1. 本轮只覆盖一种 `2-of-2 -> 2-of-3` rejoin，不代表所有 partial failover 变体都已覆盖。
2. 当前仍缺 shared network / release train，以及 `MAINNET-3` 的 genesis binding / ceremony / QA pass。
3. 当前对外口径仍保持 `limited playable technical preview` + `crypto-hardened preview`，不因此升级。
