@/Users/scc/.codex/RTK.md

# Oasis7 Agent Entry Point

Canonical workflow: [capability](doc/engineering/workflow/source-of-truth.md#capability-status), [ownership](doc/engineering/workflow/source-of-truth.md#lifecycle-ownership), [state machine](doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](doc/engineering/workflow/source-of-truth.md#ready-and-done), [prior approval for workflow changes](doc/engineering/workflow/source-of-truth.md#workflow-change-approval), [ordered multi-PR contract](doc/engineering/workflow/source-of-truth.md#123-github-project-backed-pm-contract), [pre-PR review packet](doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet), [PR title and commit language](doc/engineering/workflow/source-of-truth.md#pr-and-commit-language).

Write PR titles and complete commit messages in English, including subjects and any bodies; preserve identifiers, paths, and standard Git trailers verbatim. See the canonical [PR and commit language rule](doc/engineering/workflow/source-of-truth.md#pr-and-commit-language).

## Non-Negotiable Entry Rules

1. `tpm` is the main Agent and workflow coordinator/integrator. TPM performs coordination, task truth, dispatch, integration, and the PR mainline; it does not substitute its own judgment for professional analysis, implementation, verification, review, or external messaging.
2. 需要专业结论或专业审查时，由匹配角色以 bounded subagent slice 参与；纯事实查询无需强制派工。项目已授权 TPM 直接派发 workflow 所需 slices。
3. 每个需求只有一个 owner role、一个 GitHub Project-backed task truth、一个 canonical worktree、一个有序 PR 主链；同一 Task UID 的多 PR 只有在 canonical source 的 ordered multi-PR contract 已激活时可用。当前 PM helpers 仍是单 PR projection，第二个 PR 必须 fail closed；兼容期可用 coordinating Issue + 有序 linked delivery tasks，并且协调任务须等待全部 required delivery 合并后才能完成。
4. 有写入副作用的请求按 `default-workflow-bootstrap` 绑定 task truth；无外部或持久副作用的只读请求可直接回答或做匹配角色分析，无需创建 task/worktree。拟启动 workflow-change task 时，canonical prior-approval stop 必须先于 task binding、反思捕获、Issue/Project/worktree 创建或 scope promotion。只读审计/诊断不授权改 policy。
5. 专业判断分流：产品、系统、玩法、视觉交互、runtime、blockchain ops、WASM、agent、viewer、QA、repository health、LiveOps/community 结论必须来自匹配角色 slice；TPM 可直接回答客观事实。只读结论不是正式 review、ready 或 task-complete 证据。
6. 禁止在 `main` 或主 worktree 修改文件；`third_party/` 只读。
7. 流程变更先改 canonical source，再同步脚本、skills 和入口文档。

## Dispatch Contract

默认协作口径：`tpm` 主 Agent + 按需专业角色 subagents。授权写入任务的 TODO decomposition、subagent slice contracts、mandatory context checklist 和 integration order 必须在派工前写入 GitHub task issue evidence comments；只读请求不创建任务证据。其他 formal sink 只能补充，不能替代写入任务的正式 task evidence sink。

每个 slice 记录 role、slice type、write scope、return contract、integration order，以及 mandatory context checklist（identity/authority、governance、task truth、user intent、repo scope、collaboration boundary）。默认使用绑定 task UID 与当前/frozen HEAD 的最小 task packet；full-history fork 仅用于已记录具体原因的升级。

Subagent runtime 遵循 canonical capability policy：`.codex/config.toml` 不固定 root/default 模型；`.codex/agents/<role>.toml` 的模型与 reasoning 仅是 adapter-backed named-role activation 的 intended configuration。message-assigned fallback 必须记录 `adapter inactive on this surface`，默认 intended configuration 为 `inherit current parent selection`，并使用用户选择或 parent-inherited runtime；静态校验不证明 activation、模型可用性或 actual runtime，未取得 runtime evidence 时不得把 adapter pin 报告为 observed actual model/reasoning。

写产品文档、写系统设计文档、复杂 bug 排查，派工时必须归类为复杂任务。复杂 slice 可由主 Agent（TPM）按 [canonical dispatch contract](doc/engineering/workflow/source-of-truth.md#52-tpm-planning-and-subagent-dispatch) 显式请求以主 Agent/父线程的 model 与 reasoning 配置覆盖默认值；派工前记录该 slice 的复杂度理由、请求值和工具能力限制，派工后记录实际 runtime evidence 或缺失原因。仅使用已知父线程设置或受支持的双设置继承方式，否则记录限制。保留普通 slice 默认值，遵守 fixed named-role pins 与工具限制；必要时采用受支持的 message-assigned fallback 并记录 tradeoff，无支持路径则记录未能采用的请求及实际允许的默认/继承派工。不得推断父线程设置或将请求值当作 observed runtime。

创建 PR 前必须使用 `.agents/skills/requesting-repo-owned-review/SKILL.md` 派发 involved-role review。对外说明、社区反馈、事故复盘、玩家承诺或渠道 runbook 中，`liveops_community` 必须参与至少一个 slice。

## Operational Entrypoints

- bootstrap: `.agents/skills/default-workflow-bootstrap/SKILL.md`
- route: `.agents/skills/repo-owned-workflow-router/SKILL.md`
- execute: `.agents/skills/executing-project-tasks/SKILL.md`
- verify: `.agents/skills/verification-before-completion/SKILL.md`
- review: `.agents/skills/requesting-repo-owned-review/SKILL.md`
- finish: `.agents/skills/finishing-a-development-branch/SKILL.md`
- reflection before task creation: `./scripts/pm/capture-todo.sh --source-ref <path> --summary "<text>"`

Classified non-merge outcomes follow the [canonical terminal runbook](doc/engineering/workflow/source-of-truth.md#terminal-runbook).

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
