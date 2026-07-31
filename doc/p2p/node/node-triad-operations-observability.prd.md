# oasis7 三节点运维观测合同

- 对应设计文档: `doc/p2p/node/node-triad-operations-observability.design.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

## 权威边界

本专题是 real-environment triad 的当前 operator 与 evidence authority。它定义节点 inventory、health/status/host/process/traffic/WASM 采样、bounded artifact、summary 解释和 fail-closed 行为；runtime 的 consensus、status schema 与 replay 语义仍由对应 runtime authority 定义，QA 决定 release/evidence verdict。

它不声明主网、public-chain-grade reachability、长期告警平台、自动 deploy/restart/rollback/restore/state-sync 或 release readiness。一次 status read、fixture 或 monitor run 不创建、续期或升级运行证据。

## 目标

提供一个可重复执行、可审计且 fail-closed 的 triad operator/evidence 入口，使同一窗口可以同时定位链健康、宿主机资源、traffic、WASM 与 bounded reachability 的异常。

## 范围

覆盖 repo-owned monitor、inventory、host/service 与 status 输入、summary 产物、fixture 验证和 evidence interpretation；不覆盖 consensus 实现、自动 remediation、外部监控平台或发布放行。

## 接口 / 数据

`/healthz`、`/v1/chain/status`、host/process sample、traffic/WASM window、`overall.status`、per-node alerts、module breakdown 和 `optimization_candidates` 是本专题的接口与 artifact contract。私钥、seed、SSH password、完整 env 秘密和原始业务载荷不得进入产物。

## Inventory 与采样合同

当前 triad 以 `local_node`、`sequencer_ecs`、`storage_ecs` 标识；实际 runtime role 必须从同一窗口的 sampled status 读取，不能由标签推断。默认 service labels 为 `oasis7-triad-observer.service`、`oasis7-triad-sequencer.service`、`oasis7-triad-storage.service`，部署 inventory 可以显式覆盖。

| 输入 | 当前合同 | fail-closed 行为 |
| --- | --- | --- |
| host/process | CPU、load、memory、storage、systemd service、runtime PID/CPU/memory/thread 采样 | SSH 不可达、PID 缺失、service 无法读取或 storage 不可读必须成为 node alert。 |
| chain health | `/healthz`、`/v1/chain/status`、height/progress、consensus、replication、storage、reward、transactions 与 execution/state-root projection | snapshot 不是 `pass_candidate` 时 summary 至少为 `blocked`，不得被资源正常覆盖。 |
| windows | traffic 与 WASM cumulative snapshot 以同一 run-dir 汇总为窗口 summary | reset/restart 缩短窗口必须显式标识；history 按窗口和 buffer bounded retention。 |
| reachability | 消费 status 的 bounded path projection、active path mix、fallback reason 与 confidence | 不重新推导 canonical path truth，不导出 unbounded peer-id labels，不把 control-plane bytes 当 NIC overhead。 |

## 产物与解释

Canonical command is `scripts/p2p-real-env-observability-monitor.sh`; it composes triad snapshot, host monitor, traffic monitor and WASM monitor. It writes machine-readable JSON and Markdown summary containing `overall.status`、per-node alerts、module breakdown and `optimization_candidates`.

`pass_with_resource_alerts` 或 `pass_with_module_alerts` 仅可在 underlying snapshot 为 `pass_candidate` 时使用。summary 的可读诊断不构成 public-chain claim、长期可用性承诺或发布许可。

## 验证与恢复边界

- `bash -n scripts/p2p-real-env-node-host-sample.sh scripts/p2p-real-env-host-monitor.sh scripts/p2p-real-env-observability-monitor.sh`
- `python3 -m py_compile scripts/p2p-real-env-host-summary.py scripts/p2p-real-env-observability-summary.py`
- `bash scripts/p2p-real-env-host-monitor.test.sh`
- `bash scripts/p2p-real-env-observability-monitor.test.sh`

真实环境采样、部署、restart、rollback、restore 与 state sync 需要独立授权和每次环境证据。restart 只能是诊断或临时恢复，不能替代对代码、配置或部署根因的修复。

## 里程碑

- M1：host/process、snapshot、traffic、WASM 与 merged summary contract 落地。
- M2：fixture 和 canonical command 保持可运行，持续积累 separately authorized real-environment evidence。

## 风险

- 远端系统命令格式、SSH 或 service state 的差异可能使输入缺失；必须显式 alert。
- 将局部资源、relay 或 control-plane 指标误读为 public-chain/release verdict 会扩大本专题边界。
