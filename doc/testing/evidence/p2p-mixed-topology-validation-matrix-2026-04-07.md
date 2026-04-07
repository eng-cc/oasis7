# P2P Mixed Topology Validation Matrix (2026-04-07)

审计轮次: 1

## Meta
- 责任角色:
  - `qa_engineer`
- 关联专题:
  - `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md`
  - `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.project.md`
- 当前结论:
  - `partial`

## 执行摘要
1. 把 `scripts/p2p-mixed-topology-matrix.sh` 的 summary 扩成机器可读 evidence contract，新增 `external_evidence.*` 与 `evidence_contract.*` 字段，显式输出 `required_exact_ready / full_proxy_ready / shared_network_pass_blockers`。
2. 修掉 proxy case 的两类环境假阳性：不再依赖预编译 `oasis7_chain_runtime` binary，也不再复用默认 `561x/563x` 端口段。
3. 修掉 `oasis7_chain_runtime` 与 `P2PARCH-4` lane gate 的接线冲突：`observer` 不再无条件启用 `feedback_p2p`。
4. latest `full` live run 已真实执行 7 个 exact case 与 2 个 proxy case；exact 全通过，但两条 proxy soak 都以一致 failure signatures 失败，因此 current full-tier truth 仍是 audited `partial`，不是 `pass`。

## 执行命令
```bash
./scripts/p2p-mixed-topology-matrix-smoke.sh
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime feedback_p2p_is_disabled_for_observer_role -- --nocapture
./scripts/p2p-mixed-topology-matrix.sh \
  --tier full \
  --shared-window-evidence-ref doc/testing/evidence/shared-network-shared-devnet-follow-up-window-2026-03-24.md \
  --shared-window-evidence-ref doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md \
  --out-dir .tmp/p2p_mixed_topology_validation
```

## 结果
- latest full summary:
  - `.tmp/p2p_mixed_topology_validation/20260407-120951-full/summary.json`
  - `overall_status=failed`
  - `case_count=9`
  - `exact_case_count=7`
  - `proxy_case_count=2`
  - `required_exact_ready=true`
  - `full_proxy_ready=false`
  - `shared_network_pass_inputs_ready=false`
- exact cases:
  - `nat_private_role_policy=ok`
  - `validator_hidden_boundary=ok`
  - `relay_only_lane_budget=ok`
  - `cgnat_relay_path_ranking=ok`
  - `bootstrap_poisoning_dedupe=ok`
  - `relay_budget_detection=ok`
  - `path_failover_selection=ok`
- proxy case summaries:
  - `sentry_loss_proxy_longrun`:
    - `.tmp/p2p_mixed_topology_validation/20260407-120951-full/sentry-loss-proxy/20260407-121001/summary.json`
    - `overall_status=failed`
    - `topology=triad_distributed`
  - `mixed_topology_release_proxy`:
    - `.tmp/p2p_mixed_topology_validation/20260407-120951-full/mixed-topology-release-proxy/20260407-121506/summary.json`
    - `overall_status=failed`
    - `topology=triad`

## Failure Signatures
- `sentry_loss_proxy_longrun`:
  - `metric_gate=last_error_samples=104`
  - `consensus_hash_divergence count=40 heights=2,4,11,12,15,16,18,20,21,22,23`
  - `running_false_samples=8`
  - `committed_height_not_monotonic nodes=sequencer`
  - `known_peer_heads_zero_samples=223`
  - `http_failure_samples=16`
- `mixed_topology_release_proxy`:
  - `metric_gate=last_error_samples=104`
  - `consensus_hash_divergence count=39 heights=2,4,11,12,15,16,18,20,21,22,23`
  - `running_false_samples=8`
  - `committed_height_not_monotonic nodes=sequencer`
  - `known_peer_heads_zero_samples=116`
  - `http_failure_samples=16`

## Evidence Contract Snapshot
- external evidence:
  - `shared_window_evidence_refs`:
    - `doc/testing/evidence/shared-network-shared-devnet-follow-up-window-2026-03-24.md`
    - `doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md`
  - `dedicated_lab_evidence_refs=[]`
  - `pass_uplift_decision_ref=null`
- executable boundary:
  - `required_exact_ready=true`
  - `full_proxy_ready=false`
  - `stronger_full_tier_truth_ready=false`
- claim readiness:
  - `mixed_topology_full_tier_status=full_failed`
  - `stronger_full_tier_truth_blockers`:
    - `fix_failed_matrix_cases`
    - `dedicated_sentry_or_nat_lab_evidence_ref`
  - `shared_network_pass_blockers`:
    - `fix_failed_matrix_cases`
    - `producer_qa_pass_uplift_decision_ref`

## Current Real Environment Note
- 当前已知可直接调度的真实多节点环境:
  - `local_node_count=1`
  - `aliyun_node_count=2`
- 该环境当前可合理承接:
  - `P2PARCH-6` follow-up real runs
  - `本机节点 + cloud public` mixed-topology drills
  - `bootstrap_poisoning / sentry_loss / path_failover / relay_budget / release_proxy` 类真实三节点回归
- 该环境当前不能单独证明:
  - `CGNAT` truth 已真实覆盖
  - dedicated `sentry/NAT lab` truth 已具备
  - 独立 operator / ASN diversity 已具备更强外部证据
  - `P2PARCH-7` shared-network `pass` gate 已满足
- 记录原则:
  - 后续若以这组环境追加 real-run evidence，应在 summary 或 evidence packet 中继续显式标注“`1` 本机 + `2` 阿里云”这一边界，避免把它误写成完整 mixed-topology lab truth。

## 结论
- 这轮工作把 `P2PARCH-6` 从“只有 full dry-run 计划”推进成“有真实 full proxy 执行和明确 failure signatures 的 audited partial”。
- current blocker 已不再是脚本无法自举或默认端口冲突，而是 proxy soak 本身暴露出的真实 `consensus/recovery` 失败签名。
- 在这些 proxy failure signatures 被修平之前，`P2PARCH-6` 不得宣称 `full_proxy_ready=true`；`P2PARCH-7` 也只能继续保持 `partial`，不能口头升级为 `pass`。
