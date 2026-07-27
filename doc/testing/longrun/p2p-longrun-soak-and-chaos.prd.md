# P2P 长跑、Soak 与 Chaos 测试合同

- 对应设计文档: `doc/testing/longrun/p2p-longrun-soak-and-chaos.design.md`
- 对应项目管理文档: `doc/testing/longrun/p2p-longrun-soak-and-chaos.project.md`

## 目标

定义 S9 对 P2P、DistFS 与共识持续运行的稳定测试合同：以
`scripts/p2p-longrun-soak.sh` 编排可重复的多节点 soak，采集结构化证据，
按确定门禁判断持续推进、复制、恢复与不变量状态，并可叠加固定或持续
chaos 以及 feedback 流量探针。

本三件套吸收七个 dated longrun 专题中的五组 S9 测试语义。历史实现任务、
短窗样本和完成状态只保留在 Git 与 GitHub task evidence，不作为当前运行或
发布就绪证明。

## 范围

- `soak_smoke`、`soak_endurance`、`soak_release` 是 S9 执行档位，不等同
  Cargo `test_tier_*`，也不单独构成发布批准。
- 拓扑至少覆盖 `triad` 和 `triad_distributed`；若声称 chaos/recovery，
  必须在同一 run 中保留注入与恢复窗口证据。
- 核心门禁覆盖 committed height 单调推进、consensus hash 一致、peer head
  新鲜度、stall、lag、DistFS failure ratio 与 invariant 状态。
- `metric_gate=insufficient_data` 在 smoke 中只能产生告警，在 endurance 或
  release 档位必须失败；缺样本、跳过档位或只存在模板都不是 pass。
- 当前 state-sync、commit closure、observer catch-up 与 claim tier 由
  `game-world-state-sync-commit-closure-2026-06-26.prd.md` 拥有，本专题只提供
  S9 执行证据。

## 接口 / 数据

- 每次 run 必须输出 `run_config.json`、`timeline.csv`、`summary.json`、
  `summary.md`、节点日志；失败时必须输出 `failures.md`。
- chaos 场景必须输出 `chaos_events.log`，并使 plan、continuous 与 total
  计数和 completed/failed 明细一致。恢复结论必须与注入后的观测窗口关联，
  不能从最终快照推断。
- 固定模板 `doc/testing/chaos-plans/p2p-soak-endurance-full-chaos-v1.json`
  是版本化、可复跑的 180 分钟输入基线；模板存在或短窗兼容验证不等于
  endurance 已通过。
- continuous chaos 记录 start、interval、event limit、actions、seed、
  restart-down/pause duration；同 seed 与相同输入应复现同一串行调度。
- feedback probe 按节点轮询并交替提交 bug/suggestion，输出
  `feedback_events.log` 以及 success/failed/total。单次 submit 失败应记录而
  不终止核心 run，但不得被解释为 feedback replication 或生产流量就绪。
- 脚本以当前 `oasis7_chain_runtime` 和 `/v1/chain/status` /
  `/v1/chain/balances` 为采样合同；旧 viewer-node 路径与旧 epoch 字段仅是
  历史 provenance。缺失字段必须标记 unavailable，禁止伪造兼容值。

## 验收

- AC-1：命令 `rc=0`，summary 与 timeline 完整，`overall_status=ok` 且无
  topology failure。
- AC-2：endurance/release 的 metric gate 为 pass；committed height、
  consensus hash、peer freshness、stall/lag/DistFS/invariant 门禁满足当前
  profile 阈值。
- AC-3：启用 chaos 时，配置、seed、时间线、结果与计数可审计；缺恢复窗口
  证据时不得声明 recovered。
- AC-4：启用 feedback 时，日志明细与 success/failed/total 精确对账；
  submit 结果不得改变共识或 DistFS 判定语义。
- AC-5：手册、脚本、模板和本权威保持一致；阈值或 summary schema 变化需
  补对应 endurance 回归。

## 非目标与声明边界

- 不定义跨物理机、跨地域或真实公网故障编排，不改变共识、DistFS 或
  feedback 协议。
- 本地/代理 triad、短窗 smoke、固定模板、historical `[x]` 或 dated sample
  不证明 real-env、public-testnet、mainnet、rollback/restore 或 release
  readiness。
- 手工复制 validator data、checkpoint 或 seed 不是 live-candidate recovery
  证据；真实环境声明必须提供同窗口 inventory、topology、readiness lane 与
  专业 owner 结论。
- 日志、模板与示例不得包含 signer 私钥、凭据或私有 endpoint。

## 里程碑

- M1：S9 profile、topology、metrics、artifacts 与 gate 形成稳定合同。
- M2：固定/持续 chaos、seed replay 与 recovery-window 证据形成稳定合同。
- M3：feedback traffic probe 与 DistFS/runtime 专业 authority 完成边界接线。
- M4：dated 源专题语义回填、活跃引用修复并删除。

## 风险

- 单机资源竞争、startup 抖动或窗口过短可能造成指标噪声；按 profile 和
  artifact 解释，不通过放宽 gate 消除噪声。
- chaos 密度、seed 或事件叠加可能降低可比性；固定基线优先，continuous
  仅作探索覆盖。
- 历史完成状态容易被误写成当前 readiness；所有 pass 必须绑定新的 run。

## 追溯

- S9 operator 入口：`testing-manual.md`
- claim/evidence boundary：
  `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md`
- runtime/DistFS feedback replication：
  `doc/p2p/distfs/distfs-feedback-ledger-and-replication.prd.md`
- S10 gameplay/settlement/mint：
  `doc/testing/longrun/s10-five-node-real-game-soak.prd.md`
