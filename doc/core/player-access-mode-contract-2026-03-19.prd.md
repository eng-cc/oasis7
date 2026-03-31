# oasis7：玩家访问模式总契约（Standard 3D / Software-Safe / Pure API）（2026-03-19）

- 对应设计文档: `doc/core/player-access-mode-contract-2026-03-19.design.md`
- 对应项目管理文档: `doc/core/player-access-mode-contract-2026-03-19.project.md`

审计轮次: 6

## 1. Executive Summary
- Problem Statement: 当前仓内已经形成 `3D`、`software_safe` 与 `pure API` 三条实际可用路径，但模式定义仍分散在 `world-simulator`、`game`、testing 与 evidence 文档中，容易把“玩家入口模式”“弱图形兜底模式”“无 UI 长玩模式”与 `OpenClaw` 的执行 lane 混为一谈；同时 `pure_api` 是否要求 active LLM access 的口径在代码、脚本与文档之间也已漂移。
- Proposed Solution: 在 `core` 建立一份跨模块三模式总契约，统一三种玩家访问模式的命名、默认用途、fallback 规则、证据门禁、允许宣称项与禁止宣称项，并明确 `player_parity / headless_agent / debug_viewer` 属于执行 lane，而不是玩家入口模式。
- Success Criteria:
  - SC-1: `standard_3d`、`software_safe`、`pure_api` 三种模式在 `core` 中具备唯一命名、默认用途、放行边界与禁宣称项。
  - SC-2: 发布、QA、playability 与 testing 相关文档在引用三模式时不再混用 `execution lane` 语义。
  - SC-3: 任何视觉质量或截图语义结论都必须显式绑定 `standard_3d`，不得借 `software_safe` 或 `pure_api` 代签。
  - SC-4: 任何“无 GPU 可玩”结论都必须显式绑定 `software_safe`，不得误写成标准 3D 兼容。
  - SC-5: 任何“无 UI 持续游玩 / canonical 玩家语义 / formal pure_api gameplay”结论都必须显式绑定 `pure_api`，并声明 active LLM access 为正式游玩前置；不得外推到视觉等价、no-LLM playability 或 LLM 专属动作豁免。
  - SC-6: `OpenClaw` 的 `player_parity / headless_agent / debug_viewer` 与三模式的关系在同一入口中可追溯，且不会再被操作者误解为第四、第五套玩家入口。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要统一“现在到底有哪三种模式、各自能承诺什么”的产品口径。
  - `qa_engineer`: 需要按模式给出可审计结论，而不是把不同证据混写成一个笼统 PASS。
  - `viewer_engineer` / `runtime_engineer` / `agent_engineer`: 需要清楚哪些是玩家访问模式，哪些只是执行 lane 或 debug 旁路。
  - `liveops_community`: 需要知道对外能说什么、不能说什么，避免过度承诺。
- User Scenarios & Frequency:
  - 发布前评审：每个候选版本至少 1 次，用于确认结论绑定到正确模式。
  - QA / playability 采证：每次 required/full 或人工长玩结论产出时执行。
  - 新专题建模：任何再涉及 Viewer / no-GPU / pure API / OpenClaw 模式描述时都必须回查一次。
  - 对外口径整理：每次需要解释“为什么这里能玩、但不代表画面通过”时使用。
- User Stories:
  - PRD-CORE-009: As a `producer_system_designer`, I want one contract for `standard_3d / software_safe / pure_api`, so that product, QA, and release claims all use the same mode taxonomy.
  - PRD-CORE-009-A: As a `qa_engineer`, I want each evidence bundle to map to exactly one player access mode, so that conclusions do not over-claim.
  - PRD-CORE-009-B: As a `viewer_engineer`, I want `software_safe` and `standard_3d` explicitly separated, so that weak-graphics fallback does not silently redefine the visual acceptance bar.
  - PRD-CORE-009-C: As an `agent_engineer`, I want `pure_api` and `headless_agent` clearly distinguished, so that no-UI player parity and OpenClaw execution lanes do not fork into ambiguous product language.
