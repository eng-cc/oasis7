# game PRD Project

审计轮次: 17

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-GAME-001 (PRD-GAME-001) [test_tier_required]: 完成 game PRD 改写，建立玩法设计总入口。
- [x] TASK-GAME-002 (PRD-GAME-001/002) [test_tier_required]: 补齐核心玩法循环（新手/经济/战争）验收矩阵。
- [x] TASK-GAME-003 (PRD-GAME-002/003) [test_tier_required]: 建立可玩性问题分级与修复闭环模板。
- [x] TASK-GAME-004 (PRD-GAME-003) [test_tier_required]: 对接发布前可玩性门禁与回归节奏。
- [x] TASK-GAME-005 (PRD-GAME-001/002/003) [test_tier_required]: 对齐 strict PRD schema，补齐关键流程/规格矩阵/边界异常/NFR/验证与决策记录。
- [x] TASK-GAME-006 (PRD-GAME-004) [test_tier_required]: 新增微循环反馈可见性 PRD 与项目文档，完成文档树挂载。
- [x] TASK-GAME-007 (PRD-GAME-004) [test_tier_required]: 落地 runtime 协议与 viewer 反馈闭环并完成回归验证（子任务 `TASK-GAMEPLAY-MLF-001/002/003/004` 已全部完成，见 `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.project.md`）。
- [x] TASK-GAME-008 (PRD-GAME-005) [test_tier_required]: 新增“分布式执行共识/治理共识/身份与反女巫”专题 PRD 与项目管理文档，完成根文档追踪映射。
- [x] TASK-GAME-009 (PRD-GAME-005) [test_tier_required]: 落地 tick 证书链与 `state_root/events_hash` 一致性校验实现（含 replay/save-load 闭环）。
- [x] TASK-GAME-010 (PRD-GAME-005) [test_tier_required]: 落地治理 `timelock + epoch` 生效门禁与紧急刹车/否决约束。
- [x] TASK-GAME-011 (PRD-GAME-005) [test_tier_required + test_tier_full]: 落地身份信誉/抵押权重、女巫检测与惩罚申诉闭环。
- [x] TASK-GAME-012 (PRD-GAME-006) [test_tier_required]: 新增长期在线 P0 生产硬化专题 PRD 与项目管理文档，完成根文档追踪映射。
- [x] TASK-GAME-013 (PRD-GAME-006) [test_tier_required]: 落地状态权威分层（传播层/裁决层）与冲突仲裁拒绝路径。
- [x] TASK-GAME-014 (PRD-GAME-006) [test_tier_required + test_tier_full]: 补齐确定性回放 + 快照回滚 runbook 与演练门禁。
- [x] TASK-GAME-015 (PRD-GAME-006) [test_tier_required + test_tier_full]: 落地反作弊/反女巫对抗检测、惩罚与申诉证据链强化。
- [x] TASK-GAME-016 (PRD-GAME-006) [test_tier_required]: 建立经济源汇审计与通胀/套利告警阈值门禁。
- [x] TASK-GAME-017 (PRD-GAME-006) [test_tier_required]: 补齐可运维性能力（SLO、告警、灰度、灾备演练）与发布阻断规则。
- [x] TASK-GAME-018 (PRD-GAME-004) [test_tier_required]: 执行微循环可玩性视觉优化二期（控制结果显著化、玩家模式减负、世界可读性增强）并以手动截图闭环验收（见 `TASK-GAMEPLAY-MLF-005/006/007/008`）。
- [x] TASK-GAME-019 (PRD-GAME-001) [test_tier_required]: 同步 `doc/game/README.md` 与 `doc/game/prd.index.md` 的模块入口索引，补齐近期 gameplay 专题与根目录收口口径。
  - 收口证据:
    - `doc/game/gameplay/gameplay-micro-loop-visual-closure-evidence-2026-03-10-round009.md`
    - `doc/playability_test_result/card_2026_03_10_23_27_43.md`
    - `doc/game/gameplay/gameplay-visual-evidence-linkage-2026-03-10.md`
  - QA 结论:
    - `TASK-GAMEPLAY-MLF-005/006/007/008` 已全部完成，当前未见高优先级阻断；更长录屏留给后续 release gate 抽样继续观察。
