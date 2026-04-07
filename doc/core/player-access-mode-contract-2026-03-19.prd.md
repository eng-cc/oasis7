# oasis7：玩家访问模式总契约（Standard 3D / Software-Safe / Pure API）（2026-03-19）

- 对应设计文档: `doc/core/player-access-mode-contract-2026-03-19.design.md`
- 对应项目管理文档: `doc/core/player-access-mode-contract-2026-03-19.project.md`

审计轮次: 7

## 1. Executive Summary
- Problem Statement: 当前三模式 taxonomy 虽已冻结为 `standard_3d / software_safe / pure_api`，但既有口径仍把 `software_safe` 写成“弱图形兜底”，把 `standard_3d` 默认为主 Web 入口；这已经与新的产品方向冲突，因为浏览器主入口需要优先保证 formal gameplay，而不是先把视觉 fidelity 置为默认门槛。
- Proposed Solution: 在 `core` 将三模式重新分工为“`software_safe` = 主要正式 Web 入口、`standard_3d` = 显式 opt-in 的高保真视觉/截图/QA 入口、`pure_api` = 一等公民的无 UI/自动化/长稳入口”，同时继续保留 `agent_direct_connect/provider_loopback_http` 兼容 alias、`agent_decision_source + agent_provider_backend/contract/transport/url/auth/connect_timeout_ms/profile` 正式 provider 维度与 `execution_lane` 的三层分离。
- Success Criteria:
  - SC-1: `software_safe` 在 `core` 契约中被定义为默认 Formal Web gameplay mode，而不是弱图形 fallback-only mode。
  - SC-2: `standard_3d` 在 `core` 契约中被定义为视觉质量、空间语义、截图/QA 的 opt-in 模式，而不是默认 Web 主路径。
  - SC-3: `pure_api` 继续保持一等公民模式，负责无 UI、自动化、长稳与集成场景；不得因 `software_safe` 升格而降为 debug-only。
  - SC-4: release / QA / playability 结论在引用三模式时继续显式绑定 `mode_id`，且不再把“formal Web gameplay”误绑到 `standard_3d`。
  - SC-5: `agent_direct_connect/provider_loopback_http` 在当前产品中只能作为兼容 alias；正式 operator-facing 口径必须回写为 `agent_decision_source + agent_provider_backend/contract/transport/url/auth/connect_timeout_ms/profile + agent_execution_lane`，且与三种玩家访问模式的关系在同一入口中可追溯，不会再被误解为新的玩家入口或唯一配置模型。
  - SC-6: `non-3D`、`2D 优先`、`弱图形优先` 继续只表示 delivery priority / interaction scope，不得被重新包装成新的 mode alias。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要统一“浏览器正式主入口到底是哪一个模式”的产品口径。
  - `qa_engineer`: 需要把“视觉验收通过”和“正式 Web 可玩性通过”拆成不同 mode claim。
  - `viewer_engineer` / `runtime_engineer`: 需要知道 Web 主链路优先补什么能力，哪些动作继续留在 `pure_api` 或专门控制面。
  - `liveops_community`: 需要清楚对外能承诺“浏览器能正式玩到什么程度”，以及不能承诺什么。
- User Scenarios & Frequency:
  - 产品路线评审：每次决定 Web 主入口策略时执行。
  - QA / 发布采证：每次输出 Web formal gameplay 或 visual QA 结论时执行。
  - 下游专题建模：任何涉及 Viewer 主入口、browser auth、formal gameplay、launcher handoff 的专题都必须回查一次。
  - 对外口径整理：每次需要解释“浏览器正式入口为什么不是 3D 主画面”时使用。
- User Stories:
  - PRD-CORE-009: As a `producer_system_designer`, I want `software_safe` to be the primary formal Web mode in the global taxonomy, so that the browser mainline optimizes for playable closure instead of graphics prerequisites.
  - PRD-CORE-009-A: As a `qa_engineer`, I want `standard_3d` and `software_safe` claims explicitly separated, so that a visual PASS never substitutes for formal Web gameplay and vice versa.
  - PRD-CORE-009-B: As a `viewer_engineer`, I want `software_safe` to carry the required formal Web action surface, so that I can prioritize the bounded gameplay UI instead of treating it as a perpetual fallback.
  - PRD-CORE-009-C: As an `agent_engineer`, I want `pure_api` to remain first-class for no-UI automation and durable integration, so that Web-first prioritization does not collapse headless or scripted use cases.
  - PRD-CORE-009-D: As a `producer_system_designer`, I want `non-3D` wording constrained to delivery priority or interaction scope, so that stage strategy still does not mutate into a fake mode taxonomy.
