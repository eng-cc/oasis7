# viewer-live 文档入口

本目录保留 `oasis7_viewer_live` 的两项已完成 CLI 语义变更，供历史追溯和精确检索；它们不是当前 launcher 或 gameplay 的默认操作入口。

## 从这里开始

- 想确认当前 `oasis7_viewer_live` 的职责、允许参数与 legacy 参数拒绝语义：先读 [`doc/p2p/prd.md`](../prd.md) 的 Viewer 控制面边界。
- 想运行 Viewer、做 Web 回归或区分 formal gameplay 与 observer/debug：先读 [`doc/world-simulator/viewer/viewer-manual.manual.md`](../../world-simulator/viewer/viewer-manual.manual.md) 与 [`testing-manual.md`](../../../testing-manual.md)。
- 想追溯“默认启用 LLM”的完成记录：读 [`oasis7-viewer-live-llm-default-on-2026-02-23.prd.md`](oasis7-viewer-live-llm-default-on-2026-02-23.prd.md) 及同名 design/project。
- 想追溯 `--no-llm` 的显式 observer/debug 回退语义：读 [`oasis7-viewer-live-no-llm-flag-2026-02-23.prd.md`](oasis7-viewer-live-no-llm-flag-2026-02-23.prd.md) 及同名 design/project。

## 阅读边界

- 两组专题的 project 均标记为完成；保留它们是为了 PRD/design/project 配对完整性、审计记录和文件名检索，不表示仍有独立执行队列。
- 当前正式 gameplay 默认走 active LLM path；`--no-llm` 仅适用于直接 `oasis7_viewer_live` 的 observer/debug 诊断，不能作为 launcher 成功或正式 gameplay 的证据。
- 当前行为真值由 `doc/p2p/prd.md`、Viewer 手册和测试手册共同维护。这里不复述 CLI 全量参数表或旧 release/node 控制面内容。

## 维护约定

新增或改变 `oasis7_viewer_live` 的实际 CLI / launcher 行为时，先更新模块 PRD 与现行 Viewer 手册；仅在需要保留可审计的专题变更记录时才新增本目录文档。目录与索引职责遵循 [`doc/engineering/doc-governance/doc-structure-standard.design.md`](../../engineering/doc-governance/doc-structure-standard.design.md)。
