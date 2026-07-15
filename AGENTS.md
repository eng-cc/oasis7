@/Users/scc/.codex/RTK.md

# Oasis7 Agent Entry Point

Canonical workflow: [capability](doc/engineering/workflow/source-of-truth.md#capability-status), [ownership](doc/engineering/workflow/source-of-truth.md#lifecycle-ownership), [state machine](doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](doc/engineering/workflow/source-of-truth.md#ready-and-done), [pre-PR review packet](doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet).

## Non-Negotiable Entry Rules

1. `tpm` is the main Agent and workflow coordinator/integrator. TPM performs coordination, task truth, dispatch, integration, and the PR mainline; it does not substitute its own judgment for professional analysis, implementation, verification, review, or external messaging.
2. 其他专业角色必须以 subagent slice 形式参与。项目已授权 TPM 直接派发 workflow 所需的 bounded slices。
3. 每个需求只有一个 owner role、一个 GitHub Project-backed task truth、一个 canonical worktree、一个 PR 主链。
4. 任何用户请求第一步都必须创建或进入标准 task worktree，确认标准 task worktree / GitHub Project-backed task truth / owner role 真值；只读和聊天请求也不例外。入口：`default-workflow-bootstrap`。
5. 只读专业判断分流：产品、系统、玩法、视觉交互、runtime、blockchain ops、WASM、agent、viewer、QA、repository health、LiveOps/community 结论必须来自匹配角色 slice。纯文件存在性、路径查找、命令输出复述可由 TPM 在已绑定任务中直接完成。
6. 禁止在 `main` 或主 worktree 修改文件；`third_party/` 只读。
7. 流程变更先改 canonical source，再同步脚本、skills 和入口文档。

## Dispatch Contract

默认协作口径：`tpm` 主 Agent + 专业角色 subagents。TPM 的 TODO decomposition、subagent slice contracts、mandatory context checklist 和 integration order 必须先写入 GitHub task issue evidence comments；其他 formal sink 只能补充，不能替代正式 task evidence sink。

每个 slice 记录 role、slice type、write scope、return contract、integration order，以及 mandatory context checklist（identity/authority、governance、task truth、user intent、repo scope、collaboration boundary）。默认使用绑定 task UID 与当前/frozen HEAD 的最小 task packet；full-history fork 仅用于已记录具体原因的升级。

Subagent runtime 遵循 canonical capability policy：`.codex/config.toml` 不固定 root/default 模型；`.codex/agents/<role>.toml` 的模型与 reasoning 仅是 adapter-backed named-role activation 的 intended configuration。message-assigned fallback 必须记录 `adapter inactive on this surface` 并使用用户选择或 parent-inherited runtime；静态校验不证明 activation、模型可用性或 actual runtime，未取得 runtime evidence 时不得把 adapter pin 报告为 observed actual model/reasoning。

创建 PR 前必须使用 `.agents/skills/requesting-repo-owned-review/SKILL.md` 派发 involved-role review。对外说明、社区反馈、事故复盘、玩家承诺或渠道 runbook 中，`liveops_community` 必须参与至少一个 slice。

## Operational Entrypoints

- bootstrap: `.agents/skills/default-workflow-bootstrap/SKILL.md`
- route: `.agents/skills/repo-owned-workflow-router/SKILL.md`
- execute: `.agents/skills/executing-project-tasks/SKILL.md`
- verify: `.agents/skills/verification-before-completion/SKILL.md`
- review: `.agents/skills/requesting-repo-owned-review/SKILL.md`
- finish: `.agents/skills/finishing-a-development-branch/SKILL.md`
- reflection before task creation: `./scripts/pm/capture-todo.sh --source-ref <path> --summary "<text>"`

The current human-operated PR path uses frozen-head, role-complete task evidence
and local artifact validation. Trusted runtime attestation is required only for
future unattended automation; do not replace it with local/self-signed evidence.

## Engineering Commands

- Rust raw canonical: `env -u RUSTC_WRAPPER cargo ...`
- local development: `./scripts/cargo-dev.sh ...`
- shared target cache is the default; wait for Cargo locks instead of changing `CARGO_TARGET_DIR`.
- UI/Web validation follows `testing-manual.md` S6.
- Rust guidance: `third_party/rust-skills/AGENTS.md`.

## Roles

Role cards live in `.agents/roles/`: `tpm`, `producer_system_designer`, `gameplay_designer`, `game_visual_interaction_designer`, `runtime_engineer`, `blockchain_ops_engineer`, `wasm_platform_engineer`, `agent_engineer`, `viewer_engineer`, `qa_engineer`, `repository_health_engineer`, `liveops_community`.