- Critical User Flows:
  1. Flow-PCM-001（模式判定）:
     `读取用户目标 -> 判断是 formal Web gameplay / visual validation / no-UI automation -> 绑定到 software_safe / standard_3d / pure_api 中唯一一项`
  2. Flow-PCM-002（Web 主入口路由）:
     `玩家从浏览器进入 -> 若未显式要求 3D，则默认进入 software_safe -> 如需高保真视觉验证，再显式切到 standard_3d`
  3. Flow-PCM-003（formal Web claim）:
     `执行 Web 试玩 -> 若结论涉及正式可玩性，则证据必须绑定 software_safe -> 若涉及画面/截图/空间语义，则另起 standard_3d claim`
  4. Flow-PCM-004（agent 直连接入对齐）:
     `涉及 agent 直连 -> 先判定玩家访问模式 -> 再记录 compat_access_alias=agent_direct_connect/provider_loopback_http（如适用）与正式 provider 维度 agent_decision_source + agent_provider_backend/contract/transport/url/auth/connect_timeout_ms/profile -> 最后附加 player_parity / headless_agent / debug_viewer 执行 lane -> 禁止任一层反向冒充玩家模式`
  5. Flow-PCM-005（纯接口使用场景）:
     `若用户目标是自动化、长稳、CLI、集成或无 UI 操作 -> 绑定 pure_api -> 保持一等公民，但不把它误写成浏览器主入口`
  6. Flow-PCM-006（对外宣称）:
     `准备 release/playability 口径 -> 读取 mode claim envelope -> 仅输出该模式允许承诺的内容 -> 超出范围则降级或补证据`
  7. Flow-PCM-007（优先级话术归类）:
     `文档里出现 non-3D/2D 优先 -> 先判断它描述的是交付优先级还是真实 mode -> 若在说真实入口，必须回写 software_safe / standard_3d / pure_api`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 玩家访问模式注册表 | `mode_id`、`surface_type`、`default_use_case`、`claim_scope`、`forbidden_claims`、`gameplay_prerequisites` | 任一文档/证据引用模式时必须先绑定注册表条目 | `draft -> frozen -> audited` | `software_safe`、`standard_3d`、`pure_api` 为唯一三项；不允许新增同层别名 | `producer_system_designer` 拥有冻结权；模块 owner 联审 |
