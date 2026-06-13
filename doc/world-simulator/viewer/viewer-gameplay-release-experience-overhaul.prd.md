# Viewer 发行体验改造（游戏化优先）

- 对应设计文档: `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.project.md`

审计轮次: 5

## ROUND-002 物理合并
- 本文件为主文档（当前权威入口）。
- `immersion-phase8~10` 内容已物理合并入本文件，对应阶段文档已合并并从仓库移除（不再保留 archive 目录）。

## 1. Executive Summary
- 将 `oasis7_viewer` 默认体验从“调试工具”切换为“可发行游戏前端”。
- 保持世界画面为第一视觉焦点，隐藏技术细节并改为按需显式唤出。
- 提升新玩家可上手性：进入后快速知道“当前目标”和“下一步动作”。
- 增强情绪反馈：关键事件必须有可感知的视觉/文本反馈。

## 2. User Experience & Functionality
- 修改 `crates/oasis7_viewer` 的默认 UI 行为与展示优先级。
- 引入面向玩家的体验模式（Player）与保留调试能力的模式（Director）。
- 优化默认模块可见性、默认面板状态、入口可发现性、基础提示层。
- 在不改协议字段的前提下，重排现有状态信息在 UI 中的展示方式。

## 非目标
- 本阶段不改动 `oasis7` 网络协议与仿真核心业务逻辑。
- 本阶段不引入重资产美术包（大型模型/贴图替换作为后续任务）。
- 本阶段不改动 third_party 代码。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 环境变量与模式解析
- 新增环境变量：`OASIS7_VIEWER_EXPERIENCE_MODE`
  - `player`：默认玩家模式（世界优先、面板默认隐藏）。
  - `director`：导演/调试模式（保持现有可观测能力）。
- 未设置或非法值时，默认 `player`。

### 运行时资源
- 新增资源：`ViewerExperienceMode`（Player/Director）。
- 新增启动策略：按模式覆盖以下默认状态。
  - `RightPanelLayoutState`（是否默认隐藏面板、顶部折叠）。
  - `RightPanelModuleVisibilityState`（默认模块集合）。

### UI 结构约束
- Player 模式默认：
  - 右侧主面板隐藏。
  - 保留单点“打开面板”入口。
  - 打开后仅显示轻量模块（总览优先，技术模块按需展开）。
- Director 模式默认：
  - 保持现有调试导向行为。

### 可发现性增强
- 为关键入口增加显式提示文案（例如：快捷键提示、模式提示）。
- 不依赖用户阅读手册即可发现“如何打开面板”和“如何聚焦对象”。

## 5. Risks & Roadmap
- [x] M1：文档与任务拆解完成（设计文档 + 项目管理文档）。
- [x] M2：体验模式框架落地（资源、环境变量、默认策略切换）。
- [x] M3：Player 模式默认降噪落地（默认隐藏面板 + 轻量默认模块）。
- [x] M4：入口可发现性与基础提示完成（面板入口提示、模式提示）。
- [x] M5：测试回归、项目文档状态回写、devlog 收口。

## 验收指标
- 默认启动时，世界画面可视占比显著提升（右侧大面板不再常驻）。
- 新用户无需文档可在首屏找到“打开面板”入口。
- Player 模式下首屏技术模块数量明显减少。
- Director 模式仍可访问原有调试能力。

### Technical Risks
- 默认隐藏面板可能导致“信息不可见”感受上升。
  - 对策：提供显式入口按钮与清晰提示文案。
- 双模式并行可能引入状态分歧。
  - 对策：模式策略仅影响“默认值”，运行时状态保持统一数据结构。
- 旧测试可能绑定既有默认 UI 行为。
  - 对策：补充模式相关测试并更新默认行为断言。

## 当前结论
- 默认体验已切换至 `Player`：右侧面板默认隐藏，首屏世界视图占比显著提高。
- 已保留 `Director` 路径：通过 `OASIS7_VIEWER_EXPERIENCE_MODE=director` 可回到调试导向默认值。
- Web 闭环 smoke 已验证：
  - `window.__AW_TEST__` 可用；
  - `connectionStatus=connected`；
  - `canvasCount=1`；
  - console 无 error（仅浏览器 warning）。

