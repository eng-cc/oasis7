@/Users/scc/.codex/RTK.md

# Oasis7 Agent Entry Point

通用规则见 [开发流程规范](doc/engineering/workflow/source-of-truth.md)。主执行者负责分析、实现、自测和组织适用评审，并在满足规范、必要检查及用户授权后推进合入和交付；专业协作按复杂度与风险使用。

- 在独立分支修改；不在 main 或主 worktree 编辑。保留用户和其他执行者的工作。
- 工作区先检查 `git worktree list --porcelain` 并优先复用干净、未占用的固定路径；确需新增使用 `./scripts/new-task-worktree.sh <module> <task> --base <ref>`，遵循 `oasis7.task-worktrees-root` 配置。除用户明确指定外，不用 App `create_worktree`、worktree fork 或 handoff 绕过仓库入口。
- 首次本地编译前执行 `./scripts/cargo-dev.sh --prepare-target`；已有普通 target 不自动删除。日常反馈用开发 wrapper，正式验收保留 canonical 命令和参数。
- `third_party/` 只读；Rust 工作参考 `third_party/rust-skills/AGENTS.md`。
- 一个 PR 对应一个清楚目的，可原子修改多个 crate；Issue 和 Project 按需使用。
- CI 控制、安全和兼容边界变化需要独立评审。专业关注点见 `.agents/roles/`，方法见 `.agents/skills/README.md`。

## 常用命令

- Rust canonical：`env -u RUSTC_WRAPPER cargo ...`
- 本地开发：`./scripts/cargo-dev.sh ...`
- 使用共享 Cargo 缓存，等待锁，不修改 `CARGO_TARGET_DIR`。
- 测试：`testing-manual.md`；UI/Web 见 S6。
- Agent 游戏功能验证须按[规范触发范围](doc/engineering/workflow/source-of-truth.md#agent-gameplay-selftest)执行[自测操作手册](doc/testing/manual/agent-runtime-self-playtest.manual.md)：主 Agent 组织独立试玩、读取正式游戏 Skill、核对世界结果并修复复测；缺前置如实报告，普通 CI 不代签实玩。
- 快捷检查：`./scripts/pre-commit.sh`；文档：`./scripts/doc-governance-check.sh`。
- PR：`gh pr create`、`gh pr checks <pr> --required`。
- 合入：满足适用评审、检查及用户限制后，执行 `gh pr merge <pr> --squash --match-head-commit <expected-head>`。

PR 标题和 commit message 使用英文。
