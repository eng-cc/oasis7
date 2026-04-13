# oasis7 主链 Token 创世参数正式执行清单（2026-03-22）

审计轮次: 1

## Meta
- Owner Role: `runtime_engineer`
- Review Roles: `producer_system_designer`, `qa_engineer`
- Source Topic: `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
- Freeze Status: `logic_frozen_address_binding_pending`
- Frozen `main_token_config.initial_supply`: `10,000,000,000 OC`
- Runtime Anchor:
  - `Action::InitializeMainTokenGenesis`
  - `MainTokenGenesisAllocationPlan`
  - `MainTokenGenesisAllocationBucketState`
  - recipient `vested_balance`

## 1. Freeze Gates
- Gate-1: `main_token_config.initial_supply` 已由 `producer_system_designer` 冻结为 `10,000,000,000 OC`；后续如需改动，必须新开治理专题重审，不得在执行清单里临时改值。
- Gate-2: 所有 bucket `ratio_bps` 总和必须保持 `10000`。
- Gate-3: 所有 bucket `genesis_liquid` 继续固定为 `0`；创世后只允许通过 `ClaimMainTokenVesting` 转成 liquid。
- Gate-4: 所有 `recipient_account_id` / multisig 地址在真实 mint 前必须从 slot 升级为具体链上账户，否则 QA 结论最多为 `conditional_draft_only`。
- Gate-5: 执行金额必须使用 runtime 真值 rounding 规则，不允许手工改表补差额。

## 2. Allocated Amount Rule
- Step-1: 对每个 bucket 先计算 `floor(initial_supply * ratio_bps / 10000)`。
- Step-2: 将 remainder 按 `ratio_bps` 降序、`bucket_id` 升序逐个 `+1` 分配，直到 remainder 清零。
- Step-3: 最终 `sum(allocated_amount)` 必须严格等于 `initial_supply`。
- Step-4: event 应用后，每个 recipient 对应账户的 `vested_balance` 为其名下所有 bucket 的 `allocated_amount` 聚合值。

## 3. Slot Registry
| slot_id | expected_object | current_value | freeze_requirement | status |
| --- | --- | --- | --- | --- |
| `acct.team_core_vesting.v1` | team core vesting recipient account | `TBD_BEFORE_MINT` | 必须绑定真实 recipient account | `pending_binding` |
| `acct.early_contributor_reward.v1` | reward reserve recipient account | `TBD_BEFORE_MINT` | 必须绑定真实 reward reserve account | `pending_binding` |
| `acct.node_service_custody.v1` | node service custody account | `TBD_BEFORE_MINT` | 必须绑定真实 custody account | `pending_binding` |
| `acct.staking_custody.v1` | staking custody account | `TBD_BEFORE_MINT` | 必须绑定真实 custody account | `pending_binding` |
| `acct.ecosystem_governance.v1` | ecosystem governance account | `TBD_BEFORE_MINT` | 必须绑定真实 governance account | `pending_binding` |
| `acct.security_council_reserve.v1` | security reserve account | `TBD_BEFORE_MINT` | 必须绑定真实 security reserve account | `pending_binding` |
| `acct.foundation_ops.v1` | ops reserve account | `TBD_BEFORE_MINT` | 必须绑定真实 ops reserve account | `pending_binding` |
| `msig.genesis.v1` | genesis controller multisig | `threshold_ed25519=2-of-3`、public-only signer set 已冻结；governance account 仍为 `TBD_BEFORE_MINT` | 必须绑定真实 genesis governance account，并保持 §3A signer set 不变，除非重走 ceremony | `pending_binding` |
| `msig.team_core.v1` | team multisig / beneficiary controller | `threshold_ed25519=2-of-3`、public-only signer set 已冻结；governance account 仍为 `TBD_BEFORE_MINT` | 必须绑定真实 team multisig account，并保持 §3A signer set 不变，除非重走 ceremony | `pending_binding` |
| `msig.reward_reserve.v1` | reward reserve multisig | `threshold_ed25519=2-of-3`、public-only signer set 已冻结；governance account 仍为 `TBD_BEFORE_MINT` | 必须绑定真实 reward reserve governance account，并保持 §3A signer set 不变，除非重走 ceremony | `pending_binding` |
| `msig.node_committee.v1` | node service committee | `threshold_ed25519=2-of-3`、public-only signer set 已冻结；governance account 仍为 `TBD_BEFORE_MINT` | 必须绑定真实 node committee governance account，并保持 §3A signer set 不变，除非重走 ceremony | `pending_binding` |
| `msig.staking_governance.v1` | staking governance multisig | `threshold_ed25519=2-of-3`、public-only signer set 已冻结；governance account 仍为 `TBD_BEFORE_MINT` | 必须绑定真实 staking governance account，并保持 §3A signer set 不变，除非重走 ceremony | `pending_binding` |
| `msig.ecosystem_governance.v1` | ecosystem governance multisig | `threshold_ed25519=2-of-3`、public-only signer set 已冻结；governance account 仍为 `TBD_BEFORE_MINT` | 必须绑定真实 ecosystem governance account，并保持 §3A signer set 不变，除非重走 ceremony | `pending_binding` |
| `msig.security_council.v1` | security council multisig | `threshold_ed25519=2-of-3`、public-only signer set 已冻结；governance account 仍为 `TBD_BEFORE_MINT` | 必须绑定真实 security council governance account，并保持 §3A signer set 不变，除非重走 ceremony | `pending_binding` |
| `msig.foundation_ops.v1` | ops multisig | `threshold_ed25519=2-of-3`、public-only signer set 已冻结；governance account 仍为 `TBD_BEFORE_MINT` | 必须绑定真实 ops governance account，并保持 §3A signer set 不变，除非重走 ceremony | `pending_binding` |

## 3A. Controller Signer Policy Freeze（public-only）
- Freeze Source:
  - operator-local batch id: `oasis7-governance-batch-20260323-01`
  - producer decision: all controller slots use `threshold_ed25519 2-of-3`
  - repo only records public keys and threshold; private keys remain offline/manual-custody material

| controller_slot_id | threshold | allowed_public_keys | freeze_note |
| --- | --- | --- | --- |
| `msig.genesis.v1` | `2` | `6249e5a58278dbc4e629a16b5d33f6b84c39e3ceeb10e963bb9ef64ea4daac30`, `7014e88a6336ec91fc7e6ffb044b50232e4411ec403f90123fa8a202a3420a04`, `f4ecbcb4cbff4acb76cd4bf80fd3b6589a5c7ca2ac2f812380acb7b2cfa4a27c` | `InitializeMainTokenGenesis` controller slot；真实 governance account 仍待绑定 |
| `msig.team_core.v1` | `2` | `af4464d90726b2b95fc1e11b1cb239aeec97fa51a125f256a6958560c18f37d9`, `fe2d7e39d6f9e8e433461dd37e183916e2649ecfcbaf303f965ae212efd7a417`, `ec19e25cbc91ed1227d5111c1ddb1d707ebab2b759ac1788f496026ca9d26826` | team beneficiary/controller slot |
| `msig.reward_reserve.v1` | `2` | `6048b17486ad456dbfbb3edbe0f6aadcaafb56d124e55a54d89395a1d3975005`, `f2bc659515a295e3af4334100873c25739561c299eb1793546180a0d8aa9bde9`, `a690676bd80cff4afc9fcd22b4ea00f63fc4cd2c44aa800a5035aff8c910c063` | reward reserve controller slot |
| `msig.node_committee.v1` | `2` | `76f8cc7ab99d56f3a3be2ae2abbf92a858f63cf9ddb3b2a2cc87d43c1ad8f621`, `bce5d93527c68832fb33ca992447f6ae00106b31de45590d03d9fbb09c1fefae`, `67386d1eff67d3aa960251e0204a558fbc6eafcddccc922b55b30239f798342d` | node service custody controller slot |
| `msig.staking_governance.v1` | `2` | `13c160fc0f516b9a5663aa00c2a5446be6467f68ce341fdd79cdb64224dffd20`, `10fa4d90abf753ec1aa54aee3ea53bab25f43e7078897e1fb6a3777af2255bcb`, `c5c478f1a86b1ecdfa2d09af65f673d2835ee65f35ebd237270295d3773c2ba4` | staking custody controller slot |
| `msig.ecosystem_governance.v1` | `2` | `0241f2e23305407676f2a5cec6d154da74944b2a366b2b2b6913cb746d402d0e`, `960137cd5d675a517daed5f14ea6bea460e196fda4310a581ecd448f3bcd20b4`, `f01a2f8e033d38b369af6bb9a80814a97d749a89ac9d071cc2fdfde1b1010b8a` | ecosystem governance controller slot |
| `msig.security_council.v1` | `2` | `d09de9413371ae42f643e4f8f31e2139611d1617809375b1ad884df3fb089448`, `aa738a832b0d3bf371d231a0bd8502fd411f2a9723246e5d7d215e8fb0ecbb7c`, `f852493e575e33647c005fe2fb43eb15963c73e4213c411e3920f25b498a6980` | security reserve controller slot |
| `msig.foundation_ops.v1` | `2` | `c8f1638d96af35f906b19f980dbc9a58230ede673138718669313331d5e4d753`, `f49f6521a5c1bd48772ff35d6f9f62e1a25b9d5d1be02247f67fe27d3dfd2d4b`, `e4343b9228e33ffe773653b3caf056b28cc56888917deae63dacdbcf20f03516` | foundation ops controller slot |

## 4. Bucket Execution Sheet
| bucket_id | ratio_bps | allocated_amount | recipient_slot_id | recipient_account_id | controller_slot_id | signer_policy | start_epoch | cliff_epochs | linear_unlock_epochs | claim_cadence | runtime_target | freeze_status | notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `team_long_term_vesting` | `2000` | `2,000,000,000 OC` | `acct.team_core_vesting.v1` | `TBD_BEFORE_MINT` | `msig.team_core.v1` | `threshold_ed25519 2-of-3`；public-only signer set 见 §3A；另需团队受益拆分表 | `0` | `365` | `1095` | quarterly | `MainTokenGenesisAllocationPlan -> bucket state -> team recipient vested_balance` | `ready_pending_address_binding` | 个人受益拆分表必须单列并接受 `<=1500 bps` 审计 |
| `early_contributor_reward_reserve` | `1500` | `1,500,000,000 OC` | `acct.early_contributor_reward.v1` | `TBD_BEFORE_MINT` | `msig.reward_reserve.v1` | `threshold_ed25519 2-of-3`；public-only signer set 见 §3A；发放仍需 producer approval | `0` | `0` | `3650` | batch-by-batch only | `MainTokenGenesisAllocationPlan -> bucket state -> reward reserve vested_balance` | `ready_pending_address_binding` | limited preview 期间保持独立 reward reserve，不并入 `ecosystem_pool` |
| `node_service_genesis_custody` | `2000` | `2,000,000,000 OC` | `acct.node_service_custody.v1` | `TBD_BEFORE_MINT` | `msig.node_committee.v1` | `threshold_ed25519 2-of-3`；public-only signer set 见 §3A | `0` | `180` | `1825` | governance batch | `MainTokenGenesisAllocationPlan -> bucket state -> node service custody vested_balance` | `ready_pending_address_binding` | 这是 custody account，不是 `node_service_reward_pool` |
| `staking_genesis_custody` | `1500` | `1,500,000,000 OC` | `acct.staking_custody.v1` | `TBD_BEFORE_MINT` | `msig.staking_governance.v1` | `threshold_ed25519 2-of-3`；public-only signer set 见 §3A | `0` | `180` | `1825` | governance batch | `MainTokenGenesisAllocationPlan -> bucket state -> staking custody vested_balance` | `ready_pending_address_binding` | 这是 custody account，不是 `staking_reward_pool` |
| `ecosystem_governance_reserve` | `1500` | `1,500,000,000 OC` | `acct.ecosystem_governance.v1` | `TBD_BEFORE_MINT` | `msig.ecosystem_governance.v1` | `threshold_ed25519 2-of-3`；public-only signer set 见 §3A | `0` | `90` | `1460` | quarterly | `MainTokenGenesisAllocationPlan -> bucket state -> ecosystem governance vested_balance` | `ready_pending_address_binding` | 不等于公开营销池 |
| `security_reserve_emergency` | `1000` | `1,000,000,000 OC` | `acct.security_council_reserve.v1` | `TBD_BEFORE_MINT` | `msig.security_council.v1` | `threshold_ed25519 2-of-3`；public-only signer set 见 §3A | `0` | `0` | `0` | emergency only | `MainTokenGenesisAllocationPlan -> bucket state -> security reserve vested_balance` | `ready_pending_address_binding` | 常态不 claim，仅事故或防御动作使用 |
| `foundation_ops_reserve` | `500` | `500,000,000 OC` | `acct.foundation_ops.v1` | `TBD_BEFORE_MINT` | `msig.foundation_ops.v1` | `threshold_ed25519 2-of-3`；public-only signer set 见 §3A | `0` | `90` | `730` | monthly or quarterly | `MainTokenGenesisAllocationPlan -> bucket state -> ops reserve vested_balance` | `ready_pending_address_binding` | 运营与基础设施盘 |

## 5. Pre-Mint Checklist
- [x] 冻结 `main_token_config.initial_supply` 为 `10,000,000,000 OC`
- [ ] 绑定全部 `recipient_account_id`
- [ ] 绑定全部 `controller_slot_id` 对应真实 multisig / governance account
- [x] 冻结全部 controller slot 的 `threshold / allowed_public_keys`（public-only；当前统一为 `2-of-3`）
- [ ] 输出创始人个人受益拆分表，并证明任一自然人直接受益 `<=1500 bps`
- [ ] 用本清单 + `token-genesis-allocation-audit-template-2026-03-22.md` 跑 QA 审计
- [ ] 获得最终 QA `pass`
- [ ] 确认执行 payload 使用 runtime rounding 规则，而不是人工 spreadsheet 改写

## 6. Execution Order
1. 冻结 `initial_supply`
2. 绑定 slot registry 到真实链上账户
3. 依据 runtime rounding 规则生成 7 条 `MainTokenGenesisAllocationPlan`
4. 交 QA 填正式 audit template
5. QA `pass` 后再准备 `InitializeMainTokenGenesis`
6. 创世后抽查 recipient `vested_balance` 与 bucket `allocated_amount`

## 7. Not Ready Conditions
- 任何 `recipient_account_id = TBD_BEFORE_MINT`
- 任何 controller multisig 未冻结 signer rule
- 创始人个人受益拆分表缺失
- QA 仍为 `conditional_draft_only` 或 `block`
