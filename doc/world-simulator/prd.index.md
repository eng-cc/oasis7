# world-simulator PRD 文件级索引

审计轮次: 8

更新时间：2026-07-25

## 用途
- 本页是 `world-simulator` 的文件级索引，优先用于按文件名精确检索、确认配对关系与维持互链可达性。
- 如果你是第一次进入本模块，先读 `doc/world-simulator/README.md`；需要规格或执行真值时，再读 `prd.md` / `project.md`。

## 入口
- 模块 PRD：`doc/world-simulator/prd.md`
- 模块设计总览：`doc/world-simulator/design.md`
- 模块标准执行入口：`doc/world-simulator/project.md`

## 快速分流
- 想先回答模块在做什么、能力边界是什么：先读 `doc/world-simulator/prd.md`
- 想先回答当前在推进什么、谁在负责、哪里被阻断：先读 `doc/world-simulator/project.md`
- 想先进入 Viewer 热点子域，而不是直接面对 199 份 Viewer Markdown：先读 `doc/world-simulator/viewer/README.md`
- 想先进入 Launcher 热点子域，而不是直接面对 80+ 份启动器文档：先读 `doc/world-simulator/launcher/README.md`
- 想先进入场景初始化、seed/location 或 asteroid-fragment 主题，而不是直接逐篇查找场景专题：先读 `doc/world-simulator/scenario/README.md`
- 想直接执行 Viewer / Web 闭环 / 操作步骤：先读 `doc/world-simulator/viewer/viewer-manual.manual.md`
- 想继续按文件名或子域精确下钻：直接使用下方完整清单

## 活跃补充文档
- `doc/world-simulator/viewer/README.md`：`viewer/` 热点子域 landing page，适合先做簇级分流，再决定进入 `manual`、`viewer` canonical / `software_safe` compat alias 或 runtime live 专题。
- `doc/world-simulator/viewer/viewer-visual-design-spec-2026-06-05.design.md`：Viewer / player-facing surface 的 canonical 视觉设计规范，覆盖视觉方向、层级、pixel-world 可读性、响应式/可访问性与视觉 review gate。
- `doc/world-simulator/viewer/viewer-brand-system-2026-06-05.design.md`：Viewer 视觉规范 companion，覆盖 brand book、语义 token、icon/status vocabulary、资产语言、稳定 DOM hook 与扩展截图矩阵。
- `doc/world-simulator/viewer/viewer-frontend-structure-standard-2026-07-06.prd.md`：Viewer 前端 `js/html/jsx` 结构治理标准，覆盖 source/generated/compat taxonomy、分层模型、拆分触发条件、组件/模块抽象边界与验证矩阵。
- `doc/world-simulator/viewer/viewer-manual.manual.md`：Viewer / Web 闭环 / operator 手册，不在下方 PRD 三件套长表中展开。
- `doc/world-simulator/viewer/viewer-pixel-world-player-leverage-production-readability-2026-05-28.brainstorm.md`：pixel-world 商业化下一轮 bounded brainstorming，聚焦玩家因果、行动反馈、生产可读性与后续 runtime/viewer 协议候选。
- `doc/world-simulator/launcher/README.md`：`launcher/` 热点子域 landing page，适合先做簇级分流，再决定进入 release/distribution、control plane、explorer、runtime 边界或 self-guided 专题。
- `doc/world-simulator/launcher/game-client-launcher-cross-surface-action-parity.prd.md`：native/Web Launcher 配置、设置、反馈与转账受控动作的当前专业入口；共享真实的 executable/blocked/result/recovery 语义，保留平台字段和适配差异，并且不将 submit acceptance、近期历史或控制面可达外推为 settlement、持久性、完全 parity、可玩性或发行结论。
- `doc/world-simulator/launcher/game-client-launcher-feedback.prd.md`：launcher feedback 当前 authority，收敛 native Ready 后远端提交失败时的本地回落与 Web 控制面代理边界；三个 2026-03 源三件套已删除，追溯仅使用 Git 与 `.pm` task evidence。
- `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.prd.md`：launcher control plane、共享 UI schema 与 GUI-agent operator HTTP-JSON 接口当前 authority；capabilities 响应是 action-list truth，hosted 的枚举 operator 路由 peer-IP gate 不构成 general auth/readiness 结论。
- `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer.prd.md`：launcher explorer 当前 authority，收敛既有概览、七个只读业务视图与状态呈现；五组历史源三件套已退役删除，历史追溯仅使用 Git 与 GitHub task issue evidence，且不得将历史 mainnet-grade/public-chain 命名外推为 readiness、公开服务、结算或 validator 承诺。
- `doc/world-simulator/scenario/README.md`：`scenario/` 子域 landing page，适合先按场景文件、world initialization、seed/location、资源生成或 asteroid-fragment 问题分流，再进入精确专题。
- launcher transfer 当前入口：`doc/world-simulator/launcher/game-client-launcher-cross-surface-action-parity.prd.md` 承接早期 `PRD-WORLD_SIMULATOR-004/005` 的链上转账基础条款；旧 `world-simulator` launcher blockchain-transfer singleton 分册与四组 2026-03 源三件套均已退役删除，不再作为 active supporting doc。
- `doc/world-simulator/llm/llm-provider-agent-direct-connect-review-2026-04-06.md`：`provider agent direct connect` 的正式 review，适合在判断双模式产品完整性、实施差距和后续 remediation 时定向进入。
- `doc/world-simulator/llm/provider-agent-dual-mode-contract.md`：`Local Provider` 双轨模式的 observation / action contract supporting spec。
- `doc/world-simulator/llm/provider-agent-profile-oasis7_p0_low_freq_npc-2026-03-13.md`：`Local Provider` `P0` 默认 profile supporting spec，用于解释 provider-side 行为约束与 parity 口径。

## 覆盖规则（ROUND-005 统一）
- 纳入规则：纳入 `doc/world-simulator/**` 下所有 `*.prd.md` 与同名 `*.project.md`。
- 活跃补充：`*.manual.md` 与仍被当前模块 PRD / 项目态直接引用的 supporting spec，可在“活跃补充文档”区定向列出，但不并入下方三件套长表。
- 排除规则：不纳入 `doc/devlog/**` 与非 PRD 配对文档（如临时草稿/日志快照）。
- 按需进入：复签结论、状态收口、evidence、report、template 等审计留痕保留可检索性；除非它们重新成为当前 operator 或 owner 的直接入口，否则不进入默认首屏。
- 历史入口：root world-simulator PRD/project legacy redirect shells 已删除，不作为主索引分母。
- 当前入口：本目录 `prd.md` / `project.md` 是 world-simulator PRD 与执行状态主入口。

## 历史证据入口

以下文档只作为 archive-only evidence，用于审计追溯，不作为 world-simulator 日常阅读入口：

- historical world-simulator PRD review checklist snapshot（后续已删除；当前 world-simulator truth 见 `doc/world-simulator/README.md`、`doc/world-simulator/prd.index.md`、`doc/world-simulator/project.md` 与 `doc/world-simulator/prd.md`）
- Git history
- Git history
- Git history
- Viewer 旧控制反馈三件套与二次历史归档说明均已删除；当前正式控制反馈的产品结果读 `doc/product/world-rules-core-gameplay/first-session-and-continuation.prd.md`，Viewer/API 合同读 `doc/world-simulator/prd.md`，间接控制玩法规则读 `doc/game/gameplay/gameplay-indirect-control-agency-contract.prd.md`；原始 retired slug 只从 Git history / GitHub task issue evidence 追溯。
- Viewer Chat prompt-presets、profile editing、scroll 与 Agent Prompt default-value prefill 四组三件套已迁入 `doc/product/agents-world-simulation/agent-conversation-and-prompt-control.{prd,design,project}.md` 并物理删除；当前 surface、AgentChat、PromptControl、profile/override、鉴权与验证仍以 `doc/world-simulator/prd.md`、Viewer 手册和专业实现证据为准，历史 EGUI 布局不得外推为当前能力。
- Viewer EGUI 控制区高级调试折叠三件套已退役删除；该 2026-02 已完成专题的历史审计证据从 Git history 与 Git history 追溯，当前 Viewer 操作与 Web 闭环入口改读 `doc/world-simulator/viewer/viewer-manual.manual.md` 与 `doc/world-simulator/viewer/README.md`。
- Launcher native legacy cleanup 三件套已退役删除；该 2026-03 已完成专题只作为 `oasis7_client_launcher` native cleanup 历史证据保留在 git history 与 GitHub task issue evidence comments，不能被外推为当前 web launcher/control-plane 字段退役结论。
- 已删除的 intent/distributed/runtime closure 与 M4 market/hardware/data/governance closure 由 Git history 与 GitHub task evidence 追溯；当前阅读入口继续走下方活跃专题、`doc/world-simulator/project.md` 与长期 world-simulator、P2P、runtime、gameplay 权威。
- kernel rule-hook、Wasm readiness/executor/sandbox bridge/module-governance、Rust-to-Wasm build-suite、simulator `PowerStorage`、内建 `Compound/Hardware` hard-migration 与 runtime required 十项临时 ignore 九组 completed 三件套已退役删除。当前 simulator adapter / resource removal boundary 读 `doc/world-simulator/design.md` 与 `m4/industrial-resource-flow-contract.prd.md`；ABI、executor、artifact lifecycle 与 canonical build 读 `doc/world-runtime/wasm/` 和 `doc/world-runtime/prd.md`，历史实施只从 Git history / GitHub task issue evidence 追溯。