- [x] TASK-GAME-020 (PRD-GAME-001/002) [test_tier_required]: 冻结前期工业引导闭环（首个制成品/工厂），并拆出 runtime / viewer / QA 落地任务与验收指标。
- [x] issue-162-industrial-chain-legibility-closeout (PRD-GAME-012) [test_tier_required]: `producer_system_designer` 已将 `#162` 的现行 closeout trace 收口到正式 evidence，明确当前仓库已具备工业链状态、停机分类、恢复提示与首个工业里程碑的玩家可读反馈；同时保留 active-LLM `trust gate = hold`、`first capability gate = not_run` 作为独立 blocker，不混写为同一 issue。 Trace: .pm/tasks/task_4da3948c1c2c457c9529ee661e4af03d.yaml
- [x] issue-160-first-capability-closeout (PRD-GAME-012) [test_tier_required]: `runtime_engineer` 已把 active-LLM formal lane 的 capability blocker 从 `post_onboarding.establish_first_capability / 20%` 长停收口到 fresh pass evidence：`environment.current_observation` 现已显式暴露 build-ready context，`900s` formal sample 已推进到 `post_onboarding.choose_first_expansion_tradeoff / 92%`，并给出 `trustGateResult=pass`、`firstCapabilityResult=pass`。 Trace: .pm/tasks/task_4261de9e42ac422c9ecc63525740fbb9.yaml
- [x] TASK-GAME-021 (PRD-GAME-007) [test_tier_required]: 新增 `PostOnboarding` 阶段目标链专题 PRD / design / project，并完成 `game` 根 PRD、顶层设计主文档、索引与 devlog 挂载。
- [x] TASK-GAME-022 (PRD-GAME-007) [test_tier_required]: 为 `#46 PostOnboarding` 补齐无 UI live-protocol smoke、测试手册入口与协议证据回写，明确 headed Web/UI 与 no-UI 验证边界。
- [x] TASK-GAME-023 (PRD-GAME-008) [test_tier_required]: 新增“纯 API 客户端等价”专题 PRD / design / project，并完成 `game` 根 PRD、顶层设计主文档、索引与 devlog 挂载。
- [x] TASK-GAME-024 (PRD-GAME-001/008) [test_tier_required]: 收口 `game` 根 PRD / project 中当前真值 `cargo -p` 命令与纯 API 客户端源码路径的 `oasis7` 品牌。
  - 产物文件:
    - `doc/game/prd.md`
    - `doc/game/project.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "cargo test -p oasis7|cargo test -p oasis7_viewer|crates/oasis7/src/bin/oasis7_pure_api_client.rs" doc/game/prd.md doc/game/project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-GAME-025 (PRD-GAME-001/004/006) [test_tier_required]: 收口 `gameplay` 专题中当前真值实现锚点与 `cargo -p` 命令的 `oasis7` 品牌。
  - 产物文件:
    - `doc/game/gameplay/gameplay-war-politics-mvp-baseline.md`
    - `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
    - `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.project.md`
    - `doc/game/project.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "cargo test -p oasis7|cargo test -p oasis7_viewer|crates/oasis7|crates/oasis7_builtin_wasm_modules" doc/game/gameplay/gameplay-war-politics-mvp-baseline.md doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-GAME-026 (PRD-GAME-001/004/005/007/008) [test_tier_required]: 收口其余活跃 `gameplay` 专题中的当前源码锚点与 `cargo -p` 命令，统一使用 `oasis7` / `oasis7_viewer` / `oasis7_proto` 口径。
  - 产物文件:
    - `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
    - `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
    - `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.project.md`
    - `doc/game/gameplay/gameplay-release-production-closure.project.md`
    - `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md`
    - `doc/game/gameplay/gameplay-top-level-design.project.md`
    - `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06.prd.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "cargo test -p oasis7|crates/oasis7|crates/oasis7_viewer|crates/oasis7_proto" doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.project.md doc/game/gameplay/gameplay-release-production-closure.project.md doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md doc/game/gameplay/gameplay-top-level-design.project.md doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06.prd.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-GAME-027 (PRD-GAME-001/002/005) [test_tier_required]: 收口更早期 `gameplay` 活跃专题中遗漏的 runtime crate 路径与 `cargo -p` 命令，统一到 `oasis7` / `crates/oasis7*`。
  - 产物文件:
    - `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.prd.md`
    - `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.project.md`
    - `doc/game/gameplay/gameplay-runtime-governance-closure.project.md`
    - `doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.project.md`
    - `doc/game/project.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "cargo test -p oasis7|cargo check -p oasis7|crates/oasis7/src/runtime" doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.prd.md doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.project.md doc/game/gameplay/gameplay-runtime-governance-closure.project.md doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-GAME-028 (PRD-GAME-009) [test_tier_required]: 新增“封闭 Beta 准入门禁”专题 PRD / design / project，并完成 `game` 根 PRD、`gameplay-top-level-design` 主文档、索引、handoff 与 devlog 挂载。
  - 产物文件:
    - `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
    - `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.design.md`
    - `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.project.md`
    - `doc/game/prd.md`
    - `doc/game/project.md`
    - `doc/game/prd.index.md`
    - `doc/game/README.md`
    - `doc/game/gameplay/gameplay-top-level-design.project.md`
    - `doc/game/gameplay/producer-to-runtime-task-game-029-closed-beta-runtime-evidence-2026-03-21.md`
    - `doc/game/gameplay/producer-to-viewer-task-game-030-closed-beta-first-screen-2026-03-21.md`
    - `doc/game/gameplay/producer-to-qa-task-game-031-closed-beta-unified-gate-2026-03-21.md`
    - `doc/game/gameplay/producer-to-liveops-task-game-032-closed-beta-candidate-runbook-2026-03-21.md`
    - `doc/devlog/2026-03-21.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "PRD-GAME-009|internal_playable_alpha_late|closed_beta_candidate" doc/game/prd.md doc/game/project.md doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-GAME-029 (PRD-GAME-009) [test_tier_required + test_tier_full]: `runtime_engineer` 已收口 five-node no-LLM soak、replay/rollback drill 与 longrun release gate 的候选版本证据，形成封闭 Beta 准入所需的 runtime 最小硬证据包。
- [x] TASK-GAME-030 (PRD-GAME-009) [test_tier_required]: `viewer_engineer` 已完成同候选 headed Web/UI rerun、`AgentNotFound` 历史噪音降级与首屏人工复核，`PostOnboarding` 主目标/进度/下一步建议现在可作为封闭 Beta 候选级首屏入口。
- [x] TASK-GAME-031 (PRD-GAME-009) [test_tier_required + test_tier_full]: `qa_engineer` 已建立统一 `closed_beta_candidate` release gate，串联 headed Web/UI、pure API、no-UI smoke、longrun/recovery 与 trend baseline；最近 7 天 trend baseline 刷新后，当前统一 gate 正式结论为 `pass`。
- [x] TASK-GAME-032 (PRD-GAME-009) [test_tier_required]: `liveops_community` 收口封闭 Beta 候选 runbook、招募/反馈/事故回流模板与禁语清单；在 `producer_system_designer` 放行前继续保持 `technical preview` 口径。
  - 产物文件:
    - `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.prd.md`
    - `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.design.md`
    - `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.project.md`
    - `doc/playability_test_result/templates/closed-beta-candidate-incident-templates-2026-03-22.md`
    - `doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md`
    - `doc/readme/prd.md`
    - `doc/readme/project.md`
    - `doc/readme/prd.index.md`
    - `doc/readme/README.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "closed beta candidate|technical preview|incident template|candidate-feedback" doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.prd.md doc/playability_test_result/templates/closed-beta-candidate-incident-templates-2026-03-22.md doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-GAME-033 (PRD-GAME-009) [test_tier_required]: `producer_system_designer` 已基于 `TASK-GAME-029/030/031/032` 的统一证据完成阶段评审，决定当前继续保持 `internal_playable_alpha_late`；理由是 unified gate `pass` 只证明技术门收口，而当前 claim envelope 与 liveops 节奏仍继续维持 `limited playable technical preview`，暂不切换到 `closed_beta_candidate` 口径。
