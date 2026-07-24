# `oasis7_viewer_live` 决策模式（项目与历史追溯）

> 对应需求: `doc/p2p/viewer-live/oasis7-viewer-live-decision-mode.prd.md`
> 对应设计: `doc/p2p/viewer-live/oasis7-viewer-live-decision-mode.design.md`

## 状态

- 状态：`documented_current_authority`。
- 本轮完成：将 LLM 默认与 `--no-llm` 语义收敛到稳定三件套，并把子域入口和 P2P 文件索引指向该 authority。
- 本轮未做：不修改 CLI、launcher、runtime、Web UI、DOM、测试或 release 配置。

## 历史范围与当前归属

| 历史专题 | 已完成范围 | 当前归属 |
| --- | --- | --- |
| 2026-02 默认 LLM 历史专题 | 默认 `llm_mode=true`、帮助文案与参数解析回归 | 默认 LLM 合同。 |
| 2026-02 `--no-llm` 历史专题 | 显式 `--no-llm` 解析与诊断路径 | observer/debug-only Script 合同。 |

## 任务拆解

- [x] viewer-live-decision-mode-stable-authority (PRD-P2P-010) [test_tier_required]: 建立当前 PRD/design/project，并回填默认 LLM 与 observer/debug-only `--no-llm` 语义。 Trace: https://github.com/eng-cc/oasis7/issues/2569 (task_478961979bbf43fe81997816547f8258)
- [x] viewer-live-decision-mode-routing (PRD-P2P-010) [test_tier_required]: 修复 Viewer-live README 与 P2P 文件索引的默认路由。 Trace: https://github.com/eng-cc/oasis7/issues/2569 (task_478961979bbf43fe81997816547f8258)
- [x] viewer-live-decision-mode-source-retirement (PRD-P2P-010) [test_tier_required]: 完成活跃引用审计，删除两组已吸收的 2026-02 源三件套。 Trace: https://github.com/eng-cc/oasis7/issues/2569 (task_478961979bbf43fe81997816547f8258)

## 依赖

- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `scripts/worktree-harness.sh`
- `doc/world-simulator/viewer/viewer-manual.manual.md`

## 当前验证责任

- 参数行为：`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_viewer_live -- --nocapture`。
- 文档迁移：`./scripts/doc-governance-check.sh && ./scripts/readme-link-check.sh && git diff --check`。

这些是模块维护证据，不是 provider 可用性、launcher gameplay、release readiness 或 production 健康结论。若改变无 LLM 入口的可见交互、Web/launcher 行为或 player claim，需重新取得 viewer、视觉交互、runtime 和 QA 的相应证据。