| 默认路由策略 | `entry_intent`、`ui_required`、`graphics_fidelity_required`、`llm_required`、`default_mode_id` | 根据目标先选模式，再决定是否允许 3D/浏览器/无 UI | `unclassified -> classified -> executed -> evidenced` | formal Web gameplay 默认优先 `software_safe`；视觉验收才进入 `standard_3d`；无 UI 长稳进入 `pure_api` | QA / release 记录人必须显式写入 |
| formal Web action envelope | `web_gameplay_actions[]`、`excluded_actions[]`、`handoff_surface` | 确定哪些正式玩法动作必须在 `software_safe` 提供，哪些动作显式转交其他入口 | `proposed -> bounded -> implemented -> verified` | `software_safe` 覆盖浏览器正式玩法主链路；资产/治理等专门动作可继续排除并注明 handoff | `viewer_engineer` / `runtime_engineer` / `producer_system_designer` 联审 |
| 证据标签 | `mode_id`、`claim_scope`、`blocked_by`、`environment_class`、`execution_lane` | 生成证据时必须记录模式与阻断类别 | `captured -> reviewed -> accepted/rejected` | 任一证据包只能有一个主 `mode_id`；lane 可附加但不能替代 mode | 所有证据维护者可写，`qa_engineer` 复核 |
| claim envelope | `allowed_claims`、`forbidden_claims`、`requires_browser`、`requires_hardware_gpu`、`requires_canonical_player_gameplay` | 发布/评审前按模式检查允许与禁止宣称项 | `proposed -> bounded -> published` | 若证据跨模式冲突，取更窄 claim；未标模式不得发布 | `producer_system_designer` / `liveops_community` 共同使用 |
| agent provider 映射 | `agent_decision_source`、`agent_provider_backend`、`agent_provider_contract`、`agent_provider_transport`、`agent_provider_url`、`agent_provider_auth_token_ref`、`agent_provider_connect_timeout_ms`、`agent_provider_profile`、`compat_aliases` | 将当前 provider-backed 直连路径记录为结构化 provider 维度；仅在兼容迁移场景保留 `agent_direct_connect/provider_loopback_http` alias | `undefined -> mapped -> documented` | 接入 alias 只回答“历史上怎么叫”；正式 provider 维度回答“当前通过哪类决策源、后端、契约、传输与配置接到 runtime” | `agent_engineer` / `producer_system_designer` 联审 |
| execution lane 映射 | `execution_lane`、`lane_scope`、`player_mode_binding`、`observer_only` | 将 `player_parity/headless_agent/debug_viewer` 作为执行 lane 附加到模式上，而非替代模式 | `unbound -> bound -> audited` | lane 只回答“怎么执行/怎么观战”，不回答“这是哪种玩家入口” | `agent_engineer` / `viewer_engineer` / `runtime_engineer` 联审 |
| 优先级/范围兼容语义 | `priority_label`、`interaction_scope`、`bound_mode_ids`、`forbidden_mode_aliases` | 文档若使用 `non-3D`/`2D 优先`，必须显式说明它只是阶段优先级或交互范围 | `implicit -> clarified -> audited` | 优先级词汇只能描述“当前先做什么”，不能替代 `mode_id` | `producer_system_designer` 冻结；模块 owner 回写 |
- Acceptance Criteria:
  - AC-1: `software_safe`、`standard_3d`、`pure_api` 在同一文档中具备唯一命名、默认用途、claim envelope 与禁宣称项。
  - AC-2: `software_safe` 明确是浏览器主入口对应的 formal Web gameplay mode，默认承接低保真但正式可玩的浏览器体验，而不是 fallback-only 模式。
  - AC-3: `standard_3d` 明确是高保真视觉、截图语义、空间 QA 与 opt-in visual review 模式；未通过硬件 WebGL / headed 证据时不得给出视觉放行。
  - AC-4: `pure_api` 明确保持一等公民地位，负责无 UI、自动化、长稳与集成场景；formal gameplay 仍要求 active LLM access，但它不再承担“主要浏览器入口”的职责。
  - AC-5: `software_safe` 的主玩法 claim 不得自动外推到资产/治理/转账等未暴露在该 Web surface 上的动作面；如未暴露，必须在 contract 里写明 handoff。
  - AC-6: `player_parity / headless_agent / debug_viewer` 继续被定义为 execution lane，而不是新的玩家访问模式。
  - AC-7: `non-3D`、`2D 优先`、`弱图形优先` 继续被定义为交付优先级或交互范围描述，而不是模式别名。
  - AC-8: `testing-manual.md`、`world-simulator`、`game` 与 provider-backed 相关专题均能从本契约追溯到对应下游文档。
- Non-Goals:
  - 不在本专题直接实现 `software_safe` 或 `pure_api` 代码。
  - 不在本专题把 `standard_3d`、`software_safe` 与 `pure_api` 合并成单一入口。
  - 不在本专题把资产/治理/转账等高级动作自动并入 `software_safe`，除非下游专题另有明确 PRD。
  - 不把 `2D/3D` 相机模式、`render profile` 档位或脚本参数升格为新的玩家访问模式。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - 文档治理检查脚本 `./scripts/doc-governance-check.sh`
  - `testing-manual.md`、`world-simulator` 与 `game` 相关 PRD/evidence 作为下游事实源
  - provider 双轨模式 supporting spec 作为 execution lane 参考
  - provider loopback integration spec 作为 `provider_backed + provider_local_bridge + worldsim_provider_v1 + loopback_http` 当前组合映射参考
