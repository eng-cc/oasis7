# oasis7：Web UI agent-browser 闭环测试手册（2026-02-28）

- 对应操作手册: `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
- 对应设计文档: `doc/testing/manual/web-ui-playwright-closure-manual.design.md`
- 对应项目管理文档: `doc/testing/manual/web-ui-agent-browser-closure-manual.project.md`

审计轮次: 9

## 1. Executive Summary
- Problem Statement: Web UI 验收若缺少统一启动、采样、门禁与故障分级，且未区分 Viewer 页面与 launcher 控制面的驱动优先级，容易出现“看起来可用但证据不可复现”的假通过。
- Proposed Solution: 保留 agent-browser 作为 Viewer 页面默认闭环方案，但把“逐步执行命令与证据采样”下沉到 `web-ui-agent-browser-closure-manual.manual.md`；本 PRD 只维护边界、成功标准与发布/阻断口径。同时显式规定 `oasis7_web_launcher` / launcher Web 控制面先用 GUI Agent 驱动产品动作，再用页面做状态与字段校验，并统一接入当前 repo-owned Web 回归脚本与 fail-fast 处置。
- Success Criteria:
  - SC-1: S6 Web 闭环流程可由手册命令一键复现，并明确区分 Viewer 与 launcher 控制面两类 surface。
  - SC-2: 验收口径强制 `open ... --headed`，并默认附带 `--use-angle=gl,--ignore-gpu-blocklist`；若仍命中 `SwiftShader/software rendering` 继续阻断。
  - SC-3: 至少输出 `snapshot + console + screenshot + state` 证据。
  - SC-4: 当前 repo-owned Web 回归脚本可直接复用手册约束。
  - SC-5: 文档迁移后统一 `.prd.md/.project.md` 命名并通过治理检查。
  - SC-6: `oasis7_web_launcher` 的产品动作路径默认走 GUI Agent，不再把 canvas 直点或纯 agent-browser 动作作为首选执行链路。
  - SC-7: 当 Viewer 进入 `renderMode=software_safe` 且 auth bootstrap 可用时，QA 可通过专用脚本稳定复验 prompt apply/rollback、chat ack 与消息流采样，并将 `agent_spoke` 缺失分级为可追溯失败签名。
  - SC-8: 当 runtime 以 `OASIS7_RUNTIME_AGENT_CHAT_ECHO=1` 启动时，software-safe Web 闭环应能稳定看到标准 `AgentSpoke` 事件，并将其汇入统一消息流。

## 2. User Experience & Functionality
- User Personas:
  - Web 闭环执行者：按手册运行 agent-browser 并归档证据。
  - 启动器控制面执行者：通过 GUI Agent 驱动 `oasis7_web_launcher` 产品动作，并用页面做结果核对。
  - 发布负责人：用一键脚本快速判断是否可放行。
  - 故障值守人员：根据 fail-fast 等级快速定位问题归属。
- User Scenarios & Frequency:
  - 每次 Viewer Web 相关改动后执行 S6 smoke。
  - 每次 launcher Web 控制面或 GUI Agent 接口改动后，先执行 GUI Agent 动作闭环，再做页面状态核验。
  - 每次候选发布前执行主入口与 `software_safe` Web 回归脚本。
  - 连接失败或渲染崩溃时按 F1~F4 处置并归档证据。
- User Stories:
  - PRD-TESTING-WEB-001: As a Web 闭环执行者, I want deterministic startup and sampling commands, so that I can reproduce browser behavior reliably.
  - PRD-TESTING-WEB-002: As a 发布负责人, I want hard GPU/headed gate and clear pass criteria, so that release decisions are defensible.
  - PRD-TESTING-WEB-003: As a 故障值守人员, I want fail-fast taxonomy with actions, so that incident triage is fast and consistent.
- Critical User Flows:
  1. Flow-WEB-001: `启动 oasis7_game_launcher -> 端口/主页自检 -> 打开 Web 页`
  2. Flow-WEB-002: `Viewer 页面执行 agent-browser 语义步骤 -> 采集状态/日志/截图 -> 关闭会话`
  3. Flow-WEB-002A: `launcher 控制面执行 GUI Agent 动作 -> 页面核对状态/字段 -> 归档响应与截图`
  3. Flow-WEB-003: `执行 GPU/headed 硬门禁 -> 检测软件渲染关键字 -> pass/fail`
  4. Flow-WEB-004: `触发 F1~F4 -> 输出分级结论 -> 归档证据并阻断放行`
  5. Flow-WEB-005: `运行 qa-loop/full-coverage -> 汇总产物 -> 发布评审`
  6. Flow-WEB-006: `强制 software_safe -> 选择 Agent -> prompt apply/rollback -> agent chat -> 采集 chatHistory / agent_spoke 签名`
  7. Flow-WEB-007: `以 env-gated runtime echo 启动 viewer live -> 发送 agent_chat -> step 推进 -> 观测标准 AgentSpoke 进入消息流`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 启动与自检 | `--live-bind`、`--web-bind`、viewer URL、端口监听 | 启动 launcher 并检查 4173/5011 与主页可达 | `booting -> ready` | 先端口后 URL，再进入采样 | 执行者可操作 |
| GPU 硬门禁 | `--headed`、`--use-angle=gl,--ignore-gpu-blocklist`、renderer/console 关键字 (`SwiftShader` 等) | 采样前执行硬门禁检查 | `gating -> pass/fail` | headed 若仍是软件渲染也 fail | 发布/测试共同遵循 |
| agent-browser 采样 | `snapshot`、`eval`、`console`、`screenshot`、`getState` | 基于 `__AW_TEST__` 执行语义步骤 | `sampling -> evidence` | 至少 1 张截图 + state 字段完整 | 执行者产出，发布者审阅 |
| launcher 控制面驱动 | `/api/gui-agent/capabilities`、`/api/gui-agent/state`、`/api/gui-agent/action`、页面字段快照 | 先通过 GUI Agent 执行动作，再用浏览器页面校验结果 | `action_requested -> applied -> verified` | launcher 控制面默认优先，不得被 canvas 直点替代 | 执行者与发布负责人共同审阅 |
| 会话防抖 | `close-all`、fail-fast 预检查 | 每轮清理残留会话并快速失败 | `cleanup -> opened -> stable` | 先清会话后 open，减少残留干扰 | 执行者维护 |
| 发行验收脚本 | `viewer-primary-web-entry-regression.sh`、`viewer-software-safe-step-regression.sh`、`viewer-software-safe-chat-regression.sh` | 一键执行当前 Web 门禁并输出总结 | `running -> summarized` | 先主入口，再 gameplay/blocker，再 prompt/chat | 发布负责人触发 |
| software_safe prompt/chat 回归 | `scripts/viewer-software-safe-chat-regression.sh`、`chatHistory`、`lastPromptFeedback`、`lastChatFeedback` | 强制进入 `software_safe` 并执行 apply/rollback/chat smoke | `bootstrapped -> acked -> evidenced` | 先验 apply/rollback/chat ack，再看 `agent_spoke` 是否在时限内出现 | QA/Viewer owner 共审 |
| 故障分级 | F1~F4 签名、处置动作、证据清单 | 识别错误并匹配处置流程 | `detected -> triaged -> archived` | 连接问题优先于可玩性判定 | 值守与维护者执行 |
- Acceptance Criteria:
  - AC-1: 手册提供可直接复制的启动/采样/门禁命令。
  - AC-2: 明确禁止 headless 验收与软件渲染口径，并声明 headed 若仍落到 SwiftShader 也不得放行。
  - AC-3: 定义最小通过标准（canvas、`__AW_TEST__`、`console error=0`、截图）。
  - AC-4: 提供 F1~F4 分级与对应处置动作。
  - AC-5: 发布脚本产物路径与门禁规则可追溯到本手册。
  - AC-6: 本专题迁移后引用更新到新命名并通过治理检查。
  - AC-7: 手册必须显式声明 `Viewer(agent-browser)` 与 `launcher(GUI Agent first)` 的执行边界，不得让执行者误把 launcher 控制面当作纯 agent-browser 页面驱动对象。
  - AC-8: `scripts/viewer-software-safe-chat-regression.sh` 能产出 `software-safe-chat-summary.json/md`、浏览器环境快照与状态快照，稳定覆盖 prompt apply/rollback、chat ack 与玩家出站消息流；若在时限内未观测到 `agent_spoke`，必须输出可追溯 warning/fail 签名。
  - AC-9: 当 runtime 开启 `OASIS7_RUNTIME_AGENT_CHAT_ECHO=1` 时，手工或自动化 Web 闭环都能观测到一条标准 `AgentSpoke` 事件进入 `chatHistory`，且不依赖自然 LLM 回复。
- Non-Goals:
  - 不在本专题替代 native 抓图应急链路。
  - 不在本专题重构 Viewer 业务逻辑或渲染实现。
  - 不在本专题扩展非 Web 场景测试规范。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: `agent-browser` CLI（二进制命令）用于 Viewer 页面自动化，默认通过 `--use-angle=gl,--ignore-gpu-blocklist` 固定硬件 WebGL 路径（可用 `AGENT_BROWSER_ARGS` 覆盖）；`oasis7_web_launcher` 的 GUI Agent 接口用于 launcher 控制面动作驱动；执行环境需保证两者均可直接调用。
- Evaluation Strategy: 通过语义动作成功率（`__AW_TEST__` 可用性）、门禁通过率和故障分级命中率评估闭环质量。

## 4. Technical Specifications
- Architecture Overview: Web 闭环按 surface 分为两条路径：`oasis7_viewer_live` / Viewer 页面由 `web-ui-agent-browser-closure-manual.manual.md` 负责逐步执行与证据采样，PRD 负责约束与验收；`oasis7_web_launcher` / launcher Web 控制面由 GUI Agent 负责产品动作驱动，再由页面校验状态与字段；发布脚本承载标准化验收与总结。
- Integration Points:
  - `testing-manual.md`
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
  - `scripts/run-viewer-web.sh`
  - `scripts/viewer-primary-web-entry-regression.sh`
  - `scripts/viewer-software-safe-step-regression.sh`
  - `scripts/viewer-software-safe-chat-regression.sh`
  - `agent-browser` CLI（通过 `PATH` 调用）
  - `oasis7_web_launcher` GUI Agent 接口（`/api/gui-agent/*`）
  - `window.__AW_TEST__`（`runSteps/setMode/focus/select/sendControl/getState`）
- Edge Cases & Error Handling:
  - F1 `ERR_CONNECTION_REFUSED`: launcher 未就绪或已退出；先确认端口监听再重试。
  - F2 渲染初始化崩溃（如 `RuntimeError: unreachable`、`CONTEXT_LOST_WEBGL`）：立即归档证据并标记失败。
  - F3 `connecting + tick=0` 长时间不推进：先执行 `play` 并额外观察约 12 秒，仍无推进则失败。
  - F4 URL 在 `source` 场景解析失败：强制使用带引号 URL，避免 `&` 被 shell 截断。
  - 会话残留：每轮前 `close-all`，同名 session 在重新 `open` 前也应先执行 `close`，降低 daemon/session 干扰。
  - headed 仍落到 SwiftShader/software renderer：按环境阻断处理，不得把“窗口能打开”误判成可玩性通过；默认先尝试 `--use-angle=gl,--ignore-gpu-blocklist`，并归档 `browser_env.json`。
  - 视觉门禁假通过：full coverage 需额外校验 `capture_status.txt` 的 `connection_status=connected` 与 `snapshot_ready=1`。
- Non-Functional Requirements:
  - NFR-WEB-1: 首轮 Web smoke 在环境就绪后 5 分钟内完成首个 verdict。
  - NFR-WEB-2: 证据产物当前固定在历史兼容目录 `output/playwright/`，并在手册中显式标注。
  - NFR-WEB-3: 门禁误报率可控，必须通过 fail-fast 分类输出原因。
  - NFR-WEB-4: 关键脚本参数/命令口径在主手册与分册中保持一致。
  - NFR-WEB-5: Viewer Web 验收必须归档 renderer 证据（如 `browser_env.json`），确保能区分硬件路径与 software renderer。
- Security & Privacy: 采样日志与截图不得包含凭据，控制台输出仅保留问题定位所需信息。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (WPCM-1): 从 `testing-manual.md` 拆分 Web agent-browser 分册并建立唯一入口。
  - v1.1 (WPCM-2): 增加启动前自检、会话防抖与 F1~F4 fail-fast 处置。
  - v2.0 (WPCM-3): 强化 GPU + headed 硬门禁与软件渲染阻断。
  - v2.1 (WPCM-4): 对齐一键 QA/full coverage 产物门禁与失败策略。
  - v2.2 (WPCM-5): strict schema 人工迁移与命名统一收口。
- Technical Risks:
  - 风险-1: 环境波动导致端口/连接不稳定，引发假失败。
  - 风险-2: 软件渲染误入验收口径，导致性能与视觉结论失真。
  - 风险-3: 脚本与手册偏离，造成执行路径不一致。
  - 风险-4: 仅凭截图判定导致“有图但不可用”漏检。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-WEB-001 | WPCM-1/2/5 | `test_tier_required` | 启动与采样命令审阅 + 文档治理检查 | Web smoke 执行稳定性 |
| PRD-TESTING-WEB-002 | WPCM-2/3/4 | `test_tier_required` | GPU/headed 门禁与最小通过标准核验 | 发布验收真实性 |
| PRD-TESTING-WEB-003 | WPCM-2/4/5 | `test_tier_required` | F1~F4 分级处置流程抽样 + 脚本产物校验 | 故障处置与审计追溯 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-WEB-001 | Viewer 页面默认走 agent-browser；launcher 控制面默认走 GUI Agent | 继续以 native 抓图为默认；或让 launcher 控制面也直接走纯 agent-browser/canvas | 不同 surface 已具备不同机器接口，按边界分流才能同时获得稳定动作驱动与可审计页面证据。 |
| DEC-WEB-002 | 验收强制 headed + GPU | 允许 headless 或软件渲染 | 避免性能/视觉口径失真。 |
| DEC-WEB-003 | 语义化 `__AW_TEST__` 操作优先 | 纯坐标点击脚本 | 减少 UI 变动导致的脆弱性。 |
| DEC-WEB-004 | 失败分级 F1~F4 + 证据归档 | 仅记录通用失败日志 | 缩短定位时间并提升复盘质量。 |
| DEC-WEB-005 | legacy 文档逐篇人工迁移 | 脚本批量改写 | 保证历史约束和执行语义完整。 |
| DEC-WEB-006 | Viewer Web 默认固定 `--use-angle=gl,--ignore-gpu-blocklist`，若 headed 仍是 software renderer 则继续阻断 | 仅要求 `--headed` 不固定后端 | 当前环境中 headed 默认仍可能回退 SwiftShader，必须把硬件后端策略写进脚本与手册。 |

## 原文约束点映射（内容保真）
- 原“目标：统一 Web 闭环启动、采样、门禁、排障” -> 第 1 章 Problem/Solution/SC。
- 原“S6 启动命令、自检、GPU+headed、采样步骤与会话防抖” -> 第 2 章流程/规格矩阵 + 第 4 章技术规格。
- 原“最小通过标准（canvas、`__AW_TEST__`、console、截图）” -> 第 2 章 AC。
- 原“Fail Fast F1~F4 与处置” -> 第 4 章 Edge Cases & Error Handling。
- 原“一键发行验收与 full coverage 门禁” -> 第 2 章 Flow-WEB-005 + 第 5 章 roadmap。
- 原“Web 默认链路、native fallback、语义操作约定、调试收尾建议” -> 第 4 章架构/集成与 NFR。
