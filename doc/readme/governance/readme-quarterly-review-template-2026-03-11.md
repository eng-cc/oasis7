# README 季度口径审查模板（2026-03-11）

审计轮次: 4

## Meta
- Review ID:
- Quarter:
- Date:
- Owner Role: `producer_system_designer`
- Review Partners: `qa_engineer`
- Trigger: `quarterly` / `ad-hoc-major-change`

## Inputs
- `README.md`
- `doc/README.md`
- `scripts/readme-link-check.sh`
- 相关模块主 PRD / site / core 文档

## Review Checklist
- [ ] 执行 `./scripts/readme-link-check.sh`
- [ ] README 顶层叙事不与 `doc/product/` 产品真值及对应专业模块 PRD 冲突；冲突时先裁定权威源，再回写 README
- [ ] README 与 site 不出现超出当前 evidence 的“已上线 / public launch”等状态宣称；变化时同步复核中英文入口
- [ ] 世界规则、玩家权能、WASM、runtime 与 viewer 只保留导航级摘要，具体规则链接到产品或专业权威
- [ ] README / `doc/README.md` 指向产品树、`testing-manual.md` 和专业主 PRD 的链接可用
- [ ] site、产品 PRD、专业主 PRD或公开状态文案变化时，触发本模板并在对应 project / GitHub task evidence 回写
- [ ] README 不重复定义详细行为或实现；发现重复时迁移语义、修复引用并删除被完整吸收的源内容
- [ ] 将问题写入修复记录模板

## Findings
| ID | 类型 | 影响范围 | 结论 | owner |
| --- | --- | --- | --- | --- |
| RQ-ISSUE-001 |  |  |  |  |

## Result
- 总结论: `pass` / `fix_required`
- 需要升级的事项:
- 回写文件:
