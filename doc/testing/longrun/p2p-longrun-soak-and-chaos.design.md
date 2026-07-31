# P2P 长跑、Soak 与 Chaos 设计

- 对应需求文档: `doc/testing/longrun/p2p-longrun-soak-and-chaos.prd.md`
- 可变任务状态与历史: GitHub task issue evidence comments

## 设计定位

以 `scripts/p2p-longrun-soak.sh` 为 S9 编排入口，将 runtime 启停、拓扑、
采样、门禁、chaos/feedback 注入和证据归档分层。测试层消费 runtime、
network、consensus 与 DistFS 的专业合同，不复制或改写这些实现权威。

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
