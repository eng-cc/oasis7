# DistFS 生产化硬化设计

- 对应需求文档：`doc/p2p/distfs/distfs-production-hardening.prd.md`
- 对应项目记录：`doc/p2p/distfs/distfs-production-hardening.project.md`

## 设计定位

本设计将 MIG-067..075 的九个完成阶段收敛为一个历史可追溯、以当前代码为行为真值的稳定入口。它覆盖本地文件索引保护、local-CAS challenge 自探测、reward-runtime 配置/状态接线和 adaptive backoff；不扩张为分布式一致性、远程证明或生产恢复设计。

## 结构与边界

- 文件索引层：CAS precondition、审计、孤儿判定和 manifest 导入导出保护本地 `files_index`；并发冲突由调用者处理，不能由接口名推导跨进程线性化。
- 本地 probe 层：cursor 和 policy 从本地 CAS 选择 blob，生成本地自检报告；它不验证远程 provider 也不构成多节点 attestation。
- reward-runtime 层：`oasis7_chain_runtime` 解析 `--reward-distfs-*` 参数，加载/原子写入 probe state，并把错误降为 warning + default cursor，保持主链路可用。
- adaptive 层：每轮预算、原因分类 multiplier 和最大 backoff 控制局部 I/O 压力；状态字段默认化确保旧 probe state 可读。
- reporting 层：当前对外 epoch report 只承载 aggregate checks/failures/ratio。详细 cursor/config/backoff 是本地状态而非外部 metrics contract。

## 运行面约束

- probe-state 缺失可默认初始化；不可读或 malformed 状态会警告后默认化。这是 best-effort scheduler continuity，不是数据、checkpoint 或 state-sync 的恢复路径。
- 不以重启掩盖 state、配置或 blob 失败。运维应保存状态文件、日志和稳定错误，再依现行 runbook 处理环境根因。
- distributed provider、replica maintenance、拓扑和 NodeRuntime 最佳努力轮询以 `distfs-distributed-resilience` 为权威；其失败语义不得被本地自探测文档覆盖。
- 当前代码/测试是行为锚点；Phase 5 的详细 epoch-report 描述已被 aggregate-report 现状替代，Phase 8 的 CLI 归属为 chain runtime，Phase 9 的 backoff 不是外部 telemetry。

## 验证入口

- `crates/oasis7_distfs/src/{lib.rs,manifest.rs,challenge_scheduler.rs}`。
- `crates/oasis7/src/bin/oasis7_chain_runtime/{cli.rs,distfs_probe_runtime.rs,reward_runtime_worker.rs}`。
- 本地回归：`env -u RUSTC_WRAPPER cargo test -p oasis7_distfs --lib`。
- 涉及节点、真实数据可用性或恢复声明时，使用 `testing-manual.md` S9A 的 state-sync/blob closure、triad、real-env 和 release 分层验证；本设计不把本地测试提升为这些证据。
