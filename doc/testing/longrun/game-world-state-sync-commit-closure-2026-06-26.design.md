# oasis7: Game World State Sync and Commit Closure Design

- 对应需求文档: `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md`
- 可变任务状态与历史: GitHub task issue evidence comments

审计轮次: 1

## 1. 设计定位
本设计补足 S9A 与 S10 之间的可读桥梁：单节点 runtime 合同只能证明本地执行正确，多节点状态同步/提交闭环必须证明 committed world state 能在 sequencer、storage、validator、observer 之间持续传播、恢复并投影。

## 2. Claim Boundary
| Claim | 最低证据 | 可以声明 | 不得声明 |
| --- | --- | --- | --- |
| `module_required` | runtime required + node/net/libp2p/consensus/distfs + mixed-topology required | 本地合同可集成 | 真实多节点持续稳定 |
| `module_full` | mixed-topology full + triad longrun + state-sync closure | proxy/triad 下可推进和恢复 | real-env/public-testnet ready |
| `integration_required` | 真实游戏 seed/world state + S10 或等价多节点 + API/viewer projection | 游戏世界状态链路无明显漂移 | 公开网络 readiness |
| `release_full` | real-env/public_testnet readiness lane + 同窗口证据 | live candidate 候选信心 | mainnet 或外部市场信心 |

## 3. Test Matrix
| 层级 | 节点/拓扑 | 覆盖对象 | 关键检查 |
| --- | --- | --- | --- |
| Phase 1 | single node | action、execution record、receipt、state hash、replay | same input same result; checkpoint/rollback recovers |
| Phase 2 | deterministic crates | node/net/consensus/distfs 合同 | libp2p path、consensus/finality、blob/store 基础合同 |
| Phase 3 | triad/proxy | commit propagation、peer heads、gap sync | committed height 单调、consensus hash 一致、peer heads 新鲜 |
| Phase 4 | observer/state sync | checkpoint、bundle、blob closure、observer catch-up | missing blob = 0; observer 自动追高 |
| Phase 5 | five-node game soak | real gameplay data、settlement、storage/observer roles | lag/stall/settlement/distfs gate 通过 |
| Phase 6 | real-env/release | same-window public_testnet/live candidate | readiness lanes pass; manifest 非 placeholder |

## 4. Evidence Shape
每次执行至少记录：
- command 和环境：git sha、world id、node ids、ports、duration、Bash 版本。
- commit 证据：submitted action、execution record、receipt、state hash、committed height。
- sync 证据：network height、peer heads、gap sync/state sync 结果、observer catch-up。
- closure 证据：state-sync bundle、blob closure report、missing blob count。
- projection 证据：`/v1/chain/status` 与 API/viewer projection 的同窗口样本。
- evidence packet：`doc/testing/templates/state-sync-closure-evidence-packet-template.md` 作为 `module_full` state-sync closure 汇总模板；不得把该模板文件本身作为 pass evidence。
- S10 summary contract：`scripts/s10-five-node-game-soak.sh` 在 `summary.json` 输出 `api_viewer_projection`，在 `summary.md` 输出 `API / Viewer Projection Contract`；默认 `status=not_collected`，真实 pass 需同窗口 API/viewer evidence refs。
- readiness lane binding：`scripts/network-tier-public-testnet-readiness.sh` 要求 `api_viewer_projection_ready` active lane；pass 证据不能是 template/placeholder。

## 5. Failure Routing
- `consensus_hash_divergence`: runtime/node/consensus 联合排查。
- `committed_height_not_monotonic`: runtime commit path 与 sequencer 状态排查。
- `known_peer_heads_zero_samples`: net/node peer-head publication 排查。
- `http_failure_samples`: node service health、status endpoint、port allocation 和 local process lifecycle 排查。
- `missing_blob_count > 0`: distfs/store closure 排查。
- `observer_catch_up_failed`: state-sync/checkpoint/observer bootstrap 排查。
- readiness lane `partial` / `block`: blockchain ops + QA 收口，不允许 release claim。

## 6. Integration Points
- `testing-manual.md#s9a链上大世界状态底座自闭环`
- `doc/testing/longrun/p2p-longrun-soak-and-chaos.prd.md`
- `doc/testing/longrun/s10-five-node-real-game-soak.prd.md`
- `scripts/game-world-state-sync-commit-module-required.sh`
- `doc/testing/templates/state-sync-closure-evidence-packet-template.md`
- `scripts/p2p-mixed-topology-matrix.sh`
- `scripts/p2p-longrun-soak.sh`
- `scripts/p2p-export-state-sync-bundle.sh`
- `scripts/p2p-verify-state-sync-closure.sh`
- `scripts/s10-five-node-game-soak.sh`
- `scripts/s10-five-node-game-soak-summary.test.sh`
- `scripts/network-tier-public-testnet-readiness.sh`
- `doc/testing/templates/public-testnet-readiness-lanes.example.tsv`

## 7. Design Risks
- Proxy triad 是可执行近似，不是 dedicated physical network lab。
- 单次 short soak 可发现明显阻断，但不能替代 release endurance。
- API/viewer projection 只证明状态可见性，不证明玩法好玩或真实玩家意愿。
- state-sync closure report 只能证明 blob 引用闭包；observer 自动追高仍需单独证据。
- 手工复制数据目录、checkpoint 或 seed 只能作为 break-glass/recovery 线索，不能作为 live-candidate readiness 证据。
