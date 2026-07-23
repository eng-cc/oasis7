# Gameplay 视觉证据与 Playability / Testing 对接说明（2026-03-10）

审计轮次: 4

## 目的
- 统一 `TASK-GAME-018` 的视觉证据包与 `playability_test_result` / `testing` 模板的引用关系。
- 避免同一轮视觉优化在 `game`、`playability`、`testing` 三处重复填写但口径不一致。

## 对接规则
1. `doc/game/gameplay/gameplay-micro-loop-visual-closure-evidence-template-2026-03-10.md` 负责记录 baseline、对照图、录屏、视觉评估。
2. `doc/playability_test_result/templates/playability-release-evidence-bundle-template.md` 负责引用该视觉证据包，并补充卡片评分与高优问题摘要。
3. `doc/testing/templates/release-evidence-bundle-template.md` 负责记录执行命令、日志、截图路径与最终测试结论。
4. 若视觉证据包结论不是 `pass`，则 playability 证据包最高只能为 `blocked`，testing 证据包需在结论摘要中显式标注。

## 字段映射
| game 视觉证据字段 | playability 字段 | testing 字段 |
| --- | --- | --- |
| `关键截图` | `卡片覆盖范围` / `关联测试 / 运行证据` | `UI / 体验证据` |
| `辅助证据.录屏` | `关联测试 / 运行证据.录屏` | `UI / 体验证据.视频` |
| `视觉评估结论` | `结论摘要` | `结论摘要` |
| `总结合论` | `建议结论` | `总结论` |

## 最小审查清单
- 是否为同一轮 `TASK-GAME-018` 使用同一份视觉证据包 ID。
- playability / testing 是否都引用到同一批截图 / 录屏路径。
- 若 `MLF-007` 未完成，是否同步把结论维持为 `blocked`。


## 本轮实例
- game 视觉证据：`doc/game/gameplay/gameplay-micro-loop-visual-closure-evidence-2026-03-10-round009.md`
- playability 证据包：`doc/playability_test_result/evidence/playability-release-evidence-bundle-task-game-018-2026-03-10.md`
- testing 证据包：`doc/testing/evidence/release-evidence-bundle-task-game-018-2026-03-10.md`
- core go/no-go 记录：Git history
- 当前链路状态：`TASK-GAME-018` 已完成任务关闭，但候选级总评仍取决于其它 `P0` 证据是否齐备。
