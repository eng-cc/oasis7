# world-simulator PRD 分册：统一验收清单（场景 / Viewer / 启动器）

审计轮次: 6
## 目标
- 将场景系统、Viewer Web 闭环、启动器能力的验收口径统一为一份可执行清单，避免各链路各自为政。
- 让 `PRD-WORLD_SIMULATOR-001/002` 在同一验收表里可追踪、可复跑、可留证据。
- 作为 `TASK-WORLD_SIMULATOR-002` 的交付物，供后续 `TASK-WORLD_SIMULATOR-003/004` 复用。

## 范围
- In Scope:
  - 场景系统基线（scenario matrix）验收命令与通过标准。
  - Viewer Web-first 闭环（agent-browser）验收命令与证据标准。
  - 启动器（统一启动 + 客户端启动器）核心路径验收命令与通过标准。
  - 统一证据模板（命令、产物路径、结论）。
- Out of Scope:
  - 不替代 `testing-manual.md` 的分层全量手册。
  - 不新增业务功能或新的测试框架。

## 统一验收门禁（2026-03-03 基线）

| Gate-ID | 模块 | 通过标准 | 必跑命令（示例） | 证据要求 |
| --- | --- | --- | --- | --- |
| G1 | 场景系统 | 场景 ID 与模板稳定，核心场景可回放 | `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required scenario_specs_match_ids -- --nocapture`<br>`env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required scenarios_are_stable -- --nocapture` | 终端日志（通过/失败） |
| G2 | Viewer Web 闭环 | 页面可加载、至少 1 张截图、主入口与 `software_safe` 状态可追溯 | 按 `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md` 执行 S6（如 `./scripts/viewer-primary-web-entry-regression.sh`、`./scripts/viewer-software-safe-step-regression.sh`） | `output/playwright/*.png` + `output/playwright/viewer/console.log` |
| G3 | 启动器统一入口 | launcher 参数解析与 URL 组装路径通过 | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher` | 终端日志（通过/失败） |
| G4 | 启动器客户端 | 配置校验、链路提交流程可执行 | `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher` | 终端日志（通过/失败） |
| G5 | 文档追踪 | PRD 与 project 状态一致、依赖可达 | `./scripts/doc-governance-check.sh` | 治理脚本输出（OK） |

## 执行顺序
1. 先跑 G5（文档治理）确认验收清单版本有效。
2. 跑 G1（场景系统）确认输入基线稳定。
3. 跑 G3 + G4（启动器两条链路）确认启动与交互入口有效。
4. 最后跑 G2（Viewer Web 闭环）沉淀截图与 console 证据。

## PRD-ID 映射
- `PRD-WORLD_SIMULATOR-001`:
  - G1（场景稳定性）
  - G3/G4（启动器链路稳定性）
- `PRD-WORLD_SIMULATOR-002`:
  - G2（Web-first Viewer 闭环）
  - G5（文档与流程一致性）

## 证据模板入口
- Web-first 与 LLM 统一证据卡模板：`doc/world-simulator/prd/acceptance/web-llm-evidence-template.md`
- 体验质量趋势跟踪入口：`doc/world-simulator/prd/quality/experience-trend-tracking.md`

## 证据记录模板
```md
- Gate-ID:
- 命令:
- 结果: pass/fail
- 证据路径:
- 备注:
```

## 风险与约束
- 若仅运行 G1/G3/G4 而缺失 G2，会产生“功能绿灯但 UI 闭环未验证”的误判。
- 若 G2 只截图不看 console，会遗漏运行时错误噪声回归。
- 若清单版本与 `testing-manual.md` 口径漂移，应以 `testing-manual.md` 为主并回写本清单。