- Critical User Flows:
  1. Flow-PCM-001（模式判定）:
     `读取用户目标 -> 判断是视觉验收 / 弱图形可玩 / 无 UI 长玩 -> 绑定到 standard_3d / software_safe / pure_api 中唯一一项`
  2. Flow-PCM-002（fallback 判定）:
     `Web 启动 -> 若硬件 3D 可用则走 standard_3d -> 若图形环境受限且允许 fallback 则走 software_safe -> 若用户无需浏览器则走 pure_api；若缺少可用 LLM provider 则 formal gameplay 阻断`
  3. Flow-PCM-003（证据归档）:
     `执行测试或人工试玩 -> 证据包记录 mode_id / claim_scope / blocked_by -> 输出仅属于该模式的结论`
  4. Flow-PCM-004（执行 lane 对齐）:
     `涉及 OpenClaw -> 先判定玩家访问模式 -> 再附加 player_parity / headless_agent / debug_viewer 执行 lane -> 禁止 lane 反向冒充玩家模式`
  5. Flow-PCM-005（对外宣称）:
     `准备 release/playability 口径 -> 读取 mode claim envelope -> 仅输出该模式允许承诺的内容 -> 超出范围则降级或补证据`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 玩家访问模式注册表 | `mode_id`、`surface_type`、`entry_path`、`default_use_case`、`fallback_target`、`claim_scope`、`forbidden_claims` | 任一文档/证据引用模式时必须先绑定注册表条目 | `draft -> frozen -> audited` | `standard_3d`、`software_safe`、`pure_api` 为唯一三项；不允许新增同层别名 | `producer_system_designer` 拥有冻结权；模块 owner 联审 |
| 默认路由策略 | `entry_intent`、`graphics_env`、`ui_required`、`llm_required`、`fallback_allowed` | 根据目标选择模式，不得“先跑了再事后解释” | `unclassified -> classified -> executed -> evidenced` | 先按用户目标分流，再按环境约束分流；若目标冲突，以更窄 claim 为准 | QA / release 记录人必须显式写入 |
| 证据标签 | `mode_id`、`claim_scope`、`blocked_by`、`environment_class`、`execution_lane` | 生成证据时必须记录模式与阻断类别 | `captured -> reviewed -> accepted/rejected` | 任一证据包只能有一个主 `mode_id`；lane 可附加但不能替代 mode | 所有证据维护者可写，`qa_engineer` 复核 |
| claim envelope | `allowed_claims`、`forbidden_claims`、`requires_hardware_gpu`、`requires_browser`、`requires_canonical_player_gameplay` | 发布/评审前按模式检查允许与禁止宣称项 | `proposed -> bounded -> published` | 若证据跨模式冲突，取 claim 更窄的一侧；未标模式不得发布 | `producer_system_designer` / `liveops_community` 共同使用 |
| execution lane 映射 | `execution_lane`、`lane_scope`、`player_mode_binding`、`observer_only` | 将 `player_parity/headless_agent/debug_viewer` 作为执行 lane 附加到模式上，而非替代模式 | `unbound -> bound -> audited` | lane 只回答“怎么执行/怎么观战”，不回答“这是哪种玩家入口” | `agent_engineer` / `viewer_engineer` / `runtime_engineer` 联审 |
| fallback 与阻断语义 | `fallback_reason`、`blocked_reason`、`degraded_to`、`recovery_hint` | 环境失败时给出显式降级或阻断，不允许静默改口径 | `normal -> degraded -> blocked/recovered` | `standard_3d` 失败可降到 `software_safe`；`pure_api` 不因浏览器问题而被判失败 | QA 记录；模块 owner 回写 |
- Acceptance Criteria:
  - AC-1: `standard_3d`、`software_safe`、`pure_api` 在同一文档中具备唯一命名、默认用途、fallback 规则与禁宣称项。
  - AC-2: `standard_3d` 明确是视觉质量、截图语义、高保真交互与产品主画面验收口径；未通过硬件 WebGL / headed 证据时不得给出视觉放行。
  - AC-3: `software_safe` 明确是无 GPU / 弱图形环境下的最小玩法闭环与调试兜底口径；不得被用来宣称标准 3D 兼容或视觉等价。
  - AC-4: `pure_api` 明确是无 UI 正式玩家入口，但 formal gameplay 仍要求 active LLM access；不得被用来宣称截图语义、视觉质量、no-LLM playability 或 LLM 专属动作豁免。
  - AC-5: `player_parity / headless_agent / debug_viewer` 在本契约中被定义为执行 lane，而不是玩家访问模式。
  - AC-6: 任一 release/playability/testing 结论若跨模式借证据，必须显式降级 claim 或补齐缺失证据。
  - AC-7: `testing-manual.md`、`world-simulator`、`game` 与 `OpenClaw` 相关专题均能从本契约追溯到对应下游文档。
  - AC-8: 新增 topic 后，`doc/core/prd.md`、`doc/core/project.md`、`doc/core/prd.index.md` 与 `doc/core/README.md` 同步回写。