## Phase 8~10 增量记录（ROUND-002 物理合并）
- 原阶段文档已合并并删除，以下记录用于追溯。

### Phase 8：信息分层减负与焦点收敛

#### 1. Executive Summary
- 在保持“可随时指挥 Agent”前提下，进一步降低 Player 首屏的信息密度，避免左侧引导卡片重复表达同一目标。
- 让玩家在隐藏态与展开态都能快速识别“当前应该做什么”，减少任务文案和引导文案的视觉竞争。
- 在面板展开时压缩任务 HUD 体量，确保视线焦点优先回到世界场景和指挥面板。

#### 2. User Experience & Functionality
- `crates/oasis7_viewer` Player 模式 UI 减负改造：
  - 调整“下一步目标提示卡”与“新手引导卡”的共存策略，消除重复提示。
  - 调整任务 HUD 在不同面板状态下的布局锚点与信息密度（展开态更紧凑）。
  - 保留并强化“任务目标 + 指挥入口 + 世界反馈”三要素的可达性。
- 不改动 Director 模式行为。

#### 非目标
- 不改动仿真协议、事件语义和后端链路。
- 不改动 `third_party` 代码。
- 不在本阶段引入新的 UI 框架或复杂动画系统。

#### 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

#### 4. Technical Specifications

##### 复用状态
- `PlayerOnboardingState`：用于判定当前步骤是否仍需展示新手引导。
- `RightPanelLayoutState`：用于判定面板隐藏/展开，并驱动 HUD 密度切换。
- `PlayerGuideProgressSnapshot` 与 `PlayerGuideStep`：用于任务目标与引导步骤一致化。

##### 新增/调整行为
- 目标提示卡显示策略：
  - 仅在“面板隐藏且当前步骤引导卡已收起”时显示，避免与引导卡重复。
- 任务 HUD 自适应策略：
  - 面板隐藏且引导卡显示时，下移任务 HUD 锚点避免叠压。
  - 面板展开时启用紧凑模式（保留核心目标与进度，降低次要文案占比）。

#### 5. Risks & Roadmap
- M1：第八阶段建档完成（设计 + 项管）。
- M2：完成目标提示卡与引导卡去重策略，并有单测覆盖。
- M3：完成任务 HUD 锚点与紧凑模式改造，并有单测覆盖。
- M4：完成回归测试、Web 闭环验证与文档收口。

##### Technical Risks
- 风险 1：提示层收敛后，部分玩家可能遗漏“下一步目标”。
  - 对策：保留顶部 compact HUD 的 objective 字段，并在引导卡收起后恢复底部目标提示卡。
- 风险 2：展开态任务 HUD 过度压缩导致可读性下降。
  - 对策：仅压缩冗余文案与奖励块，保留目标标题、进度条与关键动作按钮。
- 风险 3：新增状态分支导致行为不可预测。
  - 对策：将显示策略抽为纯函数并增加定向单测，保证规则可验证。

