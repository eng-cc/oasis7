# Local Provider vs 内置 Agent parity 评分卡模板（2026-03-12）

审计轮次: 2

适用范围: `PRD-WORLD_SIMULATOR-038` 的 `P0/P1/P2` parity 验收，覆盖自动 benchmark 结果与 QA/producer 主观试玩结论。

---

## 一、评审信息
- 评审人:
- 角色: `qa_engineer` / `producer_system_designer`
- 评审日期:
- provider 版本:
- adapter 版本:
- protocol 版本:
- agent_profile:
- parity 层级: `P0` / `P1` / `P2`
- 场景批次:
- 是否与 builtin 同批次同 seed 对比: [ ]是 [ ]否

## 二、结论刻度（单选）
- `blocked`: 发布阻断；不得扩大覆盖范围。
- `failed`: 未达行为等价或命中阻断线；保持 `experimental`。
- `conditional_pass`: 行为等价基本达标，但必须附带限制条件。
- `pass`: 达到当前层行为等价通过线；是否允许默认启用仍需看 `latency_class`。

## 三、自动指标汇总
- builtin completion rate:
- Local Provider completion rate:
- completion rate gap(pp):
- builtin invalid action rate:
- Local Provider invalid action rate:
- timeout rate:
- builtin median extra wait(ms):
- Local Provider median extra wait(ms):
- median extra wait gap(ms):
- builtin p95 extra wait(ms):
- Local Provider p95 extra wait(ms):
- p95 extra wait gap(ms):
- latency class: [ ]A(default-candidate) [ ]B(experimental-only) [ ]C(blocked)
- trace completeness(%):
- recoverable error resolution rate(%):
- context drift count（如适用）:
- benchmark 证据路径:

## 四、自动指标判定（逐项勾选）
### 4.1 行为等价硬门禁
- [ ] `completion_rate_gap` 满足当前层通过线
- [ ] `invalid_action_rate` 满足当前层通过线
- [ ] `timeout_rate` 满足当前层通过线
- [ ] `relative_wait_gap_median/p95` 满足当前层通过线
- [ ] `trace_completeness` 满足当前层通过线
- [ ] `recoverable_error_resolution_rate` 满足当前层通过线
- [ ] 未触发行为等价阻断线

### 4.2 发布 / 默认启用附加门槛
- [ ] `latency_class = A`，可作为默认启用候选
- [ ] `latency_class = B`，仅允许 `experimental` / 受限试点
- [ ] `latency_class = C`，必须阻断

## 五、主观试玩评分（1-5）
- 行为达成感（1-5）: [ ]1 [ ]2 [ ]3 [ ]4 [ ]5
- 等待体感（1-5）: [ ]1 [ ]2 [ ]3 [ ]4 [ ]5
- 记忆连续性（1-5）: [ ]1 [ ]2 [ ]3 [ ]4 [ ]5 [ ]N/A
- 错误可恢复性（1-5）: [ ]1 [ ]2 [ ]3 [ ]4 [ ]5
- 可解释性 / 可调试性（1-5）: [ ]1 [ ]2 [ ]3 [ ]4 [ ]5
- 与 builtin 体验接近程度（1-5）: [ ]1 [ ]2 [ ]3 [ ]4 [ ]5

## 六、关键观察
- 最佳表现路径:
- 最差表现路径:
- 与 builtin 相比最明显的差异:
- 玩家是否会明显感知 provider 已切换: [ ]会 [ ]不会 [ ]不确定
- 若会，主要原因:

## 七、阻断项检查
- [ ] 出现 runtime 权威绕过或未受白名单约束动作
- [ ] 出现无法恢复且无清晰错误提示的失败
- [ ] 出现会话串线 / trace 归因错误
- [ ] 出现明显上下文漂移且 trace 无法解释
- [ ] `latency_class = C`
- [ ] 无阻断项

## 八、最终结论
- 行为等价结论: [ ]blocked [ ]failed [ ]conditional_pass [ ]pass
- 发布 / 默认启用结论: [ ]blocked [ ]experimental_only [ ]default_candidate
- 允许状态: [ ]保持 `experimental` [ ]进入下一层 parity [ ]允许默认启用
- 必须修复项:
- 可延期项:
- 备注:

## 九、签署
- `qa_engineer`:
- `producer_system_designer`:
- 如为 `conditional_pass`，附加限制条件:
