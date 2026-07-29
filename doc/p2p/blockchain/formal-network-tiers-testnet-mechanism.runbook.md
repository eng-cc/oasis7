# oasis7 正式网络分层与 `public_testnet` live-candidate checklist（Companion Runbook）

- 对应需求文档: `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md`
- 对应设计文档: `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.project.md`

审计轮次: 1

## 稳定 authority 与证据边界

- 本文件是 formal `public_testnet` live-candidate checklist 的稳定专业 authority；它聚合 lane owner、evidence、命令和 claim boundary，但不是节点部署、升级、重启、回滚或恢复 SOP。上述运行面操作只可按 `public-testnet-governed-bootstrap.runbook.md` 与当轮 deployment truth 执行。
- 文中带日期的 manifest、bundle、lane TSV、health snapshot 和 current verdict 都是对应证据窗口的历史输入，不是当前节点健康或 release readiness 的持续声明。任何真实执行、对外状态或发布判断前，必须重新采集当轮 bundle、genesis、bootstrap peer、host/service 和 status evidence。
- `producer_system_designer` 维护 checklist/政策边界；`qa_engineer` 收口 release-blocking 与 claims review verdict；`liveops_community` 负责任何外部发布口径。runbook 的 `ready_for_live_candidate` 结果本身不授权部署、发布或对外恢复承诺。

## Meta
- Owner Role: `producer_system_designer`
- Review Roles:
  - `qa_engineer`
  - `liveops_community`
- Scope: formal `public_testnet` 从 skeleton / governed-bootstrap rehearsal 进入 controlled `ready_for_live_candidate` 前的最小执行闭环
- Current Verdict: controlled `ready_for_live_candidate` evidence is present for the current 11-lane packet; not a public launch, mainnet, production OC settlement, or public validator admission claim.

## 1. 适用范围
- 本 runbook 只定义 formal `public_testnet` 的 live-candidate checklist。
- 当前仓库已经具备：
  - `network_tier_manifest` schema / validate / smoke
  - `public_testnet` readiness review 脚本
  - live-candidate lane scaffold
  - governed-bootstrap rehearsal evidence
  - live `public_testnet` public RPC / explorer / guarded faucet / reset evidence
  - formal all-pass 11-lane TSV / readiness verdict
  - controlled `ready_for_live_candidate` evidence for the current packet
- 当前仓库还不具备：
  - live public launch go-ahead
  - public validator admission / onboarding
  - mainnet or production OC settlement authorization
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
- manifest 仍是 `example.invalid`、template ref、`specified_skeleton_only` 占位输出，或当前 active readiness packet 未通过 all-pass controlled live-candidate review。
- `release_candidate_bundle_ref` 不存在或不是当前候选版本真值。
- live-candidate required lane 任一没有 owner 或 evidence ref。
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

