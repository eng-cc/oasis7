# oasis7 legacy shared-network rehearsal / release-train background（LiveOps Runbook）

- Retained legacy provenance. Current network-tier authority: `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md`.
- Current system-level security gate authority: `doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.prd.md`.

审计轮次: 5

> Current canonical source: use this runbook only for legacy shared-network /
> release-train operations. Current network-tier truth lives in
> GitHub Issue / GitHub Project.

## Meta
- Owner Role: `liveops_community`
- Review Role: `producer_system_designer`
- Scope: `shared_devnet/staging/canary promotion + freeze + rollback + run window + public claims gate`
- Source boundary:
- This retained runbook is historical `shared_devnet/staging/canary` rehearsal/rollback provenance, not an active release or public-testnet authority.
  - `testing-manual.md`

## 1. 适用范围
- 本 runbook 只定义 legacy network-rehearsal / release-train 的执行方法；它不定义 formal `public_testnet`、`mainnet` 或玩家意义上的公开大世界上线。
- 当前 `shared_devnet` legacy rehearsal 已在 2026-05-24 追溯结论中达到 `pass / eligible_for_promotion`；这只证明 historical network-rehearsal evidence 已闭环，不等于 live `public_testnet`、`mainnet`、public launch 或赛季上线。
- 当前总 verdict 已更新为 `shared_devnet legacy rehearsal pass; formal public_testnet/mainnet still gated`。
- 在 formal `public_testnet` live-candidate readiness 与后续 release/public claims gate 通过前，对外只允许：
  - `limited playable technical preview`
  - `crypto-hardened preview`
  - `shared_devnet rehearsal evidence is recorded as pass, but it is legacy/rehearsal evidence only`
  - `formal public_testnet/mainnet readiness remains separately gated`

## 2. 开窗前输入
每次开任何 track 窗口前，必须先固定以下输入：

- 同一份已校验的 `release_candidate_bundle`
- 当前 track 的 QA gate `summary.json/md`
- `fallback_candidate_id` 与 `fallback_class`
  - `formal_pass_candidate`
  - `bootstrap_restore_ready`（仅允许首条 `shared_devnet pass` 使用）
- 窗口元数据：
  - `window_id`
  - `track`
  - `candidate_id`
  - `start_at`
  - `end_at`
  - `owners_on_duty`
  - `claim_envelope`
  - `evidence_root`
- 值班 owner 至少覆盖：
  - `runtime_engineer`
  - `qa_engineer`
  - `liveops_community`

## 3. 硬阻断条件
- `release_candidate_bundle` 缺字段、路径失效或 hash 漂移：立即 `freeze`
- 当前 track QA gate 为 `block`：不得开窗
- 上一轨不是 `pass` 却申请 promotion：直接 `hold`
- `staging/canary` 没有 formal fallback candidate 却申请下一轨：直接 `hold`
- `shared_devnet` 若还在争取首条 `pass` 且没有受审计 `bootstrap_restore_ready` fallback：直接 `hold`
- 共享访问入口、值班 owner、evidence root 未冻结：直接 `hold`
- required mixed-topology lane 仍停留在 baseline / proxy 近似、没有对应 track 的正式结论：直接 `hold`
- mixed-topology lane 试图记 `pass` 但没有 same-window evidence 对账或缺少 producer/QA pass-uplift decision ref：直接 `hold`
- mixed-topology lane 的 `pass_uplift_decision_ref` 只出现在备注、聊天记录或口头结论里，没有进入正式模板/脚本产物：也视为 `hold`
- shared-access lane 试图记 `pass` 但没有 shared endpoint、independent operator handoff 或 access evidence 任一正式字段：直接 `hold`
- rollback lane 试图记 `pass` 但没有 fallback gate、fallback owner、restore steps 或 restoration scope 任一正式字段：直接 `hold`
- 对外口径越过 preview 边界：立即 `freeze`

## 4. 三层执行循环

### 4.1 `shared_devnet`
- 目标：
  - 首次把统一 `candidate_id` 放进多人共享环境
  - 留下 shared access、统一版本、mixed-topology baseline 和 rollback 目标的正式记录
