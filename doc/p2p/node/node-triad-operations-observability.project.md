# 三节点运维观测合同（项目追踪）

- 对应需求文档: `doc/p2p/node/node-triad-operations-observability.prd.md`
- 对应设计文档: `doc/p2p/node/node-triad-operations-observability.design.md`

## 任务拆解

- [x] node-triad-operations-observability-consolidation (PRD-P2P-025) [test_tier_required]: 吸收 host/process、snapshot/traffic/WASM composition、merged summary、bounded reachability 与 fixture contracts。 Trace: #2682 (task_172abebb99354d4fad395aa05a581193)

## 当前验证入口

- `bash scripts/p2p-real-env-host-monitor.test.sh`
- `bash scripts/p2p-real-env-observability-monitor.test.sh`
- `./scripts/doc-governance-check.sh` 与 `./scripts/readme-link-check.sh`

## 依赖

- `scripts/p2p-real-env-triad-snapshot.sh`
- `scripts/p2p-real-env-host-monitor.sh`
- `scripts/p2p-real-env-traffic-monitor.sh`
- `scripts/oasis7-node-wasm-metrics-monitor.sh`
- `testing-manual.md`

## 状态

- 当前阶段：active operations authority；真实 triad 长窗口 evidence、release conclusion、长期告警、自动恢复和真实环境变更仍需分别记录专业证据。
- 历史实施过程由 Git history 与 GitHub task evidence 追溯。
