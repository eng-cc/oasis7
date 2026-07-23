# Governance Registry Live-World Drill Evidence: `governance.finality.v1` (2026-03-24)

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
- 目标: 在默认 execution world 上完成首轮真实 finality governance registry drill，并冻结 finality rotation 的 signer-id 语义。

## 执行范围
- source world:
  - `output/chain-runtime/viewer-live-node/reward-runtime-execution-world`
- baseline manifest:
  - operator-local `public_manifest.json`
  - batch id: `oasis7-governance-batch-20260323-01`
- target slot:
  - `governance.finality.v1`
- replaced signer:
  - `signer03`
- replacement signer:
  - `signer04`
- replacement public key:
  - `0125630c7502ed27a93e4ed3007eb49d111cb97612bc23f7452cfa29af94acbb`
- live drill bundle:
  - `output/governance-drills/20260324-finality-live-world-signer04/`
- 对照失败样本:
  - `output/governance-drills/20260324-finality-live-world/`

## 关键运行规则
1. finality slot 不接受“保持原 signer_id，仅替换公钥”的 rotation。
2. 对 `governance.finality.v1` 做真实 rotation 时，必须同时替换 signer node identity，即使用新的 `replacement_signer_id`。
3. 当前已验证可通过的语义是：`signer03 -> signer04`，同时保持 slot 仍为 `2-of-3`。

## 执行步骤
1. 对默认 world 做 baseline 审计，确认起始状态仍为 `ready_for_ops_drill`。
2. 导入 pass manifest，把 finality signer 从 `signer03` 轮换到 `signer04`，并做真实审计。
3. 导入 negative manifest，把 finality slot 人为降成 `2-of-2`，确认 hard gate 阻断。
4. 导回 baseline manifest，并做 restore audit，确认默认 world 已恢复回 baseline truth。

## 执行命令
- baseline pre-audit:
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest <operator-local-public-manifest.json> --strict-manifest-match --require-single-failure-tolerance`
- pass import/audit:
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_import -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest output/governance-drills/20260324-finality-live-world-signer04/manifests/rotated_pass_manifest.json`
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest output/governance-drills/20260324-finality-live-world-signer04/manifests/rotated_pass_manifest.json --strict-manifest-match --require-single-failure-tolerance`
- block import/audit:
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_import -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest output/governance-drills/20260324-finality-live-world-signer04/manifests/degraded_block_manifest.json`
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest output/governance-drills/20260324-finality-live-world-signer04/manifests/degraded_block_manifest.json --strict-manifest-match --require-single-failure-tolerance`
- restore import/audit:
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_import -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest <operator-local-public-manifest.json>`
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_governance_registry_audit -- --world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --public-manifest <operator-local-public-manifest.json> --strict-manifest-match --require-single-failure-tolerance`

## 关键产物
- bundle summary:
  - `output/governance-drills/20260324-finality-live-world-signer04/summary.json`
  - `output/governance-drills/20260324-finality-live-world-signer04/summary.md`
- backup:
  - `output/governance-drills/20260324-finality-live-world-signer04/world-backup-pre-drill/`
- manifests:
  - `output/governance-drills/20260324-finality-live-world-signer04/manifests/rotated_pass_manifest.json`
  - `output/governance-drills/20260324-finality-live-world-signer04/manifests/degraded_block_manifest.json`
- logs:
  - `output/governance-drills/20260324-finality-live-world-signer04/logs/*`
- failed semantic sample:
  - `output/governance-drills/20260324-finality-live-world/logs/pass_import.stderr`

## 结果摘要
- baseline pre:
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
- pass case:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
  - `governance.finality.v1`: `threshold=2`、`signer_count=3`、`tolerated_failures=1`
- negative block case:
  - `import_rc=0`
  - `audit_rc=2`
  - `overall_status=failover_blocked`
  - `governance.finality.v1`: `threshold=2`、`signer_count=2`、`tolerated_failures=0`
  - 失败签名:
    - `single_failure_blocks_slot`
    - stderr: `governance registry audit failed: at least one slot cannot tolerate a single signer failure`
- restore:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
  - 默认 world 已恢复回 baseline truth

## 失败语义对照
- 先前曾尝试“保持 `signer03`，仅替换公钥”的 finality pass manifest。
- 该路径在真实默认 world 上失败，错误签名为:
  - `GovernancePolicyInvalid`
  - `finality signer binding conflicts with existing node identity`
- 结论:
  - controller slot 与 finality slot 的 rotation 语义不同
  - finality slot 受 node identity 绑定约束，不能把“同 signer_id 换公钥”误判为合法 rotation

## QA 结论
- 本轮已经不是 clone-world dry-run，而是 default/live execution world 的真实 finality drill。
- 结论可以正式记为：
  - finality slot 的真实 `2-of-3` rotation case 可通过，但要求 `signer03 -> signer04` 这类新 signer node id 替换
  - 同 slot 的 `2-of-2` degrade case 会被 hard gate 正常阻断
  - drill 完成后默认 world 能恢复回 baseline，不留下错误 registry

## 边界与遗留
1. 本轮只验证了首个 finality live drill 语义，不代表 finality signer custody、ceremony 和更大范围 failover 覆盖已经全部完成。
2. `MAINNET-2` 的真实 finality drill blocker 已进一步收敛，但 `MAINNET-3` 的 genesis binding / ceremony / QA pass 仍是后续主 blocker。
3. 当前对外口径仍保持 `limited playable technical preview` + `crypto-hardened preview`，不因此升级。
