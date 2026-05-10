# world-simulator PRD 分册：体验质量趋势跟踪

审计轮次: 5
## 目标
- 建立 simulator 体验质量的趋势化跟踪口径，避免只看单次通过/失败。
- 将 Web-first 闭环、LLM 链路、核心可玩性指标纳入同一追踪面板。
- 作为 `TASK-WORLD_SIMULATOR-004` 的交付物，为后续发布评审提供连续证据。

## 范围
- In Scope:
  - 指标定义（启动、交互、稳定性、LLM 行为）。
  - 数据来源（S6/S8 与相关脚本输出）。
  - 周期与归档规则（每日/每周基线）。
- Out of Scope:
  - 不在本任务实现自动可视化平台（Grafana/BI）。
  - 不修改现有测试脚本行为，仅定义追踪口径。

## 指标面板（v1）

| 维度 | 指标 | 目标/阈值 | 数据来源 |
| --- | --- | --- | --- |
| 启动可用性 | `web_launch_success_rate` | 最近 7 次 >= 99% | `viewer-primary-web-entry-regression` 摘要 |
| 闭环稳定性 | `web_console_error_count` | 每次 S6 必须为 0 | `output/playwright/viewer/console.log` |
| 交互有效性 | `semantic_step_pass_rate` | 最近 7 次 >= 95% | `output/playwright/viewer/release-qa-summary-*.md` |
| LLM 稳定性 | `llm_metric_gate_pass_rate` | 最近 7 次 >= 95% | `.tmp/llm_stress/*/summary.txt` |
| LLM 健康度 | `llm_parse_error_rate` | 每次 run <= 阈值（按脚本 gate） | `.tmp/llm_stress/*/report.json` |
| 玩法覆盖 | `llm_action_kind_count` | 满足 release-gate profile | `llm-longrun-stress` summary/report |

## 采集节奏
- 每日：至少 1 次 S6（Web-first 闭环）。
- 每周：至少 1 次 S8（`llm-longrun-stress`，建议 `--release-gate`）。
- 发布前：连续 3 天样本，且关键指标不低于阈值。

## 采集命令基线
- Web-first（S6）：
  - `./scripts/viewer-primary-web-entry-regression.sh`
  - `./scripts/viewer-software-safe-step-regression.sh`
- LLM（S8）：
  - `./scripts/llm-longrun-stress.sh --scenario llm_bootstrap --ticks 240 --release-gate --release-gate-profile hybrid`

## 归档约定
- Web 证据：`output/playwright/viewer/`
- LLM 证据：`.tmp/llm_stress/`
- 趋势汇总建议：`output/quality-trend/world-simulator/YYYY-MM-DD.md`

## 趋势记录模板
```md
### world-simulator quality trend (YYYY-MM-DD)
- web_launch_success_rate:
- web_console_error_count:
- semantic_step_pass_rate:
- llm_metric_gate_pass_rate:
- llm_parse_error_rate:
- llm_action_kind_count:
- 判定: healthy / warning / fail
- 证据路径:
```

## 风险
- 若只记录“是否通过”而不记录趋势值，无法识别退化拐点。
- 若 S6/S8 取样频率不足，趋势结论会失真。
