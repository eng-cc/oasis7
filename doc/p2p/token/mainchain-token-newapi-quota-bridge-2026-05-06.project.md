# oasis7 主链 Token 到 LetAI Run OpenAPI 额度桥接方案（项目管理文档）

- 对应设计文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.design.md`
- 对应需求文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md`

审计轮次: 2

## 任务拆解（含 PRD-ID 映射）
- [x] mainchain-token-newapi-quota-bridge-proposal (PRD-P2P-027) [test_tier_required]: 建档 `OC -> LetAI Run OpenAPI quota` 专题 PRD / design / project，冻结 one-way bridge、独立部署、用户/项目/`token_key` 真对象模型、`bridge_ledger`、manual review fallback 与对外 claim denylist。 Trace: .pm/tasks/task_439da73e233f4395923f4d8cff007305.yaml
- [x] bridge-binding-and-route-contract (PRD-P2P-TBRIDGE-001) [test_tier_required]: 为 bridge-service 落地 repo-owned 持久化、用户绑定、deposit route、唯一 beneficiary 映射与过期策略，补最小 HTTP schema / API 合同、冲突错误与定向测试。 Trace: .pm/tasks/task_e56e4cfdb9534919a6f7bc7c6ba62ee9.yaml
- [ ] letai-openapi-resource-model-closure (PRD-P2P-TBRIDGE-002) [test_tier_required]: 扩展 bridge 持久化状态，补齐 `platform_user_id`、`platform_project_id`、`token_key`、`external_order_id`、query snapshots 与 pricing->quota 映射。 Trace: .pm/tasks/task_02c8644356c54ef0b035632c651e4ee1.yaml
- [x] letai-openapi-adapter-integration (PRD-P2P-TBRIDGE-003) [test_tier_required]: 以 LetAI Run OpenAPI 替换 generic `credit adapter`，实现 user upsert、项目创建/Token 返回、用户 topup、额度概览与日志验证。 Trace: .pm/tasks/task_02c8644356c54ef0b035632c651e4ee1.yaml
- [ ] letai-reconcile-and-verification-closure (PRD-P2P-TBRIDGE-004) [test_tier_required]: 重写 `oasis7_newapi_bridge_service` reconcile worker，使其按 `confirmed -> user -> project -> token -> topup -> verify -> reconciled/manual_review` 编排，并补齐 retry / verification mismatch 收口。 Trace: .pm/tasks/task_02c8644356c54ef0b035632c651e4ee1.yaml
- [x] letai-bridge-required-tests-and-runbook (PRD-P2P-TBRIDGE-003/004) [test_tier_required]: 重写测试桩与 operator runbook，覆盖首次建用户/项目、已有项目复用、稳定 `external_order_id` 重试、topup 查询验证与 manual review。 Trace: .pm/tasks/task_02c8644356c54ef0b035632c651e4ee1.yaml

## 状态
- 当前阶段: `letai-openapi-full-closure`
- 当前 owner: `runtime_engineer`
- 更新日期: 2026-05-14
- 当前结论:
  - 只支持 `one-way OC -> LetAI Run OpenAPI quota`
  - parent channel / platform key 由 operator 提供
  - 每个用户的 LetAI project 与 `token_key` 动态创建或复用，并持久化为 bridge 真值
  - bridge-service 必须独立部署
  - 自动 topup 必须依赖唯一入账映射和 `bridge_ledger` 幂等对账
  - LetAI topup 成功不能只看 2xx，必须附带 query verification snapshot
  - 公开兑换所、自动提现、浏览器热钱包充值都不在当前 allowlist

## 依赖
- `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
- `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
- operator-provided `--chain-base-url`
- operator-provided LetAI `--letai-base-url`
- operator-provided LetAI `--letai-platform-key`
- operator-provided parent channel metadata

## 验证命令
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_newapi_bridge_service -- --nocapture`
- `rg -n "LetAI|token_key|platform_user_id|platform_project_id|external_order_id|OpenAPI|one-way|自动提现|公开兑换所" doc/p2p/prd.md doc/p2p/project.md doc/p2p/README.md doc/p2p/prd.index.md doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.design.md doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.project.md`
- `rg -n "bridge-service|operator runbook|manual_review|external_order_id|token_key|独立部署|回滚" doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.runbook.md`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 风险备注
- LetAI OpenAPI 文档当前可见字段抽取受飞书展示限制，若项目创建/Token 返回字段更新，adapter 需要跟进。
- `token_key` 是可直接调用模型的凭证，必须避免出现在公共响应和不必要日志里。
- 当前 watcher 只扫描 bridge-service 自己发出的 route，不覆盖全链模糊归属场景。
- 当前 operator review 仍只支持 `mark_resolved|close`；若需要“改额度后重发”或 richer rollback UI，需另开后续任务。

## 活跃补充文档
- `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.runbook.md`
  - 覆盖 operator 输入、推荐启动命令、首次演练、日常巡检、manual review 与回滚边界。
