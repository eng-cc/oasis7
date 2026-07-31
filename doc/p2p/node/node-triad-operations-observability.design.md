# 三节点运维观测合同设计

- 对应需求文档: `doc/p2p/node/node-triad-operations-observability.prd.md`
- 对应项目管理文档: GitHub Issue / GitHub Project

## 设计定位

以 repo-owned 脚本保持“一个控制机采样三节点、在同一 run dir 合并证据”的模式。summary 只消费 runtime 发布的 status truth；它将 host、chain、traffic、WASM 与 bounded reachability 拼接为诊断面，不能成为第二套 consensus 或 reachability 真值。

## 数据流

`health/status + host/process + traffic window + WASM window -> snapshot/host/summary helpers -> JSON + Markdown artifacts -> operator/QA evidence review`

每层都保留输入缺失、reset、degraded 和 blocked 状态。restart/replay/reorg 的 lifecycle persistence 由 runtime status/evidence contract fail closed；monitor 不能通过重启或补写历史数据掩盖它。

## 安全与运行约束

只采样资源和状态，不写 SSH password、private key、seed、完整 env 秘密或原始业务载荷。长期 alerting/TSDB、Prometheus/Grafana/OTel exporter、自动 remediation 和对外 status 承诺不属于本设计。
