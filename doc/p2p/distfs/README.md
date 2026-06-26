# DistFS 子域入口

## 从这里开始
- 想确认 DistFS 在 P2P/consensus/execution/observer 组合后的整体闭环、测试层级与 claim boundary：先读 `testing-manual.md#s9a链上大世界状态底座自闭环`；本页只负责 DistFS 子域入口。
- 想理解 DistFS 生产化增强的基线语义：先读 `distfs-production-hardening-phase1.prd.md`。
- 想查 Phase 2-9 的阶段增量：从 `distfs-production-hardening-phase1.prd.md` 的 ROUND-002 主从口径进入，再按具体 phase 文件名下钻。
- 想查 self-healing 边界：先读 `distfs-self-healing-control-plane-2026-02-23.prd.md`，再进入 polling-loop / runtime-polling-wiring 增量子文档。
- 想查路径索引、标准文件 IO、builtin wasm storage/API：直接按专题名进入对应 PRD / design / project 三件套。

## 阅读面边界
- 本页只做 DistFS 子域分流，不复制 `doc/p2p/prd.index.md` 的完整三件套长表。
- DistFS 专题的 blob closure、复制、恢复或 self-healing green 结果只能作为链上大世界状态底座的存储层证据；模块级闭环以 S9A 的 `module_required / module_full / integration_required / release_full` 分层为准。
- `distfs-production-hardening-phase1` 是 production-hardening phase 组的默认主入口；phase2-9 是阶段增量和追溯入口，不作为首读顺序。
- self-healing control-plane 是 self-healing 组的默认主入口；polling-loop 与 runtime-polling-wiring 只承载增量差异。
- 完整文件级检索仍回到 `doc/p2p/prd.index.md`。

## 主从/增量组
| 组 | 默认入口 | 增量入口 |
| --- | --- | --- |
| DistFS production hardening | `distfs-production-hardening-phase1.prd.md` | `distfs-production-hardening-phase2.prd.md` 到 `distfs-production-hardening-phase9.prd.md` |
| DistFS self-healing | `distfs-self-healing-control-plane-2026-02-23.prd.md` | `distfs-self-healing-polling-loop-2026-02-23.prd.md`, `distfs-self-healing-runtime-polling-wiring-2026-02-23.prd.md` |

## 维护规则
- 新增 DistFS 专题时，先判断是否属于已有主从/增量组；属于增量时只在本页和 `doc/p2p/prd.index.md` 的主从说明中补入口，不把它提升为默认首读文档。
- 如果增量文档开始承载新的通用边界，应先拆出新的主入口，再更新本页分流。