- [x] TASK-GAME-034 (PRD-GAME-009) [test_tier_required]: 基于最新制作人口径决策，将当前对外 claim envelope 从 `technical preview / not playable yet` 收口为 `limited playable technical preview`，并同步更新 game/readme/liveops 当前真值文档，不改变阶段判断与禁语边界。
- [x] TASK-GAME-035 (PRD-GAME-010) [test_tier_required]: 新增“受控 limited preview 执行”专题 PRD / design / project，并完成 game 根 PRD、项目文档、索引、handoff 与 devlog 挂载。
  - 产物文件:
    - `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md`
    - `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.design.md`
    - `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.project.md`
    - `doc/game/gameplay/producer-to-liveops-task-game-036-limited-preview-execution-2026-03-22.md`
    - `doc/game/gameplay/producer-to-qa-task-game-037-limited-preview-gate-watch-2026-03-22.md`
    - `doc/game/prd.md`
    - `doc/game/project.md`
    - `doc/game/prd.index.md`
    - `doc/game/README.md`
    - `doc/devlog/2026-03-22.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "PRD-GAME-010|limited playable technical preview|continue / hold / reassess|claim drift" doc/game/prd.md doc/game/project.md doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.project.md doc/game/gameplay/producer-to-liveops-task-game-036-limited-preview-execution-2026-03-22.md doc/game/gameplay/producer-to-qa-task-game-037-limited-preview-gate-watch-2026-03-22.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [ ] TASK-GAME-036 (PRD-GAME-010) [test_tier_required]: `liveops_community` 执行 1 轮 controlled builder-facing callout，按固定巡检窗口归档 `Blocking / Opportunity / Idea` 信号，并在出现 claim drift 时当轮纠偏；`2026-03-27` 首次 Moltbook 尝试因 `ERR_CONNECTION_CLOSED` 被记为渠道 incident，随后 round-1 primary channel 已切到 GitHub issue `eng-cc/oasis7#48` 并完成发布，当前待首批有效 signal 回流。
