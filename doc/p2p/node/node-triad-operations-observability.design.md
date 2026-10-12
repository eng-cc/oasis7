# 三节点运维观测合同设计

- 对应需求文档: `doc/p2p/node/node-triad-operations-observability.prd.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

## 设计定位

以 repo-owned 脚本保持“一个控制机采样三节点、在同一 run dir 合并证据”的模式。summary 只消费 runtime 发布的 status truth；它将 host、chain、traffic、WASM 与 bounded reachability 拼接为诊断面，不能成为第二套 consensus 或 reachability 真值。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [运行隔离与诊断](node-triad-operations-observability.prd.md#observability-run-contract) | 独占 run、按消费者路由、SSH 失败仍报告、持久错误不由指标发布清除 | [设计约束](#observability-run-design) | 对应模块维护者与 QA | 文档补齐不宣称实现或真实运行完成 |

## 数据流

`health/status + host/process + traffic window + WASM window -> snapshot/host/summary helpers -> JSON + Markdown artifacts -> operator/QA evidence review`

每层都保留输入缺失、reset、degraded 和 blocked 状态。restart/replay/reorg 的 lifecycle persistence 由 runtime status/evidence contract fail closed；monitor 不能通过重启或补写历史数据掩盖它。

<a id="observability-run-design"></a>
## 编排责任与终态输出

1. monitor 解析并验证配置；按 snapshot/host/traffic/WASM 的受支持输入分别构造参数，不把同一参数数组无差别转交所有子工具。
2. 写入前原子分配独占 run 目录，snapshot 单独调用也承担同样责任；冲突在写前拒绝或重新分配，不覆盖已存在目录。外层与子采集器记录同一所属关系。
3. 捕获采集失败并保留结构化不可用样本；host SSH 失败不跳过后续节点和最终报告。终态汇总同时保留非零退出、失败节点、仍有效的其他采样和 underlying snapshot 的 blocked。
4. 汇总仅引用本 run 的实际文件；失败细节限制为 label、时间、阶段和受限错误类别，不输出凭据或完整环境。
5. reward state 写入错误独立于数值发布与 invariant：采集窗口按实际 last_error/degraded 解释，不用已有 report 或无新 epoch 清错。producer 只有在对应持久化真正恢复后才清除该错误；本设计不更改持久化格式或奖励计算。

## 安全与运行约束

只采样资源和状态，不写 SSH password、private key、seed、完整 env 秘密或原始业务载荷。长期 alerting/TSDB、Prometheus/Grafana/OTel exporter、自动 remediation 和对外 status 承诺不属于本设计。


## 11. 验证设计与可追溯性

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [运行隔离与诊断](node-triad-operations-observability.prd.md#observability-run-contract) | [设计约束](#observability-run-design) | 独占 run、按消费者路由、SSH 失败仍报告、持久错误不由指标发布清除 | 候选版本的[验证入口](../../../scripts/p2p-real-env-observability-monitor.test.sh)：固定时钟并发、sentinel、service/status 参数及首个/后续/全部 SSH 失败；reward 两轮失败及真正恢复另按 worker fixture 验证；静态核对与实际执行分别记录 | 实施 PR 的候选、命令、fixture 与真实结果 | 不代签未执行回归、真实模型、浏览器、节点或发布验收 |
