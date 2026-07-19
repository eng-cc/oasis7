# 访问模式与发行就绪

## 文档身份

- 所属产品模块：玩家入口与发行
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 公开状态权威：[`README.md`](../../../README.md)
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文是长期产品分册，承载玩家访问模式、入口能力等价、可解释失败恢复以及发行升阶的组合承诺。它不保存历史阶段 verdict、协议字段、命令、样本阈值、任务状态或渠道执行记录。

## 1. 产品目标

玩家能够选择一个被明确支持的入口，理解该入口当前是可玩、仅观察还是被阻塞，并在同一版本和模式下完成发现、进入、核验、失败恢复与继续游玩。团队只有在同一候选版本的入口、运行恢复、QA 和公开沟通证据共同成立后，才能评估扩大阶段或对外 claim。

## 2. 玩家访问模式

- 正式 primary mode 只有 `viewer` 与 `pure_api`。前者提供 Web/UI 表面，后者提供无 UI 的正式玩家入口。
- alias、provider、execution lane、deployment、hosted session 或兼容路径不会形成第三种玩家模式，也不代表更高发布等级。
- 每份可玩性、parity、observer 或 blocked 结论必须绑定一个 primary mode；`viewer` 与 `pure_api` 的证据不得互相代签。
- `pure_api` 不是协议探针或自动化执行 lane 的别名。它只有在专业域规定的正式玩法前置与独立证据成立时，才能声明为可玩。

## 3. 入口能力等价

`viewer` 与 `pure_api` 可以采用不同表现方式，但必须消费同一权威世界事实和玩家语义。决定持续游玩的能力不能因入口不同而丢失：

- 当前阶段、主目标、进度、主要 blocker 与下一步。
- 受支持的核心行动、接受或拒绝、主要世界后果与恢复动作。
- 首局后的阶段承接、首次持续能力与中循环继续路径。
- 断线、重连或切换客户端后的目标、最近结果和下一步恢复。

UI 私有聚合或 API 客户端自行推导不能成为第二事实源。协议可连、能够读取 snapshot、单次 step 成功或 observer smoke 都不能单独证明正式可玩或能力等价。

## 4. 可解释失败与恢复

- 入口前置缺失、身份或会话失效、provider 初始化失败、模式不受支持或后端不可用时，玩家必须得到准确的 blocked/observer 分类、原因和恢复下一步。
- 恢复可以是 reconnect、re-register、re-auth、选择受支持模式、修复配置或稍后重试；不得静默恢复旧权限，也不得把 fallback 包装成原入口成功。
- 无 LLM、debug、probe 或 observer 路径只能支持对应的受限结论，不能代签 `pure_api` 或 `viewer` 的正式可玩性。
- 对外说明先纠正当前状态，再给最小解释和正式反馈/升级入口；不得用模糊措辞把技术通路包装成已发布体验。

## 5. 统一候选门禁与公开口径

- 阶段或公开 claim 的评估必须绑定同一候选版本，而不是拼接不同版本、不同模式或不同专题的局部 green。
- 候选证据至少覆盖适用的 `viewer`、`pure_api`、权威运行与恢复、发行资产、QA 汇总及 LiveOps/公开说明同步。
- 任一硬门失败、缺证或只具备历史样本时，保持较低承诺并记录 blocker；不能用 source-tree pass、单专题 pass 或旧 go 记录代签整体验证。
- release gate 通过只是升阶前提，不自动改变当前阶段或公开 claim。正式变更还需要产品决策、QA 结论、LiveOps 同步，并最终反映到根 `README.md`。
- 根 `README.md` 始终是当前公开状态与 claim envelope 的唯一权威；本分册不固化某次 Alpha、Beta 或 preview verdict。

## 6. 组合验收

- AR-1：所有入口和证据都能归一到 `viewer` 或 `pure_api`，不存在由 alias、provider、deployment 或 session context 派生的第三种模式。
- AR-2：同一候选版本分别证明 `viewer` 与 `pure_api` 的阶段、目标、阻塞、下一步、核心动作、主要因果与重连恢复；两种模式证据不互相代签。
- AR-3：blocked/observer 样例能说明真实原因和恢复路径，且不会继续保留错误的 playable/parity 结论。
- AR-4：统一候选门禁把入口、权威运行与恢复、发行资产、QA 和公开 claim 绑定到同一版本；局部或历史 green 无法代签。
- AR-5：公开口径变更可追踪到产品决策、专业验证、QA 结论、LiveOps 同步和根 README；gate pass 不自动等于阶段升级。

### 6.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| AR-1 | producer_system_designer / viewer_engineer / qa_engineer | PRD-WORLD_SIMULATOR-039 / PRD-WORLD_SIMULATOR-041 / PRD-WORLD_SIMULATOR-046 / PRD-TESTING-003 | `doc/world-simulator/prd.md`; `doc/testing/prd.md` | primary mode taxonomy 与 claim/evidence 归一化审计 | test_tier_required |
| AR-2 | gameplay_designer / viewer_engineer / qa_engineer | PRD-GAME-008 / PRD-WORLD_SIMULATOR-039 / PRD-WORLD_SIMULATOR-041 / PRD-WORLD_SIMULATOR-046 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | Viewer S6 与 pure API 独立长玩/parity 对账，覆盖阶段承接与重连恢复 | test_tier_full |
| AR-3 | viewer_engineer / agent_engineer / qa_engineer | PRD-WORLD_SIMULATOR-016 / PRD-WORLD_SIMULATOR-039 / PRD-WORLD_SIMULATOR-041 / PRD-WORLD_SIMULATOR-046 / PRD-TESTING-003 | `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 前置失败、blocked/observer 分类及可执行恢复路径证据 | test_tier_required |
| AR-4 | runtime_engineer / viewer_engineer / qa_engineer / liveops_community | PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-042 / PRD-WORLD_SIMULATOR-045 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 同候选版本完整 release gate；适用时包含 S6、S9/S10 与真实发行资产 | test_tier_full |
| AR-5 | producer_system_designer / qa_engineer / liveops_community | PRD-WORLD_SIMULATOR-042 / PRD-WORLD_SIMULATOR-043 / PRD-TESTING-003 | `README.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 产品决策、QA 结论、LiveOps 同步与根 README claim 变更追踪 | test_tier_required |

## 7. Non-Goals

- 不定义协议 schema、客户端字段、状态枚举或具体错误码。
- 不冻结趋势阈值、运行时长、样本数量、性能预算或自动化命令。
- 不保存某次候选版本的 pass/block、阶段判断或任务完成状态。
- 不替代发行 runbook、渠道文案模板、事故处理或社区反馈记录。