- [ ] TASK-GAME-037 (PRD-GAME-010) [test_tier_required]: `qa_engineer` 输出 `QA Weekly / Event Verdict`，确认当前 unified gate 是否仍保持 `pass`，并在真实反馈触发时建议 `continue / conditional go / no-go`。
- [ ] TASK-GAME-038 (PRD-GAME-010) [test_tier_required]: `producer_system_designer` 基于 `TASK-GAME-036/037` 的真实执行样本，正式决定继续维持、收紧节奏，或触发下一轮阶段评审。
- [x] TASK-GAME-039 (PRD-GAME-011) [test_tier_required]: 新增“agent 认领代币成本与维护机制”专题 PRD / design / project，并完成 game 根 PRD、project、索引、README 与 devlog 挂载。
- [x] TASK-GAME-040 (PRD-GAME-011) [test_tier_required + test_tier_full]: `runtime_engineer` 已落地 agent claim canonical 状态机、claim bond/upkeep/reclaim 记账、单 owner 原子性与审计事件，并补齐 required 定向回归。
- [x] TASK-GAME-041 (PRD-GAME-011) [test_tier_required]: `viewer_engineer` 已落地 claim quote、cooldown/grace/idle reclaim 倒计时、tier cap 阻断原因与 pure API 字段对齐。
- [x] TASK-GAME-042 (PRD-GAME-011) [test_tier_required + test_tier_full]: `qa_engineer` 已验证 claim 并发、欠费、闲置回收、refund/slash 和经济源汇审计没有旁路。
- [x] TASK-GAME-043 (PRD-GAME-011) [test_tier_required]: `producer_system_designer` 已基于首轮 balance 样本复核 claim 成本曲线与 tier cap，决定当前不新开调参专题。
- [x] TASK-GAME-044 (PRD-GAME-011) [test_tier_required]: `producer_system_designer` 已将 `restricted starter claim balance` 写入 claim 专题 PRD / design / project 与 game 根入口，明确 `slot-1` 可用受限 bucket 启动但首个 claim 仍非免费。
- [x] TASK-GAME-045 (PRD-GAME-011) [test_tier_required + test_tier_full]: `runtime_engineer` 已落地 `restricted starter claim balance` bucket、slot-1 claim/upkeep 专用扣费、bond provenance、refund 拆分、transfer guard 与 snapshot/replay 兼容回归。
- [x] TASK-GAME-046 (PRD-GAME-011) [test_tier_required]: `viewer_engineer` 已补齐 restricted/liquid 余额拆分、funding mix、slot-1/slot-2 blocker、pure API canonical 字段、viewer 文案与 explorer 展示口径。
- [x] TASK-GAME-047 (PRD-GAME-011) [test_tier_required + test_tier_full]: `qa_engineer` 已建立 restricted starter balance QA 矩阵，并确认 claim/upkeep/refund/transfer guard/viewer parity 为 `pass`；当前 blocker 收敛为 restricted grant 的发放元数据、过期/回收与经济审计链未实现，证据见 `doc/testing/evidence/game-agent-claim-restricted-starter-balance-matrix-2026-03-29.md`。
- [x] TASK-GAME-048 (PRD-GAME-011) [test_tier_required]: `producer_system_designer` 已基于 QA `block` 结论完成首轮复核，决定维持 `slot-1 only / non-transferable / provenance-preserving` 边界，不收窄 restricted grant 的 lifecycle / audit 要求，并重新打开后续补齐链路。
- [x] TASK-GAME-049 (PRD-GAME-011) [test_tier_required + test_tier_full]: `runtime_engineer` 已补齐 restricted grant lifecycle：实现 `issuance_reason / issuer_id / expires_at_epoch` 状态、issue/expire/revoke 事件、issuer-scoped 发放/回收动作与 main token 审计链路，并将终态 grant 的 restricted refund 回收到 treasury。
- [x] TASK-GAME-050 (PRD-GAME-011) [test_tier_required]: `liveops_community` 已建立 restricted grant 的运营发放/撤销/过期 runbook，冻结 `allowlist / qa_seed / liveops_campaign` issuer 边界、回收条件与 incident fallback；v1 统一使用 `issuer_id=liveops`，并把 `issuance_reason` 收口到三类允许值。
- [x] TASK-GAME-051 (PRD-GAME-011) [test_tier_required + test_tier_full]: `qa_engineer` 已建立 restricted grant lifecycle / audit matrix，验证 issuance metadata、expiry/revoke、source-sink 对账与 transfer non-bypass 全部闭环，证据见 `doc/testing/evidence/game-agent-claim-restricted-grant-lifecycle-matrix-2026-03-29.md`。
- [x] TASK-GAME-054 (PRD-GAME-011) [test_tier_required]: `runtime_engineer` 已新增 `oasis7_liveops_grant_cli` 作为 restricted grant 的日常运营入口，封装 `issue/revoke/status` 并保持 runtime canonical action / world-state 真值，不开放 admin roster 直改旁路。
- [x] TASK-GAME-055 (PRD-GAME-011) [test_tier_required]: `runtime_engineer` 已新增 `scripts/oasis7-liveops-grant.sh` 作为 `oasis7_liveops_grant_cli` 的运营 wrapper，收口 `status/issue/revoke` 的位置参数和 `OASIS7_WORLD_DIR` 缺省，继续保持底层只转发 canonical CLI，不新增任何 admin / world-file 旁路。
- [x] TASK-GAME-056 (PRD-GAME-011) [test_tier_required]: `runtime_engineer` 已将 governance registry manifest/import/audit 扩成 per-slot threshold，使 `liveops` 这类低权限 restricted grant admin slot 可显式使用 `1-of-2` signer policy，而 treasury/controller 主槽位继续默认 `2-of-3`；drill 脚本也同步改为按 baseline manifest 的 slot signer count/threshold 工作。
- [x] TASK-GAME-057 (PRD-GAME-011) [test_tier_required]: `liveops_community` 已补齐 restricted grant runbook 的实操层 SOP，明确 `liveops 1-of-2` 的首次开通、governance registry 重导入后的恢复顺序、`status` 判读门槛，以及“slot-only manifest 不得直接 import 到 world”的风险提示。
- [x] TASK-GAME-058 (PRD-GAME-001) [test_tier_required]: 执行 ROUND-010 `game` 模块入口治理，为 `doc/game/README.md` 增加轻量“从这里开始”分流，明确产品目标、执行追踪、玩法总览、试玩口径与高频专题之间的阅读顺序。
- [x] TASK-GAME-059 (PRD-GAME-011) [test_tier_required + test_tier_full]: 将 restricted grant 的 daily treasury source 从 `ecosystem_pool` 拆分为独立 `restricted_starter_claim_liveops_pool`，新增 `TopUpRestrictedStarterClaimLiveopsPool` controller-governed runtime action，并同步让 liveops CLI / runbook 转向 dedicated pool 余额。
- [x] agent-claim-slot-1-auto-starter-grant (PRD-GAME-011) [test_tier_required]: `runtime_engineer` 已将首个 `slot-1` 启动金从“运营审批后 grant”改为“专用池余额足够时 claim 路径自动补足并原子认领”，并同步 canonical quote、viewer/API 摘要与 funding provenance。 Trace: .pm/tasks/task_313368c409c54cc2bcf8ef4f47919b65.yaml
- [x] TASK-GAME-060 (PRD-GAME-008) [test_tier_required]: 按最新产品设定收口 `pure_api` 正式游玩前置，把 `game` 根 PRD / project 与 active 专题改写为“active LLM access required；无 LLM 仅 observer/debug”，并同步替换当前门禁命令中的 `--no-llm` 示例。
- [x] TASK-GAME-061 (PRD-GAME-012) [test_tier_required]: 新增“10 分钟留存修复”专题 PRD / design / project，并完成 `game` 根 PRD / project、`gameplay-top-level-design` 主文档、索引与 task execution log 挂载。
- [x] TASK-GAME-062 (PRD-GAME-012) [test_tier_required + test_tier_full]: `viewer_engineer` 已收口首次进入与最小控制地板的前台控制门控与 ack 语义，让 headed Web/UI 与 `software_safe` 不再把明确 `blocked` / `no_progress` 压扁成伪 timeout；fresh active-LLM formal lane 的 floor blocker 与恢复状态继续由 `TASK-GAME-065` 跟踪。
- [x] TASK-GAME-063 (PRD-GAME-012) [test_tier_required]: `runtime_engineer` 已把 `PostOnboarding` 后 10 分钟工业中循环加厚为“韧性生产 -> 第一次扩产取舍 -> 通用 mid-loop”的可复跑目标包，补齐首座工厂、首个制成品、停机恢复与扩产取舍的 canonical 语义。
- [x] TASK-GAME-064 (PRD-GAME-012) [test_tier_required]: `viewer_engineer` 已收口首屏噪音、玩家身份和后果可见化，把玩家身份、当前主目标、主阻塞、立即下一步以及代价/奖励反馈抬到首屏主语义。
- [x] TASK-GAME-065 (PRD-GAME-012) [test_tier_required]: `qa_engineer` 已区分 active-LLM formal lane 与 debug/probe lane，并在复制 `main` 的 real provider `config.toml` 后完成 `3` 条 active-LLM 10 分钟正式样本；当前结论已从 `watch` 收口为 `hold`，因为 formal lane 虽已恢复 first-step floor，但仍稳定卡在 `post_onboarding.establish_first_capability / 20%`，且其中 `2` 条样本出现回退到 `first_session_loop.create_first_world_feedback / 0%` 并伴随 `logicalTime/eventSeq` 冻结。
- [x] gameplay-early-retention-focus-order (PRD-GAME-012) [test_tier_required]: `producer_system_designer` 已把当前 gameplay scope freeze 正式写回主文档：当前只允许按“`trust gate` 地板恢复 -> `PostOnboarding` capability closure -> 工业状态/停机修复可读 -> 间接控制因果与下一步”推进；在这些 early-retention blocker 清空前，不扩大战争/治理/元进度在首局中的曝光，也不允许把 debug/probe lane 结果包装成正式留存进展。 Trace: .pm/tasks/task_886e2ef4878645a6a6ab69c588dce57e.yaml
- [x] agent-claim-slot-1-onboarding-flow (PRD-GAME-011) [test_tier_required]: `viewer_engineer` 已为新账号首个 `slot-1` 认领补齐专用 onboarding 流：当 canonical `owned_claim_count=0` 且 `next_claim_quote.slot_index=1` 时，PostOnboarding HUD 会展示 claim CTA，要求玩家先选中未认领目标，再执行 `Prepare -> Confirm` 显式确认；链路复用 canonical quote / blocker，并通过 `actor_agent_id` 把 claimer actor 与 claim target 正确分离。 Trace: .pm/tasks/task_d02fe08db044492d9f0bfbcf645a4ccc.yaml