- 开窗前：
  - 固定共享访问入口
  - 固定 `P2PARCH-6` mixed-topology baseline evidence
  - 生成 `promotion_record`
  - 固定 `rollback_target_candidate_id`
  - 若尚无历史 `shared_devnet pass` candidate`，则补齐一条受审计 `bootstrap_restore_ready` fallback：至少包含 `restore_steps_ref`、`fallback_owner_ref`、`restoration_scope`
- 收窗判定：
  - `shared-network-track-gate` 为 `pass` 才可申请进入 `staging`
  - 若 shared access 退化成单 owner 私有访问，最多只能记 `partial`
  - 若 shared-access 想记 `pass`，除 shared endpoint 与 operator handoff 外，还必须固定独立 access evidence；三项缺任一都只能继续 `partial`
  - 若 mixed-topology 仍只有 baseline / proxy 近似，没有 same-window shared 结论，最多只能记 `partial`
  - 若 rollback target 只有 fallback bundle 或“未来再补”的占位，没有受审计 fallback gate / restore steps / owner ref / scope，最多只能记 `partial`
  - 若 mixed-topology 想记 `pass`，除 same-window shared evidence 外还必须固定 producer/QA 联审通过的 pass-uplift decision ref
  - mixed-topology 的 pass-uplift 决议必须进入正式 evidence 字段，而不是只写在 Notes；否则即使 reviewer 同意，也不能把 lane 升到 `pass`
  - 可先用 `shared-devnet-blocker-packet` 生成 `shared_access` / `mixed_topology_baseline` / `rollback_target_ready` 三份 draft，再等待真实窗口证据填充

### 4.2 `staging`
- 目标：
  - 在独立升级窗口里完成 promotion / rollback rehearsal
  - 为 `canary` 准备 incident 模板和恢复证据
- 开窗前：
  - 上一轨 `shared_devnet=pass`
  - 固定 upgrade window
  - 生成新的 `promotion_record`
  - 预填 `incident_template`
  - 固定 same-candidate `mixed_topology_rehearsal` evidence plan
- 收窗判定：
  - 只有 `staging` gate 为 `pass` 才可申请进入 `canary`
  - 任何 required lane 退回 `partial/block` 都先 `hold`

### 4.3 `canary`
- 目标：
  - 在固定小流量观察窗里验证 freeze、incident、exit 决策
- 开窗前：
  - 上一轨 `staging=pass`
  - 固定 `canary_window_start/end`
  - 固定 `freeze_owner`
  - 生成新的 `promotion_record`
  - 固定 mixed-topology claim review 输入与对外口径边界
- 收窗判定：
  - 必须留下 `incident_review`
  - 必须留下 `exit_decision`
  - 必须留下 `mixed_topology_claim_review`
  - 没有这三项，不得记 `canary` 完成

## 5. Freeze / Rollback

### 5.1 何时 `freeze`
- commit/world/governance 真值漂移
- track gate 退回 `block`
- 共享访问失效或值班 owner 断档
- 对外口径越界
- 事故影响未明，继续 promotion 风险更高

### 5.2 `freeze` 时必须立刻做什么
1. 记录 `incident_id`
2. 把当前窗口状态写成 `frozen`
3. 停止新的 promotion 和外部升级表述
4. 写明 `freeze_reason`
5. 指定 `runtime_engineer` 是否执行 `rollback`

### 5.3 何时 `rollback`
- 已有明确 fallback candidate
- 当前 candidate 已不满足 track 最小通过标准
- 需要通过回退恢复连续性

### 5.4 `rollback` 完成条件
- 回退到最近一次 `pass` 的 candidate bundle
- 留下 `rollback_started_at` / `rollback_completed_at`
- 留下恢复后 evidence path
- 结论只能写成 `rolled_back` 或 `restored`

## 6. 模板入口
- `doc/testing/templates/shared-network-promotion-record-template.md`
- `doc/testing/templates/shared-network-incident-template.md`
- `doc/testing/templates/shared-network-incident-review-template.md`
- `doc/testing/templates/shared-network-exit-decision-template.md`
- `doc/testing/templates/shared-network-mixed-topology-gate-template.md`
- `doc/testing/templates/shared-network-shared-access-check-template.md`
- `doc/testing/templates/shared-network-rollback-target-template.md`

## 7. 对外口径执行
- 没有 producer 新批复前，不因单次 shared window 或单次 canary 观察而升级 public claim。
- 公开沟通禁止出现：
  - `production release train is established`
  - `network rehearsal fully validated`
  - `mainnet-grade testing maturity`
  - `public_testnet is live`
  - `public large shared world is launched`
- 外部追问统一回到：
  - `当前仍是 limited playable technical preview。`
  - `安全与治理硬化在推进，但仍是 crypto-hardened preview。`
  - `shared_devnet rehearsal evidence 已补到 pass，但它只作 legacy/rehearsal evidence。`
  - `formal public_testnet / mainnet / public large-world launch 仍需单独 gate。`

## 8. 回写要求
- 每个窗口至少回写一次：
  - GitHub task issue evidence comments
  - 对应 track 的 QA gate `summary.json/md`
  - 当前 topic 的 GitHub Issue / GitHub Project
- 若出现 `freeze` / `rollback` / claim 风险，还必须补：
  - incident 文档
  - owner follow-up
  - 下一步是 `promote`、`hold` 还是 `rollback`

## 9. 当前结论
- 当前 oasis7 已具备：
  - candidate bundle 真值
  - QA gate scaffold
  - liveops promotion/freeze/rollback/run window/public claims runbook
  - first `shared_devnet` dry-run candidate / gate / promotion / incident 产物
  - 2026-05-24 legacy `shared_devnet` pass / eligible-for-promotion 追溯结论
- 当前 `mixed_topology_baseline` 已有正式 pass evidence：
  - `doc/testing/evidence/legacy-shared-devnet-provenance-2026-07-26.md`（保留的 lane-specific record 索引见该 authority）
- legacy network-rehearsal `shared_devnet` verdict 当前是 `pass / eligible_for_promotion`；但该 pass 不升级 public claims，也不等于 `public_testnet` 或正式在线大世界。
- 当前 release-train 剩余边界:
  - `staging/canary` 仍需要按本 runbook 另行开窗验证
  - formal `public_testnet` readiness 由 network-tier six-lane gate 单独判定
  - public launch / season / large shared-world claim 仍受 product, liveops, security, and gameplay gates 约束
