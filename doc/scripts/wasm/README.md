# scripts WASM 文档入口

## 从这里开始

- 需要发布级 deterministic WASM 构建、Docker builder、receipt、身份或跨宿主 evidence：先读 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`。
- 需要追溯旧 nightly + `-Z build-std` builtin WASM 脚本的参数、CI 环境约束或 hash 漂移治理背景：按需读取下方 historical triplet。
- 需要按文件名精确定位本子域材料：使用 `doc/scripts/prd.index.md`；它不是本子域的首读入口。

## 阅读边界

本目录的 `builtin-wasm-nightly-build-std` triplet 是已完成的历史实现证据，不是发布链路入口。它不定义 Docker-only publishable build、build receipt、single canonical token 或 cross-host evidence；这些现行契约统一由 world-runtime WASM canonical pipeline 管理。

| 文档 | 用途 | 状态 |
| --- | --- | --- |
| `builtin-wasm-nightly-build-std.prd.md` | 旧 build-std 输入、参数与 hash 漂移治理背景 | historical only |
| `builtin-wasm-nightly-build-std.design.md` | 旧脚本切片的设计边界 | historical only |
| `builtin-wasm-nightly-build-std.project.md` | 已完成任务与当时依赖记录 | historical only |

## 保留与维护规则

- 该 triplet 仍被 scripts 项目记录、world-runtime 历史入口和 core 审计记录精确引用，保留为可追溯证据，不删除。
- 新的发布级 WASM 设计或任务不得回写到本目录；应绑定 world-runtime WASM canonical pipeline。
- 若历史脚本参数需要更正，只改对应 historical 文档，并保持其“非发布入口”边界明确。