### T9 物理尺度与间接控制对齐（2026-05-07）
- [x] gameplay-physical-scale-contract-freeze (PRD-GAME-013) [test_tier_required]: `producer_system_designer` 已新增“物理尺度与间接控制对齐”专题 PRD / design / project，并完成 `game` 根入口、`gameplay` 主文档、索引与当前 task execution log 挂载；当前正式主路线继续保持“间接控制的文明模拟”，不把 `1cm` 写成 Minecraft 式逐块直接操作承诺。 Trace: .pm/tasks/task_5dfbbe7c8c0c4557bef2b49612da3081.yaml
- [x] runtime-native-resolution-declaration (PRD-GAME-013) [test_tier_required]: `runtime_engineer` 已把现有 coarse-grained runtime 子系统正式写成可 grep 的原生分辨率声明表，覆盖 `chunk-grid`、`asteroid-fragment-voxel`、`asteroid-fragment-spacing`、`movement-energy-cost`、`power-transfer-distance`、`location-site-actions` 与 `fragment-block-geometry`，并补齐厘米映射 / rounding / snapping 定向测试。 Trace: .pm/tasks/task_303dedfe38b04036a198c256cc858e29.yaml
- [x] viewer-scale-surface-truth-labeling (PRD-GAME-013) [test_tier_required]: `viewer_engineer` 已把 formal Web entry 的 `World Scale` 面板和地点列表补成玩家可读的尺度真值表面，正式显示 canonical `1cm`、world bounds、选中锚点坐标/半径、最近地点真实距离，并明确 marker/zoom 只服务可读性，不可当作真实几何尺寸。 Trace: .pm/tasks/task_103c448874b7494a8312418995889098.yaml
- [x] agent-action-contract-boundary-alignment (PRD-GAME-013) [test_tier_required]: `agent_engineer` 已把 dual-mode / action contract 的现行动作面收口为低频间接控制白名单，并显式把 `jump / attack / use_item / block_editing` 回收到 future embodied candidate gate。 Trace: .pm/tasks/task_15890765ee3b4188a1e2766973f392fc.yaml
- [x] qa-scale-consistency-matrix (PRD-GAME-013) [test_tier_required]: `qa_engineer` 已在 `doc/testing/evidence/gameplay-scale-consistency-matrix-2026-05-07.md` 完成最终矩阵复核，确认四层尺度合同在 runtime/viewer/agent 三侧一致，并补齐 blocker 签名归档。 Trace: .pm/tasks/task_8205baa6d2fb46388b11c1eed340fdf5.yaml
- `PRD-GAME-013` 当前规划切片已在 `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.project.md` 全部收口；该专题完成不等于 `PRD-GAME-012` trust/capability gate 已恢复。

