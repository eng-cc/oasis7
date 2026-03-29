# Gameplay Agent 认领代币成本与维护机制（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`

审计轮次: 7

## 任务拆解

- [x] TASK-GAMEPLAY-AGC-001 (`PRD-GAME-011`) [test_tier_required]: `producer_system_designer` 已建立 agent claim 成本专题，冻结“首个也不免费”的规则边界、三段式成本结构、回收条件与 root 文档挂载。
- [x] TASK-GAMEPLAY-AGC-002 (`PRD-GAME-011`) [test_tier_required + test_tier_full]: `runtime_engineer` 已落地 canonical claim 状态机、main token 扣费/锁定/退款/惩罚记账、epoch upkeep 结算与事件审计，并补齐 claim/release/grace/reclaim 的 required 定向回归。
- [x] TASK-GAMEPLAY-AGC-003 (`PRD-GAME-011`) [test_tier_required]: `viewer_engineer` 已把 canonical claim 概览接入 `player_gameplay.agent_claim`，补齐 pure API `--player-gameplay-only` 所需的未认领报价、已认领状态、cooldown / grace / idle reclaim 倒计时与 cap 阻断原因，并在 viewer 选中 agent 详情中落地对应文本。
- [x] TASK-GAMEPLAY-AGC-004 (`PRD-GAME-011`) [test_tier_required + test_tier_full]: `qa_engineer` 已建立 claim 并发、欠费、闲置、cap、refund/slash 与经济审计回归矩阵，并产出 `doc/testing/evidence/game-agent-claim-abuse-matrix-2026-03-27.md`。
- [x] TASK-GAMEPLAY-AGC-005 (`PRD-GAME-011`) [test_tier_required]: `producer_system_designer` 已完成首轮平衡复核，结论为继续维持当前 `slot multiplier / grace_epochs / penalty_bps / tier cap` 默认值，不新开调参专题，待真实 claim 分布与 liveops/QA 信号再触发下一轮。
- [x] TASK-GAMEPLAY-AGC-006 (`PRD-GAME-011`) [test_tier_required]: `producer_system_designer` 已将 `restricted starter claim balance` 写入 PRD / design / project 与 game 根入口，明确“首个 claim 仍非免费、但 slot-1 可由受限 bucket 启动”的新边界。
- [x] TASK-GAMEPLAY-AGC-007 (`PRD-GAME-011`) [test_tier_required + test_tier_full]: `runtime_engineer` 已落地 `restricted starter claim balance` bucket、slot-1 claim/upkeep 专用扣费、bond provenance、refund 拆分、transfer guard、snapshot/replay 兼容字段与定向回归。
- [x] TASK-GAMEPLAY-AGC-008 (`PRD-GAME-011`) [test_tier_required]: `viewer_engineer` 已补齐 restricted/liquid 余额拆分、funding mix、slot-1/slot-2 blocker、pure API canonical 字段、viewer 文案与 explorer 展示口径。
- [x] TASK-GAMEPLAY-AGC-009 (`PRD-GAME-011`) [test_tier_required + test_tier_full]: `qa_engineer` 已建立 restricted starter balance QA 矩阵，并确认 claim/upkeep/refund/transfer guard/viewer parity 为 `pass`；当前 blocker 收敛为 `issuance_reason / issuer_id / expires_at_epoch` grant lifecycle 与对应审计链仍未实现，证据见 `doc/testing/evidence/game-agent-claim-restricted-starter-balance-matrix-2026-03-29.md`。
- [x] TASK-GAMEPLAY-AGC-010 (`PRD-GAME-011`) [test_tier_required]: `producer_system_designer` 已基于 QA `block` 结论完成首轮复核，决定维持 `slot-1 only / non-transferable / provenance-preserving` 边界，不收窄 restricted grant 的 lifecycle / audit 要求，并重新打开后续 runtime / liveops / QA 补齐链路。
- [x] TASK-GAMEPLAY-AGC-011 (`PRD-GAME-011`) [test_tier_required + test_tier_full]: `runtime_engineer` 已落地 restricted grant lifecycle：补齐 `issuance_reason / issuer_id / expires_at_epoch` 持久化状态、issue/expire/revoke canonical 事件、issuer-scoped 发放/回收动作与 main token 源汇审计链路，并将 grant 终态后的 restricted bond refund 重定向回 treasury。
- [x] TASK-GAMEPLAY-AGC-012 (`PRD-GAME-011`) [test_tier_required]: `liveops_community` 已建立 restricted grant 的运营发放口径与回收 runbook，冻结 `allowlist / qa_seed / liveops_campaign` 的 issuer 边界、过期策略、撤销条件与 incident fallback；v1 统一使用 `issuer_id=liveops`，并把 `issuance_reason` 收口到三类允许值。
- [ ] TASK-GAMEPLAY-AGC-013 (`PRD-GAME-011`) [test_tier_required + test_tier_full]: `qa_engineer` 建立 restricted grant lifecycle / audit matrix，验证 issuance metadata、expiry/revoke、source-sink 审计与 transfer non-bypass 全部闭环。