- Non-Goals:
  - 不在本专题重做 `standard_3d`、`software_safe` 或 `pure_api` 的实现代码。
  - 不在本专题修改 OpenClaw observation/action contract 细节。
  - 不把 `2D/3D` 相机模式、`render profile` 档位或具体测试脚本参数当作新的玩家访问模式。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - 文档治理检查脚本 `./scripts/doc-governance-check.sh`
  - `testing-manual.md`、`world-simulator` 与 `game` 相关 PRD/evidence 作为下游事实源
  - `OpenClaw` 双轨模式 supporting spec 作为 execution lane 参考
- Evaluation Strategy:
  - 检查三模式是否具备唯一命名与无歧义 claim envelope。
  - 抽样核对 `software_safe`、`pure_api`、`OpenClaw dual-mode`、testing evidence 是否仍使用一致术语。
  - 若发现“同一结论同时宣称视觉放行与无 GPU 兜底”“把 lane 当 mode”或“pure API 外推到 LLM 视觉等价”，则判为不通过。

## 4. Technical Specifications
- Architecture Overview:
  - 本专题位于 `core`，负责冻结项目级玩家访问模式 taxonomy。
  - `world-simulator` 继续拥有 `standard_3d` 与 `software_safe` 的实现与具体验收。
  - `game` 继续拥有 `pure_api` 的 canonical 玩家语义、正式动作面、LLM-required gameplay gate 与 parity 验收。
  - `OpenClaw` 专题继续拥有 `player_parity / headless_agent / debug_viewer` 的 execution lane contract，但必须服从本专题的 mode/lane 分层。
