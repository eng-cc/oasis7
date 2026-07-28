# oasis7 Runtime：DistFS 标准文件读写接口（项目管理文档）

- 对应设计文档: `doc/p2p/distfs/distfs-standard-file-io.design.md`
- 对应需求文档: `doc/p2p/distfs/distfs-standard-file-io.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] DFIO-1 (PRD-P2P-MIG-080)：设计文档与项目管理文档落地。
- [x] DFIO-2 (PRD-P2P-MIG-080)：实现 `FileStore` 与本地文件索引（`files_index.json`）。
- [x] DFIO-3 (PRD-P2P-MIG-080)：补齐单元测试并完成 crate 级回归。
- [x] DFIO-4 (PRD-P2P-MIG-080)：回写状态文档与 devlog。

## 依赖
- doc/p2p/distfs/distfs-standard-file-io.prd.md
- `crates/oasis7_distfs`
- `doc/p2p/prd.md`（分布式运行时与复制恢复合同）
- `README.md`（crate 分工）

## 状态
- 当前阶段：DistFS 标准文件读写接口阶段完成（DFIO-1~DFIO-4 全部完成）。
- 下一步：进入上层分布式能力链路，接入文件路径接口到 runtime/net 的调用面。
- 最近更新：2026-02-16。
