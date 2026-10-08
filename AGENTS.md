@/Users/scc/.codex/RTK.md

# Oasis7 Agent Entry Point

通用规则见 [开发流程规范](doc/engineering/workflow/source-of-truth.md)。主执行者可直接分析、实现、自测；专业协作按复杂度与风险使用。

- 在独立分支修改；不在 main 或主 worktree 编辑。保留用户和其他执行者的工作。
- `third_party/` 只读；Rust 工作参考 `third_party/rust-skills/AGENTS.md`。
- 一个 PR 对应一个清楚目的，可原子修改多个 crate；Issue 和 Project 按需使用。
- CI 控制、安全和兼容边界变化需要独立评审。专业关注点见 `.agents/roles/`，方法见 `.agents/skills/README.md`。

## 常用命令

- Rust canonical：`env -u RUSTC_WRAPPER cargo ...`
- 本地开发：`./scripts/cargo-dev.sh ...`
- 使用共享 Cargo 缓存，等待锁，不修改 `CARGO_TARGET_DIR`。
- 测试：`testing-manual.md`；UI/Web 见 S6。
- 快捷检查：`./scripts/pre-commit.sh`；文档：`./scripts/doc-governance-check.sh`。
- PR：`gh pr create`、`gh pr checks <pr> --required`。
- 满足适用评审、检查及用户限制后：`gh pr merge <pr> --squash --match-head-commit <expected-head>`。

PR 标题和 commit message 使用英文。