## 依赖

- `doc/game/gameplay/gameplay-engineering-architecture.md`
- `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
- `doc/p2p/token/mainchain-token-allocation-mechanism.prd.md`
- `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
- `testing-manual.md`

## 状态

- 更新日期: 2026-03-29
- 当前状态: in_progress
- 当前 owner: `qa_engineer`
- 下一任务: `TASK-GAMEPLAY-AGC-013`（建立 restricted grant lifecycle / audit matrix，验证 expiry/revoke/source-sink 与 non-bypass）
- 已完成补充:
  - `TASK-GAMEPLAY-AGC-001` 已新增 `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.{prd,design,project}.md`，并将 `PRD-GAME-011` 挂入 game 根 PRD / project / 索引 / README。
  - `TASK-GAMEPLAY-AGC-002` 已在 `crates/oasis7/src/runtime/` 落地 `ClaimAgent / ReleaseAgentClaim` 动作、claim 状态持久化、自动 upkeep/grace/idle reclaim processor 与 main token 账本联动。
  - `TASK-GAMEPLAY-AGC-003` 已在 `crates/oasis7/src/viewer/runtime_live/` 为 `player_gameplay` 增加 canonical `agent_claim` 快照，并在 `crates/oasis7_viewer/src/ui_text_claims.rs` / agent 详情文案中补齐 claim owner、状态、bond/upkeep、release/grace/forced reclaim 倒计时与未认领报价 blocker。
  - `TASK-GAMEPLAY-AGC-004` 已在 `crates/oasis7/src/runtime/tests/agent_claims.rs` 补齐并发单 owner 原子性、tier cap / slot 成本、grace 内恢复、release refund、欠费/闲置 reclaim slash-refund 对账断言，并把结果沉淀到 `doc/testing/evidence/game-agent-claim-abuse-matrix-2026-03-27.md`。
  - `TASK-GAMEPLAY-AGC-005` 已完成首轮 producer review：在当前缺少真实 claim 持有分布与 liveops 信号的前提下，继续维持 `slot multiplier=1.0/1.5/2.0`、`grace_epochs=2`、`forced_reclaim_penalty_bps=2000`、`tier cap=1/2/3`，暂不新开调参专题。
  - `TASK-GAMEPLAY-AGC-006` 已将 `restricted starter claim balance` 正式写入 PRD / design / project，并同步回写 `doc/game/prd.md` 与 `doc/game/project.md`：首个 claim 仍保持非零 canonical 成本，但允许 `slot-1` 使用受限 bucket 启动，不再要求 limited preview / allowlist 账号必须先持有可转账 liquid。
  - `TASK-GAMEPLAY-AGC-007` 已在 `crates/oasis7/src/runtime/` 为主账本增加 `restricted_starter_claim_balance` bucket，并把 claim upfront、epoch upkeep、release refund、forced reclaim refund 全部切到 provenance-aware split：`slot-1` 先花 restricted 再补 liquid，`slot-2/3` 仍只能花 liquid；bond refund 也按 restricted/liquid 来源回写原 bucket，不再洗回 transferable balance。
  - `TASK-GAMEPLAY-AGC-008` 已在 `crates/oasis7/src/viewer/runtime_live/claim_snapshot.rs`、`crates/oasis7/src/simulator/persist.rs`、`crates/oasis7_viewer/src/ui_text_claims.rs`、`crates/oasis7/src/bin/oasis7_chain_runtime/{transfer_submit_api,transfer_submit_explorer_p1_api}.rs` 补齐 restricted/liquid/eligible canonical 字段、funding mix、slot blocker、transfer-only guard 文案与 explorer 显示。
  - `TASK-GAMEPLAY-AGC-009` 已新增 `doc/testing/evidence/game-agent-claim-restricted-starter-balance-matrix-2026-03-29.md`，并通过 runtime/viewer/transfer/explorer 定向用例确认：`slot-1` restricted 消费、mixed funding refund provenance、`slot-2` blocker、restricted-only transfer reject 与 explorer 非转账口径都已闭环；但 restricted grant 的 `issuance_reason / issuer_id / expires_at_epoch` 发放与过期回收仍缺实现，因此 QA 总 verdict 为 `block` 而非 `pass`。
  - `TASK-GAMEPLAY-AGC-010` 已完成首轮 producer review：当前不收窄 `PRD-GAME-011` 范围，也不把 restricted starter balance 扩成更宽用途余额；继续维持 `slot-1` 专用、不可转账、refund 保留 provenance 的边界，并将后续工作拆成 `TASK-GAMEPLAY-AGC-011/012/013` 去补齐 restricted grant 的发放元数据、过期/撤销生命周期与经济审计链。
  - `TASK-GAMEPLAY-AGC-011` 已在 `crates/oasis7/src/runtime/` 增加 `RestrictedStarterClaimGrantState/Status`、issue/revoke 动作与 issued/expired/revoked 事件、`restricted_starter_claim_grants` 持久化 map、epoch 自动过期处理与 issuer-scope 校验；grant 发放从 `MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL` 出账，grant 撤销/过期则把剩余 restricted spendable 退回同一 treasury bucket，形成 source-sink 审计闭环。
  - `TASK-GAMEPLAY-AGC-011` 同步把 claim release / forced reclaim 的 restricted refund sink 显式写入 canonical 事件；当 grant 已 `expired/revoked` 时，后续 restricted bond refund 不再回到 beneficiary restricted bucket，而是定向退回 treasury，避免受限启动金在生命周期终态后被重新激活。
  - `TASK-GAMEPLAY-AGC-012` 已新增 `doc/game/gameplay/gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md`，将 v1 运营口径收口为：统一使用 `issuer_id=liveops`，仅允许 `preview_allowlist / qa_seed / liveops_campaign` 三类 `issuance_reason`，并明确发放前检查、expiry 策略、推荐 `revoke_reason` 与 incident fallback；`qa_seed` 请求仍可由 `qa_engineer` 发起，但正式 issue / revoke 统一由 `liveops_community` 执行。
  - runtime v1 当前实现使用临时 base defaults：`activation fee=100`、`claim bond=200`、`upkeep=25`、`activation burn=50%`，并按 `reputation_score < 10 / >= 10 / >= 25` 映射 `tier-0 / tier-1 / tier-2+`；这些值供当前实现和测试闭环使用，本轮 producer review 结论为先不因 restricted starter balance 额外改价，后续仅在 lifecycle/liveops 真实数据出现异常时再新开调参专题。
  - 本轮 required 验证已覆盖：首个 claim 非免费、重复认领拒绝、release cooldown refund、欠费 grace -> forced reclaim、idle warning -> forced reclaim。
  - 本轮 viewer / API required 验证已覆盖：
    - `env -u RUSTC_WRAPPER cargo check -p oasis7 --lib`
    - `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib viewer::runtime_live::tests::compat_snapshot_exposes_player_agent_claim_overview -- --nocapture`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime preflight_transfer_rejects_restricted_only_balance -- --nocapture`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime explorer_p1_endpoints_return_expected_payloads -- --nocapture`
  - 本轮 QA required/full 验证已覆盖：
    - `env -u RUSTC_WRAPPER cargo check -p oasis7 --lib`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_required runtime::tests::agent_claims:: -- --nocapture`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_full runtime::tests::agent_claims:: -- --nocapture`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib viewer::runtime_live::tests::compat_snapshot_ -- --nocapture`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime preflight_transfer_rejects_restricted_only_balance -- --nocapture`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime transfer_accounts_endpoint_exposes_restricted_balance_separately -- --nocapture`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime explorer_p1_endpoints_return_expected_payloads -- --nocapture`
  - 本轮 liveops required 验证已覆盖：
    - `rg -n "issuer_id = liveops|preview_allowlist|qa_seed|liveops_campaign|revoke_reason|Incident Fallback" doc/game/gameplay/gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- 阻断条件:
  - 若 runtime 无法保证同一 agent 的单 owner 原子性，则 claim 功能不得进入实现态。
  - 若 restricted starter balance 能通过普通转账、slot-2/3 claim 或 explorer 总额误读洗成可转账资产，则不得合入。
  - 若 viewer / pure API 无法给出 canonical claim 成本、funding source 与倒计时，则不得宣称 claim 机制可正式使用。
  - 若经济审计无法覆盖 activation fee、upkeep、refund/slash 与 restricted grant，则不得合入。
  - 当前 runtime grant lifecycle 与 liveops issuer runbook 已闭环，但 QA lifecycle/audit matrix 仍缺正式结论，因此本专题统一 QA verdict 继续保持 `block`，直到 `TASK-GAMEPLAY-AGC-013` 闭环。
- 说明:
  - 本专题是 gameplay 规则与经济边界，不是现实货币付费系统。
  - v1 默认不拍死绝对价格，只先冻结结构、状态机与不可突破的边界。
  - 当前 claim QA 真值已完成，但仓库内仍存在与本专题无关的 `oasis7_hosted_access` bin 编译问题；本轮已通过 `--lib` 定向运行 claim runtime suite，不把该独立缺陷误判为 agent claim blocker。
  - 2026-03-29 reopening 的新增范围是“受限启动资金来源”，不是推翻 claim 成本结构；`activation fee + claim bond + upkeep` 与“首个 claim 非免费”仍保持不变。