- Evaluation Strategy:
  - 检查三模式是否具备唯一命名与无歧义 claim envelope。
  - 抽样核对 formal Web gameplay 结论是否默认绑定 `software_safe` 而不是 `standard_3d`。
  - 抽样核对 `pure_api` 是否仍被保留为 first-class no-UI mode，而不是被降为 debug alias。
  - 若发现“把 visual QA 当 formal Web PASS”“把 lane 当 mode”“把 non-3D 当 mode”“把 pure API 降成 debug-only”，则判为不通过。

## 4. Technical Specifications
- Architecture Overview:
  - 本专题位于 `core`，负责冻结项目级玩家访问模式 taxonomy。
  - `world-simulator` 继续拥有 `software_safe` 与 `standard_3d` 的实现与具体验收；其中 `software_safe` 负责 formal Web 主链路，`standard_3d` 负责高保真视觉面。
  - `game` 继续拥有 `pure_api` 的 canonical 玩家语义、正式动作面、LLM-required gameplay gate 与 parity 验收。
  - `world-simulator/llm` 继续拥有 provider-backed 路径与 `player_parity / headless_agent / debug_viewer` execution lane contract，但必须服从本专题的 mode/access/provider/lane 分层；`agent_direct_connect/provider_loopback_http` 仅能作为兼容 alias 保留。
