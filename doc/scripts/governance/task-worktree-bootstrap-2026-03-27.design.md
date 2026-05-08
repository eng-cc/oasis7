# oasis7: task worktree bootstrap 标准入口（2026-03-27）设计

- 对应需求文档: `doc/scripts/governance/task-worktree-bootstrap-2026-03-27.prd.md`
- 对应项目管理文档: `doc/scripts/governance/task-worktree-bootstrap-2026-03-27.project.md`

审计轮次: 2

## 1. 设计定位
把“每个新需求先开独立 worktree”的团队规范沉到可执行脚本，降低多任务并行的启动成本和人为命名漂移；同时收紧例外授权口径，避免“先写一版”“先不要提交”“顺手改一下”这类模糊表述重新把是否切 worktree 退回主观判断。

## 2. 设计结构
- 校验层：确认当前目录位于 git worktree，源 worktree 默认 clean。
- 命名层：将 `<module> <task>` slug 化，派生默认 branch/path。
- 冲突层：检查目标路径是否存在、目标 branch 是否已在其他 worktree 检出。
- 授权层：只接受用户显式授权“复用当前 worktree / 就在这里改 / 不要切新 worktree”这类明确指令；其余模糊表述一律按“仍需新开 worktree”处理。
- 执行层：调用 `git worktree add` 创建新 branch 或附着已有 branch。
- 文档层：`--init-docs` 时检查 `doc/<module>/prd.md`、`doc/<module>/project.md` 与当日 `doc/devlog/YYYY-MM-DD.md`。
- harness 层：`--with-harness` 时在新 worktree 中后台触发 `./scripts/worktree-harness.sh up`，并把 bootstrap 日志与状态文件路径回传给上层；该路径继承 formal gameplay 的 active LLM preflight。
- 输出层：打印下一步命令；需要自动化时输出 JSON 摘要，并保证 JSON 模式下 stdout 纯净。

## 3. 关键接口 / 入口
- 主入口：`scripts/new-task-worktree.sh`
- 规则入口：`AGENTS.md`
- 后续验证入口：`scripts/worktree-harness.sh`

## 4. 约束与边界
- 默认只负责创建 worktree，不创建业务文档模板。
- 默认路径放在 repo 同级 `worktrees/` 目录，避免把 worktree 再嵌回仓库目录树。
- 允许显式覆盖 `--base`、`--branch`、`--path`，但默认命名必须保持稳定。
- 文档改动、脚本改动、测试改动、仅改话术都按“新需求”处理，不能因改动看起来小就复用已有 worktree。
- 只有用户明确授权复用当前 worktree 时才允许跳过 bootstrap；“先写一版”“先不要提交”“顺手改一下”都不构成例外授权。
- 若开工后才发现切错 worktree，必须立即说明并切走；“已经开始改了几行”不是例外。
- `--json` 模式只输出单个 JSON 对象，方便脚本消费。
- `--init-docs` 仅做存在性检查和下一步提示，不自动创建缺失文档。
- `--with-harness` 默认走当前 formal gameplay 的 LLM-required launcher path，且采用异步预热；人类输出写 bootstrap 日志，不污染 JSON stdout。

## 5. 设计演进计划
- 先落地标准入口与围栏。
- 再考虑补充模板化命名策略和模块初始化辅助。
- 如后续需要，再叠加与 harness / 编辑器的更深联动。
