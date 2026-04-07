# oasis7 shared network / release train 最小执行形态（2026-03-24）

- 对应设计文档: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.project.md`

审计轮次: 2
## 1. Executive Summary
- Problem Statement: oasis7 现在已经具备 `required/full`、Web-first 闭环、S9/S10 长跑和首轮真实 governance drill 证据，但这些验证大多仍停留在本地或单 owner 控制的 execution world。即便 `P2PARCH-6` 已补出 mixed-topology matrix，shared network / release train 若没有把 mixed-topology 作为显式 required lane，团队仍可能把“本地矩阵通过”误记成“共享环境已准备好 promotion”。
- Proposed Solution: 冻结一份 producer-owned 的 shared network / release train minimum PRD，定义 `shared_devnet -> staging -> canary` 三层最小执行轨道、`release_candidate_bundle` 单一真值、promotion/rollback/freeze 规则和 QA/liveops 证据门禁，并把 mixed-topology baseline / rehearsal / claim review 升级为各轨必经 lane，让 oasis7 把 benchmark 中的 `L5 shared network/release train` 从“仅有缺口描述”升级成正式 execution workstream。
- Success Criteria:
  - SC-1: 明确冻结不少于三层的 shared execution track：`shared_devnet`、`staging`、`canary`，并写清各层目标、输入、owner 和通过标准。
  - SC-2: 明确冻结统一的 `release_candidate_bundle` 字段集合，要求同一候选版本在三个轨道之间可追溯、可回滚、不可口头漂移。
  - SC-3: 明确区分 `complete / partial / blocked` 三种 shared-network 状态，避免把“单机 smoke”误记成 release train。
  - SC-4: 明确当前 public claims 在 shared network 未执行前仍只能维持 `limited playable technical preview` 与 `crypto-hardened preview`。
  - SC-5: 输出 producer 可直接排任务的 project 拆解，至少覆盖 `runtime_engineer`、`qa_engineer`、`liveops_community` 三个角色。
  - SC-6: 明确 shared-devnet / staging / canary 三轨都必须包含 mixed-topology required lane；仅有 matrix baseline 时最多记为 `partial`，不得直接 promotion 或升级 claims。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要知道 shared network 最小完成态是什么，才能决定下一个阶段投入。
  - `runtime_engineer`：需要知道一个候选版本必须带哪些 build/world/governance 真值，才能进入共享轨道。
  - `qa_engineer`：需要知道 shared devnet/staging/canary 各自的 `pass/block/partial` 判定和证据模板。
  - `liveops_community`：需要知道什么时候能做 canary，对外还能说什么，事故时如何 freeze/rollback。
- User Scenarios & Frequency:
  - 每次准备把一个 runtime/world/governance 组合升级成共享候选版本时执行一次。
  - 每次准备从 `shared_devnet` 推到 `staging` 或 `canary` 时执行一次 promotion gate 复核。
  - 每次发生候选版本事故、回滚或外部口径复评时执行一次 freeze/review。
- User Stories:
  - PRD-P2P-RTMIN-001: As a `producer_system_designer`, I want a minimum shared-network model, so that release-train discussions stop relying on vague intuition.
  - PRD-P2P-RTMIN-002: As a `runtime_engineer`, I want one immutable candidate bundle definition, so that shared tracks run the same versioned truth instead of ad hoc local state.
  - PRD-P2P-RTMIN-003: As a `qa_engineer`, I want per-track pass/partial/block rules and evidence expectations, so that shared-network readiness can be audited.
  - PRD-P2P-RTMIN-004: As a `liveops_community`, I want explicit promotion/freeze/rollback rules and claims boundaries, so that external communication stays behind execution truth.
- Critical User Flows:
  1. Flow-P2P-RTMIN-001: `本地 required/full + governance drill + baseline docs 通过 -> 生成 release_candidate_bundle -> 提交 shared_devnet`
  2. Flow-P2P-RTMIN-002: `shared_devnet 连续运行并沉淀证据 -> QA 判定 pass -> producer 批准 promotion -> 进入 staging`
  3. Flow-P2P-RTMIN-003: `staging 完成升级窗口/恢复/回滚演练 -> liveops 批准小流量 canary -> canary 留下 incident-free 或 rollback evidence`
  4. Flow-P2P-RTMIN-004: `任一轨道出现版本漂移/事故/缺证据 -> 立即 freeze promotion -> 回滚到上一通过 bundle -> public claims 保持 preview`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算/判定规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Shared track 定义 | `track_id/purpose/world_scope/access_mode/owner_roles/min_entry_gate` | 冻结 `shared_devnet`、`staging`、`canary` 的用途与入口门禁 | `draft -> frozen` | 至少三层；缺任一层则 release train 仍为 `blocked` | `producer_system_designer` 拍板，`runtime_engineer`/`qa_engineer`/`liveops_community` 联审 |
| Release candidate bundle | `candidate_id/git_commit/runtime_build/world_snapshot_ref/governance_manifest_ref/evidence_refs` | 把单一候选版本推进到共享轨道 | `draft -> promoted -> retired` | 三个轨道必须引用同一 `candidate_id`；字段缺失则不得 promotion | `runtime_engineer` 生成，`qa_engineer` 校验 |
| Promotion gate | `from_track/to_track/gate_inputs/gate_result/approved_by` | 根据证据决定是否升级到下一轨道 | `pending -> pass/block` | 上一轨道未 `pass`、无 rollback bundle、缺 mixed-topology required lane、或 evidence 不全时一律 `block/hold` | `qa_engineer` 给出结论，`producer_system_designer`/`liveops_community` 审批 |
| Freeze / rollback | `incident_id/affected_track/fallback_candidate_id/freeze_reason/recovery_status` | 事故时冻结 promotion 并回滚到前一 bundle | `idle -> frozen -> restored` | 回滚目标必须是最近一次 `pass` 的 candidate；只有“停在当前环境观察”不算恢复 | `liveops_community` 执行，`runtime_engineer` 支持 |
| Claims gate | `claim_phrase/min_track_status/reject_reason` | 根据 shared-network 真值决定对外口径 | `draft -> enforced` | 只要 `shared_devnet/staging/canary` 任一缺失或仅 `partial`，就不得说 release train 已建立 | `liveops_community` 执行，producer 审批 |
- Acceptance Criteria:
  - AC-1: 本专题必须冻结 `shared_devnet`、`staging`、`canary` 三层 shared track 的目标、owner、最小入口门禁和通过标准。
  - AC-2: 本专题必须冻结统一的 `release_candidate_bundle` 字段集合，至少包含 `candidate_id`、`git_commit`、`runtime_build`、`world_snapshot_ref`、`governance_manifest_ref`、`evidence_refs`。
  - AC-3: 本专题必须明确什么情况只能记为 `partial`，至少包括：仅本地单机运行、没有共享访问、没有固定 candidate id、没有 rollback bundle、没有 QA 证据。
  - AC-4: 本专题必须明确：shared network / release train 未完成前，仍不得使用 `production release train is established`、`mainnet-grade testing maturity` 或任何高于当前 preview 的口径。
  - AC-5: `testing-manual.md` 必须能找到本专题入口，并明确它是 benchmark `L5` 的正式 execution 入口，而不是已完成能力。
  - AC-6: `doc/p2p/project.md` 必须建立 `TASK-P2P-040` 任务链，并拆出后续 runtime/QA/liveops 子任务。
  - AC-7: 本专题必须明确 shared-network 当前状态仍是 `specified_not_executed`，不得把建档误写成完成执行。
  - AC-8: 本专题必须给出 first shared-devnet dry run、first staging rehearsal、first canary rehearsal 的顺序与阻断条件。
  - AC-9: 本专题必须明确 mixed-topology 是 `shared_devnet/staging/canary` 的 required lane；`P2PARCH-6` matrix baseline 只能作为 shared-devnet 的起始输入，不能单独构成 shared-network `pass` 或 public claim 升级依据。
- Non-Goals:
  - 不在本专题内直接搭建真实 shared devnet/testnet/canary 环境。
  - 不在本专题内升级 `limited playable technical preview` 或 `crypto-hardened preview` 口径。
  - 不替代 `governance drill`、`genesis ceremony` 或 `fuzz/property` 各自专题。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: shared network / release train minimum 是一条位于本地验证之后、公开口径之前的执行轨道。输入是已通过 `required/full`、S6、S9/S10、governance drill 基线的 `release_candidate_bundle`；中间经过 `shared_devnet -> staging -> canary` 逐级 promotion；输出是 `pass/block/rollback` 证据与 claims decision。它不新增共识协议，而是把版本固定、world 真值、治理 manifest、证据留档和升级窗口管理纳入统一流程。
- Integration Points:
  - `testing-manual.md`
  - `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.design.md`
  - `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.prd.md`
  - `doc/p2p/project.md`
- Edge Cases & Error Handling:
  - 若一个环境只有单 owner 本地访问、没有共享节点或共享运维窗口，则最多记为 `partial_local_only`，不能记为 shared track。
  - 若 `shared_devnet`、`staging`、`canary` 跑的是不同 commit、不同 world snapshot 或不同 governance manifest，则 promotion 直接 `block_version_drift`。
  - 若没有最近一次 `pass` 的 fallback candidate，就不得进入下一轨道；发生事故时只能 `freeze`，不能宣称可回滚。
  - 若 staging 只是 shared_devnet 的别名、没有独立升级窗口/恢复演练/QA 判定，则不得记为 `staging_ready`。
  - 若 canary 没有明确的 duration、incident review 与 freeze 条件，则不得记为 `canary_complete`。
  - 若 governance truth、genesis truth 或 claims boundary 发生变化但未更新 candidate bundle 编号，则该 bundle 失效，必须重新编号。
  - 若 shared-network 证据只包含命令记录，没有 `summary/status/incident` 结论，则 promotion 只能记为 `partial_evidence_missing`。
  - 若 shared-devnet 只引用 `P2PARCH-6` matrix baseline、没有 same-window mixed-topology 结论，则最多记为 `partial_mixed_topology_baseline_only`。
  - 若 shared-devnet 试图把 mixed-topology lane 记为 `pass`，但没有 producer/QA 联审留下的 pass-uplift decision ref，则必须回退到 `partial_missing_pass_decision_ref`。
- Non-Functional Requirements:
  - NFR-P2P-RTMIN-1: 每个 track 的每次 promotion 必须对应唯一 `candidate_id`，且能回链到 `git_commit/runtime_build/world_snapshot_ref/governance_manifest_ref`。
  - NFR-P2P-RTMIN-2: 任一 track 若无共享访问方式、无 owner、无 evidence path、无 rollback target，则不得标记为 `pass`。
  - NFR-P2P-RTMIN-3: `shared_devnet -> staging -> canary` 必须保持严格单向 promotion；不得跳级进入 canary。
  - NFR-P2P-RTMIN-4: 所有 shared-network 结论必须使用 `pass/partial/block/frozen/restored` 这些显式状态，不得使用 `looks_good` 一类口头结论。
  - NFR-P2P-RTMIN-5: 在 `shared_devnet/staging/canary` 三层都有最新审计轮次的正式证据前，公开口径不得出现 `release train established`、`shared network validated` 或更高成熟度描述。
  - NFR-P2P-RTMIN-6: shared-network 证据不得包含私钥、助记词、离线签名材料或 operator 私密基础设施细节。
  - NFR-P2P-RTMIN-7: mixed-topology required lane 必须显式区分 `baseline/rehearsal/claim review` 三种阶段；proxy drill 不得在 runbook 或 claims gate 中冒充 dedicated sentry/NAT lab 真值。
  - NFR-P2P-RTMIN-8: 任何把 shared-devnet mixed-topology lane 提升为 `pass` 的结论，都必须同时固定 same-window evidence ref 与 producer/QA 审计通过的 pass-uplift decision ref，避免仅靠脚本开关改变 gate 语义。
- Security & Privacy: 本专题涉及共享环境与升级轨道，但不引入新的密钥托管方案。任何 candidate bundle、运行记录与证据都只能引用公钥、版本号、world snapshot 标识和审计结论，不得把敏感 custody 材料写入仓库。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 冻结 three-track model、candidate bundle、promotion/freeze/rollback 规则和 claims gate。
  - v1.1: 执行 first shared-devnet dry run，产出正式 QA/evidence 模板。
  - v1.2: 执行 first staging rehearsal 与 rollback drill，补齐 liveops 窗口与 incident 模板。
  - v2.0: 执行 first canary rehearsal，并把 shared-network gate 接入常规 release 评审。
- Technical Risks:
  - 风险-1: 若不冻结 candidate bundle 真值，shared-network 很容易退化成“每层各跑各的版本”。
  - 风险-2: 若没有 rollback/freeze 机制，canary 一旦出事故就只能靠口头操作，不可审计。
  - 风险-3: 若把“共享聊天群里有人能连上”误记为 shared network 完成，会继续高估测试成熟度。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-RTMIN-001 | RTMIN-0/1 | `test_tier_required` | PRD/design/project 建档，three-track model 与 current verdict 冻结 | producer 发布阶段判断 |
| PRD-P2P-RTMIN-002 | RTMIN-1 | `test_tier_required` | candidate bundle schema、version pin 与 drift blocker 设计冻结 | runtime release artifact 管理 |
| PRD-P2P-RTMIN-003 | RTMIN-2/4/5 | `test_tier_required` + `test_tier_full` | QA pass/partial/block 模板、shared-devnet/staging/canary rehearsal 证据 | shared-network 审计与回归 |
| PRD-P2P-RTMIN-004 | RTMIN-3/5 | `test_tier_required` | promotion/freeze/rollback/claims gate 冻结与 liveops runbook | 对外口径与事故响应 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-P2P-RTMIN-001 | 先冻结 `shared_devnet -> staging -> canary` 的最小三层模型 | 继续只说“以后做 devnet/testnet” | 只有明确轨道、owner 和 gate，shared network 才能从概念变成执行面。 |
| DEC-P2P-RTMIN-002 | 采用统一 `release_candidate_bundle` 作为 promotion 单一真值 | 每个轨道各自记录版本 | 多轨道若没有统一 candidate id，就无法审计漂移、回滚和 claims 归因。 |
| DEC-P2P-RTMIN-003 | 在 shared-network 完成前继续维持 preview claims | 因为已有 drill 与 benchmark 就提前升级 testing/public claims | benchmark 已明确 `L5` 仍缺失，不能跨过 shared-network 真实执行。 |
