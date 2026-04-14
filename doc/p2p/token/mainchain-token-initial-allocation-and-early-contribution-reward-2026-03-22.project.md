# oasis7 主链 Token 初始分配与早期贡献奖励口径（项目管理文档）

- 对应设计文档: `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.design.md`
- 对应需求文档: `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`

审计轮次: 6
## 任务拆解（含 PRD-ID 映射）
- [x] TIGR-0 (PRD-P2P-TOKEN-INIT-001/002/003) [test_tier_required]: 完成 Token 初始分配与早期贡献奖励专题 PRD / design / project 建档，并接入 `doc/p2p` 模块主追踪。
- [x] TIGR-1 (PRD-P2P-TOKEN-INIT-001/002) [test_tier_required]: 由 `runtime_engineer` 输出创世 bucket/account/recipient/vesting 参数表草案，明确当前实现下所有创世 bucket 都先进入 recipient `vested_balance`，并区分 custody account 与 post-genesis treasury bucket 语义。
- [x] TIGR-2 (PRD-P2P-TOKEN-INIT-002/003) [test_tier_required]: 由 `qa_engineer` 建立创世配置审计清单，覆盖 `sum=10000 bps`、单人直持上限、创世液态流通上限、首年外部释放上限与 custody/treasury 语义边界。
- [x] TIGR-3 (PRD-P2P-TOKEN-INIT-003) [test_tier_required]: 由 `liveops_community` 输出 limited preview 早期贡献奖励评分模板、证据字段、奖励建议档位与对外禁语清单，明确该流程不依赖 invite-only、也不公开固定 token 汇率。
- [x] TIGR-4 (PRD-P2P-TOKEN-INIT-002/003) [test_tier_required]: 由 `producer_system_designer` 基于 `TIGR-1~3` 做最终发行前评审，决定 early contributor reserve 在 limited preview 阶段保持多签治理执行，不并入 `ecosystem_pool`，并把“后续是否 fully on-chain”留给未来专题重审。
- [x] TIGR-5 (PRD-P2P-TOKEN-INIT-001/002) [test_tier_required]: 由 `runtime_engineer` 将 `TIGR-1` 草案收成正式执行清单，固定 slot registry、控制主体、签名规则、runtime 落点、amount rounding 规则与 pre-mint freeze gate。
- [ ] TIGR-6 (PRD-P2P-TOKEN-INIT-001/002) [test_tier_required]: 绑定真实 `recipient_account_id` / multisig 地址、补创始人个人受益拆分表，并用 QA 模板输出最终 `pass` 或 `block`。
- [x] TIGR-7 (PRD-P2P-TOKEN-INIT-001/003) [test_tier_required]: 冻结当前链上代币的正式产品名为“绿洲币 / Oasis Coin”，并明确 public naming 与 runtime `main_token.symbol` / ticker 的边界，避免把当时旧 runtime symbol 误写成产品名或误宣称本轮已完成 ticker 改名。
- [x] TIGR-8 (PRD-P2P-TOKEN-INIT-001/003) [test_tier_required]: 将当前链上代币的 runtime symbol、公钥派生账户前缀与签名鉴权前缀统一迁移到 `OC` / `oc:pk:`，并同步 API、viewer/client、liveops、脚本、测试与专题文档，清理 `AWT` / `awt:pk:` 的现行真值残留。
- [x] TIGR-9 (PRD-P2P-TOKEN-INIT-001/002) [test_tier_required]: 由 `producer_system_designer` 冻结当前 `main_token_config.initial_supply = 10,000,000,000 OC`，并把 7 个 bucket 的绝对分配额、首年外部释放绝对边界与 formal freeze sheet 的 supply gate 回写到专题文档。
- [x] TIGR-10 (PRD-P2P-TOKEN-INIT-002) [test_tier_required]: 由 `runtime_engineer` 将 `10,000,000,000 OC` frozen supply 接到 chain execution world 的 fresh-init 真值，并确保 execution driver 只在 `ReleaseDefault` 下继承该 production freeze；真实 7 bucket `recipient_account_id` / multisig 绑定仍留在 `TIGR-6` 完成前，不对外暴露可直接落盘的 production helper，也不扩散到 generic `production_hardened` world。

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
- 备注：`TIGR-7` 仅冻结 public naming，本身不直接改写 runtime symbol/account 真值；当前实现迁移见下方 `TIGR-8`。
- 边界说明：
  - `绿洲币 / Oasis Coin` 是当前链上代币的 public naming，适用于专题文档、模块入口与后续运营口径。
  - `TIGR-7` 的目标是先把 public naming 固定下来，再由后续专题统一迁移 runtime/account 当前真值。

