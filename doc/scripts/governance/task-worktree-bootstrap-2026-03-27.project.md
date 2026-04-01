# oasis7: task worktree bootstrap 标准入口（2026-03-27）（项目管理）

- 对应设计文档: `doc/scripts/governance/task-worktree-bootstrap-2026-03-27.design.md`
- 对应需求文档: `doc/scripts/governance/task-worktree-bootstrap-2026-03-27.prd.md`

审计轮次: 2

## 任务拆解（含 PRD-ID 映射）
- [x] WTB-BOOT-1 (PRD-SCRIPTS-WTB-001/002) [test_tier_required]: 新增 `scripts/new-task-worktree.sh`，实现默认 branch/path 派生、clean-source guard 与 branch/path 冲突检测。
- [x] WTB-BOOT-2 (PRD-SCRIPTS-WTB-001/002/003) [test_tier_required]: 为入口补齐 `--base`、`--branch`、`--path`、`--json`、`--allow-dirty-source` 契约与人类可读下一步提示。
- [x] WTB-BOOT-3 (PRD-SCRIPTS-WTB-003) [test_tier_required]: 同步 `AGENTS.md`、`doc/scripts/{prd,project,README,prd.index}.md` 与 `doc/devlog/2026-03-27.md`，把新标准入口收入口径。
- [x] WTB-BOOT-4 (PRD-SCRIPTS-WTB-004) [test_tier_required]: 为入口补齐 `--init-docs` 与 `--with-harness`，输出模块文档检查摘要，并可在新 worktree 中后台预热 `worktree-harness.sh up --no-llm`。
- [x] WTB-BOOT-5 (PRD-SCRIPTS-WTB-001/002/004) [test_tier_required]: 收紧 worktree 例外授权话术，同步 `AGENTS.md` 与 task-worktree bootstrap 文档，明确“文档/脚本/测试/话术改动也算新需求”“只有显式复用授权才可例外”“发现切错 worktree 后必须立即切走”。

## 关键契约

### 1. 默认命名
| 字段 | 默认值 |
| --- | --- |
| branch | `task/<module>-<task>` |
| worktrees_root | `<repo-parent>/worktrees` |
| worktree_path | `<worktrees_root>/<repo-name>-<module>-<task>` |
| base_ref | `HEAD` |

### 2. 输出字段
| 字段 | 含义 |
| --- | --- |
| `module` | 原始 module 输入 |
| `task` | 原始 task 输入 |
| `repo_name` | worktree family 使用的稳定 repo 名称 |
| `worktrees_root` | worktree family 使用的默认根目录 |
| `branch` | 最终使用的 branch |
| `worktree_path` | 最终 worktree 路径 |
| `base_ref` | 创建或附着所基于的 ref |
| `mode` | `create_new_branch` 或 `attach_existing_branch` |
| `doc_checks` | `--init-docs` 时的模块 PRD / project / 当日 devlog 检查结果 |
| `harness` | `--with-harness` 时的 bootstrap 日志、state 文件、状态与 viewer URL 摘要 |

### 3. worktree 治理口径
| 主题 | 契约 |
| --- | --- |
| 新需求识别 | 文档改动、脚本改动、测试改动、仅改话术都算新需求，默认新开独立 worktree。 |
| 例外授权 | 仅当用户显式说出“复用当前 worktree / 就在这里改 / 不要切新 worktree”时，才允许不新开。 |
| 模糊表述 | “先写一版”“先不要提交”“顺手改一下”都不构成复用授权。 |
| 错误 worktree | 开工后才发现切错 worktree 时，必须立即说明并切走，不允许把错误 worktree 继续当作任务容器。 |

## 依赖
- `AGENTS.md`
- `scripts/new-task-worktree.sh`
- `doc/scripts/project.md`

## 状态
- 更新日期：2026-04-01
- 当前阶段：已完成
- 阻塞项：无
- 下一步：若后续要继续把规则前移，可评估是否为 bootstrap 增加机器可读的“reuse authorization checklist”或错误 worktree 检测提示；当前话术收紧已覆盖根规则与专题文档。
