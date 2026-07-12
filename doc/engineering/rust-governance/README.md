# Rust 体量治理文档入口

## 从这里开始

- 想了解当前 Rust 文件体量、结构切片与 required gate 的治理边界：先读 [Rust 1200 行根治治理 PRD](rust-1200-line-root-cause-governance-2026-03-29.prd.md)。
- 想查看职责拆分原则、扫描模型与实现约束：读同专题的 [设计文档](rust-1200-line-root-cause-governance-2026-03-29.design.md)。
- 想追溯治理批次、完成记录、当前门禁状态或验证链路：读同专题的 [项目记录](rust-1200-line-root-cause-governance-2026-03-29.project.md)。

## 目录职责

- 本目录只承载 Rust 文件体量与结构切片治理的正式专题三件套。
- `README.md` 负责问题分流；PRD / design / project 分别保留规则、实现边界与执行证据，不复制其正文。
- 当前运行时真值是 `scripts/check-rust-file-size.sh`；required gate 以该脚本的实际扫描结果为准。

## 历史边界

- 已退休的 2026-02 oversized Rust file splitting round3 三件套不在本目录恢复；其审读证据保留于 `doc/core/reviews/round-*` 与 git history。
- 当前专题仍由 engineering 根入口、文件级索引与 project ledger 直接引用，故保留全部三件套，不进行删除。