## TIGR-8 迁移结论
- 正式产品名：`绿洲币 / Oasis Coin`
- 当前 runtime symbol/ticker：`OC`
- 当前公钥派生账户前缀：`oc:pk:`
- 边界说明：
  - `绿洲币 / Oasis Coin` 是当前链上代币的 public naming，现已和 runtime/account 当前真值统一。
  - `OC` 是当前 `main_token.symbol` / ticker 真值，`oc:pk:` 是当前公钥派生账户前缀，签名鉴权前缀也已同步切换到 `oc*auth:v1:`。
  - `AWT` / `awt:pk:` 不再是当前实现真值；若未来需要再次调整 symbol/account 前缀，必须另开专题评估 runtime、launcher、viewer、API 与兼容性影响。

## TIGR-9 创世绝对发行量冻结结论
- 当前冻结值：`main_token_config.initial_supply = 10,000,000,000 OC`
- 首 12 个月非团队外部释放边界：
  - 目标：`100,000,000~200,000,000 OC`
  - 硬上限：`500,000,000 OC`
- 7 个 bucket 的绝对分配额：
| bucket_id | ratio_bps | allocated_amount_at_10b |
| --- | --- | --- |
| `team_long_term_vesting` | `2000` | `2,000,000,000 OC` |
| `early_contributor_reward_reserve` | `1500` | `1,500,000,000 OC` |
| `node_service_genesis_custody` | `2000` | `2,000,000,000 OC` |
| `staking_genesis_custody` | `1500` | `1,500,000,000 OC` |
| `ecosystem_governance_reserve` | `1500` | `1,500,000,000 OC` |
| `security_reserve_emergency` | `1000` | `1,000,000,000 OC` |
| `foundation_ops_reserve` | `500` | `500,000,000 OC` |
- runtime 执行含义：
  - `allocated_amount` 仍按 runtime 真值公式计算。
  - 当前 `10,000,000,000 OC` 口径下 7 个 bucket 都能整除，`remainder = 0`，不需要额外补差额。
  - `genesis_liquid=0`、独立 reward reserve、多签审批与低流通边界不变；本次只冻结绝对总量，不提前宣称 mint-ready。

## TIGR-10 runtime fresh-init 落地结论
- chain execution world fresh-init 现在会在 `ReleaseDefault` 路径下携带 frozen `main_token_config.initial_supply = 10,000,000,000 OC`，不再沿用 generic default 的 `0`。
- `NodeRuntimeExecutionDriver::new_with_storage_profile()` 现在会按 `storage_profile.profile` 直接选择 fresh world policy：
  - `ReleaseDefault` fresh world 继续带 production-hardened policy 与 frozen `10,000,000,000 OC`
  - `DevLocal` / 其他非 release fresh world 保持 generic `main_token_config.initial_supply = 0`
- generic `RuntimeWorld::new_production_hardened()` 不再自动携带 10B supply，避免把创世 freeze 扩散到 viewer bootstrap、reward runtime worker、governance import 之类并非 chain execution world 的生产硬化场景。
- 若同一个 execution world 目录曾在 `ReleaseDefault` 下生成过 pristine frozen config，再以 `DevLocal` 重新打开时，execution driver 会把这类“未初始化主链账本但残留 production freeze”的状态清回 generic `0`，避免 profile switch 后继续误带 10B。
- 7 bucket freeze sheet 仍保持文档真值，但在 `TIGR-6` 完成真实 `recipient_account_id` / multisig 绑定前，runtime 不再对外暴露可直接初始化创世的 production helper。
- 边界保持不变：
  - fresh-init 真值已落 runtime config，但并未跳过 `InitializeMainTokenGenesis`。
  - `recipient_account_id` / multisig 真实地址仍待 `TIGR-6` 绑定；当前 freeze sheet 里的 logical custody account 命名仍只作为专题文档口径，不作为 production runtime helper 对外发布。
  - 因此本次是“配置与创世参数真值补齐”，不是“已经 mint-ready”。

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
- 最近更新：2026-04-13
- 备注：`TIGR-1~5`、`TIGR-7~9` 已完成并形成当前发行前冻结结论、正式执行清单、`10,000,000,000 OC` 创世绝对发行量与当前 runtime/account 真值；但真实地址绑定、个人拆分表和 QA 最终 `pass` 仍未完成。在新的治理专题明确前，不得把早期贡献奖励写成公开发币活动，也不得把 reward reserve 自动并入 `ecosystem_pool`。
