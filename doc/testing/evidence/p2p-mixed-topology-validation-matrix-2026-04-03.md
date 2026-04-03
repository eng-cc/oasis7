# P2P Mixed Topology Validation Matrix (2026-04-03)

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
1. 新增 `scripts/p2p-mixed-topology-matrix.sh`，把 `P2PARCH-6` mixed-topology suite 收口成一个统一入口。
2. `required` 档位真实执行 7 个 exact case，全部通过。
3. `full` 档位先执行 dry-run，确认 7 个 exact case 与 2 个 proxy longrun case 都被装配进 summary。

## 执行命令
```bash
./scripts/p2p-mixed-topology-matrix.sh --tier required --out-dir .tmp/p2p_mixed_topology_validation
./scripts/p2p-mixed-topology-matrix.sh --tier full --dry-run --out-dir .tmp/p2p_mixed_topology_validation
./scripts/p2p-mixed-topology-matrix-smoke.sh
```

## 结果
- required summary:
  - `.tmp/p2p_mixed_topology_validation/20260403-120748-required/summary.json`
  - `overall_status=ok`
  - `case_count=7`
  - `exact_case_count=7`
  - `failed_count=0`
- full dry-run summary:
  - `.tmp/p2p_mixed_topology_validation/20260403-120740-full/summary.json`
  - `overall_status=dry_run`
  - `case_count=9`
  - `exact_case_count=7`
  - `proxy_case_count=2`
- smoke summary:
  - `.tmp/p2p_mixed_topology_smoke/required/20260403-120841-required/summary.json`
  - `.tmp/p2p_mixed_topology_smoke/full/20260403-120841-full/summary.json`

## 覆盖口径
- exact:
  - `nat_private_role_policy`
  - `validator_hidden_boundary`
  - `relay_only_lane_budget`
  - `cgnat_relay_path_ranking`
  - `bootstrap_poisoning_dedupe`
  - `relay_budget_detection`
  - `path_failover_selection`
- proxy:
  - `sentry_loss_proxy_longrun`
  - `mixed_topology_release_proxy`

## 当前边界
- 当前仓库仍没有 dedicated sentry role live harness，也没有物理 NAT/CGNAT 编排实验；因此 `proxy` case 只表示“当前可执行的近似恢复 drill”，不能代替最终 mixed-topology 实证。
- 是否要把这些 proxy case 升级成 shared-network 正式 gate，需要在 `P2PARCH-7` 与 shared-network release-train 任务里继续裁决。