#### 验收结果（VRI8P3）
- 回归结果：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer` 通过（325 tests）。
  - `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer --target wasm32-unknown-unknown` 通过。
- Web 闭环（agent-browser，按 `testing-manual.md` S6）：
  - 使用 `?test_api=1` 访问 viewer，`window.__AW_TEST__` 可用，状态采样为已连接且 tick 正常推进。
  - 完成隐藏态、聚焦态、面板展开态（紧凑任务 HUD）与再次收起态截图采样。
  - Console 汇总：`Total messages: 12 (Errors: 0, Warnings: 2)`，无新增功能错误。
- 验收产物：
  - `output/playwright/viewer/phase8/phase8-hidden-default.png`
  - `output/playwright/viewer/phase8/phase8-hidden-focused.png`
  - `output/playwright/viewer/phase8/phase8-panel-open-compact.png`
  - `output/playwright/viewer/phase8/phase8-panel-hidden-after-toggle.png`
  - `output/playwright/viewer/console-2026-02-23T12-40-54-234Z.log`
- 结论：
  - 目标提示与引导提示的重复展示已消除。
  - 任务 HUD 在展开态的体量显著收敛，隐藏态与引导卡不再发生叠压。
  - “世界优先 + 可随时指挥 Agent”的布局目标在 phase8 达成。

#### 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。

### Phase 9：指挥链路前置与顶层减噪

#### 1. Executive Summary
- 继续降低 Player 首屏 UI 压力，消除顶部叠层竞争，让世界画面保持第一视觉焦点。
- 保证“玩家随时可指挥 Agent”是隐藏态下的一步操作，不需要先进入中间层再找 Chat。
- 在保留布局预设能力的同时，把其从“常驻干扰”改成“展开态高级能力”。

#### 2. User Experience & Functionality
- `crates/oasis7_viewer` Player 模式体验层调整：
  - 顶部布局预设条从隐藏态移除，仅在面板展开态展示，并下移锚点避免与顶部 HUD 叠压。
  - 在任务 HUD（隐藏态）增加“直接指挥 Agent”动作，点击后直接切到 Command 预设（展开面板+打开 Chat）。
  - 增加纯函数测试覆盖，确保新显示策略与一键指挥策略可回归验证。
- 不改动 Director 模式行为。

#### 非目标
- 不改动仿真协议、事件语义、后端链路。
- 不改动 `third_party`。
- 不在本阶段引入重资产特效系统与复杂动画框架。

#### 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

#### 4. Technical Specifications

##### 复用状态
- `RightPanelLayoutState`：决定面板隐藏/展开状态，驱动“布局预设条显隐”和“任务 HUD 指挥动作”显隐。
- `RightPanelModuleVisibilityState`：用于应用 Command 预设时切换 Chat/Timeline/Details 可见性。
- `PlayerGuideStep` 与 `PlayerGuideProgressSnapshot`：维持任务目标与动作按钮的语义一致性。

##### 新增/调整行为
- 顶部布局预设条策略：
  - 隐藏态不显示。
  - 展开态显示，并使用下移锚点避免与 compact HUD 同层竞争。
- 隐藏态任务 HUD 策略：
  - 新增“直接指挥 Agent”按钮。
  - 点击后直接应用 `PlayerLayoutPreset::Command`，保证 Chat 入口一步可达。

#### 5. Risks & Roadmap
- M1：第九阶段建档完成（设计 + 项管）。
- M2：顶部布局预设条减噪策略完成并有单测覆盖。
- M3：隐藏态任务 HUD 一键指挥入口完成并有单测覆盖。
- M4：回归测试、Web 闭环验收与文档收口完成。

##### Technical Risks
- 风险 1：隐藏态移除布局预设条后，玩家可能不易发现“布局切换”能力。
  - 对策：该能力保留在展开态，且隐藏态强调“直接指挥 Agent”主路径。
- 风险 2：新增任务 HUD 按钮过多导致操作犹豫。
  - 对策：仅在隐藏态显示“直接指挥 Agent”，展开态不重复显示。
- 风险 3：布局锚点调整引入新的 UI 叠压。
  - 对策：把显隐/锚点规则抽成纯函数并补充定向单测。

#### 验收结果（VRI9P3）
- 回归结果：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer` 通过（328 tests）。
  - `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer --target wasm32-unknown-unknown` 通过。
- Web 闭环（agent-browser，按 `testing-manual.md` S6）：
  - 使用 `?test_api=1` 访问 viewer，`window.__AW_TEST__` 可用。
  - 执行 `runSteps(...)` 与 `Tab` 展开/收起链路，采样 `getState()` 为已连接。
  - Console 汇总：`Total messages: 11 (Errors: 0, Warnings: 1)`，无功能错误。
- 验收产物：
  - `output/playwright/viewer/phase9/phase9-hidden-default.png`
  - `output/playwright/viewer/phase9/phase9-hidden-focused.png`
  - `output/playwright/viewer/phase9/phase9-panel-open-command.png`
  - `output/playwright/viewer/phase9/phase9-panel-hidden-after-toggle.png`
  - `output/playwright/viewer/console-2026-02-23T13-04-48-150Z.log`
