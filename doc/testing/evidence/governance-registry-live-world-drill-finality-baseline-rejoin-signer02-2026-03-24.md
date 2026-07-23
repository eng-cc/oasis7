# Governance Registry Live-World Drill Evidence: `governance.finality.v1` baseline rejoin (`signer02` temporary offline -> baseline restore) (2026-03-24)

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
- 目标: 补上 finality `baseline rejoin` 变体，验证同一把 finality signer 暂时离线后，可直接通过 baseline manifest 恢复，而不需要 replacement signer。

## 执行范围
- source world:
  - `output/chain-runtime/viewer-live-node/reward-runtime-execution-world`
- baseline manifest:
  - operator-local `public_manifest.json`
  - batch id: `oasis7-governance-batch-20260323-01`
- preflight clone-world bundle:
  - `output/governance-drills/20260324-finality-baseline-rejoin-clone-world-signer02/`
- live drill bundle:
  - `output/governance-drills/20260324-finality-baseline-rejoin-live-world-signer02/`
- target slot:
  - `governance.finality.v1`
- pass / rejoin mode:
  - `pass_manifest_mode=baseline`
- temporary offline signer:
  - `signer02`
  - public key: `38dac17ff403cc19de033e47be7cf7b5354635fbc5c1976d7c532e20494aace4`

## 关键运行规则
1. 这不是 compromise / revocation replacement，而是 temporary offline / rejoin 变体。
2. 当 signer 只是短暂不可用、身份未变更时，可以直接把同一把 signer 的 baseline manifest 重新导入，恢复到合法 `2-of-3`。
3. 该路径不需要新的 replacement signer，也不需要新的 public key。
4. 当前已验证的 baseline rejoin 语义是：`remove signer02 -> failover_blocked -> rejoin baseline signer02 -> ready_for_ops_drill`。

## 执行步骤
1. 先用 clone-world 跑 preflight，确认：
   - block case 把 `governance.finality.v1` 降到 `2-of-2`
   - rejoin case 把同一份 baseline manifest 重新导回 degraded world，并恢复为 `ready_for_ops_drill`
2. 对默认 world 做 baseline 审计，确认起始状态仍为 `ready_for_ops_drill`。
3. pass case 复用 baseline manifest，本质上是确认 baseline truth 可稳定导入和通过。
4. 导入 block manifest，移除 `signer02`，进入 `failover_blocked`。
5. 在 degraded world 上直接重新导入 baseline manifest，使 `signer02` rejoin。
6. 最后再次导入 baseline manifest 并 restore，确认默认 world 没有残留偏差。

## 执行命令
- clone-world preflight:
  - `./scripts/governance-registry-drill.sh --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --baseline-manifest <operator-local-public-manifest.json> --slot-id governance.finality.v1 --pass-manifest-mode baseline --replace-signer-id signer02 --out-dir output/governance-drills/20260324-finality-baseline-rejoin-clone-world-signer02`
- live-world drill:
  - `./scripts/governance-registry-live-drill.sh --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --baseline-manifest <operator-local-public-manifest.json> --slot-id governance.finality.v1 --pass-manifest-mode baseline --replace-signer-id signer02 --out-dir output/governance-drills/20260324-finality-baseline-rejoin-live-world-signer02`

## 关键产物
- clone-world:
  - `output/governance-drills/20260324-finality-baseline-rejoin-clone-world-signer02/summary.json`
  - `output/governance-drills/20260324-finality-baseline-rejoin-clone-world-signer02/summary.md`
- live-world:
  - `output/governance-drills/20260324-finality-baseline-rejoin-live-world-signer02/run_config.json`
  - `output/governance-drills/20260324-finality-baseline-rejoin-live-world-signer02/summary.json`
  - `output/governance-drills/20260324-finality-baseline-rejoin-live-world-signer02/summary.md`
  - `output/governance-drills/20260324-finality-baseline-rejoin-live-world-signer02/world-backup-pre-drill/`
  - `output/governance-drills/20260324-finality-baseline-rejoin-live-world-signer02/manifests/*.json`
  - `output/governance-drills/20260324-finality-baseline-rejoin-live-world-signer02/logs/*`

## 结果摘要
- clone-world preflight:
  - `pass_manifest_mode=baseline`
  - `block_enforcement_stage=audit_failover_gate`
  - block case `failover_blocked`
  - rejoin case `ready_for_ops_drill`
- live baseline pre:
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
- live pass case:
  - `pass_manifest_mode=baseline`
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
- live degraded block case:
  - `import_rc=0`
  - `audit_rc=2`
  - `overall_status=failover_blocked`
  - degraded finality state:
    - `threshold=2`
    - `signer_count=2`
    - `tolerated_failures=0`
- live baseline rejoin case:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
  - finality slot recovered to baseline:
    - `threshold=2`
    - `signer_count=3`
    - `tolerated_failures=1`
- live restore:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
  - 默认 world 已恢复回 baseline truth

## QA 结论
- 这轮补齐了 finality 的 baseline rejoin 变体。
- 结论可以正式记为：
  - finality slot 不仅支持 replacement rejoin，也支持同 signer 的 baseline rejoin
  - 对 temporary offline 场景，operator 可以不换 signer，直接用 baseline manifest 恢复
  - 默认 world 在该路径结束后可稳定回到 `ready_for_ops_drill`

## 边界与遗留
1. 本轮只覆盖一条 `temporary offline -> baseline rejoin` 样本，不代表所有 offline / flaky node 变体都已覆盖。
2. 当前仍缺 shared network / release train，以及 `MAINNET-3` 的 genesis binding / ceremony / QA pass。
3. 当前对外口径仍保持 `limited playable technical preview` + `crypto-hardened preview`，不因此升级。
