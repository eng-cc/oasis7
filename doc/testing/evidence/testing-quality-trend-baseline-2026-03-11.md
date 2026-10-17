# 测试质量趋势基线（2026-03-11）

历史记录边界：以下判定、命令、角色交接及“当前/下一步”均属于文中明确的采集窗口；保留原始失败、通过、未执行范围和审计定位，不作为当前规范、进度或持续跟踪义务。当前行动与验收回到[测试入口](../README.md)及对应专业 authority，使用新的候选和证据窗口；后续通过、合入或任务关闭不改写本次运行事实。

本表只统计所列 `2026-03-19` 至 `2026-03-22` 的 7 个样本；“最近 7 天”和续写建议均为当时口径，不能作为滚动的当前质量基线。新窗口另存，保留本表原始计数和结果。

审计轮次: 5

## Meta
- Baseline ID: `TEST-TREND-BASELINE-20260311`
- 统计窗口: `2026-03-19` ~ `2026-03-22`
- 汇总角色: `qa_engineer`
- 样本数: `7`
- 指标结论: `green`

## 指标定义
- 首次通过率：`首次结论=pass` 的样本数 / 总样本数。
- 最终收口率：最终结论为 `pass/pass_after_fix/conditional_pass` 的样本数 / 总样本数。
- 阶段内逃逸率：首次不是 `pass`、且问题在下游 QA/可用性审查/复验阶段才暴露的样本数 / 总样本数。
- 平均修复时长：`关闭日期 - 发现日期` 的自然日均值；same-day 记为 `0d`。

## 红黄绿阈值
| 指标 | Green | Yellow | Red |
| --- | --- | --- | --- |
| 首次通过率 | `>= 80%` | `50% ~ 79%` | `< 50%` |
| 阶段内逃逸率 | `<= 20%` | `21% ~ 40%` | `> 40%` |
| 平均修复时长 | `<= 1d` | `> 1d 且 <= 3d` | `> 3d` |

## 样本明细
| Sample ID | 来源文档 | 首次结论 | 最终结论 | 是否阶段内逃逸 | 发现日期 | 关闭日期 | 修复时长 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `POD-WEB-20260319-094056` | `doc/playability_test_result/card_2026_03_19_09_40_56.md` | `pass` | `pass` | `no` | `2026-03-19` | `2026-03-19` | `0d` | `#46 PostOnboarding` headed Web/UI required-tier 在 fresh bundle 下首次通过。 |
| `POD-NOUI-20260319-101444` | `doc/testing/evidence/post-onboarding-headless-smoke-2026-03-19.md` | `pass` | `pass` | `no` | `2026-03-19` | `2026-03-19` | `0d` | 无 UI live-protocol smoke 首次确认 `step(8) -> step(24)` 与 `RuntimeEvent` feed 正常。 |
| `CB-RUNTIME-600S-20260322-121320` | `doc/testing/evidence/closed-beta-runtime-s10-2026-03-22.md` | `pass` | `pass` | `no` | `2026-03-22` | `2026-03-22` | `0d` | clean-room `600s+` soak 与 replay/rollback drill 在当前 candidate 上一次通过。 |
| `CB-WEB-20260322-155613` | `doc/playability_test_result/card_2026_03_22_15_56_13.md` | `pass` | `pass` | `no` | `2026-03-22` | `2026-03-22` | `0d` | 同候选 headed Web/UI rerun 一次通过，首屏主目标与顶部总结保持清晰。 |
| `CB-PUREAPI-REQ-20260322-183650` | `doc/testing/evidence/pure-api-parity-validation-2026-03-19.md` | `pass` | `pass` | `no` | `2026-03-22` | `2026-03-22` | `0d` | 同候选 pure API required-tier rerun 一次通过，到达 `post_onboarding.choose_midloop_path`。 |
| `CB-PUREAPI-FULL-20260322-183750` | `doc/testing/evidence/pure-api-parity-validation-2026-03-19.md` | `pass` | `pass` | `no` | `2026-03-22` | `2026-03-22` | `0d` | 同候选 pure API full-tier rerun 一次通过，并保持 `step_c` 抽样稳定。 |
| `CB-NOUI-20260322-183832` | `doc/testing/evidence/post-onboarding-headless-smoke-2026-03-19.md` | `pass` | `pass` | `no` | `2026-03-22` | `2026-03-22` | `0d` | 同候选 no-UI smoke rerun 一次通过，时间线保持 `1 -> 9 -> 33`。 |

## 计算结果
- 首次通过率：`7 / 7 = 100%` → `green`
- 最终收口率：`7 / 7 = 100%` → `green`
- 阶段内逃逸率：`0 / 7 = 0%` → `green`
- 平均修复时长：`(0d * 7) / 7 = 0d` → `green`

## 基线解读
- 最近 7 天窗口内，candidate 相关的 headed Web/UI、pure API、no-UI 与 runtime longrun 样本均做到首次通过，说明当前阶段的 QA 入口已经从“靠下游复验兜底”转回“owner 自检后再交 QA”。
- 阶段内逃逸率在该窗口降到 `0%`，表示这批样本里没有出现“问题等到下游 QA / 可用性复验才被首次发现”的信号。
- 平均修复时长为 `0d`，不是因为没有做修复，而是当前窗口纳入的都是最终交付前已在 owner 侧消化过的一次通过样本。
- 历史 `2026-03-08` ~ `2026-03-11` 的首份 baseline 仍保留在前一审计轮次中，用于说明早期启动器 / runtime profile 曾存在明显逃逸；本轮结论代表当前阶段评审使用的最近窗口真值。

## 建议动作
- `producer_system_designer`：当前 trend baseline 已可作为 `TASK-GAME-033` 的输入，但阶段是否升级仍需结合 claim envelope 与 liveops 节奏单独决策。
- `qa_engineer`：继续按最近 7 天窗口追加样本；若后续 fresh rerun 出现非首次通过，需立即把 baseline 与 unified gate 一并回退。
- 模块 owner：维持“先在 owner 侧跑最小 evidence，再交 QA”策略，避免阶段内逃逸率重新抬头。

## 2026-10-16 解释性勘误：选样与阈值边界

原七行样本、计数、红黄绿表、历史解读及建议动作全部保留。`7/7` 是所选 run 的正确计数，不是所有任务、首次开发尝试或整个交付流程的分母；不能从这些最终交付前的成功复跑样本推出 QA 已从“下游复验兜底”转为“owner 自检”，也不能据此证明流程改善或全窗口无返工。同日 [runtime S10 原始证据](closed-beta-runtime-s10-2026-03-22.md) 中 A/C 的失败、cleanup 修复与其后 G 的 600s 候选复验均仍有效，不能用选入 G 覆盖之前失败。

若需跨窗口趋势结论，须另行明确可比较的任务/样本分母、首次观察点与选样规则，同时保留失败和修复样本；不得回写旧数字制造新的趋势结果。本表阈值仅保留为所注明历史窗口的判定口径，不因被当前专题引用而自动成为现行阈值。当前采用状态见 [趋势 PRD](../governance/testing-quality-trend-tracking-2026-03-11.prd.md#current-threshold-status)。
