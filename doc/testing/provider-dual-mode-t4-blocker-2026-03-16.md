# Local Provider 双轨模式 T4 阻断记录（2026-03-16）

- owner: `qa_engineer`
- 关联 PRD: `PRD-WORLD_SIMULATOR-040`
- 关联任务: `doc/world-simulator/llm/llm-provider-agent-dual-mode-2026-03-16.project.md` T4
- 联审建议: `producer_system_designer`、`agent_engineer`、`runtime_engineer`、`viewer_engineer`
- 文档状态: `resolved`（2026-03-17，`TASK-WORLD_SIMULATOR-152`）

## 1. 结论
- `TASK-WORLD_SIMULATOR-152` 已把真实 `player_parity` 执行 lane 接到 runtime live / launcher / parity bench / `oasis7`。
- 当前已经能分别对 `headless_agent` 与 `player_parity` 路径给出真实 Local Provider smoke 通过证据。
- T4 不再被“`player_parity` 未接线”阻断；当前状态改为 `pending / ready_for_qa`，等待 `qa_engineer` / `producer_system_designer` 基于真实双样本给出正式对照结论。
- 按 `PRD-CORE-009`，本文件中的“默认模式策略”特指 Local Provider execution lane 默认值，不等同于玩家访问模式默认入口；对外结论仍需先绑定 `software_safe` 或 `pure_api` 之一。

## 2. 已验证证据
### 2.1 环境准备
- `provider --version` 返回 `Local Provider 2026.3.13 (61d171a)`。
- `curl -sS http://127.0.0.1:18789/health` 返回 `{"ok":true,"status":"live"}`。
- 历史 `oasis7-run.sh doctor --json` 初次检查显示 Gateway 正常，但 `http://127.0.0.1:5841` bridge 未启动。

### 2.2 headless_agent 真实 smoke
- 历史执行：repo-local `oasis7-run.sh smoke --samples 1 --ticks 4 --timeout-ms 15000`
- 历史结果：自动拉起 bridge 后，`provider_loopback_http` smoke 通过。
- 历史产物：
  - `output/provider_parity/provider_parity_20260316_235632/summary/P0-001.provider_loopback_http.json`
  - `output/provider_parity/provider_parity_20260316_235632/samples/provider_loopback_http/sample_1/summary/P0-001.provider_loopback_http.json`
  - `output/provider_parity/provider_parity_20260316_235632/samples/provider_loopback_http/sample_1/raw/P0-001_sample_1.provider_loopback_http.jsonl`
- 历史关键指标：
  - `status=passed`
  - `goal_completed=1`
  - `decision_steps=4`
  - `invalid_action_count=0`
  - `timeout_count=0`
  - `trace_completeness_ratio_ppm=1000000`

### 2.3 2026-03-17 双模式复验
- headless 执行：repo-local `oasis7-run.sh smoke --samples 1 --ticks 4 --timeout-ms 15000 --execution-mode headless_agent`
- player parity 执行：repo-local `oasis7-run.sh smoke --samples 1 --ticks 4 --timeout-ms 15000 --execution-mode player_parity`
- headless 产物：
  - `output/provider_parity/provider_parity_20260317_002147/summary/P0-001.provider_loopback_http.json`
  - `output/provider_parity/provider_parity_20260317_002147/samples/provider_loopback_http/sample_1/summary/P0-001.provider_loopback_http.json`
- player parity 产物：
  - `output/provider_parity/provider_parity_20260317_002217/summary/P0-001.provider_loopback_http.json`
  - `output/provider_parity/provider_parity_20260317_002217/samples/provider_loopback_http/sample_1/summary/P0-001.provider_loopback_http.json`
- 两条链路共同指标：
  - `status=passed`
  - `goal_completed=1`
  - `decision_steps=4`
  - `invalid_action_count=0`
  - `timeout_count=0`
  - `trace_completeness_ratio_ppm=1000000`

