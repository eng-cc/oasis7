# Runtime 发布门禁指标模板

审计轮次: 4

## 目的
- 为 `TASK-WORLD_RUNTIME-004` 提供统一的 runtime 发布门禁指标记录格式。
- 让 runtime 边界清单、回归模板与 `core` go/no-go 模板之间有直接可引用的指标层。

## 使用说明
- 每个发布候选至少填写一份 runtime 指标记录。
- 指标可来自 `test_tier_required` 或 `test_tier_full`，但必须标明来源。
- 任一关键指标为 `fail` 或 `blocked` 时，runtime 维度结论不得为 `go`。

## 模板
### Meta
- 发布候选 / 版本:
- 指标记录 ID:
- 日期:
- 负责人:
- 关联 PRD-ID:
- 关联任务:
- 关联边界清单:
- 关联回归模板:
- runtime 结论: `go` / `conditional-go` / `no-go` / `blocked`

### 关键指标
| 指标 | 说明 | 来源 | 当前状态 (`pass` / `fail` / `blocked`) | 证据路径 | 备注 |
| --- | --- | --- | --- | --- | --- |
| replay / state root 一致性 | 确定性回放与恢复后状态根一致 | required / full |  |  |  |
| WASM ABI / hash / registry | 工件、接口、registry 可追溯且无漂移 | required / full |  |  |  |
| 治理状态机 / 拒绝路径 | propose/shadow/approve/apply 与拒绝原因完整 | required / full |  |  |  |
| 安全失败签名 | 越权、缺审计、receipt 断裂等失败签名为 0 | required / full |  |  |  |
| 数值语义失败签名 | 数值漂移、边界值异常、恢复后不一致等失败签名为 0 | required / full |  |  |  |
| storage / GC / replay summary | profile、GC 结果、恢复摘要满足当前候选要求 | required / full |  |  |  |

### 风险与例外
| 风险 ID | 描述 | 是否阻断 | 缓解措施 | 负责人 | 复审时间 |
| --- | --- | --- | --- | --- | --- |
| R-001 |  |  |  |  |  |

### 结论摘要
- 通过项：
- 阻断项：
- 条件放行项：
- 建议升级动作：

## 对接规则
- 若 `replay / state root 一致性`、`WASM ABI / hash / registry`、`治理状态机 / 拒绝路径` 任一项不是 `pass`，runtime 结论不得高于 `no-go`。
- 若仅 storage / GC / replay summary 存在已批准例外，可给 `conditional-go`，但必须绑定复审时间。
- 若边界清单或回归模板缺失，runtime 结论只能为 `blocked`。

## 最小审查清单
- 是否填写所有关键指标的状态与证据路径。
- 是否绑定边界清单与回归模板。
- `conditional-go` / `no-go` / `blocked` 是否具备直接证据支撑。
- 是否已把结论同步回写对应 GitHub task issue evidence、`doc/core/templates/stage-closure-go-no-go-template.md` 的评审记录，以及受影响的现行 PRD/design。
