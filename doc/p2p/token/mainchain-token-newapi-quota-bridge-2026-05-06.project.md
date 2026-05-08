# oasis7 主链 Token 到 New API 内部额度桥接方案（项目管理文档）

- 对应设计文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.design.md`
- 对应需求文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] mainchain-token-newapi-quota-bridge-proposal (PRD-P2P-027) [test_tier_required]: 建档 `OC -> New API quota` 专题 PRD / design / project，冻结 one-way bridge、独立部署、唯一入账映射、`bridge_ledger`、credit adapter、manual review fallback 与对外 claim denylist。 Trace: .pm/tasks/task_439da73e233f4395923f4d8cff007305.yaml
- [x] bridge-binding-and-route-contract (PRD-P2P-TBRIDGE-001) [test_tier_required]: 为 bridge-service 落地 repo-owned 持久化、用户绑定、deposit route、唯一 beneficiary 映射与过期策略，补最小 HTTP schema / API 合同、冲突错误与定向测试。 Trace: .pm/tasks/task_e56e4cfdb9534919a6f7bc7c6ba62ee9.yaml
- [x] bridge-ledger-and-confirmation-engine (PRD-P2P-TBRIDGE-002) [test_tier_required]: 在 `oasis7_newapi_bridge_service` 内补齐持久化 `bridge_ledger`、`next_deposit_seq`、explorer poll watcher、`committed_height - block_height + 1` 确认逻辑、route settle、重复 route 入账去重与 `underpay/overpay/duplicate/expired/topup_plan_auto_credit_not_supported` 异常收口。 Trace: .pm/tasks/task_9f86ccd0c54a45c49d56dc5e84e0809f.yaml
- [x] newapi-credit-adapter-integration (PRD-P2P-TBRIDGE-003) [test_tier_required]: 新增 operator-configured `--credit-adapter-url` / `--credit-adapter-auth-token` / `--credit-target-type` generic HTTP adapter，按稳定 `idempotency_key` 发放 quota / redeem credit，并把失败重试、最大尝试次数与 JSON receipt 回写到 `bridge_ledger`。 Trace: .pm/tasks/task_9f86ccd0c54a45c49d56dc5e84e0809f.yaml
- [x] bridge-operator-risk-runbook (PRD-P2P-TBRIDGE-004) [test_tier_required]: 输出 bridge-service 当前 operator 收口面：`POST /v1/bridge/reconcile`、后台 `--reconcile-interval-seconds`、`POST /v1/bridge/operator/review/{bridge_deposit_id}`、denylist claim 与 config-driven pricing rollback (`--pricing-rule` 回退后重启) 的最小 runbook。 Trace: .pm/tasks/task_9f86ccd0c54a45c49d56dc5e84e0809f.yaml

## 状态
- 当前阶段: `newapi-auto-credit-closure` 已完成，bridge-service 具备最小自动 credit 闭环。
- 当前 owner: `runtime_engineer`
- 更新日期: 2026-05-08
- 当前结论:
  - 只支持 `one-way OC -> New API internal quota/redeem credit`
  - bridge-service 必须独立部署
  - 自动 credit 必须依赖唯一入账映射和 `bridge_ledger` 幂等对账
  - 公开兑换所、自动提现、浏览器热钱包充值都不在当前 allowlist
  - 当前最小实现已提供 `oasis7_newapi_bridge_service` 的 `/v1/bridge/bind`、`/v1/bridge/deposit-route`、`/v1/bridge/reconcile`、`/v1/bridge/operator/review/{bridge_deposit_id}`、repo-owned `bridge-state.json`、持久化 `bridge_ledger`、后台 reconcile loop、explorer poll watcher、exact-match `--pricing-rule` 折算、generic HTTP `New API` credit adapter 与 `manual_review` / retry 状态机。
  - 当前自动 credit 只覆盖 `pricing_version` 命中的 exact amount 充值；`topup_plan_id`、underpay、overpay、duplicate route deposit、expired route deposit 仍会进入 `manual_review`，不做隐式折算。
  - active `New API` deployment path 继续由 operator 通过 `--credit-adapter-url` 注入；repo 内不写死第三方 admin path。

## 依赖
- `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
- `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
- operator-provided `--chain-base-url`
- operator-provided `--credit-adapter-url` and optional bearer token

## 验证命令
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_newapi_bridge_service -- --nocapture`
- `rg -n "New API|bridge-service|bridge_ledger|one-way|quota|redeem credit|自动提现|公开兑换所" doc/p2p/prd.md doc/p2p/project.md doc/p2p/README.md doc/p2p/prd.index.md doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.design.md doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.project.md`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 风险备注
- 没有成熟公开钱包体系前，不宜承诺大范围匿名自助充值。
- `New API` 版本漂移仍会直接影响 credit adapter 契约；当前是通过 config 避免 repo 内写死路径，不等于 drift 风险消失。
- 定价和额度补贴属于 producer / operator 联合治理项，不应在实现当天临时决定。
- 当前 watcher 只扫描 bridge-service 自己发出的 route，不覆盖“用户直接向未分配前缀地址转账”的全链模糊归属场景。
- 当前 operator review 只支持 `mark_resolved|close`；若需要“改额度后重发”或 richer rollback UI，需另开后续任务。
