# scripts 文档索引

审计轮次: 10

## 从这里开始
- 想先理解脚本模块的边界、门禁与维护口径：`doc/scripts/prd.md`
- 想看当前脚本治理任务与最近完成项：`doc/scripts/project.md`
- 想按专题文件名精确查 precommit / wasm / viewer-tools / governance 文档：`doc/scripts/prd.index.md`
- 想直接为新需求开独立 worktree：`scripts/new-task-worktree.sh` + `doc/scripts/governance/task-worktree-bootstrap-2026-03-27.prd.md`
- 想把已完成任务标准化合入本地 `main`：`scripts/land-task-worktree.sh` + `doc/scripts/governance/task-worktree-landing-2026-03-27.prd.md`
- 想预热隔离 harness 或理解 worktree 栈约束：`doc/scripts/governance/worktree-isolated-harness-2026-03-27.prd.md`

## 入口
- PRD: `doc/scripts/prd.md`
- 设计总览: `doc/scripts/design.md`
- 标准执行入口: `doc/scripts/project.md`
- 文件级索引: `doc/scripts/prd.index.md`

## 入口分工
- `README.md` 只承担 landing page 职责：帮助读者决定是先看规则、看项目状态，还是直接去某个高频脚本入口。
- `prd.md` 是脚本模块的权威规格入口，适合先理解主入口分层、参数契约、稳定性趋势与隔离约束。
- `project.md` 是执行台账，适合确认当前 worktree、landing、harness 等治理任务的完成状态。
- `prd.index.md` 是精确检索索引，适合已知专题名后按文件名直达，不适合作为第一次进入 scripts 模块时的首读入口。
- 高频脚本与治理专题承担主题真值：`new-task-worktree.sh` 负责新任务 bootstrap，`land-task-worktree.sh` 负责标准 landing，`worktree-isolated-harness` 负责隔离栈与状态文件约束。

## 模块职责
- 维护仓内高频脚本的主入口、参数契约与 fallback 围栏口径。
- 维护 worktree 级隔离 harness，让 agent / QA 能并行起栈并读取稳定状态文件。
- 维护标准化 task worktree bootstrap 与 landing 入口，让每个新需求按统一 branch/path 命名落到独立 worktree，并可选直接检查模块文档、预热 harness、标准化合入本地 `main`。
- 汇总 precommit、viewer-tools、wasm 与治理专题文档。
- 承接脚本稳定性趋势、文档门禁与运行约束收口。

## 主题文档
- `precommit/`：提交前检查与门禁策略。
- `viewer-tools/`：viewer 抓帧与纹理质检工具链路。
- `wasm/`：WASM 构建脚本与环境约束。
- `governance/`：脚本分层、参数契约、稳定性趋势、worktree harness 与 task worktree bootstrap 专题。

## 近期专题
- `doc/scripts/governance/script-entry-layering-2026-03-11.prd.md`
- `doc/scripts/governance/script-parameter-contracts-2026-03-11.prd.md`
- `doc/scripts/governance/script-stability-trend-tracking-2026-03-11.prd.md`
- `doc/scripts/governance/worktree-isolated-harness-2026-03-27.prd.md`
- `doc/scripts/governance/task-worktree-bootstrap-2026-03-27.prd.md`
- `doc/scripts/governance/task-worktree-landing-2026-03-27.prd.md`
- `doc/scripts/precommit/pre-commit.prd.md`
- `doc/scripts/viewer-tools/capture-viewer-frame.prd.md`

## 根目录收口
- 模块根目录主入口保留：`README.md`、`prd.md`、`design.md`、`project.md`、`prd.index.md`。
- 其余专题文档按主题下沉到 `precommit/`、`viewer-tools/`、`wasm/`、`governance/`。

## 维护约定
- 脚本行为变化需同步更新对应文档、测试口径与参数契约说明。
- 新增专题后，需同步回写 `doc/scripts/prd.index.md` 与本目录索引。
- `scripts/new-task-worktree.sh` 为新需求默认入口；`--init-docs` 用于检查模块 PRD / project / 当日 devlog，`--with-harness` 用于在新 worktree 中后台预热 `./scripts/worktree-harness.sh up --no-llm`。
- `scripts/land-task-worktree.sh` 为任务完成后的默认 landing 入口；负责在干净 task worktree 与干净本地 `main` worktree 之间执行标准化 rebase + fast-forward 合入，并输出必须执行的回收命令。
- 若默认高频脚本入口变化，需同步回写本目录“从这里开始”，避免 README 退化回纯专题目录页。