### T10 间接控制 control-feeling 合同（2026-05-14）
- [x] indirect-control-feeling-contract-freeze (PRD-GAME-014) [test_tier_required]: 新增“间接控制 control-feeling 合同”专题 PRD / design / project，并完成 `game` 根 PRD / project、`gameplay-top-level-design` 主文档、索引与 task execution log 挂载，正式冻结 accepted intent、主因果、打断/重排与续玩恢复四项 guarantees。 Trace: .pm/tasks/task_89828a4d2c1b4e73987103699c10fa7d.yaml
- [x] runtime-control-feeling-canonical-contract (PRD-GAME-014) [test_tier_required]: `runtime_engineer` 已把 `player_gameplay` canonical surface 扩成可直接对账的 control-feeling 合同面，新增 accepted intent、intent scope/target、status reason、last world change、resume anchor、primary blocker 与 resume-next-step 字段，并把 `gameplay_action` / `prompt_control` / `agent_chat` / world-control feedback 全部接入同一 runtime truth。 Trace: .pm/tasks/task_f3c25dd6688f40fbbcf05df9036a83ec.yaml
- [x] gameplay-high-risk-design-hardening (PRD-GAME-012/014/015) [test_tier_required]: `producer_system_designer` 已执行高风险设计修补轮，补齐 bounded-response micro loop、anti-passive control fallback、economic readability 与 mature-world anti-grind/anti-forced-dependency 约束，并同步回写 `game` 根 PRD、`gameplay-top-level-design`、`control-feeling`、`small-player lane` 与当前 execution log。 Trace: .pm/tasks/task_b23cd4919b4c481490777293b556cc70.yaml
- [x] runtime-bounded-response-fallback-contract (PRD-GAME-014/012) [test_tier_required]: `runtime_engineer` 已把 `response_window_class`、`stalled_reason`、`escalation_hint` 与 `fallback_action_*` 下沉到 canonical `player_gameplay` surface，并用 snapshot/persist 定向测试阻断“accepted 之后连续静默等待却仍伪装成 executing”的灰区。 Trace: .pm/tasks/task_b23cd4919b4c481490777293b556cc70.yaml
- [x] qa-control-feeling-and-anti-grind-matrix (PRD-GAME-014/015) [test_tier_required]: `qa_engineer` 已在 `doc/testing/evidence/gameplay-control-feeling-and-anti-grind-matrix-2026-05-23.md` 建立跨专题矩阵，绑定 `silent wait without fallback`、`world_activity_only`、`grind_only`、`forced_major_power_dependency` 四类 blocker 的 formal sample、判据与当前 `pass/watch` verdict。 Trace: .pm/tasks/task_b23cd4919b4c481490777293b556cc70.yaml
- 后续待建任务见 `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.project.md`：
  `runtime-control-feeling-canonical-contract`、`viewer-control-feeling-surface-alignment`、`agent-control-feeling-reprioritize-contract`、`qa-control-feeling-matrix`。

