# 客户端启动器跨表面受控动作项目追踪

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-cross-surface-action-parity.prd.md`
> 对应设计: `doc/world-simulator/launcher/game-client-launcher-cross-surface-action-parity.design.md`

## 状态

- 状态：`documented_current_authority`。
- 本轮只完成文档语义迁移、活跃入口修复与已吸收源文件退役；未改变 Launcher、runtime、WASM、UI 或测试行为。

## 任务拆解

- [x] launcher-cross-surface-action-parity-migration (PRD-WORLD_SIMULATOR-020-031) [test_tier_required]: 建立稳定 triplet，收敛四组 dated Launcher 跨表面动作专题并修复当前入口。 Trace: https://github.com/eng-cc/oasis7/issues/2590 (task_01f4c982f1564bccbd6d0e46176dc74c)
- [x] launcher-cross-surface-llm-settings-backfill (PRD-WORLD_SIMULATOR-020-031) [test_tier_required]: 补回 native/Web LLM 设置的本地保存、读回、失败诊断与 secret 非承诺边界。 Trace: https://github.com/eng-cc/oasis7/issues/2630 (task_54fad990c6904d45b5f7f22820c40541)

## 依赖

- 当前行为事实依赖 Launcher、runtime 和 WASM 各自的专业 authority；本文不替代它们的实现或验证。
- 文档迁移验证依赖 `./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh` 与删除源文件的 stale-reference 扫描。

## 已吸收的历史范围

- Web 必填配置分流、设置/反馈、转账闭环和 transfer parity 的四组 2026-03 三件套已迁移到本稳定 PRD/design/project。
- 历史源文件已删除；逐项实现、当时测试和完成态仅由 Git history 与 GitHub task issue evidence comments 追溯，不构成当前能力或发布结论。

## 持续维护

- 当前专业 authority：`game-client-launcher-cross-surface-action-parity.{prd,design}.md`，并与 control-plane、feedback、guided-configuration、runtime 和 WASM 文档交叉引用。
- LLM 设置边界：native 配置与 Web 浏览器本地存储的保存/读回属于 viewer surface；provider/runtime 使用、secret management、WASM lifecycle 与发布判断分别回到对应专业 authority。
- 触发复审：新增跨表面动作、改变平台字段/存储/代理边界、将 submit 结果解释为最终状态，或出现历史无界/持久性 claim。
- 所需 owner：UI 表面由 viewer_engineer，runtime 接受/结算由 runtime_engineer，WASM authority 由 wasm_platform_engineer；发布判断由 qa_engineer，产品承诺由 producer_system_designer。
