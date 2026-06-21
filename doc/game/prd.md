# game PRD

审计轮次: 11

## 目标
- 作为 game 模块的活跃玩法基线入口，回答当前玩家体验目标、阶段口径、权威专题和完成定义。
- 保持 PRD-ID、专题文档、执行任务和验证证据可追踪，但不在根 PRD 重复展开每个专题的完整规格。
- 让新增 gameplay 变更能先判断应改根基线、专题 PRD、执行 project，还是仅补索引/证据。

## 范围
- 覆盖 game 模块当前玩家侧核心循环、progression、agency、经济/claim、preview/beta gate 与可玩性验收边界。
- 覆盖 PRD-ID 到 `doc/game/project.md`、`doc/game/prd.index.md` 与高频专题 PRD 的路由关系。
- 不覆盖实现代码逐行说明、历史执行流水、专题完整 user story / matrix / decision log；这些内容保留在专题 `*.prd.md`、`*.project.md`、测试 evidence 与 `.pm` task trace。

## 接口 / 数据
- PRD 主入口: `doc/game/prd.md`
- 项目管理入口: `doc/game/project.md`
- 文件级索引: `doc/game/prd.index.md`
- 模块 landing page: `doc/game/README.md`
- gameplay 热点子域入口: `doc/game/gameplay/README.md`
- 核心玩法骨架: `doc/game/gameplay/gameplay-top-level-design.prd.md`
- 追踪主键: `PRD-GAME-xxx`
- 测试与发布参考: `testing-manual.md`
- 跨模块模式 taxonomy: `doc/core/player-access-mode-contract-2026-03-19.prd.md`

## 里程碑
- M1 (2026-03-03): 完成模块设计 PRD 主体重写与任务改造。
- M2 (2026-05-17): 形成 gameplay 热点子域入口，收口 retention、agency、scale 与 small-player lane 的高频首读路径。
- M3 (2026-06-21): 根 PRD 改为活跃玩法基线表，专题细节回到对应专题文档，降低根入口漂移风险。

## 风险
- 根 PRD 若重新堆叠专题细节，会再次和专题 PRD 争夺权威。
- 历史 gate / sample 若缺少时间戳和 evidence 指针，容易被误读为当前 blocker。
- `doc/world-simulator/*` 的 scenario / resource docs 会影响 gameplay，但不应替代 `doc/game` 的玩家侧 loop / progression 权威。

## 1. Executive Summary
- Problem Statement: game 模块的玩法、经济、preview/beta、agency 与小玩家成长线已拆成多份专题；根 PRD 需要保留当前基线和路由，而不是继续成为专题规格汇编。
- Proposed Solution: 根 PRD 只维护活跃 gameplay baseline、PRD-ID 路由、关键 gate 口径和跨模块权威边界；专题细节、历史执行与证据全部通过索引和 project trace 下钻。
- Success Criteria:
  - SC-1: 新增或变更 gameplay 能映射到一个 PRD-GAME-ID、专题 PRD 或明确的新增专题需求。
  - SC-2: 读者能在根 PRD 内判断当前玩家侧真值，再通过链接进入专题细节。
  - SC-3: 当前 stage / claim envelope / formal gate 口径不依赖历史流水解释。
  - SC-4: root PRD 不复制专题矩阵；专题文档仍保持可检索、可追踪、可验证。

