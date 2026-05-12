# oasis7 Runtime：三节点完整监控体系（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-triad-observability-stack-2026-04-23.design.md`
- 对应需求文档: `doc/p2p/node/node-triad-observability-stack-2026-04-23.prd.md`

审计轮次: 2

## 任务拆解（含 PRD-ID 映射）
- [x] triad-observability-host-monitor (PRD-P2P-025-A) [test_tier_required]: 新增 triad host/process monitor，冻结当前 real-env triad（本机 + 2 ECS，runtime role 以 sampled status 为准）的资源采样 contract，并保留 legacy label 兼容口径。 Trace: .pm/tasks/task_d3cb937a968e4a4187e2f143c9444d6f.yaml
- [x] triad-observability-merged-summary (PRD-P2P-025-B) [test_tier_required]: 新增 merged summary helper 与 triad observability wrapper，串起 snapshot/host/traffic/wasm。 Trace: .pm/tasks/task_d3cb937a968e4a4187e2f143c9444d6f.yaml
- [x] triad-observability-docs-and-smoke (PRD-P2P-025-C) [test_tier_required]: 回写 `doc/p2p` / `testing-manual.md`，补 fixture 回归与 real-env 小样本验证入口。 Trace: .pm/tasks/task_d3cb937a968e4a4187e2f143c9444d6f.yaml
- [x] triad-observability-module-breakdown (PRD-P2P-025) [test_tier_required]: 把 triad observability summary 细分到 runtime 子模块，并输出可执行的 optimization candidates。 Trace: .pm/tasks/task_129613e6a9fd421da0a2c2f79824c51c.yaml

## 依赖
- `scripts/p2p-real-env-triad-snapshot.sh`
- `scripts/p2p-real-env-traffic-monitor.sh`
- `scripts/oasis7-node-wasm-metrics-monitor.sh`
- `testing-manual.md`

## 产物文件
- `scripts/p2p-real-env-node-host-sample.sh`
- `scripts/p2p-real-env-host-summary.py`
- `scripts/p2p-real-env-host-monitor.sh`
- `scripts/p2p-real-env-observability-summary.py`
- `scripts/p2p-real-env-observability-monitor.sh`
- `scripts/p2p-real-env-host-monitor.test.sh`
- `scripts/p2p-real-env-observability-monitor.test.sh`
- `fixtures/p2p_real_env_host_monitor/history.ndjson`
- `fixtures/p2p_real_env_observability/*`
- `.pm/tasks/task_129613e6a9fd421da0a2c2f79824c51c.execution.md`
- `doc/p2p/node/node-triad-observability-stack-2026-04-23.*`
- `doc/p2p/project.md`
- `doc/p2p/prd.md`
- `doc/p2p/prd.index.md`
- `doc/p2p/node/README.md`
- `testing-manual.md`
- `.pm/tasks/task_d3cb937a968e4a4187e2f143c9444d6f.execution.md`

## 验收命令（`test_tier_required`）
- `bash -n scripts/p2p-real-env-node-host-sample.sh scripts/p2p-real-env-host-monitor.sh scripts/p2p-real-env-observability-monitor.sh scripts/p2p-real-env-host-monitor.test.sh scripts/p2p-real-env-observability-monitor.test.sh`
- `python3 -m py_compile scripts/p2p-real-env-host-summary.py scripts/p2p-real-env-observability-summary.py`
- `bash scripts/p2p-real-env-host-monitor.test.sh`
- `bash scripts/p2p-real-env-observability-monitor.test.sh`
- `bash scripts/p2p-real-env-observability-monitor.sh --samples 2 --interval-secs 2 --traffic-samples 2 --traffic-interval-secs 2 --window-minutes 1 --out-dir .tmp/p2p_real_env_observability_smoke`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前阶段：已实现；当前 merged summary 已补齐模块级 breakdown 与 optimization candidates，待在真实 triad 上继续积累长期窗口 evidence。
- 风险跟踪：
  - 远端系统命令输出若发生差异，host sample parser 需要继续兼容。
  - 当前只做 repo-owned artifact；尚未接入长期告警/时序平台。
- 下一步：
  - 若后续需要 cron/systemd 定时执行，可在当前 wrapper 之上再补 runbook / timer 模板，而不是改写 summary contract。
