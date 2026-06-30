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
- Scope: formal `public_testnet` 从 skeleton / governed-bootstrap rehearsal 进入 `ready_for_live_candidate` 前的最小执行闭环
- Current Verdict: `block` / governed-bootstrap rehearsal / not `ready_for_live_candidate`

## 1. 适用范围
- 本 runbook 只定义 formal `public_testnet` 的 live-candidate checklist。
- 当前仓库已经具备：
  - `network_tier_manifest` schema / validate / smoke
  - `public_testnet` readiness review 脚本
  - six-lane scaffold
  - governed-bootstrap rehearsal evidence
- 当前仓库还不具备：
  - live `public_testnet` public RPC/explorer/faucet/reset evidence
  - formal all-pass lane TSV / readiness verdict
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
- 公开入口输入
  - `rpc_ref`
  - `explorer_ref`
  - `faucet_ref`
- 本机 hosted-login 形态测试入口输入（若本轮要验证该入口）
  - 本机节点的 formal `public_testnet` manifest / `world_id` / `chain_id`
  - genesis / bootstrap peers / validator registry refs
  - 节点 health/status、connected peers、height/head 推进证据
  - hosted-login / launcher / viewer / pure API 指向该节点 runtime/status/API 的配置证据
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
- manifest 仍是 `example.invalid`、template ref、`specified_skeleton_only` 占位输出，或 governed-bootstrap evidence 尚未升级出 rehearsal 状态。
- `release_candidate_bundle_ref` 不存在或不是当前候选版本真值。
- 六条 lane 任一没有 owner 或 evidence ref。
- 公开 RPC/explorer/faucet 仍是私网、单机 localhost 或 placeholder。
- 本机 hosted-login 形态入口没有证明背后 chain/world state 已接入 formal `public_testnet`，或仍指向本地新建 execution world。
- 任一 validator / observer 的同步证据来自手工复制 checkpoint、手工覆盖 `data/`、或 validator-to-validator 数据目录拷贝，而不是 runtime 自动 replication/head exchange 追平或 governed bootstrap runbook 的从零重建。
- reset policy 只存在口头说明，没有正式 announcement/evidence。
- runtime bootstrap 只有 template，没有真实运行证据。
- claims boundary review 缺失，或 visible claim 越过 `testnet/resettable/guarded faucet` 边界。
- 对外沟通提前使用：
  - `live public testnet is established`
  - `public validator onboarding is open`
  - `production OC settlement`

## 4. Six-Lane Checklist
| Lane | Owner | 必须证明什么 | 最小 evidence |
| --- | --- | --- | --- |
| `public_rpc_ready` | `runtime_engineer` | 公网 RPC 已可访问，且不是 placeholder/private-only endpoint | public URL + runtime status / health snapshot |
| `explorer_public_ready` | `liveops_community` | explorer 已可公开访问，且 freshness 不落后到误导外部测试者 | public URL + freshness / landing proof |
| `faucet_guard_ready` | `liveops_community` | faucet 存在且带 guard，不是无限制开放发放 | faucet policy / rate-limit / operator guard evidence |
| `reset_policy_announced` | `producer_system_designer` | 对外已明确这是 resettable `public_testnet`，不承诺 mainnet 价值稳定性 | public reset-policy announcement ref |
| `runtime_bootstrap` | `runtime_engineer` | 候选 bundle、genesis、bootstrap peers 与 runtime bootstrap 路径都可真实启动 | bootstrap rehearsal evidence / startup summary |
| `claims_boundary_review` | `qa_engineer` | 对外 claims 已过审，不会把 preview/testnet 说成 production/mainnet | claims review note / QA verdict / denied-claims evidence |

## 5. 推荐执行顺序
1. 先冻结 candidate bundle、genesis、bootstrap peers 与目标 manifest。
2. 补齐 public RPC、explorer、guarded faucet 三个公开入口的真实 URL 与健康证据。
3. 单独发布 reset-policy announcement，明确：
   - `public_testnet`
   - `resettable`
   - `guarded_testnet_faucet`
   - `non-mainnet value semantics`
