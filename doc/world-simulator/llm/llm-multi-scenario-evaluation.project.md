# oasis7 Simulator：LLM 多场景评测基线（项目管理文档）

- 对应设计文档: `doc/world-simulator/llm/llm-multi-scenario-evaluation.design.md`
- 对应需求文档: `doc/world-simulator/llm/llm-multi-scenario-evaluation.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] LMSO30A 输出设计文档（`doc/world-simulator/llm/llm-multi-scenario-evaluation.prd.md`）
- [x] LMSO30B 扩展压测脚本参数（`--scenario` 可重复 + `--scenarios`）
- [x] LMSO30C 实现多场景执行与分场景产物落盘
- [x] LMSO30D 实现聚合 report/summary 与阈值判定
- [x] LMSO30E 30 tick 多场景验证（最少 3 个场景）
- [x] LMSO30F 更新 README / devlog / 项目状态并收口
- [x] LMSO30G 增加多场景并行执行参数（`--jobs`）并完成冒烟验证
- [x] LMSO30H 执行 `--jobs 5` 的 5 场景 30 tick 并行回归

## 依赖
- `scripts/llm-longrun-stress.sh`
- `README.md`
- `doc/world-simulator/llm/llm-multi-scenario-evaluation.prd.md`
- `doc/world-simulator/llm/llm-agent-behavior.prd.md`（当前 tool-only、预算、repair 与 multi-segment collapse 合同）

## 状态
- 当前阶段：LMSO30（已完成）
- 目标：建立跨场景评测口径，降低单场景偏差，保障链路可用性与稳定性评估可信度。
- 最近更新：完成 3 场景 30 tick 回归与文档收口（2026-02-11）。

## 回归结果（2026-02-11）
- 命令：
  - `./scripts/llm-longrun-stress.sh --ticks 30 --scenarios llm_bootstrap,power_bootstrap,resource_bootstrap --out-dir .tmp/lmso30_multi_30 --no-llm-io --max-parse-errors 5 --max-repair-rounds-max 5`
- 结果聚合（`.tmp/lmso30_multi_30/summary.txt`）：
  - `action_success_total=86`，`action_failure_total=1`
  - `llm_errors_total=0`，`parse_errors_total=3`
  - `llm_input_chars_avg_mean=1399`，`llm_input_chars_max_peak=20680`
  - `module_call_total=8`，`plan_total=16`
- 分场景提示：
  - `power_bootstrap`、`resource_bootstrap` 仍出现 `parse_errors`，说明单场景（仅 `llm_bootstrap`）会低估解析波动风险。

## 补充回归（5 场景，2026-02-11）
- 命令：
  - `./scripts/llm-longrun-stress.sh --ticks 30 --scenarios llm_bootstrap,power_bootstrap,resource_bootstrap,twin_region_bootstrap,triad_region_bootstrap --out-dir .tmp/lmso30_multi_5scenes_30 --no-llm-io --max-parse-errors 5 --max-repair-rounds-max 5`
- 聚合结果（`.tmp/lmso30_multi_5scenes_30/summary.txt`）：
  - `action_success_total=140`，`action_failure_total=9`
  - `llm_errors_total=0`，`parse_errors_total=0`
  - `llm_input_chars_avg_mean=2072`，`llm_input_chars_max_peak=19584`
  - `module_call_total=29`，`plan_total=32`
- 结论：
  - 单次 3 场景与单次 5 场景的 `parse_errors_total` 存在波动，后续应采用“固定场景集 + 重复采样”验收。

## 并行能力冒烟（2026-02-11）
- 命令：
  - `./scripts/llm-longrun-stress.sh --ticks 1 --scenarios llm_bootstrap,power_bootstrap --jobs 2 --out-dir .tmp/lmso30_parallel_smoke --no-llm-io --max-parse-errors 5 --max-repair-rounds-max 5`
- 结果：
  - 并行执行成功，聚合 `summary.txt` 输出 `jobs=2`，聚合 `report.json` 包含 `jobs` 字段。

## 并行全量回归（2026-02-11）
- 命令：
  - `./scripts/llm-longrun-stress.sh --ticks 30 --scenarios llm_bootstrap,power_bootstrap,resource_bootstrap,twin_region_bootstrap,triad_region_bootstrap --jobs 5 --out-dir .tmp/lmso30_multi_5scenes_30_jobs5 --no-llm-io --max-parse-errors 5 --max-repair-rounds-max 5`
- 聚合结果（`.tmp/lmso30_multi_5scenes_30_jobs5/summary.txt`）：
  - `action_success_total=138`，`action_failure_total=7`
  - `llm_errors_total=0`，`parse_errors_total=4`
  - `llm_input_chars_avg_mean=1948`，`llm_input_chars_max_peak=20903`
  - `module_call_total=21`，`plan_total=35`
