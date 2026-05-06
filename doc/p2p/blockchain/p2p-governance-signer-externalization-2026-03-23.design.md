# oasis7 治理 signer 外部化与轮换门禁（设计文档）

- 对应需求文档: `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.project.md`

审计轮次: 1
## 设计目标
- 把 current governance signer truth 从 local convenience 和 production governance target 两层拆开。
- 冻结 finality signer 与 controller signer 的长期真值、更新 authority 和失效恢复门禁。
- 冻结 validator / finality signer 的正式准入流程，避免新节点加入仍依赖人工改 env 或本地 `NodeConfig`。

## 当前治理 signer 真值
| Governance scope | 当前来源 | 当前问题 | 生产结论 |
| --- | --- | --- | --- |
| `finality signer` | execution world `governance_finality_signer_registry`（存在时）; 否则回退 `world/governance` deterministic local seed | runtime 真值入口已切到 world-state registry，但新增/移除 validator 仍缺正式准入/激活流程 | partial |
| `controller signer` | execution world `governance_main_token_controller_registry`（存在时）; 否则回退 `NodeConfig.main_token_controller_binding.controller_signer_policies` | controller policy 真值也可由 world-state 恢复，但 appointment / freeze / ceremony 仍未闭环 | partial |

## 目标态
| Governance scope | 目标真值 | 必需能力 | 禁止项 |
| --- | --- | --- | --- |
| `finality signer` | on-chain/world-state governance signer registry | rotation、revocation、failover、operator ownership | deterministic local seed 进入 production |
| `controller signer` | on-chain/world-state controller signer registry | threshold policy updates、rotation、revocation、audit | 仅靠单机 `NodeConfig` 维护生产真值 |

## 选定方案备注
- producer 已选定 `governance truth 直接上链`。
- 因此任何链下 external registry 都只能作为过渡工具，不得在正式完成定义里替代最终链上 truth。

## Gate 切片
1. `GOVSIGN-1 inventory`: 固定 finality/controller signer 当前来源、环境等级和 blocker。
2. `GOVSIGN-2 truth boundary`: 冻结长期真值、update authority 与禁止项。
3. `GOVSIGN-3 ops policy`: 冻结 failover、rotation、revocation 与 operator ownership。
4. `GOVSIGN-4 release dependency`: 将 governance signer gate 接入 readiness/public-claims/ceremony 前置条件。
5. `GOVSIGN-5 admission workflow`: 冻结 validator / finality signer 的申请、审核、候选、激活与撤销生命周期。

## 主流公链抽象模式
| 共同模式 | 主流公链常见做法 | oasis7 设计取向 |
| --- | --- | --- |
| 准入不是改节点本地文件 | 通过协议内注册、治理动作或 stake/validator set 更新进入候选/活跃集合 | 新 validator 不再以 `NODE_VALIDATORS_CSV` / `NODE_VALIDATOR_SIGNERS_CSV` 为长期真值入口 |
| 节点身份和签名职责分离 | operator key、validator consensus key、withdraw/controller key 分角色管理 | 区分 node identity、finality signer、controller signer，禁止把 node identity 当成 finality signer 真值 |
| 激活有状态机 | 常见 `candidate -> active`，并带 epoch/era 边界 | oasis7 目标态采用 `applied -> approved_candidate -> probation_ready -> active` |
| 轮换/撤销走治理真值 | 变更通过链上记录或正式 registry 生效，并留下审计痕迹 | 所有 validator/finality signer 激活、轮换、撤销都必须回写 world-state registry |
| 热路径和高权限路径分层 | 验证出块热签名与金库/控制 signer 分开治理 | 面向外部申请开放的是 validator / finality signer；controller signer 仍是治理内部 appointment |

## oasis7 准入目标流程
### 范围边界
- 面向外部运营者开放的只是 `validator operator + finality signer` 准入路径。
- `main token controller signer` 不走公开申请；它属于治理/金库内部槽位 appointment。

### 目标状态机
| 状态 | 含义 | 进入条件 | 退出条件 |
| --- | --- | --- | --- |
| `applied` | 申请人已提交材料，但未审核 | 提交 node identity、公网/可达性信息、finality signer 公钥、operator ownership、public manifest | 审核驳回或进入 `approved_candidate` |
| `approved_candidate` | 通过治理审核，但尚未进入活跃 validator set | producer/runtime/QA 联合确认材料完整、角色边界正确 | 进入 `probation_ready` 或撤销 |
| `probation_ready` | 候选节点已完成 reachability/同步/演练检查，可排期激活 | candidate world / shared network 演练通过，activation epoch 已冻结 | 激活进入 `active` 或回退 |
| `active` | 已在 world-state registry 内成为正式 validator/finality signer | governance action / world-state registry update 生效 | 轮换、撤销、failover 或退场 |
| `rotating_out` / `revoked` | 正在退出或已撤销 | compromise、离岗、运维替换或治理决议 | restore/replacement 完成或永久移除 |

### 申请材料
- `node_id` / `node.public_key`
- `finality_signer_public_key`
- network reachability 信息：公开地址、private/hybrid/relay 策略、bootstrap 方案
- `operator_owner` 与变更审批联系人
- public-only manifest 摘要
- 预期 `activation_epoch` 或激活窗口

### 准入流程
1. 申请人提交 node identity、finality signer 公钥和 public-only manifest，不提交私钥材料。
2. `producer_system_designer` 冻结角色与容量策略，确认当前 validator set 是否允许扩容或替换。
3. `runtime_engineer` 校验 signer key 与 node identity 没有混用，且 candidate 配置符合当前 reachability / bootstrap 架构。
4. `qa_engineer` 在 candidate world、clone-world 或 shared network 上执行 reachability、同步、registry import/audit 与 failover smoke。
5. 审核通过后，把 candidate 以非活跃形式记录到治理台账或后续 candidate registry；真正激活时再写入 `governance_finality_signer_registry` 并附带 activation epoch。
6. 到达 activation epoch 后，runtime 通过 world-state registry 恢复新的 validator membership / signer binding；`--node-validator*` 不能再作为正式准入动作。
7. 若发生 compromise、失联或替换，则走 rotation / revocation / failover，同样通过 world-state registry 留痕并生效。

### 运行时影响
- active validator set 的唯一长期真值是 execution world 里的 governance registry。
- local `NodePosConfig` 只负责 bootstrap、测试或显式运维覆盖，不负责长期成员变更。
- 新节点即使已经拿到二进制与静态配置，只要 governance registry 未激活，也不能自称正式 validator。

## 通过条件
- `MAINNET-2` 通过前必须满足：
  - finality/controller signer 都有明确长期真值。
  - local seed/config path 被明确限制在非 production。
  - failover/rotation/revocation/operator ownership 都有 gate。
  - validator / finality signer admission workflow 已冻结，且与 controller signer appointment 分层。
  - readiness project 与模块主追踪同步更新。

## 对外口径
- 当前允许：
  - `crypto-hardened preview`
  - `signed governance controller proof exists in preview path`
- 当前禁止：
  - `production governance signer externalization is complete`
  - `mainnet-grade`
