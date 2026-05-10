# Role Handoff Brief

审计轮次: 4

## Meta
- Handoff ID: `HANDOFF-SCRIPTS-002-2026-03-11-ENTRY-LAYERING`
- Date: `2026-03-11`
- From Role: `runtime_engineer`
- To Role: `qa_engineer`
- Related PRD-ID: `PRD-SCRIPTS-001/002`
- Related Task ID: `TASK-SCRIPTS-002`
- Priority: `P1`

## Goal
- 交付 scripts 分层与主入口 / fallback 规则，让 QA 在 required/full 和专项诊断之间有稳定调用边界。

## Why Now
- `scripts` 模块主项目还没有把高频脚本层级显式化；如果不先做这一步，后续参数契约和趋势统计会继续建立在漂移入口上。
- 如果不做，已删除的旧 3D 诊断脚本仍可能被误当成常规链路主入口。

## Inputs
- 代码 / 文档入口：`doc/scripts/governance/script-entry-layering-2026-03-11.prd.md`、`doc/scripts/governance/script-entry-layering-2026-03-11.project.md`
- 已完成内容：已按开发 / 发布 / 长跑 / 站点 / fallback 五层回写高频脚本清单
- 已知约束：本轮不补每个脚本的参数契约
- 依赖前置项：`AGENTS.md` 中 Web-first / fallback 规则

## Expected Output
- 接收方交付物 1：后续 required/full 文档引用 scripts 时优先使用主入口脚本
- 接收方交付物 2：若 QA 文档仍引用 fallback 脚本为主入口，按此专题回写修正
- 需要回写的文档 / 日志：后续 testing/manual 或 playability 文档按需引用

## Done Definition
- [x] 满足验收点 1：高频脚本分层表可被直接引用
- [x] 满足验收点 2：fallback 围栏与 Web-first 约束一致
- [x] 补齐测试 / 验证证据

## Risks / Blockers
- 风险：部分低频历史脚本尚未进入本轮分层表
- 阻断项：无
- 需要升级给谁：若后续发现某条 CI 主链路存在多个竞争主入口，升级给 `qa_engineer` 与 `runtime_engineer` 联合裁定

## Validation
- 建议测试层级：`test_tier_required`
- 建议验证命令：`rg -n "ci-tests.sh|release-gate.sh|run-viewer-web.sh|site-link-check.sh" doc/scripts/governance/script-entry-layering-2026-03-11.project.md doc/scripts/governance/script-entry-layering-2026-03-11.prd.md`

## Notes
- 接收方确认范围：`已接收 scripts 主入口 / fallback 分层结果，后续 testing 文档优先引用主入口脚本`
- 接收方确认 ETA：`same-day`
- 接收方新增风险：`无`