若需要判断当前需求、执行状态或专题配对关系，仍以 `doc/world-simulator/prd.md`、`doc/world-simulator/project.md` 与本索引的活跃专题清单为准。

删除候选边界：仍保留的 completed closure 三件套若 focused `rg` 证明只剩自引用、历史证据入口和可替代的 project provenance，可在独立治理切片中删除文件并保留 Git history / GitHub task evidence 追溯。

## 完整活跃专题清单（按文件名精确检索）

| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.prd.md` | `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.design.md` | `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-feedback.prd.md` | `doc/world-simulator/launcher/game-client-launcher-feedback.design.md` | `doc/world-simulator/launcher/game-client-launcher-feedback.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer.prd.md` | `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer.design.md` | `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.prd.md` | `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.design.md` | `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.prd.md` | `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.design.md` | `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-cross-surface-action-parity.prd.md` | `doc/world-simulator/launcher/game-client-launcher-cross-surface-action-parity.design.md` | `doc/world-simulator/launcher/game-client-launcher-cross-surface-action-parity.project.md` |
| `doc/world-simulator/llm/llm-agent-behavior.prd.md` | `doc/world-simulator/llm/llm-agent-behavior.design.md` | `doc/world-simulator/llm/llm-agent-behavior.project.md` |
| `doc/world-simulator/llm/decision-provider-contract.prd.md` | `doc/world-simulator/llm/decision-provider-contract.design.md` | `doc/world-simulator/llm/decision-provider-contract.project.md` |
| `doc/world-simulator/llm/provider-loopback-http-contract.prd.md` | `doc/world-simulator/llm/provider-loopback-http-contract.design.md` | `doc/world-simulator/llm/provider-loopback-http-contract.project.md` |
| `doc/world-simulator/llm/provider-agent-experience-parity.prd.md` | `doc/world-simulator/llm/provider-agent-experience-parity.design.md` | `doc/world-simulator/llm/provider-agent-experience-parity.project.md` |
| `doc/world-simulator/llm/provider-agent-dual-mode.prd.md` | `—` | `doc/world-simulator/llm/provider-agent-dual-mode.project.md` |
| `doc/world-simulator/llm/llm-factory-strategy-optimization.prd.md` | `doc/world-simulator/llm/llm-factory-strategy-optimization.design.md` | `doc/world-simulator/llm/llm-factory-strategy-optimization.project.md` |
| `doc/world-simulator/llm/llm-lmso29-stability.prd.md` | `doc/world-simulator/llm/llm-lmso29-stability.design.md` | `doc/world-simulator/llm/llm-lmso29-stability.project.md` |
| `doc/world-simulator/llm/llm-prompt-effect-receipt.prd.md` | `doc/world-simulator/llm/llm-prompt-effect-receipt.design.md` | `doc/world-simulator/llm/llm-prompt-effect-receipt.project.md` |
| `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md` | `doc/world-simulator/m4/industrial-resource-flow-contract.design.md` | `doc/world-simulator/m4/industrial-resource-flow-contract.project.md` |
| `doc/world-simulator/scenario/agent-frag-initial-spawn-position.prd.md` | `doc/world-simulator/scenario/agent-frag-initial-spawn-position.design.md` | `doc/world-simulator/scenario/agent-frag-initial-spawn-position.project.md` |
| `doc/world-simulator/scenario/asteroid-fragment-renaming.prd.md` | `doc/world-simulator/scenario/asteroid-fragment-renaming.design.md` | `doc/world-simulator/scenario/asteroid-fragment-renaming.project.md` |
| `doc/world-simulator/scenario/chunked-fragment-generation.prd.md` | `doc/world-simulator/scenario/chunked-fragment-generation.design.md` | `doc/world-simulator/scenario/chunked-fragment-generation.project.md` |
| `doc/world-simulator/scenario/fragment-spacing.prd.md` | `doc/world-simulator/scenario/fragment-spacing.design.md` | `doc/world-simulator/scenario/fragment-spacing.project.md` |
| `doc/world-simulator/scenario/scenario-asteroid-fragment-overrides.prd.md` | `doc/world-simulator/scenario/scenario-asteroid-fragment-overrides.design.md` | `doc/world-simulator/scenario/scenario-asteroid-fragment-overrides.project.md` |
| `doc/world-simulator/scenario/scenario-files.prd.md` | `doc/world-simulator/scenario/scenario-files.design.md` | `doc/world-simulator/scenario/scenario-files.project.md` |
| `doc/world-simulator/scenario/scenario-power-facility-baseline.prd.md` | `doc/world-simulator/scenario/scenario-power-facility-baseline.design.md` | `doc/world-simulator/scenario/scenario-power-facility-baseline.project.md` |
| `doc/world-simulator/scenario/scenario-seed-locations.prd.md` | `doc/world-simulator/scenario/scenario-seed-locations.design.md` | `doc/world-simulator/scenario/scenario-seed-locations.project.md` |
| `doc/world-simulator/scenario/unified-world-seed-fragment-runtime.prd.md` | `doc/world-simulator/scenario/unified-world-seed-fragment-runtime.design.md` | `doc/world-simulator/scenario/unified-world-seed-fragment-runtime.project.md` |
| `doc/world-simulator/scenario/world-initialization.prd.md` | `doc/world-simulator/scenario/world-initialization.design.md` | `doc/world-simulator/scenario/world-initialization.project.md` |
| `doc/world-simulator/viewer/viewer-minimal-system.prd.md` | `doc/world-simulator/viewer/viewer-minimal-system.design.md` | `doc/world-simulator/viewer/viewer-minimal-system.project.md` |
| `doc/world-simulator/viewer/viewer-module-visual-entities.prd.md` | `doc/world-simulator/viewer/viewer-module-visual-entities.design.md` | `doc/world-simulator/viewer/viewer-module-visual-entities.project.md` |
| `doc/world-simulator/viewer/viewer-chat-ime-egui-bridge.prd.md` | `doc/world-simulator/viewer/viewer-chat-ime-egui-bridge.design.md` | `doc/world-simulator/viewer/viewer-chat-ime-egui-bridge.project.md` |
| `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.prd.md` | `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.design.md` | `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.project.md` |
| `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.prd.md` | `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.design.md` | `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.project.md` |
| `doc/world-simulator/viewer/viewer-egui-right-panel.prd.md` | `doc/world-simulator/viewer/viewer-egui-right-panel.design.md` | `doc/world-simulator/viewer/viewer-egui-right-panel.project.md` |
| `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md` | `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.design.md` | `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.project.md` |
| `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.prd.md` | `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.design.md` | `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.project.md` |
| `doc/world-simulator/viewer/viewer-pixel-world-bridge-render-optimization-2026-05-17.prd.md` | `doc/world-simulator/viewer/viewer-pixel-world-bridge-render-optimization-2026-05-17.design.md` | `doc/world-simulator/viewer/viewer-pixel-world-bridge-render-optimization-2026-05-17.project.md` |
| `doc/world-simulator/viewer/viewer-pixel-world-commercial-rendering-loop-2026-05-28.prd.md` | `doc/world-simulator/viewer/viewer-pixel-world-commercial-rendering-loop-2026-05-28.design.md` | historical trace: `task_b399bf37eff94c44a300c55f5db739d3` / GitHub issue #1294 / `.pm/github-project-sync/task-archive.jsonl` |
| `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod-2026-05-27.prd.md` | `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod-2026-05-27.design.md` | historical trace: `task_428db5366f654c5e892ac300807cb9cc` / GitHub issue #986 / `.pm/github-project-sync/task-archive.jsonl` |
| `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.prd.md` | `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.design.md` | historical trace: `task_4ade083740bc4d9f9f9bb742a7ce153f` / GitHub issue #1011 / `.pm/github-project-sync/task-archive.jsonl` |
| `doc/world-simulator/viewer/viewer-frontend-structure-standard-2026-07-06.prd.md` | `doc/world-simulator/viewer/viewer-frontend-structure-standard-2026-07-06.design.md` | `doc/world-simulator/viewer/viewer-frontend-structure-standard-2026-07-06.project.md` |
| `doc/world-simulator/viewer/viewer-web-single-source-build-truth-2026-05-19.prd.md` | `doc/world-simulator/viewer/viewer-web-single-source-build-truth-2026-05-19.design.md` | `doc/world-simulator/viewer/viewer-web-single-source-build-truth-2026-05-19.project.md` |
| `doc/world-simulator/viewer/viewer-web-semantic-test-api.prd.md` | `doc/world-simulator/viewer/viewer-web-semantic-test-api.design.md` | `doc/world-simulator/viewer/viewer-web-semantic-test-api.project.md` |
| `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md` | `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.design.md` | `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.project.md` |
| `doc/world-simulator/viewer/viewer-websocket-http-bridge.prd.md` | `doc/world-simulator/viewer/viewer-websocket-http-bridge.design.md` | `doc/world-simulator/viewer/viewer-websocket-http-bridge.project.md` |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- 默认入口面先在 `README.md` / `prd.index.md` 收紧；若热点子域进入后仍无首读入口，则继续追加路径级治理。当前 `viewer/README.md` 即为 `viewer/` 的首个已执行子域入口。
- ROUND-002 物理合并（gameplay release）：`viewer-gameplay-release-experience-overhaul` 为主文档，`immersion-phase2~10` 均已收敛到该主文档、审计日志、git history 与 GitHub task issue evidence comments；旧阶段三件套已从仓库移除（不再保留 archive）。
- 专业权威合并（runtime-to-Web）：event-driven phase8/9/10、runtime-world LLM bridge、Viewer/node decouple、两轮 build pruning 与 runtime-fatal 的当前语义已分别归入 control-plane、world-simulator、DecisionProvider、Launcher session continuity、single-source build truth、semantic test API 与 Viewer manual；18 个历史源文件已删除，日期化完成态与 byte-size 样本仅从 Git/GitHub task evidence 追溯。

## 补充验收模板
- `doc/world-simulator/prd/acceptance/provider-agent-parity-scenario-matrix-2026-03-12.md`
- `doc/world-simulator/prd/acceptance/provider-agent-parity-score-card-2026-03-12.md`
- `doc/world-simulator/prd/acceptance/provider-agent-parity-benchmark-protocol-2026-03-12.md`
- `doc/world-simulator/prd/acceptance/provider-agent-parity-aggregation-template-2026-03-12.md`
