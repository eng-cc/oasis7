# oasis7 shared network / release train 最小执行形态（LiveOps Runbook）

- 对应需求文档: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
- 对应设计文档: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.project.md`

审计轮次: 4

## Meta
- Owner Role: `liveops_community`
- Review Role: `producer_system_designer`
- Scope: `shared_devnet/staging/canary promotion + freeze + rollback + run window + public claims gate`
- Source Docs:
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md`
  - `testing-manual.md`

## 1. 适用范围
- 本 runbook 只定义 shared-network / release-train 的执行方法；当前虽然已有 first `shared_devnet` dry run，但共享执行仍未达 `pass`。
- 当前总 verdict 已更新为 `partial`。
- 在 `RTMIN-4/5` 真实 rehearsal 留证据前，对外只允许：
  - `limited playable technical preview`
  - `crypto-hardened preview`
  - `first shared_devnet dry run is recorded, but shared execution remains partial`
  - `mixed-topology matrix baseline is pinned, but shared-network mixed-topology evidence is still incomplete`

## 2. 开窗前输入
每次开任何 track 窗口前，必须先固定以下输入：

- 同一份已校验的 `release_candidate_bundle`
- 当前 track 的 QA gate `summary.json/md`
- 最近一次 `pass` 的 `fallback_candidate_id`
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
- 没有 fallback candidate 却申请下一轨：直接 `hold`
- 共享访问入口、值班 owner、evidence root 未冻结：直接 `hold`
- required mixed-topology lane 仍停留在 baseline / proxy 近似、没有对应 track 的正式结论：直接 `hold`
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
- 收窗判定：
  - `shared-network-track-gate` 为 `pass` 才可申请进入 `staging`
  - 若 shared access 退化成单 owner 私有访问，最多只能记 `partial`
  - 若 mixed-topology 仍只有 baseline / proxy 近似，没有 same-window shared 结论，最多只能记 `partial`

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
- `doc/testing/templates/shared-network-shared-access-check-template.md`
- `doc/testing/templates/shared-network-rollback-target-template.md`

## 7. 对外口径执行
- 没有 producer 新批复前，不因单次 shared window 或单次 canary 观察而升级 public claim。
- 公开沟通禁止出现：
  - `production release train is established`
  - `shared network validated`
  - `mainnet-grade testing maturity`
- 外部追问统一回到：
  - `当前仍是 limited playable technical preview。`
  - `安全与治理硬化在推进，但仍是 crypto-hardened preview。`
  - `shared network / release train 已有首轮 shared_devnet dry run，但 shared execution 仍是 partial。`
  - `mixed-topology matrix 已建基线，但 shared-network mixed-topology gate 仍未通过。`

## 8. 回写要求
- 每个窗口至少回写一次：
  - `.pm/tasks/<TASK-UID>.execution.md`
  - 对应 track 的 QA gate `summary.json/md`
  - 当前 topic 的 `project.md`
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
- shared-network 总 verdict 当前是 `partial`，不是 `pass`。
- shared-devnet 剩余 blocker 当前收敛到：
  - `shared_access`
  - `rollback_target_ready`
  - `mixed_topology_baseline`
- 下一步不是升级 public claims，也不是直接进 `staging`，而是先把这三条 lane 提升到 `pass`。