## 2. Active Gameplay Baseline
| PRD-ID | 当前玩家侧真值 | 权威专题 / 入口 | 当前状态与验证口径 |
| --- | --- | --- | --- |
| PRD-GAME-001 | game 模块以玩法循环和玩家体验为主轴组织需求，不以功能清单平铺。 | `doc/game/gameplay/gameplay-top-level-design.prd.md` | 活跃基线；新增 gameplay 专题需回挂 `doc/game/prd.index.md` 与相关 topic project。 |
| PRD-GAME-002 | 规则层边界必须能映射到 runtime / agent / viewer 可验证语义，不能只停留在概念。 | `doc/game/gameplay/gameplay-engineering-architecture.md` | 作为实现边界参考；具体行为仍由专题与任务 trace 验证。 |
| PRD-GAME-003 | 发布前可玩性结论必须绑定证据、风险等级和 go/no-go 口径。 | `testing-manual.md`, `doc/playability_test_result/prd.md` | release / preview 相关结论必须回到 QA evidence。 |
| PRD-GAME-004 | micro-loop 要让玩家看见动作接受、推进、阻塞、反馈和下一步。 | `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md` | 历史优化已完成；后续只在回归或新体验问题出现时重开。 |
| PRD-GAME-005 | 分布式执行 / 治理能力是长期在线支撑，不是当前首局玩家主循环扩张许可。 | `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06.prd.md` | 保持长期在线验证与治理边界，不扩大早期曝光。 |
| PRD-GAME-006 | 长期在线 P0 能力需覆盖权威分层、回放/回滚、反作弊、经济闭环和运维 runbook。 | `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md` | 作为 release / ops hardening 参考，不替代 player-facing gate。 |
| PRD-GAME-007 | FirstSessionLoop 之后必须有 PostOnboarding 阶段目标、阻塞和下一步承接。 | `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md` | 当前 fresh formal truth 已不再把旧 `trust gate = hold / capability gate = not_run` 当作未解 blocker。 |
| PRD-GAME-008 | pure API 是正式玩家访问模式之一，信息粒度、动作能力和持续游玩必须与 UI 等价；正式游玩要求 active LLM access。 | `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md` | `--no-llm` 仅保留 observer/debug，不支撑正式可玩性或 parity 放行。 |
| PRD-GAME-009 | 当前阶段与 closed beta candidate 需要统一 release gate 和对外 claim envelope，不允许 topic-by-topic 拼凑升阶。 | `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md` | 当前阶段判断仍为 `internal_playable_alpha_late`；对外 claim envelope 维持 `limited playable technical preview`。 |
| PRD-GAME-010 | limited preview 必须是受控、可回流、可纠偏的真实执行闭环。 | `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md` | 当前重点是 controlled builder-facing 回流与 QA/producer 复盘。 |
| PRD-GAME-011 | agent claim 体现持续承诺：首个 claim 也非免费，但 slot-1 可按 runtime 规则使用 restricted starter funding。 | `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md` | restricted grant / upkeep / reclaim / audit 以专题与 runbook 为准。 |
| PRD-GAME-012 | 10-minute trust gate 和 first capability gate 分开判定；当前 early-retention 优先顺序是 trust floor、capability closure、工业状态可读、agency 因果与下一步。 | `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md` | 2026-04-15 `hold/not_run` 仅为历史 baseline；fresh formal truth 已更新为 `trust gate = pass`、`first capability gate = pass`。 |
| PRD-GAME-013 | oasis7 采用真实厘米尺度，但当前玩家主路线仍是间接控制文明模拟；不得把 `1cm` 写成 Minecraft 式逐块直接操作承诺。 | `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md` | 四层合同：厘米真值、coarse-grained 子系统、玩家动作粒度、表现层夸张。 |
| PRD-GAME-014 | 间接控制必须保留 agency：accepted intent、主因果、打断/重排、续玩恢复和 fallback 需要可读、可测。 | `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md` | control-feeling 相关 runtime/viewer/agent/QA 证据以专题 project 与 evidence 为准。 |
| PRD-GAME-015 | mature-world 小玩家需要不依赖立即投靠 major power 的成长线：local operator -> regional specialist -> limited-scope regional influence。 | `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.prd.md` | `protected first industrial win` 指低爆炸半径、可恢复、leverage 可见，不是新手无敌。 |

## 3. Player-Facing Authority Boundary
- `doc/game/prd.md` 只维护当前 gameplay baseline 与专题路由。
- `doc/game/project.md` 只维护当前/近期执行状态、阻断、下一步和历史 trace 指针。
- `doc/game/README.md` 与 `doc/game/gameplay/README.md` 负责首读分流，不复制完整专题清单。
- `doc/game/prd.index.md` 负责完整文件级检索。
- `doc/world-simulator/scenario/*` 与 `doc/world-simulator/m4/*` 可定义 scenario、resource、industrial loop 和 deterministic support contracts；当它们影响玩家侧 progression、resource pressure 或 onboarding 时，应回指 `doc/game` 对玩家体验口径的权威。
- resource terminology 需要区分 built-in runtime resource truth 与 module-defined gameplay material taxonomy；如需改名或改变玩家承诺，必须由 `producer_system_designer` 与 `repository_health_engineer` 共同收口。

## 4. Technical Specifications
- Architecture Overview: game 模块定义玩家侧玩法目标、循环、阶段和验收口径；runtime / world-simulator / viewer / agent 提供实现、观测与验证支撑。
- Integration Points:
  - `doc/game/gameplay/README.md`
  - `doc/game/gameplay/gameplay-top-level-design.prd.md`
  - `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
  - `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
  - `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.prd.md`
  - `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md`
  - `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md`
  - `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
  - `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`
- Edge Cases & Error Handling:
  - 若根 PRD 与专题 PRD 冲突，优先检查更新时间、topic ownership 与 `.pm` task trace；不得用根 PRD 的旧摘要覆盖专题新真值。
  - 若 evidence / project 状态与 PRD baseline 冲突，先保留 blocker 口径并派发对应 owner slice，不直接改写玩家承诺。
  - 若 world-simulator 文档描述资源/工业细节但未说明其 gameplay 权威边界，应补 authority note 或回指 game 专题。
- Non-Functional Requirements:
  - NFR-GAME-1: gameplay 变更必须可追踪到 PRD-ID、专题 PRD 或 `.pm` task trace。
  - NFR-GAME-2: formal playability / preview / release claims 必须绑定 fresh evidence，不得复用已标为 historical baseline 的旧样本。
  - NFR-GAME-3: 根入口不得复制专题完整矩阵；新增细节优先进入专题文档，再从根入口增加一行路由。
  - NFR-GAME-4: 玩家侧 current truth 必须能在 `doc/game/prd.md`、`doc/game/project.md`、对应专题和 evidence 之间闭环检索。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 保持当前 `limited playable technical preview` claim envelope 与 active gameplay baseline。
  - v1.1: 按真实 preview feedback 更新 `PRD-GAME-010`、QA verdict 和 producer decision。
  - v1.2: 若 resource terminology 继续产生误读，单独收口 built-in resource truth vs module gameplay taxonomy。
- Technical / Documentation Risks:
  - 风险-1: 历史流水再次进入根 PRD，导致根入口变重。
  - 风险-2: topic PRD 更新后未同步根 baseline 行，导致首读口径过期。
  - 风险-3: scenario / m4 支撑文档被误读为独立 gameplay promise。

## 6. Validation & Decision Record
- Test Plan & Traceability:
  - 精确检索完整专题: `doc/game/prd.index.md`
  - 当前执行与 blocker: `doc/game/project.md`
  - gameplay topic routing: `doc/game/gameplay/README.md`
  - release / preview / playability evidence: `testing-manual.md`, `doc/playability_test_result/`, `doc/testing/evidence/`
- Root PRD maintenance check:
  - 新增 PRD-GAME-ID 时，必须同步 `doc/game/prd.index.md` 和相关 topic project。
  - 改变默认首读路径时，必须同步 `doc/game/README.md` 与 `doc/game/gameplay/README.md`。
  - 改变当前 stage / claim envelope / gate verdict 时，必须同步 `doc/game/project.md`。
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-GAME-ROOT-001 | 根 PRD 维护 active gameplay baseline + topic links | 根 PRD 继续复制所有专题 user story / matrix / decision log | 降低权威漂移，保留专题可追踪性。 |
| DEC-GAME-ROOT-002 | 当前 stage 继续维持 `internal_playable_alpha_late`，claim envelope 为 `limited playable technical preview` | 用 unified gate pass 直接升级 closed beta candidate | 现有 producer 决策仍要求真实 preview 执行和回流闭环。 |
| DEC-GAME-ROOT-003 | resource / scenario 支撑文档需回指 game 玩家侧权威 | 让 m4 / scenario 文档独立定义玩家 progression promise | 防止资源压力、onboarding 与工业 loop 多头漂移。 |
