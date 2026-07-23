# Rust 文件结构治理入口

本目录只维护 Rust 文件体量与结构切片的当前治理契约：

- [Rust 1200 行与结构切片治理](rust-1200-line-root-cause-governance-2026-03-29.prd.md)

运行时真值由 `scripts/check-rust-file-size.sh` 产生，required gate 通过
`scripts/ci-tests.sh required` 调用该脚本。文档不复制扫描清单或完成任务
ledger，避免一次性 burn-down 记录重新成为当前规则。

2026-02 round3、2026-03 burn-down 批次及已删除 baseline 的历史证据保留在
GitHub task、Git history 与 git history；它们不是 active task
入口，也不在本目录恢复。
