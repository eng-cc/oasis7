# 测试发布证据包模板

审计轮次: 4

## 目的
- 为 `TASK-TESTING-003` 提供统一的发布证据包结构。
- 将命令、日志、截图、结论、风险与 `PRD-ID / 任务ID` 映射收敛到同一份模板中。
- 让 `producer_system_designer`、`qa_engineer`、模块 owner 在 go/no-go 评审时可直接消费同一份证据。

## 使用说明
- 每个发布候选、阶段收口评审或高风险专项回归都应复制本模板并回填实际值。
- `必填` 字段缺失时，证据包结论只能填 `blocked`。
- 同一证据包可引用多个模块，但必须按模块列出 `PRD-ID / 任务ID / 测试层级 / 证据路径`。
- 截图、console、summary、日志等产物需落到仓库可访问路径后再引用。

## 目录建议
| 字段 | 推荐路径 |
| --- | --- |
| required / full 日志 | `output/<module>/<candidate>/logs/` |
| Web UI 截图 / console | `output/playwright/<suite>/<candidate>/` |
| 长跑 / soak 摘要 | `.tmp/<suite>/<candidate>/` 或 `output/<suite>/<candidate>/` |
| 汇总 Markdown | `output/release/<candidate>/summary.md` |
| 汇总 JSON | `output/release/<candidate>/summary.json` |

## 结论状态
| 状态 | 含义 |
| --- | --- |
| `pass` | 证据完整且结论满足当前发布要求 |
| `fail` | 已执行但存在阻断性失败 |
| `blocked` | 证据缺失、环境不足或待确认，无法给出有效结论 |

## 模板
### Meta
- 发布候选 / 阶段:
- 证据包 ID:
- 日期:
- 汇总人:
- 总结论: `pass` / `fail` / `blocked`
- 关联 go/no-go 记录:

### 覆盖范围
| 模块 | PRD-ID | 任务ID | 测试层级 | 负责人 |
| --- | --- | --- | --- | --- |
|  |  |  |  |  |

### 执行命令
| 套件 / 检查项 | 命令 | 结果 | 日志路径 |
| --- | --- | --- | --- |
| S0 |  |  |  |
| S1 / required |  |  |  |
| S2 / full（如执行） |  |  |  |
| S6 / Web UI（如执行） |  |  |  |
| S9 / S10（如执行） |  |  |  |

### UI / 体验证据
| 类型 | 路径 | 说明 |
| --- | --- | --- |
| 截图 |  |  |
| console / semantic 结果 |  |  |
| 视频（如有） |  |  |
| playability 卡片 / 评分 |  |  |

### 长跑 / 在线证据
| 套件 | summary 路径 | failures / timeline 路径 | 结论 |
| --- | --- | --- | --- |
| S9 |  |  |  |
| S10 |  |  |  |
| 其他 soak / stress |  |  |  |

### 风险与例外
| 风险 ID | 描述 | 当前影响 | 缓解措施 | 是否阻断 | 负责人 |
| --- | --- | --- | --- | --- | --- |
| R-001 |  |  |  |  |  |

### 结论摘要
- 通过项：
- 失败项：
- 缺失证据：
- 建议结论：`pass` / `fail` / `blocked`
- 是否需要升级到 `core` go/no-go：`yes` / `no`

## 最小审查清单
- 是否填写发布候选、证据包 ID、总结论。
- 是否为每个模块填写 `PRD-ID / 任务ID / 测试层级 / 负责人`。
- 是否至少填写一条执行命令与对应日志路径。
- 若执行 UI / 长跑测试，是否补齐截图、console、summary 或 failures 路径。
- `pass/fail/blocked` 是否能被日志、截图或 summary 直接支撑。
- 是否已把可变结论回写对应 GitHub task issue evidence comments，并把稳定测试合同或证据字段回写相关 PRD/manual/evidence 文档。
