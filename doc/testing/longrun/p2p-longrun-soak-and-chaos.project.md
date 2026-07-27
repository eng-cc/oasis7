# P2P 长跑、Soak 与 Chaos 项目与追溯

- 对应需求文档: `doc/testing/longrun/p2p-longrun-soak-and-chaos.prd.md`
- 对应设计文档: `doc/testing/longrun/p2p-longrun-soak-and-chaos.design.md`

## 任务拆解

- S9 脚本、profile、threshold 与产物 schema 变化由 `qa_engineer` 维护测试
  合同，并取得受影响的 runtime/blockchain-ops 复核。
- chaos plan 变更必须保持 schema、事件 ID、时间轴和短窗兼容验证；模板版本
  不能替代实际 endurance run。
- feedback probe 只作为 S9 traffic evidence；feedback ledger/replication
  技术语义由 DistFS 稳定权威拥有。
- state-sync/commit/recovery claim 由 GWSC 稳定权威拥有。

## 历史专题吸收

| 已吸收专题 | 当前归属 |
| --- | --- |
| chain-runtime soak script reactivation | 当前 runtime/status sampling、产物与异常合同。 |
| P2P continuous chaos injection | continuous 参数、seed、串行执行与计数合同。 |
| P2P endurance chaos template | 固定 180 分钟模板的 fixture 与非证明边界。 |
| P2P feedback event injection | traffic probe、独立日志与对账合同。 |
| P2P/storage/consensus online stability | S9 profile、topology、metrics、gate 与 evidence 合同。 |

这些 dated PRD/design/project 与 endurance migration note 已在语义回填和
活跃引用修复后删除。完成任务、旧命令、dated sample 与 `.tmp` 路径继续由
Git/GitHub task evidence 保存，不能重新作为 active authority 或 readiness。

## 依赖

- `scripts/p2p-longrun-soak.sh`
- `testing-manual.md`
- `doc/testing/chaos-plans/p2p-soak-endurance-full-chaos-v1.json`
- `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md`
- `doc/p2p/distfs/distfs-feedback-ledger-and-replication.prd.md`
- `doc/testing/longrun/s10-five-node-real-game-soak.prd.md`

## 验证责任

- 文档与引用：
  `./scripts/doc-governance-check.sh`、
  `./scripts/readme-link-check.sh`、
  `./scripts/unified-world-code-terminology-scan.test.sh`。
- 脚本合同：
  `bash -n scripts/p2p-longrun-soak.sh scripts/s10-five-node-game-soak.sh`、
  `bash scripts/p2p-longrun-soak-endpoint-latency.test.sh`、
  `./scripts/s10-five-node-game-soak-summary.test.sh`。
- 阈值、summary schema 或 claim 变化必须升级到 testing manual 指定的
  S9 endurance/S10 long-window；纯权威迁移不伪造一次新的长跑 pass。

## 状态

本三件套是 S9 soak/chaos 的当前稳定专业 authority。历史实现范围已完成；
当前任何 pass、release 或网络就绪结论仍必须绑定新的 run artifacts 与专业
门禁，不从本项目文档的完成状态推导。
