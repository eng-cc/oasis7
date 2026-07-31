# scripts WASM 文档入口

## 从这里开始

- 需要发布级 deterministic WASM 构建、Docker builder、receipt、身份或跨宿主 evidence：先读 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`。
- 需要追溯旧 nightly + `-Z build-std` builtin WASM 脚本的参数、CI 环境约束或 hash 漂移治理背景：读 `doc/world-runtime/wasm/evidence.md#historical-nightly-build-std-provenance`。
- 需要按文件名精确定位本子域材料：使用 `doc/scripts/prd.index.md`；它不是本子域的首读入口。

## 阅读边界

早期 nightly build-std 专题已吸收并删除：其仍有长期价值的参数、CI 约束、hash 漂移背景和风险保留在 WASM evidence 的 historical provenance 章节；可变完成记录由 Git history 与 GitHub task evidence 承接。它不定义 Docker-only publishable build、build receipt、single canonical token 或 cross-host evidence；这些现行契约统一由 world-runtime WASM canonical pipeline 管理。

新的发布级 WASM 设计或任务不得回写到本目录；应绑定 world-runtime WASM canonical pipeline。历史参数若需更正，也应只修正 successor 的 historical record，并维持其非发布入口边界。