4. 跑 runtime bootstrap rehearsal，留下 bundle/genesis/bootstrap peer 对账证据。
5. 若要暴露本机 hosted-login 形态入口，先确认本机节点已健康接入 testnet 网络，再确认 hosted-login / launcher / viewer / pure API 读取的是该节点的 testnet world state；节点健康是必要条件，但不是充分条件。
6. 由 `qa_engineer` 审 claims boundary，确认允许/禁止表述。
7. 把六条 lane 写入正式 TSV，再运行 readiness review 脚本，只有全部 `pass` 才允许进入 `ready_for_live_candidate`。

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

本机 hosted-login 形态接入 testnet 的辅助核查：
```bash
./scripts/p2p-public-testnet-local-observer-sync.sh render \
  --local-env <local-observer-node.env> \
  --sequencer-env <sequencer.env> \
  --storage-env <storage.env> \
  --manifest-path <public-testnet-manifest.json> \
  --out <rendered-local-observer.env>

./scripts/p2p-public-testnet-preflight.sh \
  --bundle <public-testnet-bundle.json> \
  --sequencer-status-url <url> \
  --sequencer-ip <ip> \
  --sequencer-port <port> \
  --storage-status-url <url> \
  --storage-ip <ip> \
  --storage-port <port> \
  --observer-env <local-observer-node.env>
```

该入口的 evidence 必须同时说明：
- 本机节点使用的 manifest / `world_id` / `chain_id` / genesis / bootstrap peers。
- 本机节点健康、已连接 testnet peers，且 height/head 持续推进。
- hosted-login / launcher / viewer / pure API 指向该节点 world state。
- 若只完成账号登录 smoke 或本地 hosted-public-join UI smoke，不得写成已接入 `public_testnet` 大世界。
- 若节点曾通过手工 checkpoint/data copy 恢复，必须先隔离该状态，并重新走自动恢复或从当前 deployment truth 从零重建；否则不得作为 live-candidate、hosted-login 或 local test environment evidence。

辅助检查：
- `rg -n "public_testnet|ready_for_live_candidate|specified_skeleton_only" testing-manual.md doc/p2p/prd.md doc/p2p/project.md`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 7. 当前缺口（2026-06-06）
- 历史 `specified_skeleton_only` 只能说明早期 skeleton 状态；当前还必须同时参考候选输入与 governed-bootstrap 证据集：
  - `doc/testing/evidence/public-testnet-live-candidate-manifest-2026-05-22.json`
  - `doc/testing/evidence/public-testnet-live-candidate-lanes-2026-05-22.tsv`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json`
- 当前仍不能宣称 `ready_for_live_candidate`：
  - governed bootstrap manifest 仍是 `status=rehearsal`
  - six-lane readiness 仍必须以正式 lanes TSV + `network-tier-public-testnet-readiness.sh` 汇总为准
  - 只要任一 lane 仍是 `partial` / `block`，或 evidence 仍是 template / placeholder / private-only ref，就不得升级为 `ready_for_live_candidate`
- 当前 example manifest 仍只能作为 skeleton/template 使用：
  - `network_id=oasis7-public-testnet-example`
  - `chain_id=oasis7-public-testnet-example`
  - `rpc/explorer/faucet = example.invalid`

## 8. 对外口径边界
- 现在允许说：
  - `formal public_testnet mechanism is documented`
  - `current governed bootstrap evidence is rehearsal / not ready_for_live_candidate`
  - `legacy shared_devnet evidence is not a target test environment`
- 现在不允许说：
  - `live public testnet is already online`
  - `public faucet is open`
  - `public validator admission is open`
  - `mainnet-like OC settlement is available`

## 9. 回写要求
每次正式推进 live candidate checklist，至少回写：
- GitHub task issue evidence comments
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md`
- `testing-manual.md`（若 canonical 命令或 claim boundary 变化）
- lane evidence 文档与 TSV

## 10. 收口标准
- 只有当 six-lane TSV 全部为 `pass`，且 evidence 都不是 template / placeholder / private-only ref，`public_testnet` readiness review 才允许输出 `ready_for_live_candidate`。
- 在此之前，producer / QA / liveops 必须继续维持：
  - `not_ready_for_live_candidate`
  - `rehearsal_or_skeleton_only`
  - `do_not_claim_live_public_testnet`
