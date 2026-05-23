# oasis7 正式网络分层与 `public_testnet` live-candidate checklist（Companion Runbook）

- 对应需求文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md`
- 对应设计文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md`

审计轮次: 1

## Meta
- Owner Role: `producer_system_designer`
- Review Roles:
  - `qa_engineer`
  - `liveops_community`
- Scope: formal `public_testnet` 从 `specified_skeleton_only` 进入 `ready_for_live_candidate` 前的最小执行闭环
- Current Verdict: `specified_skeleton_only`

## 1. 适用范围
- 本 runbook 只定义 formal `public_testnet` 的 live-candidate checklist。
- 当前仓库已经具备：
  - `network_tier_manifest` schema / validate / smoke
  - `public_testnet` readiness review 脚本
  - seven-lane scaffold
  - placeholder skeleton evidence
- 当前仓库还不具备：
  - live `public_testnet` public RPC/explorer/faucet/reset evidence
  - 真实 lane evidence TSV
  - `ready_for_live_candidate` 结论
- 本 runbook 不覆盖：
  - 真实 public testnet 部署细节实现
  - `mainnet` 激活
  - 浏览器/launcher 新功能开发

## 2. 开始前输入
每次准备把 formal `public_testnet` 从 skeleton 推向 live candidate 前，必须先固定：

- 同一份已校验 manifest
  - 不允许继续使用 `doc/testing/templates/network-tier-public-testnet.example.json` 里的 placeholder endpoint
- 同一份 release candidate bundle
- 同一份 genesis / bootstrap peer refs
- `shared_devnet_pass` 正式 evidence
- 公开入口输入
  - `rpc_ref`
  - `explorer_ref`
  - `faucet_ref`
- 政策输入
  - reset policy announcement ref
  - faucet guard policy ref
  - claims boundary review ref
- 值班 owner
  - `runtime_engineer`
  - `qa_engineer`
  - `liveops_community`
  - `producer_system_designer`

## 3. 硬阻断条件
- manifest 仍是 `example.invalid`、template ref 或 `specified_skeleton_only` 占位输出。
- `release_candidate_bundle_ref` 不存在或不是当前候选版本真值。
- 七条 lane 任一没有 owner 或 evidence ref。
- 还没有 `shared_devnet_pass`，却试图直接宣称 `public_testnet` live candidate。
- 公开 RPC/explorer/faucet 仍是私网、单机 localhost 或 placeholder。
- reset policy 只存在口头说明，没有正式 announcement/evidence。
- runtime bootstrap 只有 template，没有真实运行证据。
- claims boundary review 缺失，或 visible claim 越过 `testnet/resettable/guarded faucet` 边界。
- 对外沟通提前使用：
  - `live public testnet is established`
  - `public validator onboarding is open`
  - `production OC settlement`

## 4. Seven-Lane Checklist
| Lane | Owner | 必须证明什么 | 最小 evidence |
| --- | --- | --- | --- |
| `shared_devnet_pass` | `qa_engineer` | formal `public_testnet` 来源不是空中楼阁，而是建立在已通过的 shared release-train 之上 | shared-devnet gate `summary.md/json` 或等价 promotion evidence |
| `public_rpc_ready` | `runtime_engineer` | 公网 RPC 已可访问，且不是 placeholder/private-only endpoint | public URL + runtime status / health snapshot |
| `explorer_public_ready` | `liveops_community` | explorer 已可公开访问，且 freshness 不落后到误导外部测试者 | public URL + freshness / landing proof |
| `faucet_guard_ready` | `liveops_community` | faucet 存在且带 guard，不是无限制开放发放 | faucet policy / rate-limit / operator guard evidence |
| `reset_policy_announced` | `producer_system_designer` | 对外已明确这是 resettable `public_testnet`，不承诺 mainnet 价值稳定性 | public reset-policy announcement ref |
| `runtime_bootstrap` | `runtime_engineer` | 候选 bundle、genesis、bootstrap peers 与 runtime bootstrap 路径都可真实启动 | bootstrap rehearsal evidence / startup summary |
| `claims_boundary_review` | `qa_engineer` | 对外 claims 已过审，不会把 preview/testnet 说成 production/mainnet | claims review note / QA verdict / denied-claims evidence |

## 5. 推荐执行顺序
1. 先冻结 candidate bundle、genesis、bootstrap peers 与目标 manifest。
2. 用 shared-devnet pass 作为 promotion 输入，确认不是直接越级从 local/private 环境起跳。
3. 补齐 public RPC、explorer、guarded faucet 三个公开入口的真实 URL 与健康证据。
4. 单独发布 reset-policy announcement，明确：
   - `public_testnet`
   - `resettable`
   - `guarded_testnet_faucet`
   - `non-mainnet value semantics`
5. 跑 runtime bootstrap rehearsal，留下 bundle/genesis/bootstrap peer 对账证据。
6. 由 `qa_engineer` 审 claims boundary，确认允许/禁止表述。
7. 把七条 lane 写入正式 TSV，再运行 readiness review 脚本，只有全部 `pass` 才允许进入 `ready_for_live_candidate`。

## 6. Canonical Commands
```bash
./scripts/network-tier-manifest.sh validate \
  --manifest <public-testnet-manifest.json>

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest <public-testnet-manifest.json> \
  --lanes-tsv <public-testnet-lanes.tsv>

./scripts/network-tier-exit-review.sh \
  --manifest <public-testnet-manifest.json>
```

辅助检查：
- `rg -n "public_testnet|ready_for_live_candidate|specified_skeleton_only" testing-manual.md doc/p2p/prd.md doc/p2p/project.md`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 7. 当前缺口（2026-05-18）
- 当前 example manifest 仍是：
  - `network_id=oasis7-public-testnet-example`
  - `chain_id=oasis7-public-testnet-example`
  - `rpc/explorer/faucet = example.invalid`
- 当前 placeholder evidence 仍明确：
  - 不证明 public RPC reachability
  - 不证明 explorer freshness
  - 不证明 guarded faucet enforcement
  - 不证明 reset-policy announcement
  - 不证明 `ready_for_live_candidate`
- 当前仓库还没有：
  - live `public_testnet` candidate bundle ref
  - live `public_testnet` lanes TSV
  - real public endpoint evidence
  - claims boundary 审核结论

## 8. 对外口径边界
- 现在允许说：
  - `formal public_testnet mechanism is documented`
  - `current verdict is specified_skeleton_only`
  - `shared_devnet is not public_testnet`
- 现在不允许说：
  - `live public testnet is already online`
  - `public faucet is open`
  - `public validator admission is open`
  - `mainnet-like OC settlement is available`

## 9. 回写要求
每次正式推进 live candidate checklist，至少回写：
- `.pm/tasks/<TASK-UID>.execution.md`
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md`
- `testing-manual.md`（若 canonical 命令或 claim boundary 变化）
- lane evidence 文档与 TSV

## 10. 收口标准
- 只有当 seven-lane TSV 全部为 `pass`，且 evidence 都不是 template / placeholder / private-only ref，`public_testnet` readiness review 才允许输出 `ready_for_live_candidate`。
- 在此之前，producer / QA / liveops 必须继续维持：
  - `specified_skeleton_only`
  - `do_not_claim_live_public_testnet`