- Integration Points:
  - `testing-manual.md`
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
  - `doc/world-simulator/llm/llm-openclaw-agent-dual-mode-2026-03-16.prd.md`
  - `doc/world-simulator/llm/openclaw-agent-dual-mode-contract-2026-03-16.md`
  - `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
  - `doc/testing/evidence/pure-api-parity-validation-2026-03-19.md`
- Edge Cases & Error Handling:
  - 标准 Web 3D 在 `render_mode=standard` 下命中弱图形环境：结果只能记 `blocked_by=graphics_env`；不得偷偷借 `software_safe` 结论代签。
  - `render_mode=auto` 自动降到 `software_safe`: 结果必须记录 `degraded_to=software_safe` 与 `software_safe_reason`，且视觉结论仍视为未验证。
  - `software_safe` 在 OpenClaw real-play 下只看到 `debug_viewer` observer-only 提示：此时可证明弱图形观战链路可用，但不能证明 Agent 主执行依赖 Viewer。
  - `pure_api` 缺少 canonical `stage/goal/progress/blocker/next_step`: 结论必须降级为 `observer_only`，不得继续宣称正式入口。
  - `pure_api` 在 no-LLM 或 provider init 失败下命中 `llm_mode_required` / `llm_init_failed`: 必须记为 gameplay blocked，而不是 playable parity；`--no-llm` 只允许保留 observer/debug 结论。
  - 同一评审结论同时使用 `standard_3d` 截图与 `pure_api` 长玩证据：必须拆成两个 claim，或在总述中明确“视觉放行”和“无 UI 持续游玩放行”是两条不同结论。
  - 操作者把 `headless_agent` 写成第三种玩家模式：视为 taxonomy 错误，必须回写修正后才能放行。
- Non-Functional Requirements:
  - NFR-PCM-1: 三模式 taxonomy 在 `core` 中只有一份正式定义，不允许出现第二份同层定义。
  - NFR-PCM-2: 新增涉及三模式的对外/QA/testing 文档 1 个工作日内必须完成与 `core` 契约对齐。
  - NFR-PCM-3: 发布候选级结论中，100% 玩家访问模式相关证据都必须显式标注 `mode_id`。
  - NFR-PCM-4: 所有跨模式 claim 冲突必须在评审时显式拆分，不允许用“综合 PASS”掩盖边界。
  - NFR-PCM-5: execution lane 与 player access mode 的混淆命中数在后续文档审查中应为 0。
- Security & Privacy:
  - 本专题不新增权限模型，但要求任何模式分类都不得绕开既有 runtime 鉴权边界。
  - `pure_api` 与 `player_parity` 的语义范围不得借 taxonomy 变更泄露玩家不应看到的隐藏真值。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`PCM-1`): 冻结三模式命名、默认用途、fallback 规则与 claim envelope。
  - v1.1 (`PCM-2`): 将发布、testing、playability 与 OpenClaw 相关入口对齐到 mode/lane 双层术语。
  - v2.0 (`PCM-3`): 如后续出现新的正式玩家访问模式，再经 `core` 显式升格；否则不得新增同层模式别名。
- Technical Risks:
  - 风险-1: 若继续把 `software_safe`、`pure_api` 与 `headless_agent` 混写，会导致 release claim 失真。
  - 风险-2: 若自动 fallback 不记录 mode 变化，标准 3D 回归会被弱图形兜底掩盖。
  - 风险-3: 若 pure API 结论被外推到视觉或 LLM 等价，会形成超出证据范围的对外承诺。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-CORE-009 | `TASK-CORE-028` | `test_tier_required` | 检查三模式总契约 PRD / design / project 存在且互链 | 项目级模式 taxonomy 统一 |
| PRD-CORE-009 | `TASK-CORE-028` | `test_tier_required` | 检查 `doc/core/prd.md`、`doc/core/project.md`、`doc/core/prd.index.md`、`doc/core/README.md` 已同步挂载专题 | core 主入口可达性 |
| PRD-CORE-009 | `TASK-CORE-028` | `test_tier_required` | `./scripts/doc-governance-check.sh` + `git diff --check` | 文档树一致性与引用可达性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-PCM-001` | 将 `standard_3d / software_safe / pure_api` 定义为唯一三种玩家访问模式 | 继续让各模块专题各自定义模式，不做总契约 | 当前冲突已跨越模块边界，必须在 `core` 收口。 |
| `DEC-PCM-002` | 将 `player_parity / headless_agent / debug_viewer` 明确定义为 execution lane | 把它们直接并列为第四、第五种玩家模式 | 这些字段回答的是执行/观测方式，不是玩家访问面。 |
| `DEC-PCM-003` | 采用 claim-first 约束：先定义每种模式“能承诺什么”，再让证据归档 | 先跑出证据再事后解释模式 | 没有 claim envelope，QA 与对外口径会持续漂移。 |
