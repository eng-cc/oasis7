# oasis7: worktree-isolated harness 主入口（2026-03-27）（项目管理）

- 对应设计文档: `doc/scripts/governance/worktree-isolated-harness-2026-03-27.design.md`
- 对应需求文档: `doc/scripts/governance/worktree-isolated-harness-2026-03-27.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] WTH-1 (PRD-SCRIPTS-HARNESS-001/002) [test_tier_required]: 新增 `scripts/worktree-harness.sh` / `scripts/worktree-harness-lib.sh`，实现 worktree 身份解析、端口组分配、状态文件落盘和 `up/down/status/url/logs/smoke` 主入口。
- [x] WTH-2 (PRD-SCRIPTS-HARNESS-001/002/003) [test_tier_required]: 扩展 `run-game-test.sh` / `run-producer-playtest.sh` 契约，支持 worktree 级 `output-dir`、`run-id`、`meta-file`、`json-ready` 与 bundle 根目录注入。
- [x] WTH-3 (PRD-SCRIPTS-HARNESS-002/003) [test_tier_required]: 同步 `testing-manual.md`、`doc/scripts/{README,prd.index,project,prd}.md` 与 `doc/devlog/README.md`，将 worktree harness 正式收入口径。

## 关键契约

### 1. Harness 动作
| 动作 | 输入 | 输出 | 说明 |
| --- | --- | --- | --- |
| `up` | `--no-llm/--with-llm`、`--bundle-mode`、`--smoke-timeout` | `state.json`、ready payload、stdout 摘要 | 起当前 worktree 隔离栈 |
| `down` | 无 | 更新后的 `state.json` | 停止 launcher pid，关闭对应浏览器 session |
| `status` | `--json` 可选 | 状态摘要或完整 JSON | 供 agent 读取当前 worktree 栈状态 |
| `url` | 无 | `viewer_url` | 机器可读获取 URL |
| `logs` | 无 | runtime / artifact 路径 | 快速跳转当前 worktree 产物 |
| `smoke` | `--timeout` 可选 | 截图 / state / 最小 verdict | 复用当前 worktree 栈做最小闭环 |

### 2. 状态文件字段
| 字段 | 含义 |
| --- | --- |
| `worktree_id` | 当前 git worktree 的稳定短 id |
| `worktree_path` | 当前 worktree 绝对路径 |
| `viewer_url` | 当前 stack viewer URL |
| `viewer_port` | Viewer HTTP 端口 |
| `web_bind` | Web bridge bind 地址 |
| `live_bind` | live TCP bind 地址 |
| `chain_status_bind` | chain status bind 地址 |
| `bundle_dir` | 当前 worktree 隔离 bundle 根目录 |
| `artifact_dir` | 当前 worktree 证据根目录 |
| `launcher_pid` | 当前 launcher 进程 |
| `browser_session` | 当前 worktree 的 agent-browser session name |

## 依赖
- `scripts/run-game-test.sh`
- `scripts/run-producer-playtest.sh`
- `testing-manual.md`
- `doc/scripts/project.md`

## 状态
- 更新日期：2026-03-27
- 当前阶段：已完成
- 阻塞项：无
- 下一步：若要继续提升 agent 自治度，后续可新增 worktree 级 observability sidecar 专题。