- 结论：
  - 隐藏态已移除布局预设条，首屏顶部竞争减弱。
  - 展开态布局预设条下移后不再与 compact HUD 同位叠压。
  - 隐藏态任务 HUD 已提供“直接指挥 Agent”入口，玩家可一键进入 Chat 指挥路径。

#### 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。

### Phase 10：新手流程闭环与认知减负

#### 1. Executive Summary
- 修复新手流程中的“进度跳步”问题，确保教程按真实行为推进，不再出现 2/4 直接到 4/4。
- 把第 4 步操作提示从抽象描述改成可执行动作提示，让玩家明确“下一步点哪里”。
- 继续压缩隐藏态信息密度，减少首屏多卡片并行发声导致的认知负担。

#### 2. User Experience & Functionality
- `crates/oasis7_viewer` Player 模式 UI/引导逻辑调整：
  - 调整 `PlayerGuideProgressSnapshot` 的完成判定，新增“执行反馈”门槛后再允许 4/4 完成。
  - 调整 `ExploreAction` 阶段的任务动作文案，改为具体可执行的控制建议。
  - 调整隐藏态入口信息层级，避免与新手引导卡/任务 HUD 同时形成过多主卡片。
  - 补充纯函数与引导流程相关单测。
- 不改动 Director 模式调试语义。

#### 非目标
- 不改动仿真协议、事件语义与后端链路。
- 不改动 `third_party`。
- 不引入新的大型美术资源或复杂动效系统。

#### 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

#### 4. Technical Specifications

##### 复用状态
- `ViewerState.events`：用于判定是否出现“执行反馈”。
- `RightPanelLayoutState`：用于隐藏态/展开态的引导层级策略。
- `PlayerGuideStep` / `PlayerGuideProgressSnapshot`：维持教程步骤与任务 HUD 一致性。

##### 新增/调整行为
- 教程完成门槛：
  - `ExploreAction` 不再由“已选中目标”直接达成。
  - 需要出现“新反馈事件”后才计入第 4 步完成（忽略历史事件，避免首帧秒完成）。
- 文案策略：
  - 第 4 步动作提示改为“点击直接指挥并发送一条指令”等具体可执行动作。
- 隐藏态减噪：
  - 玩家模式隐藏态不再渲染右上大入口卡，保留边缘抽屉与任务 HUD 指挥入口。
  - Director 模式保持原有右上入口卡，避免调试能力回退。

#### 5. Risks & Roadmap
- M1：第十阶段建档完成（设计 + 项管）。
- M2：教程进度门槛与第 4 步动作文案改造完成并有单测覆盖。
- M3：隐藏态减噪策略完成并有定向验证。
- M4：回归测试、agent-browser 新手流程复测与文档收口完成。

##### Technical Risks
- 风险 1：第 4 步门槛提高后，部分场景下完成速度下降。
  - 对策：采用“历史事件不计入、仅新反馈事件计入”策略，避免首帧秒完成同时防止门槛过严。
- 风险 2：隐藏态入口卡减弱后可发现性下降。
  - 对策：保留边缘呼出入口与任务 HUD 指挥按钮双入口。
- 风险 3：文案更具体后中英文一致性回归。
  - 对策：同步更新中英文文案并覆盖关键断言测试。

#### 验收记录（VRI10P3）
- 回归命令：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer`
  - `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer --target wasm32-unknown-unknown`
- Web 闭环（agent-browser）：
  - 使用 `oasis7_viewer_live + run-viewer-web.sh + agent-browser` 复测新手流程（隐藏态 -> 展开面板 -> 选择目标 -> 隐藏态复查）。
  - 控制台记录：`output/playwright/viewer/console-2026-02-23T13-41-21-606Z.log`（`Errors: 0`）。
  - 截图产物：
    - `output/playwright/viewer/phase10/step0-hidden-initial.png`
    - `output/playwright/viewer/phase10/step1-open-panel-tab.png`
    - `output/playwright/viewer/phase10/step2-selected-agent.png`
    - `output/playwright/viewer/phase10/step3-after-play-feedback.png`
    - `output/playwright/viewer/phase10/step4-hidden-no-top-card.png`

#### 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