## 3. 已解除的根因
### 3.1 代码修复点
以下位置现在已经支持真实 `player_parity` lane：
- `crates/oasis7/src/viewer/runtime_live/llm_sidecar.rs`：通过 `OASIS7_AGENT_PROVIDER_EXECUTION_MODE` 解析并透传 runtime live Local Provider execution mode。
- `crates/oasis7/src/bin/oasis7_game_launcher.rs`：新增 `--agent-execution-lane`，把 execution mode 透传给 `oasis7_viewer_live`。
- `crates/oasis7/src/bin/oasis7_provider_parity_bench.rs`：新增 `--execution-mode`，真实 Local Provider parity bench 不再固定为 `headless_agent`。
- `scripts/provider-parity-p0.sh` 与历史 repo-local `oasis7-run.sh`：新增 execution mode 参数并贯通 smoke / play 操作路径。

### 3.2 对 T4 的当前影响
- “缺少真实 `player_parity` lane”这一代码阻断已经解除。
- T4 已完成正式执行；双模式默认策略与阻断结论已在本文件第 4 节冻结。

## 4. QA / Producer 对照结论（2026-03-17）
### 4.1 同场景样本对比
- 对照场景：`P0-001` / `llm_bootstrap` / `seed=5` / `4 ticks` / `agent_profile=oasis7_p0_low_freq_npc`。
- `headless_agent`：`completion_time_ms=23894`、`decision_steps=4`、`invalid_action_count=0`、`timeout_count=0`、`trace_completeness_ratio_ppm=1000000`、`median_latency_ms=5890`、`p95_latency_ms=6316`。
- `player_parity`：`completion_time_ms=23829`、`decision_steps=4`、`invalid_action_count=0`、`timeout_count=0`、`trace_completeness_ratio_ppm=1000000`、`median_latency_ms=5914`、`p95_latency_ms=6236`。
- 差值结论：`decision_steps`、`invalid_action_count`、`timeout_count`、`trace_completeness_ratio_ppm`、`context_drift_count` 全部一致；`median_latency_ms` 差值 `+24ms`，`p95_latency_ms` 差值 `-80ms`，未观察到足以改变玩法判定的体验偏差。

### 4.2 QA 结论
- 结论：`pass`。当前样本已满足 `PRD-WORLD_SIMULATOR-040` T4 对“同一 Local Provider 场景、双模式对照采证、输出默认模式与阻断结论”的要求。
- 风险评语：本轮结论只覆盖 `P0-001` 单场景、单样本，不替代 `PRD-WORLD_SIMULATOR-038` 对 builtin/Local Provider 多样本 parity 的统计门禁。
- 回归口径：`headless_agent` 继续作为无 GUI / 无 GPU / 无浏览器环境下的回归主链路；`player_parity` 保留为体验对照 lane。

### 4.3 Producer 结论
- 默认模式策略：冻结 `headless_agent` 为 CI / server / 自动回归默认模式；冻结 `player_parity` 为制作人 / QA 的体验对照与准入门禁；`debug_viewer` 继续保持 observer-only 旁路层。
- 放行边界：只有当后续 `player_parity` 在目标场景内持续维持与 `headless_agent` 一致的结果口径时，才允许把“玩家体验等价”口径外推到 `PRD-WORLD_SIMULATOR-038`。

## 5. 后续建议
- 下一必需动作：推进 `PRD-WORLD_SIMULATOR-038` 的真实 builtin/Local Provider parity 扩面采证与 QA/producer 双签。
- 当前口径：
  - `headless_agent`：继续作为无 GUI / 无 GPU 回归主链路。
  - `debug_viewer`：继续作为旁路观战/解释层。
  - `player_parity`：作为体验对照 lane 保留，不作为默认回归主链路。
  - 如需形成玩家入口结论，需另行补记对应的 `software_safe` 或 `pure_api` 玩家访问模式，不得仅凭 lane 名称对外宣称。
