# Fix Pre-commit（预提交失败修复脚本）

- 对应设计文档: `doc/scripts/precommit/precommit-remediation-playbook.design.md`
- 对应项目管理文档: `doc/scripts/precommit/precommit-remediation-playbook.project.md`

审计轮次: 4


## 目标
- 提供一个一键修复入口，处理本地 `pre-commit` 常见失败（重点是 Rust 格式化不一致）。
- 将“修复 + 复检”流程标准化，减少重复手工命令。

## 范围
- **范围内**：
  - 新增 `scripts/fix-precommit.sh`。
  - 执行全仓 Rust 格式化并将变更重新加入暂存区。
  - 调用既有 `scripts/pre-commit.sh` 做 commit baseline 复检，并保留显式 `./scripts/ci-tests.sh required` 作为较重门禁的手动补跑入口。
- **范围外**：
  - 不修改 `pre-commit` hook 安装方式。
  - 不新增 lint/type-check 等其他检查项。

## 接口 / 数据
- 脚本路径：`scripts/fix-precommit.sh`
- 调用方式：`./scripts/fix-precommit.sh`
- 归属说明：本专题是 pre-commit 失败修复流程的唯一权威入口；`pre-commit.prd.md` 仅保留跳转引用。
- 执行顺序：
  1. `env -u RUSTC_WRAPPER cargo fmt --all`
  2. `git add -u`（将已跟踪文件的格式化结果加入暂存区）
  3. `./scripts/pre-commit.sh`（内部使用 `rustfmt --edition 2021` 处理暂存 Rust 文件，并执行 `./scripts/ci-tests.sh commit`）
  4. 若需要补跑重门禁：`./scripts/ci-tests.sh required`

## 里程碑
- **M1**：输出设计文档与项目管理文档。
- **M2**：实现修复脚本并完成可执行校验。
- **M3**：更新任务日志并回填状态。

## 风险
- **执行耗时**：默认只会触发 commit baseline；若再补跑 `./scripts/ci-tests.sh required`，总耗时取决于机器性能与 `oasis7` required shard 规模。
- **暂存区变化**：`git add -u` 会更新已跟踪文件的暂存状态，提交前需再次确认 diff。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
