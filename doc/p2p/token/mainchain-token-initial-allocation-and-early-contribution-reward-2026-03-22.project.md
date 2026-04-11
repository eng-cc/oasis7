# oasis7 主链 Token 初始分配与早期贡献奖励口径（项目管理文档）

- 对应设计文档: `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.design.md`
- 对应需求文档: `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] TIGR-0 (PRD-P2P-TOKEN-INIT-001/002/003) [test_tier_required]: 完成 Token 初始分配与早期贡献奖励专题 PRD / design / project 建档，并接入 `doc/p2p` 模块主追踪。
- [x] TIGR-1 (PRD-P2P-TOKEN-INIT-001/002) [test_tier_required]: 由 `runtime_engineer` 输出创世 bucket/account/recipient/vesting 参数表草案，明确当前实现下所有创世 bucket 都先进入 recipient `vested_balance`，并区分 custody account 与 post-genesis treasury bucket 语义。
- [x] TIGR-2 (PRD-P2P-TOKEN-INIT-002/003) [test_tier_required]: 由 `qa_engineer` 建立创世配置审计清单，覆盖 `sum=10000 bps`、单人直持上限、创世液态流通上限、首年外部释放上限与 custody/treasury 语义边界。
- [x] TIGR-3 (PRD-P2P-TOKEN-INIT-003) [test_tier_required]: 由 `liveops_community` 输出 limited preview 早期贡献奖励评分模板、证据字段、奖励建议档位与对外禁语清单，明确该流程不依赖 invite-only、也不公开固定 token 汇率。
- [x] TIGR-4 (PRD-P2P-TOKEN-INIT-002/003) [test_tier_required]: 由 `producer_system_designer` 基于 `TIGR-1~3` 做最终发行前评审，决定 early contributor reserve 在 limited preview 阶段保持多签治理执行，不并入 `ecosystem_pool`，并把“后续是否 fully on-chain”留给未来专题重审。
- [x] TIGR-5 (PRD-P2P-TOKEN-INIT-001/002) [test_tier_required]: 由 `runtime_engineer` 将 `TIGR-1` 草案收成正式执行清单，固定 slot registry、控制主体、签名规则、runtime 落点、amount rounding 规则与 pre-mint freeze gate。
- [ ] TIGR-6 (PRD-P2P-TOKEN-INIT-001/002) [test_tier_required]: 绑定真实 `recipient_account_id` / multisig 地址、补创始人个人受益拆分表，并用 QA 模板输出最终 `pass` 或 `block`。
- [x] TIGR-7 (PRD-P2P-TOKEN-INIT-001/003) [test_tier_required]: 冻结当前链上代币的正式产品名为“绿洲币 / Oasis Coin”，并明确 public naming 与 runtime `main_token.symbol` / ticker 的边界，避免把现有 `AWT` symbol 误写成产品名或误宣称本轮已完成 ticker 改名。

## TIGR-1 产物（本地草案，待 review）
| bucket_id | ratio_bps | recipient | start_epoch | cliff_epochs | linear_unlock_epochs | genesis_liquid | ownership_note |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `team_long_term_vesting` | `2000` | `protocol:team-core-vesting` | `0` | `365` | `1095` | `0` | 团队多签/受益人表内部分配，个人份额不得突破 `1500 bps` 上限 |
| `early_contributor_reward_reserve` | `1500` | `protocol:early-contributor-reward` | `0` | `0` | `3650` | `0` | 贡献奖励多签储备，不等于 marketing airdrop 池 |
| `node_service_genesis_custody` | `2000` | `protocol:node-service-custody` | `0` | `180` | `1825` | `0` | 创世 custody，后续是否并入/补充 treasury 需单独决议 |
| `staking_genesis_custody` | `1500` | `protocol:staking-custody` | `0` | `180` | `1825` | `0` | 创世 custody，和 runtime `staking_reward_pool` 分开 |
| `ecosystem_governance_reserve` | `1500` | `protocol:ecosystem-governance` | `0` | `90` | `1460` | `0` | 生态治理储备 |
| `security_reserve_emergency` | `1000` | `protocol:security-council-reserve` | `0` | `0` | `0` | `0` | 安全委员会应急盘 |
| `foundation_ops_reserve` | `500` | `protocol:foundation-ops` | `0` | `90` | `730` | `0` | 运营与基础设施盘 |

## TIGR-1 验证
- `ratio_bps` 总和为 `10000`
- 项目战略控制总和为 `5000 bps`
- 协议长期储备总和为 `3500 bps`
- 全部 bucket `genesis_liquid=0`
- `recipient` 当前均为 custody account 命名草案，不假装已初始化 treasury bucket

## TIGR-5 正式执行清单结论
- 产物：
  - `doc/p2p/token/mainchain-token-genesis-parameter-freeze-sheet-2026-03-22.md`
- 已冻结内容：
  - 7 个 bucket 的 `recipient_slot_id / controller_slot_id / signer_policy / start_epoch / cliff_epochs / linear_unlock_epochs / claim cadence`
  - runtime 真值落点：`InitializeMainTokenGenesis -> MainTokenGenesisAllocationBucketState -> recipient.vested_balance`
  - `allocated_amount` 计算规则：先 `floor(initial_supply * ratio_bps / 10000)`，再按 `ratio_bps` 降序、`bucket_id` 升序分配 remainder
- 仍待完成：
  - 真实 `recipient_account_id` / multisig 地址绑定
  - 创始人个人受益拆分表
  - QA 最终 `pass`

## TIGR-4 评审结论
- 决策：当前 limited preview 期间，`early_contributor_reward_reserve` 保持 `protocol:early-contributor-reward` 独立 reward multisig 执行路径，不并入 `ecosystem_pool`。
- 理由：
  - 当前 runtime 创世语义仍以 recipient `vested_balance` / custody account 为主，不支持把该储备直接视作已初始化的 treasury bucket。
  - `TIGR-3` 的贡献奖励模板强调 contribution-based review；若现在并入 `ecosystem_pool`，会把贡献奖励和生态 grant 混成一套公开口径。
  - limited preview 阶段优先级是低流通、低承诺、强人工审核，而不是尽快 fully on-chain 化。
- 后续触发条件：
  - 至少形成 1~2 轮真实贡献奖励台账。
  - 审批链、台账与 QA 审计能稳定复用。
  - 再由新专题评估是否迁移到 proposal-bound / treasury-bound 执行路径。

## TIGR-7 命名结论
- 正式产品名：`绿洲币 / Oasis Coin`
- 当前 runtime symbol/ticker：`AWT`
- 边界说明：
  - `绿洲币 / Oasis Coin` 是当前链上代币的 public naming，适用于专题文档、模块入口与后续运营口径。
  - `AWT` 仍仅表示 runtime `main_token.symbol` / ticker 字段真值；本轮没有改动链上字段、API schema、客户端展示值或测试断言。
  - 若后续需要把 `AWT` 一并替换成新的 ticker/symbol，必须另开专题评估 runtime、launcher、viewer、API 与兼容性影响。

## 依赖
- `doc/p2p/token/mainchain-token-allocation-mechanism.prd.md`
- `doc/p2p/token/mainchain-token-allocation-mechanism-phase2-governance-bridge-distribution-2026-02-26.prd.md`
- `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.prd.md`
- `doc/testing/evidence/token-genesis-allocation-audit-template-2026-03-22.md`
- `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`
- `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.md`
- `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.prd.md`
- `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.md`
- `doc/p2p/token/mainchain-token-genesis-parameter-freeze-sheet-2026-03-22.md`
- `doc/p2p/prd.md`
- `doc/game/prd.md`
- `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md`
- `crates/oasis7/src/runtime/main_token.rs`
- `testing-manual.md`

## 状态
- 当前阶段：active
- 下一步：执行 `TIGR-6`，绑定真实 `recipient_account_id` / multisig 地址、补创始人个人受益拆分表，并基于 QA 模板输出最终 `pass/block`。
- 最近更新：2026-04-11
- 备注：`TIGR-1~5` 与 `TIGR-7` 已完成并形成当前发行前冻结结论、正式执行清单与正式命名口径；但真实地址绑定、个人拆分表和 QA 最终 `pass` 仍未完成。在新的治理专题明确前，不得把早期贡献奖励写成公开发币活动，也不得把 reward reserve 自动并入 `ecosystem_pool`。
