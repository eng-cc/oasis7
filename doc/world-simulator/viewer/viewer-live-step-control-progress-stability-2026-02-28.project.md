# Viewer Live `step` 控制推进稳定性修复（2026-02-28）项目管理文档

- 对应设计文档: `doc/world-simulator/viewer/viewer-live-step-control-progress-stability-2026-02-28.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-live-step-control-progress-stability-2026-02-28.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] T1 复现 `step accepted 但无推进` 并定位到 live+consensus 处理链路
- [x] T2 修复 paused 会话下 `ConsensusCommitted` 未推进的问题
- [x] T3 为 consensus 路径 step 增加短时重试以缓解瞬时无回放
- [x] T4 新增回归测试并执行 targeted test_tier_required
- [x] T5 执行 A/B 实机复测并记录结果

## 依赖
- doc/world-simulator/viewer/viewer-live-step-control-progress-stability-2026-02-28.prd.md
- `scripts/run-game-test-ab.sh`
- `scripts/run-game-test.sh`
- `crates/oasis7/src/viewer/live_split_part1.rs`
- `crates/oasis7/src/viewer/live/tests.rs`
- `doc/playability_test_result/game-test.prd.md`

## 状态
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 当前阶段：已完成（T1~T5）
- 结论：
  - 根因链路可稳定复现；修复后相关单测通过。
  - 实机 A/B 复测中 B 段仍有波动（两次 step 命中在不同轮次互换），说明问题从“路径性阻断”收敛为“异步时延波动”，需后续协议层语义增强。

## 风险
- `step` 当前仍是异步提交语义，`accepted` 不等于立即可见推进。
- 仅靠 server 侧重试难以保证每次 `step` 在固定窗口内都有 world delta。

## 后续候选
- 引入 `step` completion ack（按 request_id 回传“已推进/超时无推进”）。
- 将控制反馈从帧计数阈值改为 wall-clock 阈值，减少设备帧率差异造成的恢复时机抖动。
