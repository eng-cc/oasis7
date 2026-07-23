# Governance Registry Clone-World Drill Evidence: `msig.foundation_ops.v1` (2026-03-24)

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
- 当前结论: `pass_for_clone_world`
- 目标: 用 clone-world 样本验证治理 signer rotation / revocation / failover runbook 已具备可重复执行的 `pass + block` 门禁，不再停留在纯文档层。

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
- drill bundle:
  - `output/governance-drills/20260324-foundation-ops-clone-world/`

## 执行命令
- replacement key 生成:
  - `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_pure_api_client -- keygen`
- clone-world drill:
  - `./scripts/governance-registry-drill.sh --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world --baseline-manifest <operator-local-public-manifest.json> --slot-id msig.foundation_ops.v1 --replace-signer-id signer03 --replacement-public-key c9fdbca61c675f6abf208406ee6e4b41b4e49d0cf8e7885c6d2dd74f1326db52 --out-dir output/governance-drills/20260324-foundation-ops-clone-world`

## 关键产物
- 汇总:
  - `output/governance-drills/20260324-foundation-ops-clone-world/run_config.json`
  - `output/governance-drills/20260324-foundation-ops-clone-world/summary.json`
  - `output/governance-drills/20260324-foundation-ops-clone-world/summary.md`
- manifests:
  - pass case: `output/governance-drills/20260324-foundation-ops-clone-world/manifests/rotated_pass_manifest.json`
  - block case: `output/governance-drills/20260324-foundation-ops-clone-world/manifests/degraded_block_manifest.json`
- logs:
  - `output/governance-drills/20260324-foundation-ops-clone-world/logs/baseline_audit.{stdout,stderr,rc}`
  - `output/governance-drills/20260324-foundation-ops-clone-world/logs/pass_import.{stdout,stderr,rc}`
  - `output/governance-drills/20260324-foundation-ops-clone-world/logs/pass_audit.{stdout,stderr,rc}`
  - `output/governance-drills/20260324-foundation-ops-clone-world/logs/block_import.{stdout,stderr,rc}`
  - `output/governance-drills/20260324-foundation-ops-clone-world/logs/block_audit.{stdout,stderr,rc}`

## 结果摘要
- baseline:
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
- pass case:
  - `import_rc=0`
  - `audit_rc=0`
  - `overall_status=ready_for_ops_drill`
  - `msig.foundation_ops.v1`: `threshold=2`、`signer_count=3`、`tolerated_failures=1`
- block case:
  - `import_rc=0`
  - `audit_rc=2`
  - `overall_status=failover_blocked`
  - `msig.foundation_ops.v1`: `threshold=2`、`signer_count=2`、`tolerated_failures=0`
  - 失败签名:
    - `single_failure_blocks_slot`
    - stderr: `governance registry audit failed: at least one slot cannot tolerate a single signer failure`

## QA 结论
- `pass`:
  - clone-world runbook 已能稳定产出 producer 要求的两类结论：
    - 正向 rotation case 维持 `2-of-3` 并通过 hard gate
    - 负向 degrade case 降为 `2-of-2` 后被 hard gate 正常阻断
- `not_passed_yet`:
  - 这轮证据只证明工具链、manifest 变换和 QA 判定逻辑正确。
  - 它不等于 default/live execution world 的真实治理 drill 已完成。
  - 它也不等于 `MAINNET-2` 或 `BENCH-G1` 全部关闭。

## 当前阻断与下一步
1. clone-world 证据已经可作为 `BENCH-G1` 的第一阶段样本。
2. 下一步应在 default/live execution world 上执行同一流程，并沉淀正式 `pass/block` 证据。
3. 在 default/live evidence 出来前，项目仍不得宣称“主流公链级治理 drill 成熟度”或同等口径。
