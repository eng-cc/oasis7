# `world-simulator/scenario` 文档入口

本目录收敛世界初始化、场景文件、seed/location 与 asteroid-fragment 生成专题。首次阅读先按要回答的问题选择一个专题；不要把已完成的命名迁移或专项基线误读成当前世界运行态，当前执行状态仍以 `../project.md` 和 GitHub task evidence 为准。

## 从这里开始

| 需要回答的问题 | 首读专题 | 边界 |
| --- | --- | --- |
| 当前可用场景、稳定断言和场景加载契约是什么？ | `scenario-files.prd.md` | 场景文件与验证矩阵的入口；不替代模块级执行状态。 |
| 新世界如何初始化，基础、多地点与 P2P 场景各服务什么验证？ | `world-initialization.prd.md` | 初始化场景矩阵与用途；不是 production readiness 结论。 |
| seed、地点和 deterministic spawn 如何生成？ | `scenario-seed-locations.prd.md` | location/agent 初始化专题；统一资源 manifest 语义继续读 `unified-world-seed-fragment-runtime.prd.md`。 |
| chunk、frag 资源预算、补种与 onboarding 如何衔接？ | `chunked-fragment-generation.prd.md` | 生成和预算主入口；首局资源体验再读 `frag-resource-balance-onboarding.prd.md` 与 `agent-frag-initial-spawn-position.prd.md`。 |
| asteroid fragment 的 override、spacing、设施基线或命名迁移在哪里？ | `scenario-asteroid-fragment-overrides.prd.md` | 专项配置入口；按问题下钻 `fragment-spacing`、`scenario-power-facility-baseline` 或 `asteroid-fragment-renaming`。 |

## 阅读与维护边界

- 每个专题的 `*.prd.md`、`*.design.md`、`*.project.md` 分别保存规格、设计与执行证据；本页只负责簇级首读分流，不复制其参数、测试或完成状态。
- 模块范围、当前执行状态和精确文件检索分别回到 `../prd.md`、`../project.md` 和 `../prd.index.md`。
- 审计显示本目录 10 组专题都仍被模块、游戏、测试或 core review 记录引用；即使日期较早、部分事项已完成，也没有同时满足“明确现行替代物 + 零活跃调用”的删除条件。因此本轮不删除任何专题或审计证据。
- 新增或退休 scenario 专题时：更新本页的首读分流与边界、保留 `../prd.index.md` 的精确 triplet 行；共享目录规则以 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为准。
