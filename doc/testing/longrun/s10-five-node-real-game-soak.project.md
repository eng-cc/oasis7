# oasis7：S10 五节点真实游戏数据在线长跑套件（项目管理文档）

- 对应设计文档: `doc/testing/longrun/s10-five-node-real-game-soak.design.md`
- 对应需求文档: `doc/testing/longrun/s10-five-node-real-game-soak.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] S10REAL-0 (PRD-TESTING-LONGRUN-S10REAL-001/002): 完成 S10 设计文档与项目管理建档。
- [x] S10REAL-1 (PRD-TESTING-LONGRUN-S10REAL-001): 实现 `scripts/s10-five-node-game-soak.sh` 五节点编排（启动/停止/输出目录/帮助/dry-run）。
- [x] S10REAL-2 (PRD-TESTING-LONGRUN-S10REAL-001/002): 落地 S10 指标门禁与 `summary.json/summary.md` 证据产物。
- [x] S10REAL-3 (PRD-TESTING-LONGRUN-S10REAL-002): 接入 `testing-manual.md` S10 章节并收口触发矩阵。
- [x] S10REAL-4 (PRD-TESTING-LONGRUN-S10REAL-002/003): 增加可控 `reward-runtime-epoch-duration-secs`，修复 `no_epoch_reports` 可观测性问题。
- [x] S10REAL-5 (PRD-TESTING-LONGRUN-S10REAL-002/003): 修复 `minted_records_empty`（结算发布条件 + mint 前置身份绑定）。
- [x] S10REAL-6 (PRD-TESTING-LONGRUN-S10REAL-003): 修复 no-LLM 长跑 stall（sequencer-only 执行 hook + 状态隔离 + auto-attest 策略）。
- [x] S10REAL-7 (PRD-TESTING-LONGRUN-S10REAL-003): 修复 execution bridge 非连续 committed 高度偶发卡死（容错续跑 + 单测）。
- [x] S10REAL-8 (PRD-TESTING-LONGRUN-S10REAL-002/003): 修复结算签名错误（node_id 派生 signer + CBOR 编码）。
- [x] S10REAL-9 (PRD-TESTING-LONGRUN-S10REAL-002): 将 settlement apply failure ratio 纳入硬门禁并输出统计字段。
- [x] S10REAL-10 (PRD-TESTING-LONGRUN-S10REAL-002): 连续多轮长跑验证（3x600s + 1x1200s）并形成发布结论。
- [x] S10REAL-11 (PRD-TESTING-004): 专题文档按 strict schema 人工迁移并统一 `.prd.md/.project.md` 命名。
- [x] s10-distfs-probe-authority (PRD-TESTING-LONGRUN-S10REAL-004) [test_tier_required]: 吸收 dated DistFS probe bootstrap 的空集 seed、幂等/非阻断和 `insufficient_data != pass` 合同；历史实现与 run 证据留在 Git/GitHub。 Trace: #2677 (task_1c1eef2779924e01832e86bebb74a6f2)

## 依赖
- doc/testing/longrun/s10-five-node-real-game-soak.prd.md
- `scripts/s10-five-node-game-soak.sh`
- `scripts/p2p-longrun-soak.sh`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `testing-manual.md`
- `doc/testing/prd.md`
- `doc/testing/project.md`

## 状态
- 更新日期：2026-03-03
- 当前阶段：已完成
- 阻塞项：无
- 下一步：无（项目收口完成）
- 证据边界：本项目完成态不证明当前 S10、release、public-testnet 或 mainnet readiness；每次结论仍需新的 summary/timeline/节点证据与完整门禁。
