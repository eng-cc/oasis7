# P2P 长跑、Soak 与 Chaos 设计

- 对应需求文档: `doc/testing/longrun/p2p-longrun-soak-and-chaos.prd.md`
- 可变任务状态与历史: PR、实际 CI 与评审记录（Issue 按需）

## 设计定位

以 `scripts/p2p-longrun-soak.sh` 为 S9 编排入口，将 runtime 启停、拓扑、
采样、门禁、chaos/feedback 注入和证据归档分层。测试层消费 runtime、
network、consensus 与 DistFS 的专业合同，不复制或改写这些实现权威。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [Reward 时间线](p2p-longrun-soak-and-chaos.prd.md#reward-timeline-source-contract) | 原始计数、零分母与缺失值逐列一致 | [导出设计](#reward-timeline-source-design) | QA / reward-runtime 维护者 | 不修改奖励与门禁，不代签真实节点 |

## 数据流

1. 解析 profile、duration、topology、threshold、chaos 与 feedback 参数，
   在启动前拒绝非法组合。
2. 以 `oasis7_chain_runtime` 拉起 triad/triad-distributed，经过 startup grace
   后周期读取 status/balances。
3. 固定 chaos plan 与 continuous scheduler 共用串行执行器；每次事件记录
   scheduled、completed/failed 及恢复观察窗口。
4. feedback scheduler 独立记录 submit 结果，不让瞬时业务探针失败终止核心
   采样或篡改核心 gate。
5. 聚合 run_config、timeline、summary、failures、chaos/feedback logs 与节点
   日志；最终状态只能从完整证据和 profile 规则计算。

## Chaos 约束

- 固定 plan 使用稳定 event ID 和递增时间轴，作为跨版本可比基线。
- continuous scheduler 以 seed 决定动作/节点序列，受 start、interval 与
  max-events 限制；固定与持续事件必须串行，避免并发注入破坏可解释性。
- 180 分钟模板覆盖 restart、pause、disconnect 与 sequencer/storage/observer
  轮换，但它只是 executable fixture。当前 run 的 topology、duration、threshold
  与 artifacts 才能支持本次结论。
- recovery 由事件后窗口的 committed progress、peer catch-up、hash/lag 与
  invariant 共同判断；最后一个健康采样不能抹去中间失败。

## 门禁与异常

- `insufficient_data`、stale peer head、non-monotonic height、hash mismatch、
  missing blob、observer 未追平或非零关键失败都必须显式保留。
- smoke 可把单纯样本不足降为 warning；endurance/release 不得降级。
- 进程退出、超时或采样失败须保留节点日志和 `failures.md`。
- macOS 默认 Bash 3.2 不满足脚本约束；执行 S9/S10 使用 Bash 4+。
- 本设计不声明 checkpoint/restore、state-sync bundle 或 observer recovery；
  这些结论回到 GWSC 权威。

## 稳定接口

- `scripts/p2p-longrun-soak.sh`
- `doc/testing/chaos-plans/p2p-soak-endurance-full-chaos-v1.json`
- `/v1/chain/status`、`/v1/chain/balances`
- `run_config.json`、`timeline.csv`、`summary.json`、`summary.md`
- `failures.md`、`chaos_events.log`、`feedback_events.log`、`nodes/*`

## 非承诺

本设计只定义可重复的测试与证据解释。它不把任何 local/proxy/short-window
结果升级为生产拓扑、长期 endurance、public-testnet、mainnet 或 release
就绪，也不把流量注入成功升级为业务服务 SLA。

<a id="reward-timeline-source-design"></a>
## Reward 时间线导出

产品适用性：N/A；reason=本节只规定已采集诊断计数到 CSV 的导出和空值解释，不改变奖励、结算或共识规则；scope=reward timeline exporter 的逐列序列化。真实节点、消费者、长跑与恢复验收不在此不适用范围，仍按原专业合同独立执行。

采样层将同一 node/time 的 reward 计数传给 exporter，按固定 header 写入 `total_checks`、`failed_checks` 和比率；S10 还传 settlement attempts/failures。正分母用实际计数计算比率，已知 0/0 保留零计数与空比率，缺失指标保留空数值并回链同次 unavailable 告警。CSV 解析须区分空值与 0，不以独立 aggregate/gate 正确为逐行正确的证明。

本节为 exporter 和消费者应共同满足的设计，不声称当前脚本已修复；不改变 reward 计算、结算、累计摘要聚合或既有门禁。验收按列回读 5/0、5/1、0/0、missing 和 S10 8/1 的离线 fixture，不启动节点。

## 11. 验证设计与可追溯性

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [Reward 时间线](p2p-longrun-soak-and-chaos.prd.md#reward-timeline-source-contract) | [导出设计](#reward-timeline-source-design) | 同次样本、header、分母、空值一致 | 候选版本[现有验证入口](../../../testing-manual.md)，补充逐行 5/0、5/1、0/0、missing、S10 8/1 fixture；原有 summary 断言不代签新 exporter 回归 | 实施 PR 的输入、CSV、解析断言与实际退出结果 | 本次文档修订不表示回归已运行，不证明真实长跑或结算 |

## 范围、结构边界与风险

本设计范围是 S9 的编排、采样、故障注入与原始证据解释；结构由数据流、chaos 约束和门禁组成。短窗口、拓扑替代、恢复末次健康样本或 CSV 空值误读都可能造成假通过；原始失败和未证明范围必须保留，GWSC 恢复义务仍独立。
