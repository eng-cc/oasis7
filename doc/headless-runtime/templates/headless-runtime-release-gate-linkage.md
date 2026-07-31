# Headless-Runtime 长稳门禁对接说明

审计轮次: 4

## 目的
- 为 `TASK-NONVIEWER-004` 提供 headless-runtime 与 testing 模块之间的门禁对接说明。
- 明确 headless-runtime 长稳证据如何进入 `doc/testing/templates/release-evidence-bundle-template.md` 与 `core` go/no-go 评审。

## 对接规则
1. `doc/headless-runtime/checklists/lifecycle-auth-consistency-checklist.md` 提供生命周期 / 鉴权边界结论。
2. `doc/headless-runtime/templates/longrun-archive-incident-template.md` 提供长稳归档与故障追溯证据。
3. `doc/testing/templates/release-evidence-bundle-template.md` 中的“长跑 / 在线证据”字段必须引用上述 headless-runtime 产物路径。
4. 若 lifecycle / auth 清单为 `fail` 或 `blocked`，则 testing 证据包中对应 longrun 结论不得为 `pass`。
5. 若 headless-runtime 证据缺失，则 `core` go/no-go 中对应 `P1` 风险项必须显式标记为未完成。

## 引用字段映射
| headless-runtime 产物 | testing 证据包字段 | core go/no-go 字段 |
| --- | --- | --- |
| 生命周期 / 鉴权清单结论 | `执行命令` / `长跑 / 在线证据` | `P1 风险附注` |
| 长稳归档 / incident 模板 | `长跑 / 在线证据` | `P1 风险附注` |
| 失败签名 / 恢复动作 | `风险与例外` | `P1 风险附注` |

## 最小审查清单
- 是否在 testing 证据包中引用 headless-runtime 证据路径。
- 是否将 lifecycle / auth / longrun 的 `fail/blocked` 同步到发布结论。
- 是否在 `doc/headless-runtime/prd.md` 与 `doc/testing/prd.md` 中保持一致口径。
