# task_d27fd4eafe4c41bdb046d3fe3765033f Execution Log

- task_uid: task_d27fd4eafe4c41bdb046d3fe3765033f
- title: move validator set and signer bindings to genesis truth
- owner_role: runtime_engineer
- worktree_hint: /home/scc/worktrees/oasis7-p2p-validator-genesis-truth

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-05-06 12:24:18 CST / runtime_engineer
- 完成内容: 复用 execution world 里的 `governance_finality_signer_registry` 作为 `oasis7_chain_runtime` 的 validator membership / signer binding 真值入口；当 registry 存在时，runtime 会用 world-state registry 覆盖本地 `NodePosConfig`，并把 replication remote writer allowlist 与 reward runtime node identity binding 统一切到这份 effective config。补齐 `governance_registry` 单测、`cargo check`、模块 PRD/project 与治理 signer externalization project 回写，明确 `--node-validator*` 退回为 bootstrap 或显式运维覆盖。
- 遗留事项: 当前 world-state registry 路径默认采用三节点等权 stake 派生；若后续需要非等权 validator stake、on-chain onboarding/removal/rotation action 或 shared-network 实机证据，还需继续拆后续任务。

## 2026-05-06 12:58:00 CST / runtime_engineer
- 完成内容: 按“与当前 PR 算一个大任务”的口径，继续在同一 task/branch 上补齐 validator / finality signer 的设计方案：将治理 signer externalization 的现状校正为 `registry-first + local fallback`，冻结主流公链通用的 `apply -> approved_candidate -> probation_ready -> active -> rotate/revoke` 准入状态机，明确 `world-state registry` 才是正式激活真值，`--node-validator*` 与 operator-local env 只保留为 bootstrap / 显式运维覆盖；同时明确 controller signer 不属于公开 validator 申请路径，而是治理内部 appointment。同步回写模块主 PRD/project 与 `MAINNET-2` readiness 文档，避免“代码已切 registry 真值、设计仍停留在 local config”。
- 遗留事项: 当前只冻结了 target workflow；candidate registry、activation action、shared-network probation、formal operator runbook 与真实治理 approval trail 仍待后续实现与实机证据收口。

## 2026-05-06 14:03:00 CST / runtime_engineer
- 完成内容: 将 validator / finality signer 准入闭环直接落入 runtime：新增 `governance_validator_admissions` world-state 持久化、`Submit/Approve/Activate/RevokeGovernanceValidatorAdmission` 四个治理动作，以及基于 `base finality registry + due admissions - revoked admissions` 的 effective finality registry 解析。`oasis7_chain_runtime` 与 `oasis7_governance_registry_audit` 现已统一读取 effective finality registry；新增独立 admission 生命周期测试，覆盖 `active` 生效、`probation_ready` 到期并入以及 revoke 后重新申请。
- 遗留事项: 更大范围 shared-network probation、更多 controller/finality live drill 变体，以及最终 governance approval / ceremony / QA `pass` 仍需后续任务继续补证据。
