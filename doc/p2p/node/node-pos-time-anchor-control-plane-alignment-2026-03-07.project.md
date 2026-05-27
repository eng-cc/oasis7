# oasis7 Runtime：PoS 时间锚定控制面参数与可观测口径对齐（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-pos-time-anchor-control-plane-alignment-2026-03-07.design.md`
- 对应需求文档: `doc/p2p/node/node-pos-time-anchor-control-plane-alignment-2026-03-07.prd.md`

审计轮次: 2

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-P2P-010-T0 (PRD-P2P-NODE-SURFACE-001/002/003) [test_tier_required]: 完成专题 PRD 与项目管理建档，并回写 `doc/p2p/prd.md` / `doc/p2p/project.md` / `doc/p2p/prd.index.md`。
- [x] TASK-P2P-010-T1 (PRD-P2P-NODE-SURFACE-001) [test_tier_required]: `oasis7_chain_runtime` 新增 PoS 时间锚定参数解析与 `NodePosConfig` 映射，并明确 `node_tick_ms` 轮询语义。
- [x] TASK-P2P-010-T2 (PRD-P2P-NODE-SURFACE-002) [test_tier_required]: launcher UI/配置字段扩展与参数透传对齐，补齐校验提示。
- [x] TASK-P2P-010-T3 (PRD-P2P-NODE-SURFACE-003) [test_tier_required]: 更新 longrun/s10 脚本、release 示例与相关文档口径；保持旧参数兼容。
- [x] TASK-P2P-010-T4 (PRD-P2P-NODE-SURFACE-001/002/003) [test_tier_required + test_tier_full]: 补齐定向测试与闭环回归，收口模块项目状态。

## 依赖
- `doc/p2p/node/node-pos-time-anchor-control-plane-alignment-2026-03-07.prd.md`
- `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.prd.md`
- `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.prd.md`
- `crates/oasis7/src/bin/oasis7_chain_runtime.rs`
- `crates/oasis7/src/bin/oasis7_game_launcher.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher.rs`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `crates/oasis7_client_launcher/src/launcher_core.rs`
- `crates/oasis7_client_launcher/src/llm_settings.rs`
- `crates/oasis7_client_launcher/src/llm_settings_web.rs`
- `crates/oasis7_launcher_ui/src/lib.rs`
- `scripts/p2p-longrun-soak.sh`
- `scripts/s10-five-node-game-soak.sh`
- `oasis7_viewer_live.release.example.toml`

## 状态
- 更新日期: 2026-03-08
- 当前状态: completed
- 下一任务: 无
- 阻塞项: 无
- 进展: `TASK-P2P-010-T0~T4` 全部完成，已完成控制面参数透传、脚本示例对齐、required/full 定向回归与任务收口。
- 进展（2026-03-08）: 已回写残留文档口径，明确 launcher 控制面为 `oasis7_chain_runtime/oasis7_game_launcher/oasis7_web_launcher/oasis7_client_launcher`，`oasis7_viewer_live` 仅保留能力边界说明。
- 说明: 本文档仅维护执行计划与任务状态；实施过程记录写入 `doc/devlog/README.md`。
