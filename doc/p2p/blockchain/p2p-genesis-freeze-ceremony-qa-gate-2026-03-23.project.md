# oasis7 创世 freeze / ceremony / QA gate（项目管理文档）

> 历史状态说明：勾选项记录 2026-03-23 的 genesis / MAINNET-4 任务状态，不证明当前创世、mint readiness 或公开口径。当前状态以根 `README.md` 为准，现行结论以对应专业文档和同窗测试证据为准。

- 对应设计文档: `doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.design.md`
- 对应需求文档: `doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.prd.md`

审计轮次: 1
## 任务拆解（含 PRD-ID 映射）
- [x] GENESIS-0 (PRD-P2P-GENESIS-001/002/003/004) [test_tier_required]: 新建创世 freeze/ceremony/QA gate 专题 PRD / design / project，并接入 `doc/p2p` 与 readiness 主追踪。
- [x] GENESIS-1 (PRD-P2P-GENESIS-001/003) [test_tier_required]: 盘点 freeze sheet 当前 `slot registry/bucket execution sheet` 真值，冻结放行与阻断条件。
- [x] GENESIS-2 (PRD-P2P-GENESIS-002/003) [test_tier_required]: 冻结 ceremony checklist 与 evidence 最小字段。
- [x] GENESIS-3 (PRD-P2P-GENESIS-002/004) [test_tier_required + test_tier_full]: 冻结 QA evidence bundle、verdict 模板与 mint-ready claim gate。
- [x] GENESIS-4 (PRD-P2P-GENESIS-004) [test_tier_required]: 将 genesis gate 回写到 readiness/public claims 依赖链。

## 当前结论
- 当前阶段:
  - 游戏阶段口径: `limited playable technical preview`
  - 安全阶段口径: `crypto-hardened preview`
  - `MAINNET-3`: completed as specification gate
- 当前 blocker:
  - freeze sheet 仍为 `logic_frozen_address_binding_pending`
  - slot registry 与 bucket execution sheet 仍有 `TBD_BEFORE_MINT` / `pending_binding` / `ready_pending_address_binding`；虽然 controller slot 的 `threshold / allowed_public_keys` 已冻结为 `2-of-3` public-only snapshot，但真实 recipient/governance account 仍未绑定
  - QA 最终 `pass` 尚未形成

## 依赖
- `doc/p2p/token/mainchain-token-genesis-parameter-freeze-sheet-2026-03-22.md`
- `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md`
- `testing-manual.md`

## 验收命令（本轮）
- `rg -n "logic_frozen_address_binding_pending|TBD_BEFORE_MINT|pending_binding|ready_pending_address_binding|conditional_draft_only|production mint ready" doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.prd.md doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.design.md doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.project.md doc/p2p/token/mainchain-token-genesis-parameter-freeze-sheet-2026-03-22.md doc/p2p/project.md`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前阶段: completed
- 下一步: 进入 `MAINNET-4`，基于 `MAINNET-1~3` 的规格 gate 和当前未完成工程项，冻结最终 public claims policy 与阶段复评结论。
- 最近更新: 2026-03-23
