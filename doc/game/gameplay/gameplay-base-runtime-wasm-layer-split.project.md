# Gameplay Base Runtime / WASM Layer Split（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.prd.md`
审计轮次: 4

## 审计备注
- 主项目入口：`doc/game/gameplay/gameplay-top-level-design.project.md`
- 本文仅维护增量任务。

## 任务拆解

### T0 建档
- [x] 新建设计文档：`doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.prd.md`
- [x] 新建项目管理文档：`doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.project.md`

### T1 Runtime 分层改造
- [x] 新增基础层实现文件，迁移模块通用治理校验逻辑
- [x] 新增 gameplay 层实现文件，迁移 gameplay 契约/冲突/就绪度逻辑
- [x] 调整 `world/mod.rs` 模块组织，形成显式分层入口

### T2 回归与收口
- [x] 运行 `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::gameplay:: -- --nocapture`
- [x] 运行 `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::modules:: -- --nocapture`
- [x] 回写项目文档状态与 `doc/devlog/README.md`

## 依赖
- `doc/game/gameplay/gameplay-engineering-architecture.md`
- `doc/game/gameplay/gameplay-runtime-governance-closure.prd.md`
- `testing-manual.md`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2
- 阻塞项：无

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
