# Governance Registry Live-World Drill Evidence: `msig.foundation_ops.v1` (2026-03-24)

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
- 目标: 在默认 execution world 上完成首轮真实 governance registry drill，并确认 world 最终恢复回 baseline truth。

## 执行范围
- source world:
  - `output/chain-runtime/viewer-live-node/reward-runtime-execution-world`
- baseline manifest:
  - operator-local `public_manifest.json`
  - batch id: `oasis7-governance-batch-20260323-01`
- target slot:
  - `msig.foundation_ops.v1`
- replaced signer:
  - `signer03`
- replacement public key:
  - `c9fdbca61c675f6abf208406ee6e4b41b4e49d0cf8e7885c6d2dd74f1326db52`
- live drill bundle:
  - `output/governance-drills/20260324-foundation-ops-live-world/`

## 执行步骤
1. 先对默认 world 做 baseline 审计，确认起始状态仍为 `ready_for_ops_drill`。
2. 导入 pass manifest，在真实默认 world 上完成一次 `2-of-3` rotation 审计。
3. 导入 negative manifest，把同一 slot 人为降成 `2-of-2`，确认 hard gate 阻断。
4. 再导回 baseline manifest，并做 restore audit，确认默认 world 不留在降级状态。

## 执行命令
- baseline pre-audit:
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest <operator-local-public-manifest.json> --strict-manifest-match --require-single-failure-tolerance`
- pass import/audit:
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_import -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest output/governance-drills/20260324-foundation-ops-live-world/manifests/rotated_pass_manifest.json`
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest output/governance-drills/20260324-foundation-ops-live-world/manifests/rotated_pass_manifest.json --strict-manifest-match --require-single-failure-tolerance`
- block import/audit:
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_import -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest output/governance-drills/20260324-foundation-ops-live-world/manifests/degraded_block_manifest.json`
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest output/governance-drills/20260324-foundation-ops-live-world/manifests/degraded_block_manifest.json --strict-manifest-match --require-single-failure-tolerance`
- restore import/audit:
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_import -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest <operator-local-public-manifest.json>`
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest <operator-local-public-manifest.json> --strict-manifest-match --require-single-failure-tolerance`

## 关键产物
- bundle summary:
  - `output/governance-drills/20260324-foundation-ops-live-world/summary.json`
  - `output/governance-drills/20260324-foundation-ops-live-world/summary.md`
- backup:
  - `output/governance-drills/20260324-foundation-ops-live-world/world-backup-pre-drill/`
- manifests:
  - `output/governance-drills/20260324-foundation-ops-live-world/manifests/rotated_pass_manifest.json`
  - `output/governance-drills/20260324-foundation-ops-live-world/manifests/degraded_block_manifest.json`
- logs:
  - `output/governance-drills/20260324-foundation-ops-live-world/logs/*`

## 结果摘要
- baseline pre:
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
- pass case:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
  - `msig.foundation_ops.v1`: `threshold=2`、`signer_count=3`、`tolerated_failures=1`
- negative block case:
  - `import_rc=0`
  - `audit_rc=2`
  - `overall_status=failover_blocked`
  - `msig.foundation_ops.v1`: `threshold=2`、`signer_count=2`、`tolerated_failures=0`
  - 失败签名:
    - `single_failure_blocks_slot`
    - stderr: `governance registry audit failed: at least one slot cannot tolerate a single signer failure`
- restore:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
  - 默认 world 已恢复回 baseline truth

## QA 结论
- 本轮已经不是 clone-world dry-run，而是 default/live execution world 的真实首轮 drill。
- 结论可以正式记为：
  - 该 low-risk controller slot 的真实 rotation case 可通过
  - 同 slot 的 `2-of-2` degrade case 会被 hard gate 正常阻断
  - drill 完成后默认 world 能恢复回 baseline，不留下错误 registry

## 边界与遗留
1. 本轮只覆盖 `msig.foundation_ops.v1` 这一 low-risk slot，不代表所有 controller slot 与 finality slot 都已完成真实 drill。
2. `MAINNET-2` 的“首轮真实 default/live drill 证据缺失”这一 blocker 已部分关闭，但最终仍需更多 slot 覆盖与 `MAINNET-3` 的 ceremony/QA pass。
3. 当前对外口径仍保持 `limited playable technical preview` + `crypto-hardened preview`，不因此升级。
