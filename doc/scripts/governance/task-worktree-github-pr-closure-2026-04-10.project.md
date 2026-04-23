# oasis7: task worktree GitHub PR closure 标准入口（2026-04-10）（项目管理）

- 对应设计文档: `doc/scripts/governance/task-worktree-github-pr-closure-2026-04-10.design.md`
- 对应需求文档: `doc/scripts/governance/task-worktree-github-pr-closure-2026-04-10.prd.md`

审计轮次: 3

## 任务拆解（含 PRD-ID 映射）
- [x] GPR-1 (PRD-SCRIPTS-GHPR-001/002/003) [test_tier_required]: 新增 `scripts/prepare-task-pr.sh`，并同步 `AGENTS.md`、`.pm/README`、scripts 模块文档与旧 landing compatibility 边界。
- [x] prepare-task-pr-local-required-recommendation (PRD-SCRIPTS-GHPR-004) [test_tier_required]: 为 `scripts/prepare-task-pr.sh` 增加 changed-path 本地 required 验证推荐摘要，输出推荐 `./scripts/ci-tests.sh required` 命令与必要的额外命令，但不自动执行。 Trace: .pm/tasks/task_f86d4971140d463193d336907f94a00c.yaml
- [x] prepare-task-pr-planner-reason-summary (PRD-SCRIPTS-GHPR-004) [test_tier_required]: 为 `scripts/prepare-task-pr.sh` 增加 changed-path planner `reason_summary` 与拆分后的 `reason_items[]` 输出，让 owner 在 PR preflight 阶段直接看到当前 scope 的命中原因，但不扩到 wasm 解释层或自动执行。 Trace: .pm/tasks/task_db7d4beaf8354e1eb7f50afd0ee0a8d6.yaml
- [x] pm-rebase-conflict-helper (PRD-SCRIPTS-GHPR-005) [test_tier_required]: 新增 `scripts/pm/rebase-conflict-helper.sh`，统一分类 `.pm/**` rebase 冲突，并把唯一允许的自动修复边界收口为 `.pm/inbox/signals.jsonl` 的 signal-id 碰撞；git-ignored 本地视图只提示保留 `main` 删除并执行 `./scripts/pm/sync-views.sh`。 Trace: .pm/tasks/task_6e23e1a96ee34d059aa62e4280a367b7.yaml

## 关键契约

### 1. PR 收口策略
| 字段 | 默认值 |
| --- | --- |
| source branch | 当前 branch |
| base branch | `main` |
| remote | `origin` |
| strategy | preflight source cleanliness + compare against `origin/main` or local `main` + optional `gh pr create` |
| cleanup | 只输出命令，不默认自动删除；PR 合入后必须执行 |

### 2. JSON 输出字段
| 字段 | 含义 |
| --- | --- |
| `source_branch` | 准备开 PR 的 task branch |
| `source_worktree` | source branch 对应 worktree |
| `base_branch` | PR base branch，默认 `main` |
| `comparison_ref` | 比较用 ref，优先 `origin/main` |
| `ahead_count` | source 相对 comparison ref 的领先提交数 |
| `behind_count` | source 相对 comparison ref 的落后提交数 |
| `rebase_required` | 是否必须先 rebase |
| `create_command` | 标准 `gh pr create` 命令 |
| `post_merge_commands` | PR 合入后必须执行的本地同步与回收命令全集 |
| `cleanup_commands` | `post_merge_commands` 的兼容别名，保持 agent 侧旧消费方可继续读取 |
| `pr_url` | `--create` 成功后返回的 PR URL |
| `local_required_validation.scope` | 当前 diff 命中的本地 required 推荐范围（`minimal`/`targeted`/`full`） |
| `local_required_validation.reason_summary` | rust required planner 的原始 `reason_summary` |
| `local_required_validation.reason_items` | 将 `reason_summary` 按 `;` 拆分后的结构化数组 |
| `local_required_validation.recommended_required_command` | 与 changed-path planner 对齐的本地 required 建议命令 |
| `local_required_validation.recommended_extra_commands` | 附加建议命令（如 viewer visual baseline） |

### 3. `.pm` Rebase 冲突辅助
- `scripts/pm/rebase-conflict-helper.sh` 默认只读分类 `.pm/**` 未合并路径，输出 `signals_jsonl` / `generated_view` / `task_yaml` / `task_execution_log` / `memory_yaml` / `stage_yaml` 等类别。
- 只有 `.pm/inbox/signals.jsonl` 在 active rebase 中允许通过 `--resolve-signals` 自动修复；helper 会保留 upstream signal id，并把 branch-local 碰撞项顺延重编号。
- 若冲突命中 `.pm/registry/tasks.yaml` 或 `.pm/roles/*/backlog/*.yaml`，helper 只建议保留 `main` 删除并执行 `./scripts/pm/sync-views.sh`，不恢复这些 git-ignored 本地视图。

## 依赖
- `AGENTS.md`
- `.pm/README.md`
- `scripts/prepare-task-pr.sh`
- `scripts/land-task-worktree.sh`
- `doc/scripts/project.md`

## 状态
- 更新日期：2026-04-23
- 当前阶段：已完成
- 阻塞项：无
- 下一步：若后续要补 wasm gate 解释层、自动等待 required checks、merge queue 辅助或更激进的 `.pm` 冲突自动修复，应新开下一轮交付专题；本轮只冻结“默认最终合流经由 GitHub PR + preflight 给出本地最小 required 推荐与 planner 原因摘要 + `.pm` rebase 冲突 helper 只自动修 `signals.jsonl`”的契约。