## 4. Live-Candidate Lane Checklist
| Lane | Owner | 必须证明什么 | 最小 evidence |
| --- | --- | --- | --- |
| `public_rpc_ready` | `runtime_engineer` | 公网 RPC 已可访问，且不是 placeholder/private-only endpoint | public URL + runtime status / health snapshot |
| `explorer_public_ready` | `liveops_community` | explorer 已可公开访问，且 freshness 不落后到误导外部测试者 | public URL + freshness / landing proof |
| `faucet_guard_ready` | `liveops_community` | faucet 存在且带 guard，不是无限制开放发放 | faucet policy / rate-limit / operator guard evidence |
| `reset_policy_announced` | `producer_system_designer` | 对外已明确这是 resettable `public_testnet`，不承诺 mainnet 价值稳定性 | public reset-policy announcement ref |
| `runtime_bootstrap` | `runtime_engineer` | 候选 bundle、genesis、bootstrap peers 与 runtime bootstrap 路径都可真实启动 | bootstrap rehearsal evidence / startup summary |
| `world_resource_provenance_ready` | `blockchain_ops_engineer` | world resource provenance 与 manifest/world/chain identity 绑定，且不是临时口头说明 | world resource provenance evidence |
| `provider_resource_provenance_ready` | `agent_engineer` | provider resource manifest / delta schema 与当前 world resource contract 兼容 | provider resource provenance evidence |
| `resource_delta_replay_ready` | `runtime_engineer` | resource delta 可从 committed runtime evidence replay，且 provisional resource 不会被当成 live truth | resource delta replay evidence |
| `api_viewer_projection_ready` | `viewer_engineer` | API 与 viewer 在同一时间窗看到同一 world-state projection | JSON evidence with same-window API/viewer projection refs |
| `same_world_hosted_entry_ready` | `viewer_engineer` | hosted-login / launcher / viewer / pure API 指向同一个 formal `public_testnet` world state，且未依赖手工 checkpoint/data copy | JSON evidence with `oasis7.same_world_hosted_entry.v1` |
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
5. 可选执行 external verifier / light-client-lite audit：用 `network-tier-external-verifier-light-client-lite.sh` 从节点进程外验证 sampled `WorldHeadProofV1` artifact，并把 `external_verifier_light_client_lite_ready` 作为 non-promotional optional lane 写入 TSV。若已有连续 proof window，再用 `network-tier-light-client-continuity-window.sh` 验证 trusted anchor、连续高度、prev-hash linkage、observed head 与基础 quorum，并把 `light_client_continuity_window_ready` 作为第二条 non-promotional optional lane 写入 TSV。两条 lane 都只能进入 `ignored_lanes`，不能替代 required lanes，也不能单独升级 `ready_for_live_candidate`。
6. 若要暴露本机 hosted-login 形态入口，先确认本机节点已健康接入 testnet 网络，再确认 hosted-login / launcher / viewer / pure API 读取的是该节点的 testnet world state；节点健康是必要条件，但不是充分条件。
7. 由 `qa_engineer` 审 claims boundary，确认允许/禁止表述。
8. 把全部 required lane 写入正式 TSV，再运行 readiness review 脚本，只有全部 `pass` 才允许进入 `ready_for_live_candidate`。

## 6. Canonical Commands
```bash
./scripts/network-tier-manifest.sh validate \
  --manifest <public-testnet-manifest.json>

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest <public-testnet-manifest.json> \
  --lanes-tsv <public-testnet-lanes.tsv>

./scripts/network-tier-external-verifier-light-client-lite.sh \
  --manifest <public-testnet-manifest.json> \
  --proof <world-head-proof.cbor> \
  --proof-ref <world-head-proof-ref> \
  --world-id <world-id> \
  --expect-height <height> \
  --observed-head-hash <head-hash> \
  --observed-state-root <state-root> \
  --from-height <from-height> \
  --sample-started-at <iso8601> \
  --sample-ended-at <iso8601> \
  --out <external-verifier-evidence.json>

./scripts/network-tier-light-client-continuity-window.sh \
  --manifest <public-testnet-manifest.json> \
  --proof-window <world-head-proof-window.json> \
  --world-id <world-id> \
  --expect-from-height <from-height> \
  --expect-to-height <to-height> \
  --expect-anchor-hash <trusted-anchor-block-hash> \
  --sample-started-at <iso8601> \
  --sample-ended-at <iso8601> \
  --out <light-client-continuity-window-evidence.json>

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

## 7. 当前状态（2026-07-06）
- 历史 `specified_skeleton_only` 只能说明早期 skeleton 状态；当前还必须同时参考候选输入与 governed-bootstrap 证据集：
  - `doc/testing/evidence/public-testnet-live-candidate-manifest-2026-05-22.json`
  - `doc/testing/evidence/public-testnet-live-candidate-lanes-2026-05-22.tsv`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv`
  - `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.md`
  - `doc/testing/evidence/public-testnet-public-surface-freshness-2026-07-03.md`
  - `doc/testing/evidence/public-testnet-governed-reset-policy-announcement-2026-07-03.md`
  - `doc/testing/evidence/public-testnet-faucet-guard-ready-2026-07-05.md`
  - `doc/testing/evidence/public-testnet-runtime-world-resource-closure-2026-07-05.md`
  - `doc/testing/evidence/public-testnet-provider-resource-provenance-2026-07-05.md`
  - `doc/testing/evidence/public-testnet-resource-delta-replay-2026-07-05.md`
  - `doc/testing/evidence/public-testnet-api-viewer-projection-2026-07-05.json`
  - `doc/testing/evidence/public-testnet-same-world-hosted-entry-2026-07-05.json`
  - `doc/testing/evidence/public-testnet-claims-boundary-review-2026-07-06.md`
