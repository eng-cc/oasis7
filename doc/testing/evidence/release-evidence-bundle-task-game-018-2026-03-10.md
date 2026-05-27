# 测试发布证据包（TASK-GAME-018 / 2026-03-10）

审计轮次: 4

## Meta
- 发布候选 / 阶段: `TASK-GAME-018 / ROUND-009`
- 证据包 ID: `TEST-BUNDLE-GAME-018-20260310`
- 日期: `2026-03-10`
- 汇总人: `qa_engineer`
- 总结论: `pass`
- 关联 go/no-go 记录: `doc/core/reviews/stage-closure-go-no-go-task-game-018-2026-03-10.md`

## 覆盖范围
| 模块 | PRD-ID | 任务ID | 测试层级 | 负责人 |
| --- | --- | --- | --- | --- |
| `game` | `PRD-GAME-004` | `TASK-GAME-018` / `TASK-GAMEPLAY-MLF-007/008` | `test_tier_required` | `viewer_engineer` / `qa_engineer` |

## 执行命令
| 套件 / 检查项 | 命令 | 结果 | 日志路径 |
| --- | --- | --- | --- |
| `viewer-halo-regression` | `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer selection_emphasis::tests:: -- --nocapture` | `PASS` | `doc/devlog/README.md` |
| `viewer-marker-regression` | `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer two_d_agent_marker_profile_ -- --nocapture` | `PASS` | `doc/devlog/README.md` |
| `S6-web-visual-capture` | `agent-browser --headed open "http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&test_api=1"` + `agent-browser eval/runSteps` + `agent-browser screenshot` + `agent-browser record start/stop` | `PASS` | `output/playwright/playability/manual-20260310-round009/console.log` |
| `doc-linkage-check` | `rg -n "TASK-GAME-018|card_2026_03_10_23_27_43|总结论: `pass`" doc/game doc/playability_test_result doc/testing doc/core` | `PASS` | `doc/devlog/README.md` |

## UI / 体验证据
| 类型 | 路径 | 说明 |
| --- | --- | --- |
| 截图 | `output/playwright/playability/manual-20260310-round009/mlf007-baseline-3d-mid.png` / `output/playwright/playability/manual-20260310-round009/mlf007-selected-3d-mid.png` / `output/playwright/playability/manual-20260310-round009/mlf007-selected-3d-near.png` / `output/playwright/playability/manual-20260310-round009/mlf007-selected-2d-marker.png` | 覆盖 baseline、3D 中近景与 2D marker。 |
| console / semantic 结果 | `output/playwright/playability/manual-20260310-round009/console.log` / `output/playwright/playability/manual-20260310-round009/console.errors.log` / `output/playwright/playability/manual-20260310-round009/state-selected-3d-mid.json` / `output/playwright/playability/manual-20260310-round009/state-selected-2d.json` | `console.errors.log` 为空；状态快照保持 connected。 |
| 视频（如有） | `output/playwright/playability/manual-20260310-round009/mlf008-visual-check.webm` | 视角切换短录屏。 |
| playability 卡片 / 评分 | `doc/playability_test_result/card_2026_03_10_23_27_43.md` | QA 评分与结论标签。 |

## 长跑 / 在线证据
| 套件 | summary 路径 | failures / timeline 路径 | 结论 |
| --- | --- | --- | --- |
| `S9` | `N/A` | `N/A` | `not_applicable` |
| `S10` | `N/A` | `N/A` | `not_applicable` |
| `ROUND-008 runtime_live` | `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.project.md` | `output/playwright/playability/manual-20260307-round008/` | `PASS` |

## 风险与例外
| 风险 ID | 描述 | 当前影响 | 缓解措施 | 是否阻断 | 负责人 |
| --- | --- | --- | --- | --- | --- |
| `R-GAME-018-001` | ROUND-009 录屏样本偏短 | 不影响当前任务关闭，影响后续发布抽样覆盖面 | 在 release gate 阶段补看更长录屏 | `no` | `qa_engineer` |

## 结论摘要
- 通过项：viewer 可读性增强代码回归通过；S6 证据完整；playability 卡片已回写。
- 失败项：无。
- 缺失证据：无任务级缺失；仅全局 release gate 仍待 core 汇总其它 P0 证据。
- 建议结论：`pass`
- 是否需要升级到 `core` go/no-go：`yes`