### T11 小玩家成长线与成熟世界承接（2026-05-17）
- [x] small-player-progression-contract-freeze (PRD-GAME-015) [test_tier_required]: `producer_system_designer` 已新增 `PRD-GAME-015` 专题 PRD / design / project，并完成 `game` 根入口、`gameplay` 主文档、索引与当前 task execution log 挂载；正式冻结 `local operator -> regional specialist -> limited-scope regional influence` 主线，并把 `protected first industrial win` 收口为低爆炸半径、可恢复和 leverage 可见的 first win，而不是新手无敌豁免。 Trace: .pm/tasks/task_d97dfa29208444a9b6a652f2a12fb65d.yaml
- [x] viewer-economic-readability-first-capability-surface (PRD-GAME-012/015) [test_tier_required]: `viewer_engineer` 已在 `software_safe` 正式玩家 surface 新增 `Capability Economics`，显式展示 `投入 / 产出 / 新用途 / 修复动作 / 下一步价值`，并用 UI/summary 定向测试阻断工业成长继续退化成只看库存和产量。 Trace: .pm/tasks/task_b23cd4919b4c481490777293b556cc70.yaml
- 后续待建任务统一收口在 `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.project.md`，避免在 gameplay 主入口重复展开未绑定 Trace 的计划行。

## 依赖
- 模块设计总览：`doc/game/design.md`
- doc/game/prd.index.md
- `doc/game/gameplay/gameplay-top-level-design.prd.md`
- `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
- `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md`
- `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md`
- `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06.prd.md`
- `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
- `doc/game/gameplay/gameplay-engineering-architecture.md`
- `doc/playability_test_result/prd.md`
- `testing-manual.md`
- `.agents/skills/prd/check.md`

## 状态
- 更新日期: 2026-05-23
- 当前状态: in_progress
- 下一任务: `#160` 当前 repo-side formal blocker 已收口；后续只在需要时由对应 owner 继续维护更宽的 retention / control-feeling / liveops 跟踪，不再把 `PostOnboarding -> first capability gate` 当作当前未解 blocker。
- 已登记待排任务: 2026-04-15 的 `trust gate = hold / capability gate = not_run` 现在仅保留为历史 baseline。当前 fresh formal truth 已更新为 `trust gate = pass`、`first capability gate = pass`；若后续再次回退，必须以新样本单独重开，而不是继续复用旧 blocker 文案。
- 当前切片进展: `task_8d2e20dd7f5c47fd8303ff55159227ba` 已修复 launcher execution-world ready contract：fresh `run-game-test --json-ready` 现在会在 `NodeRuntimeExecutionDriver` 初始化时先为 execution world / simulator mirror 落盘 `snapshot.json` 与 `journal.json`，因此当前 `main` 不再因 `reward-runtime-execution-world` 缺少初始持久化文件而在 Viewer HTTP ready 前退出。该切片只清除 startup blocker，不单独改写 trust gate / capability gate verdict。
- 历史样本（2026-04-15）: `task_1dbcc087ae374721aa0928de3cd240e2` 曾在独立 validation worktree 复用 real-provider `config.toml`，并通过 shared active-LLM stack `20260415-223459` 进行 trust gate 复验；当时两次 `software_safe` floor probe（`20260415-224143` / `20260415-224312`）都命中同一签名：`step_request.accepted=true` 但长期停留 `stage=queued`，没有 terminal `lastControlFeedback`，且 `logicalTime=1` / `eventSeq=0` 不推进。因此当时样本结论为 `trust gate = hold`、`first capability gate = not_run`。
- 口径更新（2026-05-17）: `PRD-GAME-012` 的正式 verdict 继续保持双层：`10-minute trust gate` 与 `first capability gate` 分开判定。2026-04-15 的 `trust gate = hold / capability gate = not_run` 只保留为历史 baseline；当前 fresh active-LLM formal truth 已更新为 `trust gate = pass`、`first capability gate = pass`，证据见 `doc/testing/evidence/issue-160-first-capability-closeout-2026-05-17.md`。
- 当前切片进展: `task_7bdbbf9839c74c9eb7bb8c7c161e87de`、`task_fb967ddaadde459786e286b484bc4b0c`、`task_319c1fc645b04dd185f3afb45dcd00ee`、`task_ed2dd76639264739a61a25c0d89c3352` 已分别收口 runtime-live 映射回退、短暂 LLM failure 放大、smelter-first industrial schema 漂移与能力链真值误判；这些修复支撑当前 gate 恢复，但不替代后续 fresh sample。
- 最近完成的 retention、claim、preview 和 pure-api 收口不再在本页状态区逐条滚动播报；统一回看对应 gameplay topic `*.project.md`、测试 evidence 与 `.pm/tasks/*.execution.md`。
- 阶段收口优先级: `P0`
- 阶段 owner: `producer_system_designer`
- 当前阶段判断: `internal_playable_alpha_late`
- 下一阶段目标: `closed_beta_candidate`
- 阻断条件: 若统一 `closed_beta_candidate` release gate 尚未建立或任一关键 lane `block`，当前项目仍不得升级为 `closed beta` 对外口径。
- 当前评审结论: unified gate 已 `pass`，但本轮 producer 决策继续维持 `limited playable technical preview` claim envelope，因此阶段暂不提升到 `closed_beta_candidate`。
- 当前执行重点: 将 `limited playable technical preview` 从文案边界推进为 1 轮受控、controlled builder-facing、可回流的真实执行闭环；round-1 主线程已切到 GitHub issue `eng-cc/oasis7#48`，当前待 issue 评论 / linked issue / linked PR 进入首批 `Blocking / Opportunity / Idea` 分桶；若出现 claim drift 或 unified gate 回退，必须立即收紧节奏。
- 承接约束: `TASK-GAME-029/030/031/032` 必须以同一候选版本互链，不能用不同批次专题 `pass` 拼凑升阶结论。
- PRD 质量门状态: strict schema 已对齐（含第 6 章验证与决策记录）。
- 说明: 本文档状态区只保留当前 gate、当前切片与下一步；更早轮次进展和已完成事项继续以任务清单、topic project、evidence 与 execution log 为准。