- 当前 11 条 required lanes 已全部 `pass`：
  - `public_rpc_ready`
  - `explorer_public_ready`
  - `faucet_guard_ready`
  - `reset_policy_announced`
  - `runtime_bootstrap`
  - `world_resource_provenance_ready`
  - `provider_resource_provenance_ready`
  - `resource_delta_replay_ready`
  - `api_viewer_projection_ready`
  - `same_world_hosted_entry_ready`
  - `claims_boundary_review`
- 当前 canonical readiness 命令：
  - `./scripts/network-tier-public-testnet-readiness.sh --manifest doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json --lanes-tsv doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv`
- 2026-07-06 fresh verification output:
  - `missing_required_lanes=[]`
  - `blocking_lanes=[]`
  - `partial_lanes=[]`
  - `manifest_blockers=[]`
  - `gate_result=pass`
  - `readiness_verdict=ready_for_live_candidate`
  - `live_candidate_allowed=true`
- 该结论只允许 controlled / resettable / non-mainnet `public_testnet` live-candidate 口径；它不等于 live public launch、mainnet readiness、production OC settlement、public validator admission/open onboarding、full light-client security 或 multi-client consensus equivalence。
  - governed-bootstrap manifest 仍记录 `status="rehearsal"`；这不阻止当前脚本给出 controlled `ready_for_live_candidate`，但禁止把结论扩写为 public launch、mainnet 或 no-reset release。
  - bundle provenance 仍记录 `git_worktree_dirty=true`；更高 release tier 或对外发布前应重新生成 clean bundle。
  - guarded faucet 只证明 testnet faucet 的 root/health/guarded claim/cooldown/transfer confirmation；不得扩写为无限制 public faucet、durable anti-abuse、TLS/WAF 或 production-grade faucet 运营保证。
  - `chain_proof_evidence_ready` 与 `external_verifier_light_client_lite_ready` 都是 optional / non-promotional evidence lanes；它们可提高 auditability，但不能替代 public RPC、explorer、faucet、same-world hosted entry 等 required lanes
  - 后续若任一 lane 变成 `partial` / `block`，或 evidence 变成 template / placeholder / private-only ref，就必须降回 `block` / `partial`，不得继续沿用本轮 `ready_for_live_candidate` 结论。
- 当前 governed-bootstrap manifest 的 `promotion_policy.required_gates` 已同步到 11 条 active required lanes；`shared_devnet_pass` 只保留为 legacy/rehearsal provenance，不再作为目标 `public_testnet` 的 active required gate。
- 当前 example manifest 仍只能作为 skeleton/template 使用：
  - `network_id=oasis7-public-testnet-example`
  - `chain_id=oasis7-public-testnet-example`
  - `rpc/explorer/faucet = example.invalid`

## 8. 对外口径边界
- 现在允许说：
  - `formal public_testnet mechanism is documented`
  - `current required-lane packet is complete`
  - `all 11 formal public_testnet required lanes have pass evidence`
  - `controlled public_testnet live-candidate claim is allowed by the script-generated readiness review`
  - `the public_testnet remains resettable, non-mainnet, and guarded-faucet bounded`
  - `legacy shared_devnet evidence is not a target test environment`
  - `WorldHeadProofV1 can be externally verified as light-client-lite sampled evidence when the optional verifier lane passes`
- 现在不允许说：
  - `live public testnet is already online`
  - `unrestricted public faucet is open`
  - `public validator admission is open`
  - `mainnet-like OC settlement is available`
  - `full light client security or multi-client consensus equivalence is proven`

## 9. 回写要求
每次正式推进 live candidate checklist，至少回写：
- GitHub task issue evidence comments
- `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.project.md`
- `testing-manual.md`（若 canonical 命令或 claim boundary 变化）
- lane evidence 文档与 TSV

## 10. 收口标准
- 只有当 required-lane TSV 全部为 `pass`，且 evidence 都不是 template / placeholder / private-only ref，`public_testnet` readiness review 才允许输出 `ready_for_live_candidate`。
- 在 public launch、mainnet 或 validator admission 另行通过前，producer / QA / liveops 必须继续维持：
  - `controlled_ready_for_live_candidate_only`
  - `resettable_non_mainnet_public_testnet`
  - `do_not_claim_live_public_testnet`
  - `do_not_claim_public_validator_admission`
  - `do_not_claim_mainnet_or_production_oc_settlement`
