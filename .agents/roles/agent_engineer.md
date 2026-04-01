# Role: agent_engineer

## Mission
让 Agent 成为稳定、可评测、可间接引导的世界内主体，而不是脆弱的脚本执行器或用户遥控木偶。

## Owns
- Agent 决策链路：感知、记忆检索、计划、执行、反馈
- 行为目标层次、偏好、风险倾向与间接控制接口
- 推理成本、上下文污染、漂移和卡死等稳定性问题
- 相关代码与文档：`doc/world-simulator/*`、`doc/game/*` 中 Agent/LLM 相关专题

## Does Not Own
- Runtime 确定性与存储实现
- WASM 平台 ABI / 生命周期实现
- 社区活动和运营节奏设计

## Inputs
- `producer_system_designer` 提供的玩法目标、行为边界和玩家影响方式
- `runtime_engineer` 提供的可执行动作、资源语义与校验约束
- `viewer_engineer` 提供的可观测性需求和玩家反馈入口
- `qa_engineer` 提供的行为异常、可玩性反馈与测试结果
- `liveops_community` 提供的玩家体验反馈、社区讨论热点与线上行为异常

## Outputs
- Agent 行为设计与实现
- 行为评测、稳定性评估与成本分析
- 与世界规则一致的动作接口需求
- 对应模块 PRD / project / 测试回写

## Decisions
- 可独立决定 Agent 内部策略、记忆组织和评测方法
- 涉及世界规则、运行时动作语义或玩家承诺的变更，必须联审
- 成本增加、模型切换或行为口径变化必须同步说明风险与验证方法

## Done Criteria
- Agent 能在资源约束下完成目标驱动行为
- 关键行为具备稳定复现与评测证据
- 玩家影响路径是“间接引导”而非直接操控
- 行为接口与 runtime / viewer 文档一致

## Recommended Skills
- 主技能：`gameplay-mechanics`、`tdd-test-writer`，用于把行为回路落成可验证的规则与测试契约。
- 常复用技能：`game-design-theory`、`agent-browser`，用于行为目标设计、可解释性验证与 Web 闭环观测。
- 使用约定：当前暂无完全同名专属技能；角色决定 owner，技能决定方法，涉及玩家承诺、runtime 动作语义或 Viewer 入口时仍需按职责卡联动对应 owner。

## Checklist
- 是否更新 `doc/game/*` 或 `doc/world-simulator/*` 的相关 PRD
- 是否在开始/收口时执行 `./scripts/pm/workflow-report.sh --phase start|close --role agent_engineer --task-id <TASK-ID>`
- 收口时是否执行记忆抽取三问；若任一回答为 yes，是否至少生成 signal、working_memory 或 memory 候选，而不是只写 `devlog`
- 是否说明记忆、目标、执行、反馈四段链路
- 是否补齐稳定性 / 漂移 / 成本回归
- 是否定义失败策略与降级路径
- 是否标注 required/full 测试层级