## 阶段收口角色交接
### Meta
- Handoff ID: `HO-CORE-20260310-GAME-001`
- Date: `2026-03-10`
- From Role: `producer_system_designer`
- To Role: `viewer_engineer`
- Related Module: `game`
- Related PRD-ID: `PRD-GAME-004`
- Related Task ID: `TASK-GAME-018`
- Priority: `P0`
- Expected ETA: `待接收方确认`

### Objective
- 目标描述：完成微循环可玩性视觉优化二期，并把可直接进入发布评审的截图闭环证据沉淀到跨模块证据链。
- 成功标准：控制结果显著化、玩家模式减负、世界可读性增强三项已完成，且 `qa_engineer` 已基于证据完成复核。
- 非目标：不在本轮新增 launcher / explorer 体验功能，不扩展与微循环无关的 Viewer 大改。

### Current State
- 当前实现 / 文档状态：`TASK-GAME-018` 已完成，`TASK-GAMEPLAY-MLF-005/006/007/008` 均已闭环，当前待做的是把已完成结论回填到 release gate 证据链。
- 已确认事实：core 阶段收口将玩法微循环列为 `P0`；虽然任务已关闭，但若缺少跨模块证据互链，仍不得给出最终发布 `go` 结论。
- 待确认假设：现有 ROUND-009 录屏是否足以覆盖发布评审抽样；若不足，则在 release gate 阶段补拍，不回滚当前任务关闭结论。
- 当前失败信号 / 用户反馈：当前项目仍偏“能展示”，需要把“更好玩”变成明确证据。

### Scope
- In Scope: `TASK-GAME-018`、`TASK-GAMEPLAY-MLF-005/006/007/008`、截图 / 视频 / 结论证据回写。
- Out of Scope: 新玩法分支、新区块链浏览器功能、与微循环无关的全局 UI 重构。

### Inputs
- 关键文件：`doc/game/project.md`、`doc/game/prd.md`、相关 `gameplay-micro-loop-*` 专题文档。
- 关键命令：沿用现有 Viewer / playability 截图闭环命令与手动验收流程。
- 上游依赖：`producer_system_designer` 已在 `core` 层确定该项为 `P0`；`qa_engineer` 后续复核证据。
- 现有测试 / 证据：现有手动截图验收记录与 `runtime_live` 节奏修正结果。

### Requested Work
- 工作项 1：由 `qa_engineer` 复核 `doc/game/gameplay/gameplay-micro-loop-visual-closure-evidence-2026-03-10-round009.md` 的截图、录屏与语义状态。
- 工作项 2：刷新 playability 卡片与 `TASK-GAME-018` 阻断结论。
- 工作项 3：若结论通过，把 evidence linkage 回填到 playability / testing / core 证据链。

### Expected Outputs
- 代码改动：如需，仅限支撑 `TASK-GAME-018` 的 Viewer 表达层改动。
- 文档回写：`doc/game/project.md`、必要时相关专题 `project/prd`。
- 测试记录：至少补齐 `test_tier_required` 的截图闭环与结论。
- devlog 记录：在 `doc/devlog/YYYY-MM-DD.md` 中记载结果与遗留项。

### Done Definition
- [ ] 输出满足目标与成功标准
- [ ] 影响面已核对 `producer_system_designer` / `qa_engineer`
- [ ] 对应 `prd.md` / `project.md` 已回写
- [ ] 对应 `doc/devlog/YYYY-MM-DD.md` 已记录
- [ ] required 证据已补齐

### Risks / Decisions
- 已知风险：如果只做视觉 polish 而不统一证据格式，玩法收口仍无法进入 go/no-go 评审。
- 待拍板事项：是否需要把 `TASK-GAMEPLAY-MLF-007` 进一步拆小给 `viewer_engineer`。
- 建议决策：先以最小体验闭环完成 `TASK-GAME-018`，不引入额外玩法范围扩张。

### Validation Plan
- 测试层级：`test_tier_required`
- 验证命令：沿用现有截图闭环与手动验收命令，并回写证据路径。
- 预期结果：微循环视觉增强可被截图 / 视频直接观察到，且 QA 可复核。
- 回归影响范围：game / viewer / playability 体验层。

### Handoff Acknowledgement
- 接收方确认范围：`qa_engineer 已接收 ROUND-009 证据并完成复核；TASK-GAME-018 已具备任务关闭结论`
- 接收方确认 ETA：`TASK-GAME-018 已完成；下一步转入 evidence linkage 回填`
- 接收方新增风险：`更长录屏仍建议在后续 release gate 抽样中复看，但不构成当前任务阻断`