- Integration Points:
  - `testing-manual.md`
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
  - `doc/world-simulator/llm/llm-provider-agent-dual-mode-2026-03-16.prd.md`
  - `doc/world-simulator/llm/provider-agent-dual-mode-contract-2026-03-16.md`
  - `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
  - `doc/testing/evidence/pure-api-parity-validation-2026-03-19.md`
- Edge Cases & Error Handling:
  - 浏览器默认主入口在硬件良好环境下仍落到 `software_safe`：这是符合新 contract 的预期，不得误判为“错误降级”。
  - `render_mode=standard` 显式命中弱图形环境：结果只能记 `blocked_by=graphics_env`；不得借 `software_safe` 结论代签视觉 claim。
  - `software_safe` 缺少某个未纳入其 action envelope 的动作面：结果必须记为 `not_exposed_on_software_safe`，并提供 handoff surface；不得偷写为“formal gameplay 全覆盖”。
  - `software_safe` 在 provider real-play 下只看到 `debug_viewer` observer-only 提示：此时可证明主 Web UI 与旁路观战层可用，但不能证明 Agent 主执行依赖 Viewer。
  - `pure_api` 缺少 canonical `stage/goal/progress/blocker/next_step`: 结论必须降级为 `observer_only`，不得继续宣称正式入口。
  - `pure_api` 在 no-LLM 或 provider init 失败下命中 `llm_mode_required` / `llm_init_failed`: 必须记为 gameplay blocked，而不是 playable parity；`--no-llm` 只允许保留 observer/debug 结论。
  - 同一评审结论同时使用 `software_safe` Web 证据与 `pure_api` 长稳证据：必须拆成两个 claim，或在总述中明确“浏览器正式玩法”与“无 UI 自动化/长稳”是两条不同结论。
- Non-Functional Requirements:
  - NFR-PCM-1: 三模式 taxonomy 在 `core` 中只有一份正式定义，不允许出现第二份同层定义。
  - NFR-PCM-2: 新增涉及三模式的对外/QA/testing 文档 1 个工作日内必须完成与 `core` 契约对齐。
  - NFR-PCM-3: 发布候选级结论中，100% 玩家访问模式相关证据都必须显式标注 `mode_id`。
  - NFR-PCM-4: 所有跨模式 claim 冲突必须在评审时显式拆分，不允许用“综合 PASS”掩盖边界。
  - NFR-PCM-5: execution lane 与 player access mode 的混淆命中数在后续文档审查中应为 0。
  - NFR-PCM-6: `non-3D` / `2D 优先` 与 player access mode 的混淆命中数在活跃文档审查中应为 0。
- Security & Privacy:
  - 本专题不新增权限模型，但要求任何模式分类都不得绕开既有 runtime 鉴权边界。
  - `software_safe` 作为主要 Web 入口时，仍必须显式继承现有 auth/bootstrap/strong-auth 边界；升格不代表降低权限要求。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`PCM-1`): 重写三模式默认用途与 claim envelope，明确 `software_safe` 是主 Web 模式。
  - v1.1 (`PCM-2`): 让 `world-simulator` 和 `testing` 对齐新的 formal Web vs visual QA vs pure API 使用场景。
  - v2.0 (`PCM-3`): 按新的主入口定位补齐 `software_safe` 的 formal Web action envelope 与 handoff surface。
- Technical Risks:
  - 风险-1: 若只改口径不改下游入口，仓内会同时存在“PRD 说主入口是 software_safe、README/脚本仍默认 standard”的双真值。
  - 风险-2: 若把 `software_safe` 升格为主入口却不重新定义动作边界，会形成“主入口可玩但不清楚哪些正式动作不在此 surface”的灰区。
  - 风险-3: 若 `pure_api` 在话术上被边缘化，会损失自动化、长稳与集成链路的正式地位。
  - 风险-4: 若 `standard_3d` 仍被默认拿来做 formal gameplay 放行，会继续把图形环境质量误当作浏览器正式可玩性的门槛。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-CORE-009 | `TASK-CORE-028/049/050/051/052/053/054` | `test_tier_required` | 检查三模式总契约 PRD / design / project 存在且互链，并覆盖 `software_safe` 主 Web 定位、`pure_api` first-class no-UI 定位、provider-backed mode/provider/lane 分层与 `non-3D` 优先级话术约束 | 项目级模式 taxonomy、claim 边界与 formal gameplay 分工一致性 |
| PRD-CORE-009 | `TASK-CORE-053/054/055` | `test_tier_required` | 检查 `doc/core/prd.md`、`doc/core/project.md`、`doc/core/prd.index.md`、`doc/core/README.md` 与 `doc/world-simulator/**` 的当前入口规划已同步回写或明确挂起实现前 follow-up | core 主入口与下游专题可达性 |
| PRD-CORE-009 | `TASK-CORE-053/054/055` | `test_tier_required` | `./scripts/doc-governance-check.sh` + `git diff --check` | 文档树一致性与引用可达性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-PCM-001` | 将 `software_safe / standard_3d / pure_api` 继续定义为唯一三种玩家访问模式，但把 `software_safe` 升格为主要正式 Web 入口 | 继续让 `standard_3d` 当默认 Web 主入口，`software_safe` 只做 fallback | 浏览器正式可玩性优先要覆盖最广环境，而不是把 graphics fidelity 设为默认门槛。 |
| `DEC-PCM-002` | 将 `standard_3d` 收口为显式 opt-in 的视觉/截图/QA 模式 | 保持它既是视觉模式又是默认 Web gameplay 模式 | 视觉验收与正式 Web gameplay 是两种不同 claim，必须拆开。 |
| `DEC-PCM-003` | 保持 `pure_api` 为一等公民模式，负责无 UI/自动化/长稳/集成场景 | 因 `software_safe` 升格而把 `pure_api` 降为 debug-only 或 secondary mode | Web-first 不等于 UI-only；自动化、CLI 与 durable integration 仍需要正式模式承载。 |
| `DEC-PCM-004` | 保持 claim-first 约束，并为 `software_safe` 增加 formal Web action envelope / excluded_actions 语义 | 直接宣称 `software_safe` 覆盖所有正式动作，不做边界说明 | 主入口可以有边界，但边界必须显式、可审计、可 handoff。 |
| `DEC-PCM-005` | 将 `agent_direct_connect/provider_loopback_http` 一起降为兼容 alias，并把正式 operator 配置收口为 `agent_decision_source + agent_provider_* + agent_execution_lane` | 继续把旧 provider brand 当作产品模式名，或把旧单字段 provider mode 继续当对外主入口 | 接入 alias、结构化 provider 维度、实现名与玩家访问面属于不同抽象；继续混写会让文档、CLI、env 与 QA 口径持续漂移。 |
| `DEC-PCM-006` | 将 `player_parity / headless_agent / debug_viewer` 与 `non-3D / 2D 优先` 分别限定为 execution lane、交付优先级/交互范围描述 | 把 lane 或阶段优先级话术继续包装成玩家模式 | lane 回答的是执行/观测方式，优先级词汇回答的是“当前先做什么”；两者都不是玩家访问面。 |
