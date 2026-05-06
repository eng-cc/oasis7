# oasis7 主链 Token 到 New API 内部额度桥接方案（项目管理文档）

- 对应设计文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.design.md`
- 对应需求文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] mainchain-token-newapi-quota-bridge-proposal (PRD-P2P-027) [test_tier_required]: 建档 `OC -> New API quota` 专题 PRD / design / project，冻结 one-way bridge、独立部署、唯一入账映射、`bridge_ledger`、credit adapter、manual review fallback 与对外 claim denylist。 Trace: .pm/tasks/task_439da73e233f4395923f4d8cff007305.yaml
- [x] bridge-binding-and-route-contract (PRD-P2P-TBRIDGE-001) [test_tier_required]: 为 bridge-service 落地 repo-owned 持久化、用户绑定、deposit route、唯一 beneficiary 映射与过期策略，补最小 HTTP schema / API 合同、冲突错误与定向测试。 Trace: .pm/tasks/task_e56e4cfdb9534919a6f7bc7c6ba62ee9.yaml
- [ ] bridge-ledger-and-confirmation-engine (PRD-P2P-TBRIDGE-002) [test_tier_required]: 定义 watcher 输入、确认窗口、`bridge_ledger` 状态机、幂等键与重试模型，收口 underpay / overpay / unknown user / duplicate event 的处理策略。 Trace: .pm/tasks/task_439da73e233f4395923f4d8cff007305.yaml
- [ ] newapi-credit-adapter-integration (PRD-P2P-TBRIDGE-003) [test_tier_required]: 在实现前锁定 active `New API` 部署版本的 quota / redeem credit 写入口，定义 adapter credential、幂等契约、回执落盘与 drift check。 Trace: .pm/tasks/task_439da73e233f4395923f4d8cff007305.yaml
- [ ] bridge-operator-risk-runbook (PRD-P2P-TBRIDGE-004) [test_tier_required]: 输出 operator review、close / reconcile、denylist claim、incident escalation 与 pricing rollback runbook。 Trace: .pm/tasks/task_439da73e233f4395923f4d8cff007305.yaml

## 状态
- 当前阶段: `bridge-binding-and-route-contract` 已完成；`bridge_ledger` / watcher / adapter / operator runbook 仍待后续任务实现。
- 当前 owner: `runtime_engineer`
- 当前结论:
  - 只支持 `one-way OC -> New API internal quota/redeem credit`
  - bridge-service 必须独立部署
  - 自动 credit 必须依赖唯一入账映射和 `bridge_ledger` 幂等对账
  - 公开兑换所、自动提现、浏览器热钱包充值都不在当前 allowlist
  - 当前最小实现已提供 `oasis7_newapi_bridge_service`、`/v1/bridge/bind`、`/v1/bridge/deposit-route`、repo-owned `bridge-state.json` 与活跃 route 复用/过期策略，但尚未接入链上 watcher、确认窗口或 `New API` credit adapter。

## 依赖
- `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
- `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
- active `New API` deployment version and admin-side credit contract

## 验证命令
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_newapi_bridge_service -- --nocapture`
- `rg -n "New API|bridge-service|bridge_ledger|one-way|quota|redeem credit|自动提现|公开兑换所" doc/p2p/prd.md doc/p2p/project.md doc/p2p/README.md doc/p2p/prd.index.md doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.design.md doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.project.md`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 风险备注
- 没有成熟公开钱包体系前，不宜承诺大范围匿名自助充值。
- `New API` 版本漂移会直接影响 credit adapter 契约。
- 定价和额度补贴属于 producer / operator 联合治理项，不应在实现当天临时决定。
- 当前 `deposit_account_id` 仍是 bridge-service 的 operator-assigned route 标识；真实链上 watcher / custody / confirmation truth 仍要等 `bridge-ledger-and-confirmation-engine` 才能接上自动 credit。
