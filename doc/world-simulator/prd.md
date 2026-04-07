# world-simulator PRD

审计轮次: 7

## 目标
- 建立 world-simulator 模块设计主文档，统一需求边界、技术方案与验收标准。
- 确保 world-simulator 模块后续改动可追溯到 PRD-ID、任务和测试。
- 为专题能力提供分册挂载机制，保持主 PRD 可导航、可审计。

## 范围
- 覆盖 world-simulator 模块当前能力设计、接口边界、测试口径与演进路线。
- 覆盖 PRD-ID 到 `doc/world-simulator/project.md` 的任务映射。
- 覆盖启动器链路中的链上转账能力（通过分册维护详细条款）。
- 不覆盖实现代码逐行说明与历史过程记录。

## 接口 / 数据
- PRD 主入口: `doc/world-simulator/prd.md`
- 项目管理入口: `doc/world-simulator/project.md`
- 根级兼容执行入口: `doc/world-simulator.project.md`
- 文件级索引: `doc/world-simulator/prd.index.md`
- 追踪主键: `PRD-WORLD_SIMULATOR-xxx`
- 测试与发布参考: `testing-manual.md`
- 跨模块模式 taxonomy: `doc/core/player-access-mode-contract-2026-03-19.prd.md`
- UI 视觉评审列表: `doc/ui_review_result/ui_review_list.md`
- 分册索引:
  - `doc/world-simulator/prd/acceptance/unified-checklist.md`（PRD-WORLD_SIMULATOR-001/002）
  - `doc/world-simulator/prd/acceptance/web-llm-evidence-template.md`（PRD-WORLD_SIMULATOR-002/003）
  - `doc/world-simulator/prd/acceptance/visual-review-score-card.md`（PRD-WORLD_SIMULATOR-003）
  - `doc/world-simulator/prd/quality/experience-trend-tracking.md`（PRD-WORLD_SIMULATOR-003）
  - `doc/world-simulator/prd/launcher/blockchain-transfer.md`（PRD-WORLD_SIMULATOR-004/005）
  - `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04.prd.md`（PRD-WORLD_SIMULATOR-010）
  - `doc/world-simulator/launcher/game-client-launcher-ui-schema-share-2026-03-04.prd.md`（PRD-WORLD_SIMULATOR-011）
  - `doc/world-simulator/launcher/game-client-launcher-egui-web-unification-2026-03-04.prd.md`（PRD-WORLD_SIMULATOR-012）
  - `doc/world-simulator/launcher/game-client-launcher-web-wasm-time-compat-2026-03-04.prd.md`（PRD-WORLD_SIMULATOR-013）
  - `doc/world-simulator/launcher/game-client-launcher-web-required-config-gating-2026-03-04.prd.md`（PRD-WORLD_SIMULATOR-014）
  - `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.prd.md`（PRD-WORLD_SIMULATOR-015）
  - `doc/world-simulator/launcher/game-client-launcher-web-transfer-closure-2026-03-06.prd.md`（PRD-WORLD_SIMULATOR-020）
  - `doc/world-simulator/launcher/game-client-launcher-web-settings-feedback-parity-2026-03-06.prd.md`（PRD-WORLD_SIMULATOR-021）
  - `doc/world-simulator/launcher/game-client-launcher-native-legacy-cleanup-2026-03-06.prd.md`（PRD-WORLD_SIMULATOR-022）
  - `doc/world-simulator/launcher/game-client-launcher-transfer-product-grade-parity-2026-03-06.prd.md`（PRD-WORLD_SIMULATOR-023）
  - `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-panel-2026-03-07.prd.md`（PRD-WORLD_SIMULATOR-024）
  - `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p0-2026-03-07.prd.md`（PRD-WORLD_SIMULATOR-025）
  - `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p1-address-contract-assets-mempool-2026-03-08.prd.md`（PRD-WORLD_SIMULATOR-026）
  - `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08.prd.md`（PRD-WORLD_SIMULATOR-027）
  - `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-ui-ux-optimization-2026-03-08.prd.md`（PRD-WORLD_SIMULATOR-028）
  - `doc/world-simulator/launcher/game-client-launcher-full-usability-remediation-2026-03-08.prd.md`（PRD-WORLD_SIMULATOR-029）
  - `doc/world-simulator/launcher/game-client-launcher-self-guided-experience-2026-03-08.prd.md`（PRD-WORLD_SIMULATOR-030）
  - `doc/world-simulator/launcher/game-client-launcher-web-console-gui-agent-interface-2026-03-08.prd.md`（PRD-WORLD_SIMULATOR-031）
  - `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.prd.md`（PRD-WORLD_SIMULATOR-032）
  - `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.prd.md`（PRD-WORLD_SIMULATOR-033）
  - `doc/world-simulator/launcher/game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12.prd.md`（PRD-WORLD_SIMULATOR-034）
  - `doc/world-simulator/viewer/viewer-web-runtime-fatal-surfacing-2026-03-12.prd.md`（PRD-WORLD_SIMULATOR-035）
  - `doc/world-simulator/llm/llm-decision-provider-standard-openclaw-feasibility-2026-03-12.prd.md`（PRD-WORLD_SIMULATOR-036）
  - `doc/world-simulator/llm/llm-openclaw-local-http-provider-integration-2026-03-12.prd.md`（PRD-WORLD_SIMULATOR-037）
  - `doc/world-simulator/llm/llm-openclaw-agent-experience-parity-2026-03-12.prd.md`（PRD-WORLD_SIMULATOR-038）
  - `doc/world-simulator/llm/llm-openclaw-agent-dual-mode-2026-03-16.prd.md`（PRD-WORLD_SIMULATOR-040）
  - `doc/world-simulator/llm/openclaw-agent-dual-mode-contract-2026-03-16.md`（PRD-WORLD_SIMULATOR-040 supporting spec）
  - `doc/world-simulator/llm/llm-openclaw-agent-direct-connect-review-2026-04-06.md`（PRD-WORLD_SIMULATOR-037/040 formal review）
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`（PRD-WORLD_SIMULATOR-039）
  - `doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.prd.md`（PRD-WORLD_SIMULATOR-041）
  - `doc/world-simulator/prd/acceptance/openclaw-agent-parity-scenario-matrix-2026-03-12.md`（PRD-WORLD_SIMULATOR-038）
  - `doc/world-simulator/prd/acceptance/openclaw-agent-parity-score-card-2026-03-12.md`（PRD-WORLD_SIMULATOR-038）
  - `doc/world-simulator/prd/acceptance/openclaw-agent-parity-benchmark-protocol-2026-03-12.md`（PRD-WORLD_SIMULATOR-038）
  - `doc/world-simulator/prd/acceptance/openclaw-agent-parity-aggregation-template-2026-03-12.md`（PRD-WORLD_SIMULATOR-038）
  - `doc/world-simulator/llm/openclaw-agent-profile-oasis7_p0_low_freq_npc-2026-03-13.md`（PRD-WORLD_SIMULATOR-037/038 supporting spec）
  - `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04.prd.md`（PRD-WORLD_SIMULATOR-016）
  - `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.prd.md`（PRD-WORLD_SIMULATOR-017）
  - `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.prd.md`（PRD-WORLD_SIMULATOR-018）
  - `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.prd.md`（PRD-WORLD_SIMULATOR-019）
  - `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.prd.md`（PRD-WORLD_SIMULATOR-001/002/003）

## 里程碑
- M1 (2026-03-03): 完成模块设计 PRD 主体重写与任务改造。
- M2 (2026-03-03): 完成启动器链上转账需求建模与任务拆解。
- M3: 完成 PRD 分册结构落地，主入口仅保留总览与导航。
- M4: 完成链运行时转账 API、运行时账本动作、启动器交互与闭环验收。
- M5 (2026-03-04): 完成无 GUI 服务器场景的启动器 Web 控制台能力建模与落地。
- M6 (2026-03-04): 完成启动器 native 客户端服务端分离，native/web 统一控制面链路与功能对齐。
- M7 (2026-03-06): 完成启动器 Web 端链上转账闭环补齐（控制面代理 + wasm 提交）。
- M8 (2026-03-06): 完成启动器 Web 端设置/反馈入口对齐（设置中心可用化 + 反馈代理提交）。
- M9 (2026-03-06): 完成启动器 native 遗留代码清理与测试资产收敛（移除失效状态字段与未引用旧测试文件）。
- M10 (2026-03-06): 完成启动器转账产品级需求建模（自动 nonce、账户/余额辅助、历史/最终状态可视化、native/web 同层前端一致性）。
- M11 (2026-03-07): 完成启动器转账产品级能力实现（runtime 查询 API + 控制面代理 + native/web 共享转账前端 + required/full 回归）。
- M12 (2026-03-07): 完成启动器区块链浏览器能力（explorer RPC + 控制面代理 + native/web 面板）。
- M13 (2026-03-07): 完成启动器区块链浏览器公共主链视角 P0 补齐（runtime + 控制面 + 启动器跨端 UI + 回归）。
- M14 (2026-03-08): 推进启动器区块链浏览器公共主链视角 P1（地址/合约/资产/内存池）补齐。
- M15 (2026-03-08): 完成启动器可用性与体验硬化（路径回退、禁用态提示、参数编码、状态语义、移动端可读性、favicon 噪声治理）。
- M16 (2026-03-08): 完成启动器区块链浏览器视觉与交互优化（概览分组、状态徽标、筛选恢复、列表-详情协同）。
- M17 (2026-03-08): 完成启动器全量可用性残余风险收口（配置防回写、按域并发、反馈草稿保护、顶栏响应式、转账过滤重置）。
- M18 (2026-03-08): 完成启动器自引导体验闭环建模与任务拆解（首次引导、任务流、术语解释、演示模式、配置画像、本地埋点）。
- M19 (2026-03-08): 完成 Web Console GUI Agent 全量接口（能力声明 + 统一动作执行）建模与落地。
- M20 (2026-03-09): 完成 runtime required 已知失败用例临时下线（精准白名单 `#[ignore]`）并恢复 required 测试链路可执行。
- M21 (2026-03-09): 完成 launcher 显式 execution world 输出路径收敛，确保 explorer 索引落在 `output/chain-runtime/<node_id>/reward-runtime-execution-world`。
- M22 (2026-03-12): 完成 `Decision Provider` 标准层与 `OpenClaw` 外部适配可行性建模，冻结“provider advisory / runtime authoritative”边界。
- M23 (2026-03-12): 完成 `OpenClaw(Local HTTP)` 用户机接入方案建模，明确本地发现、握手、配置、决策与回退路径。
- M24 (2026-03-12): 完成 `OpenClaw` 与内置 agent 体验等价（parity）验收方案建模，把 parity 升级为上线门禁。
- M25 (2026-04-01): 完成 Viewer 3D 可视化暂停治理，冻结当前用户交互分支为暂存态，并将当前交互主路径收口到非 3D / `software_safe` 优先。

## 风险
- 模块边界演进快，文档同步可能滞后。
- 指标口径不稳定会降低验收一致性。
- 分册与主入口不同步会导致需求追踪断裂。

## 1. Executive Summary
- Problem Statement: world-simulator 涵盖场景、viewer、LLM 与 launcher 多条链路，需求持续增长时若全部堆叠在单文档，会降低可维护性并影响变更追踪效率。
- Proposed Solution: 主 PRD 保持模块级边界与验收总口径，专题能力迁移到分册；启动器链上转账作为首个分册能力，维护完整细节。
- Success Criteria:
  - SC-1: simulator/viewer/launcher 变更全部映射 PRD-WORLD_SIMULATOR-ID。
  - SC-2: Web 闭环路径作为默认链路并保持可复现测试证据。
  - SC-3: 场景系统关键基线（初始化、资源、交互）具备稳定回归。
  - SC-4: LLM 交互链路具备可观察性与降级策略记录。
  - SC-5: 启动器链上转账需求在分册中完整定义，并与主 PRD / 项目任务保持一一映射。
  - SC-6: 分册变更后主 PRD 仍可作为唯一入口完成导航与验收追踪。
  - SC-7: 场景系统、Viewer、启动器验收口径统一到同一 checklist，并可直接映射到测试证据。
  - SC-8: Web-first 闭环与 LLM 链路具备统一证据模板，支持发布前快速复核。
  - SC-9: simulator 体验质量指标形成趋势跟踪，支持按周判定退化风险。
  - SC-10: 启动器支持“游戏进程 / 区块链进程”独立编排，且反馈入口严格受链就绪状态门控。
  - SC-11: 启动器“设置”入口升级为完整设置中心，覆盖游戏/区块链/LLM 配置并提供统一可见性。
  - SC-12: Viewer 在 Linux native + Web 双链路下打开 2D/3D 视角时不得出现粉紫屏，且关键交互必须可操作。
  - SC-13: 启动器发行包在可执行文件仍被运行时，重复打包不得出现 `Text file busy` 且不得产生“半更新”产物。
  - SC-14: 启动器支持无 GUI 服务器 Web 控制台，允许远程启动/停止并查看状态日志。
  - SC-15: 启动器 native 与 web 表单字段由同一份 UI schema 驱动，避免配置项漂移。
  - SC-16: 启动器 web 端改为复用同一套 egui UI 代码并以静态资源方式托管，消除独立 HTML 控制台分叉。
  - SC-17: 启动器 wasm 页面初始化不得触发 `time not implemented` 崩溃，`agent-browser --headed` 闭环必须可稳定采证。
  - SC-18: 启动器 Web 端不得要求 native-only 二进制路径必填，且 native 端仍保持对应必填校验。
  - SC-19: 启动器 native 与 web 必须通过同一控制面 API 编排游戏/区块链进程，并保持状态与按钮行为一致。
  - SC-20: `oasis7_viewer_live` 必须支持 runtime/world 驱动模式，并在不改 viewer 协议前提下输出可消费的 live 快照与事件。
  - SC-21: runtime 模式必须支持 `LLM/chat/prompt` 控制链路（含鉴权与错误语义），避免与 simulator 模式形成双套体验断裂。
  - SC-22: runtime live 必须补齐高频动作映射与等价回归，并移除 `oasis7_viewer_live` simulator 启动分支，统一 runtime-only 体验。
  - SC-23: runtime live 必须使用真实 LLM 决策链路（AgentRunner/LlmAgentBehavior），且 LLM 失败时硬失败，不得回退启发式。
  - SC-24: runtime live 必须达到 runtime 事件/快照 100% 覆盖，并允许通过协议扩展输出 DecisionTrace。
  - SC-25: 启动器 Web 端必须支持链上转账提交并保持与 native 一致的成功/拒绝/失败语义。
  - SC-26: 启动器 Web 端必须支持设置中心与反馈入口可用，并保持与 native 一致的操作语义（不再为禁用占位）。
  - SC-27: 启动器 native 端已迁移后的历史遗留状态字段/测试资产必须收敛，避免继续引入无效维护成本与告警噪声。
  - SC-28: 启动器转账能力必须升级为产品级体验，并在 native/web 端共用同层前端实现（门控、表单、状态机、文案一致）。
  - SC-29: 启动器必须提供区块链浏览器面板，并支持链总览、交易过滤与交易详情查询的跨端一致体验。
  - SC-30: 启动器区块链浏览器必须补齐公共主链视角 P0 能力（区块分页、`tx_hash` 详情、统一搜索、持久化索引）并保持 native/web 行为一致。
  - SC-31: 启动器区块链浏览器必须补齐公共主链视角 P1 能力（地址页、合约页、Token/NFT 资产页、mempool）并保持 native/web 行为一致。
  - SC-32: 启动器控制面与客户端必须具备可诊断且跨端一致的可用性基线（静态目录回退、禁用态提示、参数编码、stop no-op 语义、移动端可读性、默认静态资源噪声抑制）。
  - SC-33: 启动器在“启动游戏/启动区块链”遇到阻断配置时必须弹出可编辑配置引导窗口，并在首次进入时执行一次轻量自动引导。
  - SC-34: 启动器区块链浏览器必须具备可快速判读的视觉层级与低摩擦交互（概览分组、状态可视化、筛选恢复、列表-详情协同），并保持 native/web 一致。
  - SC-35: 启动器必须在高频操作与轮询并发场景保持交互稳定（本地配置不被轮询回写、请求按域并发、反馈草稿不被中断），并补齐窄屏顶栏可读性与转账过滤重置入口。
  - SC-36: 启动器需默认提供自引导体验（首次 3 步引导、任务流卡片、禁用态 CTA、术语解释、演示模式、成功配置画像与本地引导埋点），并允许专家用户无阻切换到高级配置路径。
  - SC-37: 启动器 Web Console 需提供一套面向 GUI Agent 的机器接口（能力发现 + 统一动作执行），覆盖人工可达全部功能并返回稳定结构化结果。
  - SC-38: runtime required 测试链路需支持“已知基线失败项临时下线但资产保留”的精确治理机制，避免 pre-commit 长期阻塞并保持后续恢复可追踪。
  - SC-39: 启动器托管的 `oasis7_chain_runtime` 必须显式传递 `--execution-world-dir` 到 `output/chain-runtime/<node_id>/reward-runtime-execution-world`，避免 `explorer-index.json` 落到源码目录。
  - SC-40: 启动器默认链启动在命中 stale execution world / state root mismatch 时，必须提供结构化恢复语义与至少一条非 CLI 恢复路径，避免用户只能通过底层日志手工换 node id。
  - SC-41: world-simulator 的 Agent 决策层必须支持 provider-agnostic 标准接口，使外部 agent framework（如 `OpenClaw`）可经 adapter 参与模拟，同时保持 runtime 权威、trace 连续性与可离线 required 测试。
  - SC-42: world-simulator 必须提供 `OpenClaw(Local HTTP)` 首期接入路径，使安装在用户机器上的 `OpenClaw` 可通过 `agent_decision_source=provider_backed + agent_provider_backend=openclaw + agent_provider_contract=worldsim_provider_v1 + agent_provider_transport=loopback_http` 驱动低频 agent，并具备发现、绑定、错误提示与安全回退能力；`agent_direct_connect/openclaw_local_http` 只保留为兼容 alias。
  - SC-43: `OpenClaw` provider 在纳入范围的 agent 场景中必须达到与内置 agent 可感知等价的用户体验；若未通过 parity 验收，不得默认启用或扩大覆盖范围。
  - SC-44: 启动器 / Viewer 的浏览器存储键、临时状态文件名、Canvas/字体等公开前端运行时 key 必须优先使用 `oasis7` 前缀；迁移期需兼容读取旧 `oasis7` / `oasis7` / `OASIS7_*` key，避免现有脚本与本地状态立即失效。
  - SC-45: Viewer 的 auth bootstrap object、玩家鉴权 env key、右侧面板持久化路径与诊断类 viewer env key 必须优先迁移到 `oasis7` 前缀；迁移期需同时兼容旧 `oasis7` key，避免 software-safe / native / embedded launcher 三条链路出现断裂。
  - SC-46: `oasis7_game_launcher` / `oasis7_web_launcher` 的运行时路径覆盖 env key（launcher bin、chain runtime bin、viewer/game static dir、web launcher static dir）必须优先迁移到 `OASIS7_*`，同时兼容旧 `OASIS7_*`，避免已有 bundle、shell 脚本与运维环境变量在品牌迁移时失效。
  - SC-47: Viewer 的 3D 配置、theme preset、panel/headless 行为控制与 release profile 脚本中的 `VIEWER_*` env key 必须统一到 `OASIS7_VIEWER_*`，避免渲染调参、无头回归与 operator 预设在品牌迁移后继续依赖旧前缀。
  - SC-48: Viewer operator 脚本中的抓帧、材质巡检与压力测试默认 env 写入必须统一使用 `OASIS7_VIEWER_*`，避免 capture/inspection/stress 自动化继续分叉出旧前缀入口。
  - SC-49: simulator LLM 配置链路的默认 env key 必须统一到 `OASIS7_LLM_*`；`llm_agent`、agent-scoped goal override、runtime live 测试与长稳脚本不得继续保留旧前缀入口。
  - SC-50: runtime live 的 OpenClaw provider / QA chat echo 链路默认 env key 必须优先迁移到 `OASIS7_AGENT_DECISION_SOURCE`、`OASIS7_AGENT_PROVIDER_BACKEND`、`OASIS7_AGENT_PROVIDER_CONTRACT`、`OASIS7_AGENT_PROVIDER_TRANSPORT`、`OASIS7_AGENT_PROVIDER_URL`、`OASIS7_AGENT_PROVIDER_AUTH_TOKEN`、`OASIS7_AGENT_PROVIDER_CONNECT_TIMEOUT_MS`、`OASIS7_AGENT_PROVIDER_PROFILE`、`OASIS7_AGENT_EXECUTION_LANE` 与 `OASIS7_RUNTIME_AGENT_CHAT_ECHO`；`OASIS7_AGENT_PROVIDER_MODE` 与 `OASIS7_OPENCLAW_*` 只保留为兼容 fallback，`oasis7_game_launcher` 注入、runtime_live 读取、software-safe QA 脚本与对应测试不得继续把旧单字段前缀当作唯一入口。
  - SC-51: bundle 产物中的 `run-client.sh` / `run-game.sh` / `run-web-launcher.sh` / `run-chain-runtime.sh` 默认 env 注入与 README 示例必须优先迁移到 `OASIS7_*` / `OASIS7_CHAIN_STORAGE_PROFILE`，同时兼容旧 `OASIS7_*` fallback；bundle-first operator 路径不得继续把旧前缀当作默认口径。
  - SC-52: viewer theme preset `.env` 文件中的默认导出 key 必须统一到 `OASIS7_VIEWER_*`，避免 theme 预设成为 viewer 配置链路里最后一批仍默认写旧前缀的 operator 入口。
  - SC-53: repo-owned OpenClaw lightweight runtime agent 的默认 agent id 与 setup 脚本 operator env key 必须迁移到 `oasis7_openclaw_agent` 与 `OPENCLAW_OASIS7_*`，并移除旧品牌 runtime agent 命名与运行时 fallback；`oasis7-run.sh`、skill 文档、bridge 示例与 setup 脚本不得继续把旧品牌命名当作当前默认或有效 operator 入口。
  - SC-54: `oasis7_game_launcher` / `oasis7_web_launcher` 相关测试与静态资源测试写入系统临时目录时，默认前缀必须使用 `oasis7_*`，避免本地测试产物与失败签名继续泄露旧 `oasis7_*` 品牌字符串。
  - SC-55: 内部 CI 编排脚本中的 helper 函数名与聚合入口必须优先使用 `oasis7_*` 口径，避免 `scripts/ci-tests.sh` 继续通过 `run_oasis7_*` 这类内部标识扩散旧品牌命名。
  - SC-56: `oasis7_chain_runtime` 转账提交相关测试写入系统临时目录时，默认前缀必须使用 `oasis7_*`，避免链运行时转账测试产物继续泄露旧 `oasis7_*` 内部命名。
  - SC-57: OpenClaw 首期 `P0` 默认 gameplay profile id 必须迁移到 `oasis7_p0_low_freq_npc`，并移除旧 `oasis7_p0_low_freq_npc` 兼容别名；launcher、runtime live、parity bench、bridge 与 `oasis7` operator 文档不得继续把旧 profile id 当作当前默认或有效入口。
  - SC-58: `oasis7_distfs` 与 `oasis7_consensus` 的测试临时目录前缀必须默认使用 `oasis7_*`，避免 distfs/consensus 本地回归产物继续泄露旧 `oasis7` / `oasis7` 内部品牌字符串。
  - SC-59: `oasis7` 主包内 runtime / chain-runtime / simulator 的测试临时目录、缺失路径 helper 与 builtin wasm 临时构建目录必须默认使用 `oasis7_*`，避免主包回归产物与本地 `.tmp` 工作目录继续泄露旧 `oasis7` / `oasis7` 品牌字符串。
  - SC-60: `oasis7_openclaw_parity_bench` 的自定义 OpenClaw profile 样例必须优先使用 `oasis7_*` 命名，避免 bench 参数解析测试继续把旧 `oasis7_*` profile id 当作非默认样例真值。
  - SC-61: runtime test module signer seed、governance local finality signer seed 与 chain runtime node consensus signer 派生命名空间必须优先使用 `oasis7_*` 口径，避免确定性测试密钥与 validator signer 派生链路继续内嵌旧 `oasis7` 品牌字符串。
  - SC-62: `oasis7_game_launcher` / `oasis7_web_launcher` 的 viewer 开发态 fallback 路径必须优先探测 `oasis7_viewer/dist`，同时保留 `oasis7_viewer/dist` 兼容回退，避免源码态调试链路在品牌迁移期间继续把旧 viewer 目录名当作当前默认真值。
  - SC-63: `oasis7_client_launcher` 的源码态 viewer static dir 默认探测与 launcher 端配置校验必须优先探测 `oasis7_viewer/dist`，同时保留 `oasis7_viewer/dist` 兼容回退，避免 client launcher 与其他 launcher 在品牌迁移期对 dev fallback 目录名产生分叉。
  - SC-64: `oasis7_viewer` 的 auth/node-config 相关测试临时目录前缀必须默认使用 `oasis7_*`，避免 viewer auth 回归产物继续泄露旧 `oasis7_viewer_*` 内部命名。
  - SC-65: `crates/oasis7/tests` 下的 module store 集成测试临时目录前缀与测试模块 artifact signer seed 必须优先使用 `oasis7_*` 口径，避免 integration test 产物与确定性签名种子继续内嵌旧 `oasis7` 品牌字符串。
  - SC-66: `oasis7_wasm_executor` 与 `oasis7_wasm_store` 的测试/缓存临时目录前缀必须默认使用 `oasis7_*`，避免底层 wasm 编译缓存与模块仓库回归产物继续泄露旧 `oasis7` 内部命名。
  - SC-67: `oasis7_net` 与 `oasis7_node` 的测试临时目录前缀必须默认使用 `oasis7_*`，避免网络同步、observer、replication 与 node hardening 回归产物继续泄露旧 `oasis7` 内部命名。
  - SC-68: 保留旧品牌 fallback literal 的兼容代码路径中，源码级常量/标识符命名必须优先使用 `compat` 语义而不是继续沿用 `LEGACY_*oasis7*` 口径，避免维护层面的模块名/变量名继续把旧品牌当作当前源码真值。
  - SC-69: `oasis7_viewer` 的 auth bootstrap、viewer env alias、automation 与 perf probe 兼容代码路径中，源码级常量/测试命名必须优先使用 `compat` 语义而不是继续沿用旧品牌 viewer 维护口径，避免 Viewer 维护层的模块名/变量名继续扩散旧品牌。
  - SC-70: `oasis7` runtime 的 builtin wasm、module source compile 与 simulator LLM 兼容 env 代码路径中，源码级常量/辅助函数/定向测试命名必须优先使用 `compat` 语义而不是继续沿用 `LEGACY_*OASIS7_*` 口径，避免 runtime 维护层的模块名/变量名继续把旧品牌当作当前源码真值。
  - SC-71: `oasis7_game_launcher` 与 `viewer/runtime_live` 的 launcher path、viewer auth、OpenClaw/runtime chat echo 兼容 env 代码路径中，源码级常量/辅助函数/定向测试命名必须优先使用 `compat` 语义而不是继续沿用 `LEGACY_*OASIS7_*` 口径，避免 launcher/runtime live 维护层的模块名/变量名继续把旧品牌当作当前源码真值。
  - SC-72: `oasis7_web_launcher` 与 `oasis7_client_launcher/src/main.rs` 的 launcher path、console static、字体/语言/control bind 兼容 env 代码路径中，源码级常量/辅助函数/定向测试命名必须优先使用 `compat` 语义而不是继续沿用 `LEGACY_*OASIS7_*` 口径，避免 launcher 维护层的模块名/变量名继续把旧品牌当作当前源码真值。
  - SC-73: `oasis7_web_launcher` 与 `oasis7_client_launcher` 的 launcher 测试文件中，源码级测试函数命名也必须优先使用 `compat` 语义而不是继续沿用 `before_legacy` / `legacy_name` 口径，避免测试层继续扩散旧品牌维护语义。
  - SC-74: runtime `power_bootstrap_release_manifest_full` 全量回归中的 builtin wasm 兼容 env 常量与测试局部变量命名也必须优先使用 `compat` 语义而不是继续沿用 `LEGACY_*OASIS7_*` 口径，避免 full-tier 测试层继续扩散旧品牌维护语义。
  - SC-75: `build-wasm-module` / `ci-m1-wasm-summary` / `sync-m1-builtin-wasm-artifacts` 脚本中的旧品牌 wasm env helper 与局部变量命名也必须优先使用 `compat` 语义而不是继续沿用 `wasm_legacy_env_key` / `legacy_key` 口径，避免脚本维护层继续把旧品牌当作当前源码真值。
  - SC-76: `setup-openclaw-oasis7-runtime`、`viewer-texture-inspector-lib` 与 `capture-viewer-frame` 脚本中的旧品牌 viewer/OpenClaw env helper 与局部变量命名也必须优先使用 `compat` 语义而不是继续沿用 `legacy_key` / `viewer_legacy_env_key` / `promote_legacy_viewer_envs` 口径，避免脚本维护层继续把旧品牌当作当前源码真值。
  - SC-77: Viewer 兼容回退相关的定向测试函数命名也必须优先使用 `compat` 语义而不是继续沿用 `legacy_text_format` / `legacy_dir` / `legacy_key` 口径，避免 Viewer 测试层继续扩散旧品牌维护语义。
  - SC-78: Viewer 文本 fallback helper、预 hello profile/协议 payload/player binding 兼容测试命名也必须优先使用 `compat` 语义而不是继续沿用 `legacy_*` 口径，避免协议兼容层继续扩散旧品牌式维护语义。
  - SC-79: `oasis7` 运行时的 material ledger 迁移 helper、execution bridge 旧 schema 检测 helper 与 module registry fallback 局部变量命名也必须优先使用 `compat` 或更准确的中性语义，而不是继续沿用 `legacy_*` 口径，避免 runtime 源码内部继续把兼容层维护路径表述成当前真值。
  - SC-80: `oasis7_node` commit retention 的 cold index 旧文件名兼容 alias helper、局部变量与定向测试命名也必须优先使用 `compat` 语义，而不是继续沿用 `legacy_*` 口径，避免 node replication 源码继续把旧文件名 alias 当作当前维护真值。
  - SC-81: `oasis7_distfs` 复制记录/guard 旧 JSON 兼容样例的定向测试命名与局部变量也必须优先使用 `compat` 语义，而不是继续沿用 `legacy_*` 口径，避免存储复制测试层继续扩散旧品牌式兼容维护语义。
  - SC-82: `oasis7_wasm_sdk` 的 crate name、workspace member 与 crate 目录名必须直接切到 `oasis7_wasm_sdk`，同时所有 builtin wasm 模块、模板与锁文件里的依赖名/路径/`use` 入口都要同步改名，避免继续把旧 `oasis7_wasm_sdk` 当作真实模块平台入口。
  - SC-83: `oasis7_wasm_abi` 的 crate name、workspace member 与 crate 目录名必须直接切到 `oasis7_wasm_abi`，并同步更新执行器、路由、存储、网络、runtime 与测试中的依赖名、路径与源码入口，避免继续把旧 `oasis7_wasm_abi` 当作真实 ABI 平台入口。
  - SC-84: `oasis7_wasm_router` 的 crate name、workspace member 与 crate 目录名必须直接切到 `oasis7_wasm_router`，并同步更新 runtime/world 侧依赖名、路径与源码入口，避免继续把旧 `oasis7_wasm_router` 当作真实路由平台入口。
  - SC-85: `oasis7_wasm_executor` 与 `oasis7_wasm_store` 的 crate name、workspace member 与 crate 目录名必须直接切到 `oasis7_wasm_executor` / `oasis7_wasm_store`，并同步更新 runtime、tests 与特性依赖声明，避免继续把旧 `oasis7_wasm_*` 当作真实执行/存储平台入口。
  - SC-86: `oasis7_proto` 的 crate name、workspace member 与 crate 目录名必须直接切到 `oasis7_proto`，并同步更新 distfs/net/consensus/node/runtime/viewer/launcher 与脚本中的依赖名、路径和源码入口，避免继续把旧 `oasis7_proto` 当作真实协议平台入口。
  - SC-87: `oasis7_{distfs,consensus,net,node}` 的 crate name、workspace member 与 crate 目录名必须直接切到 `oasis7_{distfs,consensus,net,node}`，并同步更新 runtime、chain runtime、viewer live、脚本与网络栈内部依赖，避免继续把旧 `oasis7_*` 当作真实复制/共识/网络/节点平台入口。
  - SC-88: `oasis7_launcher_ui` 的 crate name、workspace member 与 crate 目录名必须直接切到 `oasis7_launcher_ui`，并同步更新 client launcher / viewer / 脚本侧依赖与路径，避免共享启动器 UI 组件仍以内核旧品牌作为真实包入口。
  - SC-89: `oasis7_client_launcher` 的 crate name、workspace member 与 crate 目录名必须直接切到 `oasis7_client_launcher`，并同步更新 launcher bundle、脚本、源码态静态入口与相关依赖名，避免客户端启动器继续把旧包名当作当前真值。
  - SC-90: `oasis7_viewer` 的 crate name、workspace member 与 crate 目录名必须直接切到 `oasis7_viewer`，并同步更新 launcher fallback、Trunk/static root、theme 资产路径与脚本侧源码入口，避免 viewer 源码树仍把旧目录名当作默认运行根。
  - SC-91: `oasis7` 主 crate 的 crate name、workspace member 与 crate 目录名必须直接切到 `oasis7`，并同步更新 workspace 下游依赖、`cargo -p` 包名、源码 `use oasis7::...` 入口与运行时构建脚本，避免主 runtime 包继续以内核旧品牌作为唯一包身份。
  - SC-92: `oasis7_builtin_wasm_modules` 目录与其下 `oasis7_builtin_wasm_*` crate name 必须直接切到 `oasis7_builtin_wasm_modules` / `oasis7_builtin_wasm_*`，并同步更新 builtin manifest map、模板、模块锁文件与构建脚本，避免内置模块生态继续以内核旧品牌作为真实产物前缀。
  - SC-93: 测试手册、CI 帮助文案与 Viewer HelloAck 默认 `server` 标识必须优先使用 `oasis7` 口径；兼容 payload / env fallback 可继续保留旧 `oasis7` 值，但 repo 当前默认说明与协议默认输出不得继续把旧品牌当作现行真值。
  - SC-94: README、站点首页、Viewer 手册与 `scenario_test_runner` 等活跃入口中的当前命令、crate 路径、Viewer env key 与默认缓存路径必须切到 `oasis7` / `oasis7_*` / `OASIS7_VIEWER_*` / `.oasis7_*`；旧品牌仅允许作为兼容 fallback 被说明，不得继续以默认入口或有效路径示人。
  - SC-95: 活跃工具源码中的旧品牌 env helper / 常量命名也必须收口到 `compat_old_brand` 等中性语义；兼容字面量值可继续保留为 fallback，但内部实现不应继续使用 `LEGACY_WASM_ENV_PREFIX` 这类把旧品牌当作主语义的标识名。
  - SC-96: Viewer 运行时自身不再接受任何旧品牌 viewer alias；`viewer_env`、auth/bootstrap、automation、perf probe 与 right-panel 模块可见性状态路径必须只认 `OASIS7_VIEWER_*` / `.oasis7_viewer`。
  - SC-97: Client launcher 运行时自身不再接受任何旧品牌 launcher alias；字体/语言/control-plane env、launcher/runtime/static dir 覆盖 env、Web console static dir 覆盖 env，以及 UX/LLM 本地状态 key 必须只认 `OASIS7_*` / `oasis7_*` 当前入口。
  - SC-98: `oasis7_game_launcher` / `oasis7_web_launcher` 运行时自身不再接受任何旧品牌 launcher alias；服务端 launcher/runtime bin 覆盖 env、viewer auth bootstrap 注入对象/键名与 console/viewer static dir 覆盖 env 必须只认 `OASIS7_*` 当前入口。
  - SC-99: `viewer/runtime_live` 运行时自身不再接受任何旧品牌 provider/chat echo alias；OpenClaw provider 配置与 QA chat echo 开关必须只认 `OASIS7_*` 当前入口。
  - SC-100: runtime/simulator 自身不再接受任何旧品牌 module source / builtin wasm / LLM alias；模块源码编译、builtin wasm 获取与 LLM 配置必须只认 `OASIS7_*` 当前入口。
  - SC-101: repo-owned wasm build / sync 脚本不再接受任何旧品牌 alias；deterministic wasm build、CI summary 与 builtin wasm manifest sync 的当前 operator 入口必须只认 `OASIS7_WASM_*`。
  - SC-102: repo-owned viewer capture / texture inspector / launcher bundle 脚本不再接受任何旧品牌 viewer/profile alias；Viewer 调试脚本与 bundle wrapper 当前 operator 入口必须只认 `OASIS7_VIEWER_*` / `OASIS7_CHAIN_STORAGE_PROFILE`。
  - SC-103: repo-owned `tools/wasm_build_suite` library 不再接受任何旧品牌 alias；suite 的 manifest build / receipt / metadata 默认入口必须只认 `OASIS7_WASM_*`。
  - SC-104: `oasis7_openclaw_local_bridge` 不再接受旧 profile alias `oasis7_p0_low_freq_npc`；bridge 默认与校验入口必须只认 `oasis7_p0_low_freq_npc` 当前 profile，避免 OpenClaw 本地桥继续保留旧品牌 profile 入口。
  - SC-105: 兼容层源码清理完成后，源码内嵌负向测试中的 helper / fixture 命名也必须收口到 `old_brand_removed` / `removed_old_brand` 等中性语义；旧品牌 env/path/profile 字面量仅允许作为“已移除 alias 的负向输入”存在，不得继续作为 helper 主命名或默认 fixture 身份。
  - SC-106: viewer/client 收口后，runtime/tooling 的源码内嵌负向测试 helper / fixture 命名也必须继续收口到 `removed_old_brand` 等中性语义；旧品牌 env/profile 字面量仅允许作为“已移除 alias 的负向输入”存在，不得继续作为测试 helper 主命名或默认 fixture 身份。
  - SC-107: 活跃角色卡、core 主入口与 world-runtime 主文档必须使用当前 `oasis7*` / `OASIS7_*` 口径描述现行 crate/path/env；旧 `oasis7*` / `OASIS7_*` 仅允许保留在历史任务、归档专题或负向测试输入中，不得继续作为 owner 文档或模块主入口的当前说明。

## 2. User Experience & Functionality
- User Personas:
  - 模拟层开发者：需要统一场景与运行语义。
  - Viewer 体验开发者：需要明确 Web/Native 行为边界与验收标准。
  - 发布与测试人员：需要可执行的闭环测试与证据产物。
  - 启动器玩家：需要在同一入口内完成资产查询与转账，无需命令行。
- User Scenarios & Frequency:
  - 场景开发与调试（模拟层开发者）：日常开发日均多次，变更后即时验证场景初始化、事件推进与资源变更。
  - Viewer Web 闭环验收（Viewer 体验开发者/测试人员）：每个功能分支至少 1 次，发布前按 `test_tier_required` 执行。
  - LLM 链路核验（测试人员）：每周例行 + 发布前专项，聚焦可用性、回退策略与错误签名。
  - 启动器转账操作（启动器玩家）：按需触发，典型为启动后 1~3 次余额查询与转账提交。
  - 发布复核（发布负责人）：每个版本候选至少 1 次，汇总 checklist、证据模板与回归结论。
- User Stories:
  - PRD-WORLD_SIMULATOR-001: As a 模拟层开发者, I want unified world-simulator contracts, so that scenario evolution is stable.
  - PRD-WORLD_SIMULATOR-002: As a Viewer 开发者, I want consistent web-first UX rules, so that user paths remain predictable.
  - PRD-WORLD_SIMULATOR-003: As a 发布人员, I want reproducible simulator closure tests, so that releases are verifiable.
  - PRD-WORLD_SIMULATOR-004: As a 启动器玩家, I want to submit a blockchain transfer in launcher, so that I can move main token balances without external tools.（详见分册）
  - PRD-WORLD_SIMULATOR-005: As a 链路开发者, I want transfer requests to be replay-safe and traceable, so that transfer execution is secure and auditable.（详见分册）
  - PRD-WORLD_SIMULATOR-006: As a 启动器玩家, I want separate controls for blockchain/game startup and feedback gated by chain readiness, so that startup behavior is predictable and feedback availability is explicit.
  - PRD-WORLD_SIMULATOR-007: As a 启动器玩家, I want a complete settings center for game/blockchain/LLM, so that all launch-related configuration can be managed from one place.
  - PRD-WORLD_SIMULATOR-008: As a Viewer 开发者, I want native/web rendering defaults to avoid tonemapping fallback regressions, so that 2D/3D rendering remains stable and operable.
  - PRD-WORLD_SIMULATOR-009: As a 发布工程师, I want launcher bundle rebuild to safely replace running binaries, so that repeated packaging does not fail or leave mixed-version artifacts.
  - PRD-WORLD_SIMULATOR-010: As an 运维人员, I want a web-based launcher control plane, so that headless servers can be operated through network browsers.
  - PRD-WORLD_SIMULATOR-011: As a 启动器开发者, I want shared launcher UI schema across native/web, so that form fields and labels stay consistent.
  - PRD-WORLD_SIMULATOR-012: As a 启动器开发者, I want launcher egui UI to be reused across native/wasm with static asset serving, so that we no longer maintain a separate HTML console.
  - PRD-WORLD_SIMULATOR-013: As a 启动器开发者, I want launcher wasm to use web-compatible time primitives, so that web UI startup does not panic in browser runtime.
  - PRD-WORLD_SIMULATOR-014: As an 运维人员, I want web launcher required checks to ignore native-only binaries, so that browser startup is not blocked by irrelevant fields.
  - PRD-WORLD_SIMULATOR-015: As a 启动器玩家, I want native/web launcher to share the same control-plane API, so that game/blockchain control behavior remains fully aligned across platforms.
  - PRD-WORLD_SIMULATOR-016: As a 玩法架构开发者, I want oasis7_viewer_live to run on runtime/world with protocol compatibility, so that live behavior aligns with runtime semantics without immediate viewer model rewrite.
  - PRD-WORLD_SIMULATOR-017: As a 玩法架构开发者, I want runtime live to support llm/chat/prompt controls, so that runtime 联调不再依赖 simulator 模式兜底。
  - PRD-WORLD_SIMULATOR-018: As a 玩法架构开发者, I want runtime live action mapping coverage with parity regression and runtime-only launch path, so that oasis7_viewer_live no longer keeps a simulator fallback branch.
  - PRD-WORLD_SIMULATOR-019: As a 玩法架构开发者, I want runtime live to use true LLM decisions with 100% runtime event/snapshot coverage, so that runtime 行为与观测不再被启发式上限限制。
  - PRD-WORLD_SIMULATOR-020: As a Web 启动器玩家, I want to submit blockchain transfers in browser, so that I can complete asset interaction without native tools.
  - PRD-WORLD_SIMULATOR-021: As a Web 启动器玩家, I want settings and feedback entries to work in browser, so that I can complete the same control loop as native.
  - PRD-WORLD_SIMULATOR-022: As a 启动器开发者, I want native legacy launcher code to be removed after control-plane unification, so that code ownership and long-term maintenance are cleaner.
  - PRD-WORLD_SIMULATOR-023: As a 启动器玩家, I want a product-grade transfer experience with shared native/web frontend behavior, so that I can complete account selection, nonce handling, and final confirmation in one consistent flow.
  - PRD-WORLD_SIMULATOR-024: As a 启动器玩家, I want a blockchain explorer panel in launcher, so that I can inspect chain overview and transaction details without command-line tools.
  - PRD-WORLD_SIMULATOR-025: As a 启动器玩家, I want block/tx/search pagination in launcher explorer, so that I can inspect chain state like a public-chain explorer.
  - PRD-WORLD_SIMULATOR-026: As a 启动器玩家, I want address/contract/asset/mempool views in launcher explorer, so that I can inspect public-chain states from one panel.
  - PRD-WORLD_SIMULATOR-027: As a 启动器玩家/运维人员, I want robust launcher defaults and explicit web-side diagnostics, so that startup and troubleshooting are reliable in both desktop and mobile usage.
  - PRD-WORLD_SIMULATOR-028: As a 启动器玩家/运维人员, I want a clearer and faster explorer UI, so that I can inspect chain state and locate problematic transactions with fewer interactions.
  - PRD-WORLD_SIMULATOR-029: As a 启动器玩家/运维人员, I want launcher interactions to remain stable under polling and continuous operations, so that edits and high-frequency actions are not interrupted or silently dropped.
  - PRD-WORLD_SIMULATOR-030: As a 启动器新用户, I want launcher to guide me with actionable next steps and safe defaults, so that I can finish first launch without reading external docs.
  - PRD-WORLD_SIMULATOR-031: As a GUI Agent 编排器, I want one machine-oriented API surface in web console, so that I can execute every manual launcher operation without UI-dependent parsing.
  - PRD-WORLD_SIMULATOR-032: As a runtime 维护者, I want known required failing tests to be temporarily offlined with explicit traceability, so that pre-commit can proceed while keeping recovery signals.
  - PRD-WORLD_SIMULATOR-033: As a 启动器维护者, I want launcher to pass an explicit execution world output path to chain runtime, so that runtime-generated explorer index files always stay under `output/`.
  - PRD-WORLD_SIMULATOR-034: As a 启动器用户, I want launcher to detect and recover from stale chain execution-world conflicts, so that I can restart chain-enabled flows without reading raw runtime logs or manually changing node IDs.
  - PRD-WORLD_SIMULATOR-035: As a Web Viewer 调试者/制作人, I want browser-side fatal render failures to surface immediately in `__AW_TEST__` and scripts, so that I can distinguish graphics-environment blockers from gameplay or protocol bugs without guessing.
  - PRD-WORLD_SIMULATOR-036: As an `agent_engineer`, I want a provider-agnostic decision layer between world-simulator and external agent frameworks such as `OpenClaw`, so that I can evaluate or swap external agent runtimes without weakening runtime authority, traceability, or QA contracts.
  - PRD-WORLD_SIMULATOR-037: As a 玩家 / 制作人, I want a provider-backed OpenClaw path on my machine, with the current contract tuple `agent_decision_source=provider_backed + agent_provider_backend=openclaw + agent_provider_contract=worldsim_provider_v1 + agent_provider_transport=loopback_http`, so that I can let locally installed `OpenClaw` drive low-frequency game agents through localhost without deploying remote services.
  - PRD-WORLD_SIMULATOR-038: As a 玩家 / 制作人, I want `OpenClaw`-driven agents to feel equivalent to built-in agents in scoped gameplay scenarios, so that switching provider does not noticeably degrade the game experience.
  - PRD-WORLD_SIMULATOR-039: As a 玩家 / QA / 制作人, I want `software_safe` to be the low-fidelity but formally playable primary Web entry, so that browser gameplay no longer depends on high-fidelity graphics prerequisites while still remaining bounded and auditable.
  - PRD-WORLD_SIMULATOR-040: As a 玩家 / 制作人 / QA, I want OpenClaw agents to support both player-parity and headless execution modes, so that we can separate player-feel validation from GUI-dependent automation and keep gameplay regression runnable without graphics dependencies.
  - PRD-WORLD_SIMULATOR-041: As a 制作人 / viewer_engineer / qa_engineer, I want all 3D visualization work paused and the current user-interaction branch held as a staged reference, so that near-term capacity stays on non-3D interaction scope, especially `software_safe`, launcher/runtime interaction, and gameplay closure, while future resumption remains auditable.
- 模式分层说明：按 `PRD-CORE-009`，`PRD-WORLD_SIMULATOR-039` 对应玩家访问模式 `software_safe`，且现在承接低保真但正式可玩的主要 Web 入口；`standard_3d` 继续保留为 visual QA / screenshot / spatial review 模式，而不是默认浏览器主路径。`PRD-WORLD_SIMULATOR-037` 定义 provider-backed OpenClaw 路径（`agent_decision_source=provider_backed + agent_provider_backend=openclaw + agent_provider_contract=worldsim_provider_v1 + agent_provider_transport=loopback_http`；`agent_direct_connect/openclaw_local_http` 仅作兼容 alias），`PRD-WORLD_SIMULATOR-040` 定义 `player_parity / headless_agent / debug_viewer` execution lane；`PRD-WORLD_SIMULATOR-041` 只定义当前研发优先级与冻结策略，不新增玩家访问模式。`non-3D interaction` / `2D 优先` 在这里是 Viewer 当前 delivery priority 或 interaction scope，不是 `software_safe` 的别名，也不是新的 mode_id。当前 `standard_3d` taxonomy 继续保留，但进入暂停研发态，不再接受新的 3D feature 承诺。
- Critical User Flows:
  1. Flow-WS-001（Web-first 闭环）:
     `选择场景 -> 启动 Viewer Web -> 执行关键交互 -> 采集日志/截图/指标 -> 产出 test_tier_required 结论`
  2. Flow-WS-002（LLM 链路验证）:
     `启动链路 -> 触发 LLM 交互 -> 验证响应时延/错误处理 -> 故障时走降级路径 -> 归档证据`
  3. Flow-WS-003（启动器转账成功）:
     `启动器加载 -> 填写 from/to/amount/nonce -> 提交 -> runtime 接收并生成事件 -> UI 展示成功`
  4. Flow-WS-004（启动器转账失败）:
     `提交转账 -> 余额不足/nonce 重放/参数非法 -> API 返回结构化错误 -> UI 保留可诊断错误签名`
  5. Flow-WS-005（反馈分布式提交回退）:
     `提交反馈 -> 链状态服务 Connection refused -> 回落本地落盘 -> 展示失败原因并保留远端错误`
  6. Flow-WS-006（Viewer 粉紫屏回归）:
     `启动 native viewer -> 切换 2D/3D -> 观察渲染与交互 -> 若异常则抓取日志/截图 -> 修复后执行 Web+native 双链路回归`
  7. Flow-WS-007（Launcher 重复打包容错）:
     `已有 bundle 运行中 -> 再次执行 build-game-launcher-bundle -> 二进制替换成功 -> 产物完整可启动`
  8. Flow-WS-008（Launcher Web 控制台远程运维）:
     `SSH 启动 oasis7_web_launcher -> 浏览器访问控制台 -> 提交配置并启动 -> 观察状态/日志 -> 远程停止`
  9. Flow-WS-009（Launcher UI Schema 共享）:
     `更新共享 schema -> native 配置区渲染同步变更 -> web 控制台通过 /api/ui/schema 动态渲染同字段`
  10. Flow-WS-010（Launcher egui Web 同层复用）:
     `构建 launcher wasm 静态资源 -> oasis7_web_launcher 托管静态目录 -> 浏览器加载同一 egui UI -> 调用 /api/state|start|stop`
  11. Flow-WS-011（Launcher wasm 时间兼容闭环）:
     `headed `agent-browser` 打开 launcher Web 页面 -> wasm 初始化 -> 无 time panic -> snapshot/console/screenshot 采证`
  12. Flow-WS-012（Launcher Web 必填校验分流）:
     `浏览器加载配置 -> 必填校验 -> 不再提示 launcher/chain runtime bin 必填 -> 继续启动流程`
  13. Flow-WS-013（Launcher native/web 同控制面）:
     `启动 oasis7_web_launcher -> native/wasm 客户端轮询 /api/state -> 分别触发 /api/start|stop 与 /api/chain/start|stop -> 状态一致反馈`
  14. Flow-WS-014（Viewer live runtime 接管 Phase 1）:
     `oasis7_viewer_live -> runtime::World 驱动 step -> 适配为 simulator 协议快照/事件 -> oasis7_viewer 消费`
  15. Flow-WS-015（Viewer live runtime 接管 Phase 2）:
     `oasis7_viewer_live --llm -> PromptControl/AgentChat 鉴权通过 -> LLM 决策动作桥接 runtime 执行 -> 输出兼容事件/快照`
  16. Flow-WS-016（Viewer live runtime 接管 Phase 3）:
     `补齐 action 映射 -> 增加等价回归 -> 删除 oasis7_viewer_live simulator 启动分支 -> 统一 runtime-only 启动与错误语义`
  17. Flow-WS-017（Viewer live runtime 真 LLM 全量接管）:
     `真实 LLM 决策 -> runtime action 执行 -> 100% 事件/快照映射 -> DecisionTrace 输出 -> 无启发式 fallback`
  18. Flow-WS-018（Launcher Web 链上转账闭环）:
     `链状态就绪 -> 打开转账窗口 -> 填写 from/to/amount/nonce -> 通过 /api/chain/transfer 提交 -> 返回 action_id 或结构化错误`
  19. Flow-WS-019（Launcher Web 设置/反馈对齐）:
     `链状态就绪 -> 打开设置窗口编辑参数 -> 打开反馈窗口提交 kind/title/description -> 通过 /api/chain/feedback 返回 feedback_id/event_id 或结构化错误`
  20. Flow-WS-020（Launcher native 遗留清理）:
     `确认 native 已走统一控制面 -> 删除失效状态字段/未引用旧测试文件 -> 运行 native+wasm 回归 -> 状态与交互行为保持不变`
  21. Flow-WS-021（Launcher 转账产品级一致性）:
     `链状态就绪 -> 打开同层转账面板 -> 账户选择 + 余额辅助 + 自动 nonce -> 提交 -> pending -> confirmed/failed -> 历史可追溯`
  22. Flow-WS-022（Launcher 区块链浏览器）:
     `链状态就绪 -> 打开区块链浏览器面板 -> 查询 overview -> 按账户/状态过滤交易 -> 输入 action_id 查看详情`
  23. Flow-WS-023（Launcher 区块链浏览器 P0 公共主链视角）:
     `打开浏览器面板 -> Blocks 分页浏览 -> 点击区块看详情 -> Txs 按 tx_hash 查询 -> Search 统一检索 block/tx/action/account`
  24. Flow-WS-024（Launcher 区块链浏览器 P1 公共主链视角）:
     `打开浏览器面板 -> Address 查询余额/nonce/交易 -> Contracts 查看系统合约目录与详情 -> Assets 查看主 token 与 NFT 能力状态 -> Mempool 查看 pending 交易`
  25. Flow-WS-025（Launcher 可用性与体验硬化）:
     `源码直跑 oasis7_web_launcher -> 默认静态目录自动回退 -> 链未就绪时按钮禁用并显示原因 -> 查询参数编码后发起 explorer/search/transfer 请求 -> 未运行时 stop no-op 保留错误态 -> 移动端可读布局 + favicon 无 404 噪声`
  26. Flow-WS-026（Launcher 启动阻断配置引导）:
     `点击启动游戏/区块链 -> 检测到阻断配置 -> 弹出配置引导窗口并直接填写字段 -> 校验通过后再次启动`
  27. Flow-WS-027（Launcher 区块链浏览器视觉与交互优化）:
     `打开浏览器面板 -> 查看分组概览与状态计数 -> 在区块/交易列表点击项并同页查看详情 -> 按需应用或清空筛选 -> 跨 tab 跳转 tx_hash 完成定位`
  28. Flow-WS-028（Launcher 可用性残余风险收口）:
     `编辑高级配置并保持草稿 -> 并行执行 explorer/transfer 查询与状态轮询 -> 链状态波动时反馈窗口保持打开 -> 顶栏在窄屏自动换行 -> 转账过滤支持一键清空恢复`
  29. Flow-WS-029（Launcher 自引导首会话闭环）:
     `首次进入 -> 3 步向导 -> 任务流卡片驱动链/游戏启动 -> 禁用态 CTA 修复 -> 术语提示 + 快捷入口 -> 成功配置保存并记录引导计数`
  30. Flow-WS-030（Launcher GUI Agent 全量接口）:
     `GET /api/gui-agent/capabilities -> 选择 action -> POST /api/gui-agent/action -> 返回结构化 data+state -> 按 error_code 自动重试/降级`
  31. Flow-WS-031（runtime required 失败用例临时下线）:
     `执行 required 测试 -> 命中已知 10 项失败 -> 精确白名单加 #[ignore] 并标注原因 -> required 测试恢复可执行 -> 根因修复后逐项解除 ignore`
  32. Flow-WS-032（launcher execution world 输出路径收敛）:
     `启动 oasis7_game_launcher/oasis7_web_launcher -> 构造 oasis7_chain_runtime 参数时显式附带 --execution-world-dir -> runtime 持久化 explorer-index.json 到 output/chain-runtime/<node_id>/reward-runtime-execution-world`
  33. Flow-WS-033（Viewer 3D 工作流冻结）:
     `收到新的 Viewer 需求 -> 判断是否属于 3D scene/camera/render/material/light/post-process 可视化专题 -> 若是则标记 paused 并归档到当前用户交互分支暂存态 -> 若否则继续落到 software_safe / launcher / runtime interaction 主链路 -> 仅在恢复门禁通过后才允许重新激活 3D workstream`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 启动器转账提交 | `from_account`、`to_account`、`amount`、`nonce`；`amount` > 0；`nonce` 为非负整数 | 点击“提交转账”触发请求；非法输入阻止提交并显示错误 | `idle -> validating -> submitting -> success/failed` | 余额与 nonce 校验在 runtime 执行；`nonce` 必须严格递增 | 仅在区块链功能可用且配置合法时允许提交 |
| 顶部区块链状态指示 | `disabled/not_started/booting/ready/unreachable/config_error` | 启动后自动探针；状态变化实时刷新；错误支持 hover 详情 | `booting -> ready/unreachable/config_error` | 默认 1s 周期探测；短超时避免阻塞 UI | 状态读取只读，无写权限 |
| Viewer Web 闭环入口 | 场景标识、测试命令、产物路径、结论 | 执行 Web-first 流程并输出证据模板 | `planned -> running -> evidence_ready -> reviewed` | 按场景优先级与风险等级执行 | required gate 为默认必经路径 |
| LLM 链路验证 | 模型配置、提示模板、超时阈值、降级开关 | 触发交互与降级；记录成功/失败信号 | `ready -> invoking -> fallback(optional) -> completed` | 统计成功率、超时率、回退率 | 仅测试环境允许注入调试配置 |
| 反馈分布式提交 | 反馈文本、会话标识、链状态地址 | 远端失败时自动写本地文件，不丢失原始错误 | `submitting -> remote_failed -> local_saved -> reported` | 本地落盘路径按日期归档；错误签名保留用于回归 | 仅本地授权实例可写入反馈归档目录 |
| Viewer 色调映射兼容 | `render_profile`、`tonemapping`、`bevy feature`（含 `tonemapping_luts`） | 启动时加载后处理组件；缺失依赖时不得进入粉紫回退态 | `boot -> render_ready -> interactive` | 默认 profile 保持稳定可视；2D/3D 切换不重置错误态 | 对外仅保留配置入口；底层 feature 由构建配置管控 |
| Launcher bundle 二进制替换容错 | `OUT_DIR/bin/*`、目标文件占用状态、`--profile`、`--web-dist` | 打包时先删除目标二进制再 copy；若旧进程占用也需完成替换 | `build_start -> binaries_replaced -> web_prepared -> bundle_ready` | 同一 `OUT_DIR` 多次执行后产物版本需一致，不得残留半更新状态 | 仅本地构建者可写 bundle 输出目录 |
| Launcher Web 控制台 | `scenario/live_bind/web_bind/viewer_host/viewer_port/viewer_static_dir/llm/chain` | 浏览器点击“启动/停止”触发子进程编排与状态刷新 | `idle -> running -> stopped/exited` | bind/端口/目录先校验，失败时拒绝启动并返回错误详情 | 默认部署在受信网络，具备远程访问能力 |
| Launcher UI schema 共享 | `id/section/kind/label_zh/label_en/web_visible/native_visible` | UI 按 schema 动态渲染字段，新增字段无需双端重复定义 | `schema_loaded -> form_ready` | section 内按 schema 顺序渲染 | schema 只读；不含敏感数据 |
| Launcher egui Web 复用与静态托管 | `launcher wasm dist`、`console_static_dir`、`/api/state|start|stop` | `oasis7_web_launcher` 托管静态资源并由浏览器运行同一 egui UI | `boot -> static_ready -> interactive` | 静态请求走目录白名单，API 路径优先级高于静态路径 | 受信网络部署，禁止目录穿越 |
| Launcher wasm 时间兼容 | `WEB_POLL_INTERVAL_MS`、`last_web_poll_at`、web time primitive | 页面轮询 `/api/state` 且不触发 wasm 时间平台 panic | `interactive -> polling -> synced` | 轮询基于单调时钟差值，防并发请求堆积 | 浏览器会话可读，接口由受信网络控制 |
| Launcher Web 必填分流 | `launcher_bin`、`chain_runtime_bin`（native-only） | Web 端校验排除 native-only 必填，native 端保持阻断 | `config_loaded -> validated` | 按 `target_arch` 分流 | 平台边界一致 |
| Launcher native/web 同控制面 | `/api/state`、`/api/start`、`/api/stop`、`/api/chain/start`、`/api/chain/stop` | native 与 web 端统一走 API 触发游戏/区块链独立启停 | `service_ready -> game_running/stopped + chain_starting/ready/stopped` | 状态以服务端快照为准，客户端仅做展示与轮询 | 控制面部署在受信网络，客户端仅消费授权 API |
| Launcher Web 转账闭环 | `/api/chain/transfer`、`from/to/amount/nonce` | 浏览器端提交转账请求并展示成功/拒绝/失败 | `idle -> validating -> submitting -> success/failed` | `amount/nonce > 0`、`from != to`，账本规则以 runtime 为准 | 链就绪才允许提交 |
| Launcher 转账产品级一致性 | `from/to` 账户选择器、余额辅助、`nonce_mode(auto/manual)`、`action_id`、`final_status`、历史列表 | native/web 使用同一转账前端状态机；提交后跟踪最终状态并可查看历史 | `idle -> validating -> submitting -> accepted/pending -> confirmed/failed/timeout` | 历史按 `timestamp desc` + `action_id desc`；`auto nonce` 默认开启 | 链未就绪时入口禁用；查询只读、提交可写 |
| Launcher 区块链浏览器 | `/api/chain/explorer/overview`、`/api/chain/explorer/transactions`、`/api/chain/explorer/transaction`、`account_filter`、`status_filter`、`action_id` | 打开浏览器面板后可刷新概览、过滤交易、查询交易详情 | `closed/open` + `idle -> loading -> ready/failed` | 交易列表按 `submitted_at desc` + `action_id desc`；默认 `limit=50` | 链未就绪时入口禁用；查询只读 |
| Launcher 区块链浏览器 P0（公共主链视角） | `/api/chain/explorer/blocks`、`/block`、`/txs`、`/tx`、`/search`、`cursor/limit`、`tx_hash` | 支持区块/交易分页、交易哈希详情与统一搜索（block/tx/action/account） | `closed/open` + `idle -> loading -> ready/failed` | 区块按 `height desc`；交易按 `submitted_at desc + tx_hash desc`；`limit<=200` | 链未就绪时入口禁用；查询只读 |
| Launcher 区块链浏览器 P1（公共主链视角） | `/api/chain/explorer/address`、`/contracts`、`/contract`、`/assets`、`/mempool`、`account_id/contract_id/status/limit/cursor` | 支持地址/合约/资产/内存池查询、筛选与分页 | `closed/open` + `idle -> loading -> ready/failed` | mempool 按 `submitted_at desc + tx_hash desc`；holders 按 `balance desc`；`limit<=200` | 链未就绪时入口禁用；查询只读 |
| Launcher 可用性与体验硬化 | `viewer_static_dir` 候选路径、`chain_status`、查询参数（`account_id/contract_id/q/tx_hash/action_id`）、移动视口布局、favicon 声明、`ConfigIssue -> 字段` 引导映射 | 启动自动路径回退；禁用按钮显示原因；请求前统一 URL 编码；stop no-op 保留状态语义；小屏字段纵向可读；启动阻断时弹出可编辑配置引导 | `idle -> running` 或 `idle -> invalid_config`；`stop(no-op) -> same_state`；`start_click -> guide_open -> retry_start` | 路径按候选优先级命中；查询参数按 RFC3986 安全子集编码；引导字段按问题去重排序 | 配置编辑限本地运维；查询只读；控制面操作可写 |
| Launcher 区块链浏览器视觉与交互优化 | 概览卡片、状态徽标、筛选栏、清空动作、列表-详情协同区域 | 支持快速刷新、筛选恢复、列表点击即详情、跨 tab 跳转 | `idle -> loading -> ready/failed`（请求态可见） | 保持既有查询排序；交互减少无效往返点击 | 链未就绪时入口禁用；查询只读 |
| Launcher 全量可用性残余风险收口 | `config_dirty`、in-flight 域（`state/control/feedback/transfer_submit/transfer_query/explorer`）、`feedback_window_open`、转账历史过滤字段 | 配置编辑时阻止快照回写；请求按域并发；反馈窗口不被强制关闭；顶栏 wrapped；新增清空过滤 | `clean/dirty/synced` + `idle/inflight(domain)` + `window_open/disabled_submit` | 轮询只由 `state` 域门控；过滤清空后按默认排序刷新 | 配置编辑本地可写；查询只读；控制操作按既有权限 |
| Launcher 自引导体验闭环 | `onboarding_step/onboarding_completed`、任务卡 `ready/blocked_reason/cta`、术语 key（nonce/slot/mempool/action_id）、`last_successful_config`、`demo_mode_enabled`、本地计数器 | 默认展示 3 步引导 + 任务流卡片；禁用态提供就地 CTA；转账/浏览器提供快捷动作；成功配置自动保存并可恢复；演示模式串行动作；引导计数可见 | `onboarding: hidden/step1/step2/step3/completed`；`task: todo/doing/done/blocked`；`profile: none/saved/restored` | 任务卡按依赖顺序（链->游戏->页面）；计数器按事件单调递增；成功配置仅保留最近一次 | 本地会话可写；查询与术语解释只读 |
| Launcher GUI Agent 全量接口 | `/api/gui-agent/capabilities`、`/api/gui-agent/state`、`/api/gui-agent/action{action,payload}` | 机器端通过单一动作入口执行全部人工功能（启停/反馈/转账/浏览器查询） | `ready -> action_running -> succeeded/failed` | `query_target` 白名单映射固定 runtime 路径；动作后必须返回最新 `state` | 受信网络内可调用；沿用现有控制面权限边界 |
| runtime required 失败用例治理与恢复 | 失败测试白名单（10 项）、`#[ignore]` 注解、原因标签（builtin identity manifest / DistFS 漂移）、恢复任务 | 根因未修复前允许对白名单测试逐个加 `#[ignore]` 保留实现与断言；manifest/hash/DistFS 修复后必须移除 ignore 并恢复定向回归 | `failing -> ignored -> restored` | 临时下线数量固定为 10，恢复后必须归零；禁止模块级批量屏蔽 | 仅维护者可修改测试注解 |
| Launcher execution world 输出路径收敛 | `chain_node_id`、`--execution-world-dir`、`output/chain-runtime/<node_id>/reward-runtime-execution-world` | 启动器拉起 runtime 时显式传递 `--execution-world-dir`，避免依赖 runtime 当前工作目录 | `launcher_start -> chain_runtime_args_built -> runtime_running` | 输出目录固定规则：`output/chain-runtime/<node_id>/reward-runtime-execution-world` | 启动器维护路径可写；普通查询能力不变 |
| Launcher Web 设置/反馈对齐 | 设置窗口字段 + `/api/chain/feedback` + `kind/title/description` | 浏览器端可打开设置窗口与反馈窗口；反馈提交通过控制面代理返回结构化结果 | `settings: closed/open/saved` + `feedback: idle/validating/submitting/success/failed` | 反馈标题/描述必填；单请求 in-flight 门控 | 反馈提交仅链就绪可用；设置仅当前会话可编辑 |
| Launcher native 遗留清理 | native 失效状态字段、无效常量 `cfg` 边界、未引用旧测试文件 | 保持现有 UI/API 行为不变前提下清理历史残留 | `legacy_present -> removed -> regression_passed` | 优先删除“无读写路径/无编译入口引用”的资产 | 仅开发维护路径可修改，运行时玩家能力不变 |
| Viewer live runtime 接管 | runtime `DomainEvent`、兼容 `WorldSnapshot/WorldEvent` | 启动 `oasis7_viewer_live` 后按 Play/Step 推进 runtime，并推送兼容快照/事件 | `runtime_mode`（固定） | 事件序列保持单调；至少映射注册/移动/转移/拒绝四类事件 | 本地开发链路，默认不开放远程写接口 |
| Viewer live runtime LLM/chat/prompt 接管 | `--llm`、`PromptControl`、`AgentChat`、auth proof、nonce | 运行时允许 prompt 预览/应用/回滚与 agent chat，驱动 LLM 决策并桥接可映射动作 | `runtime_script/runtime_llm` + `profile[vN]->profile[vN+1]` | 版本单调递增；nonce 必须递增；不可映射动作输出可诊断拒绝 | 仅本地受控链路可写，鉴权签名与绑定校验必经 |
| Viewer live runtime action 覆盖与分支收敛 | `simulator_action_to_runtime`、`ActionRejected::RuleDenied`、`oasis7_viewer_live` runtime-only CLI | 补齐 runtime 可执行映射并对不可映射动作保持结构化拒绝；启动链路移除 simulator 分支 | `runtime_bridge_partial -> runtime_bridge_hardened` | 映射成功动作优先执行；不可映射动作拒绝语义稳定可回归 | 不新增远程写入口，仅本地受控链路 |
| Viewer 3D 暂停治理 | `workstream_id`、`pause_status`、`branch_state`、`allowed_change_scope`、`resume_gate` | 新的 3D 需求进入暂停池；当前用户交互分支只保留暂存参考；仅允许保证主链路不腐烂的低风险维护 | `active -> paused -> hold -> resume_review -> resumed` | 先按是否属于 3D 可视化专题分类，再按是否影响 `software_safe` / launcher / runtime 主链路决定是否允许修改 | `producer_system_designer` 冻结；`viewer_engineer` 仅可在允许范围内维护 |
- Acceptance Criteria:
  - AC-1: world-simulator PRD 覆盖场景、Viewer、LLM、启动器四条主线。
  - AC-2: world-simulator project 文档维护任务拆解与状态。
  - AC-3: 与 `doc/world-simulator/scenario/scenario-files.prd.md`、`doc/world-simulator/viewer/viewer-web-closure-testing-policy.prd.md` 等专题文档一致。
  - AC-4: 关键交互变更同步更新 testing 手册与测试记录。
  - AC-5: 分册内专题条款（接口/安全/测试）在主 PRD 中可定位、在项目文档中可执行。
  - AC-6: 统一验收清单覆盖场景、Viewer Web 闭环、启动器入口与证据模板，并与 `testing-manual.md` 一致。
  - AC-7: Web-first 与 LLM 证据模板可直接用于 S6/S8，且必填字段包含命令、产物路径、指标、结论。
  - AC-8: 体验质量趋势文档定义指标、采集节奏、归档路径与记录模板，并可落地执行。
  - AC-9: 启动器反馈分布式提交流程在链状态服务 `Connection refused` 时必须回落本地落盘，并保留远端连接失败错误签名用于诊断与回归测试。
  - AC-10: 启动器顶部必须可视化区块链启动状态（含禁用/未启动/启动中/已就绪/不可达），用于玩家快速判定链能力是否可用。
  - AC-11: 客户端启动器必须将“区块链启动/停止”与“游戏启动/停止”拆分为独立按钮；打开启动器后默认自动拉起区块链进程；游戏启动链路不再隐式托管区块链进程。
  - AC-12: 仅当区块链处于“已就绪”状态时，反馈按钮可用并允许打开反馈窗口；区块链未启动/启动中/不可达时反馈入口需明确禁用。
  - AC-13: 设置窗口必须提供完整配置分区（游戏、区块链、LLM），并覆盖启动器运行所需的核心参数编辑入口。
  - AC-14: 设置中心内的 LLM 配置（`llm.api_key/base_url/model`）必须支持文件重载与保存；游戏/区块链配置变更应即时作用于启动器内存配置。
  - AC-15: `oasis7_viewer` native 链路在默认渲染配置下不得出现 `TonyMcMapFace tonemapping requires tonemapping_luts feature` 错误；2D/3D 均可正常渲染并可交互。
  - AC-16: `scripts/build-game-launcher-bundle.sh` 在 `OUT_DIR/bin` 目标文件已存在且正被运行时，重复执行仍可成功产出完整 bundle，不得出现 `Text file busy` 或“二进制部分更新”状态。
  - AC-17: `oasis7_web_launcher` 支持在无 GUI 服务器上通过浏览器完成启动/停止、状态查询与日志查看，且打包产物提供独立入口脚本。
  - AC-18: `oasis7_client_launcher` 与 `oasis7_web_launcher` 必须消费同一份 launcher UI schema；web 控制台表单字段通过 `/api/ui/schema` 动态渲染。
  - AC-19: 启动器 web UI 必须由 `oasis7_client_launcher` 的 egui wasm 产物提供；`oasis7_web_launcher` 默认托管该静态目录且保持 API 闭环可用。
  - AC-20: 启动器 wasm 页面在 `agent-browser --headed` 打开后不得出现 `time not implemented on this platform` 或 `RuntimeError: unreachable`，并需输出 snapshot/console/screenshot 证据。
  - AC-21: 启动器 Web 端不得再提示 `launcher bin` 与 `chain runtime bin` 必填；native 端保留对应必填校验。
  - AC-22: 启动器 native 与 web 必须统一消费 `oasis7_web_launcher` 控制面 API，并支持链/游戏独立启停及一致状态展示。
  - AC-23: `oasis7_viewer_live` 可启动 runtime 驱动 live server，并保持现有 viewer 协议兼容（`WorldSnapshot/WorldEvent`）。
  - AC-24: `oasis7_viewer_live --llm` 必须支持 gameplay/prompt/chat 鉴权与控制闭环；`--no-llm` 仅保留 observer/debug，`step/play/gameplay_action/prompt/chat` 均需返回显式 `llm_mode_required` 或 `llm_init_failed` 阻断。
  - AC-25: runtime live 补齐动作映射覆盖并新增等价回归；`oasis7_viewer_live` 移除 simulator 启动分支并统一 runtime-only 路径。
  - AC-26: runtime live 使用真实 LLM 决策链路且 LLM 失败时硬失败，不得回退启发式。
  - AC-27: runtime 事件/快照映射覆盖率 100%，DecisionTrace 可被 viewer 订阅并包含错误上下文。
  - AC-28: 启动器在轮询并发场景下保持交互稳定：配置编辑不被回写、请求按域并发、反馈草稿不丢失、顶栏窄屏可读、转账过滤可一键清空。
  - AC-29: 启动器 Web 端支持转账提交（`/api/chain/transfer`），并可展示结构化 `action_id/error_code/error`。
  - AC-30: 启动器 Web 端 `设置` 与 `反馈` 入口可用，反馈提交流程通过 `/api/chain/feedback` 返回结构化成功/失败结果。
  - AC-31: 启动器 native 已失效遗留代码（状态字段/测试资产）完成清理后，`oasis7_client_launcher` 与 `oasis7_web_launcher` required 回归通过且行为无回归。
  - AC-32: 启动器转账功能升级为产品级体验（账户/余额辅助、自动 nonce、最终状态与历史可视化），且 native/web 同层前端行为一致并通过跨端回归。
  - AC-33: 启动器区块链浏览器面板支持 overview/transactions/transaction 查询，且 native/web 行为一致并通过 required 回归。
  - AC-34: 启动器区块链浏览器支持 `blocks/block/txs/tx/search`、分页与 `tx_hash` 查询，并具备重启后索引恢复能力（最近窗口）且 native/web 行为一致。
  - AC-35: 启动器区块链浏览器支持 `address/contracts/contract/assets/mempool` 五类查询（含筛选/分页/结构化错误语义），且 native/web 行为一致并通过 required 回归。
  - AC-36: 启动器可用性与体验硬化完成：源码直跑默认静态目录有效回退、wasm 禁用态提示可见、explorer/search/transfer 查询参数统一编码、stop no-op 不覆盖错误态、390x844 视口配置区可读、页面无 `favicon.ico 404` 噪声。
  - AC-37: 启动器在配置阻断时必须弹出“可编辑配置引导”窗口（非纯提示），首次进入若存在阻断项自动弹出一次；修复后可直接重试启动。
  - AC-38: 启动器区块链浏览器完成视觉与交互优化：概览分组可判读、状态可视化、筛选可一键恢复、列表与详情同页协同，且 native/web 行为一致并通过 required 回归。
  - AC-39: 启动器自引导闭环完成：首次 3 步引导、任务流卡片、禁用态 CTA、转账金额预设与时间线、浏览器快捷入口、术语解释、成功配置画像、演示模式与本地引导计数在 native/web 双端一致可用。
  - AC-40: 启动器自引导 round-2 完成：错误卡片三动作（修复配置/自动补默认值/重试）、阻塞态可执行下一步、启动前体检修复清单、跳过引导后的持续轻提示在 native/web 双端一致可用。
  - AC-41: 启动器代码治理满足工程约束：`oasis7_client_launcher` 单个 Rust 源文件不得超过 1200 行，超限时需模块化拆分且行为不回归。
  - AC-42: `oasis7_web_launcher` 新增 `/api/gui-agent/capabilities|state|action`，并通过统一动作集合覆盖人工操作全功能（启停、反馈、转账、浏览器查询），所有动作返回结构化 `{ok,action,error_code?,error?,data?,state}`。
  - AC-43: `oasis7` runtime 这 10 个历史失败项在根因未修复时只允许按白名单临时下线且保留原因注释；一旦 `m1` builtin manifest/hash/DistFS 根因修复，`#[ignore]` 数量必须回到 0，并通过定向 `wasmtime` 回归证明已恢复。
  - AC-44: Viewer 活跃手册、原生窗口标题、Web 页面 `<title>` 与弱图形页标题必须统一使用 `oasis7 Viewer` 品牌；旧 `oasis7 Viewer` 仅可作为脚本兼容匹配或历史证据上下文保留，不得继续作为当前公开标题。
  - AC-45: `doc/world-simulator/**` 仍可读历史专题的首行标题必须统一切换到 `oasis7` / `oasis7 Simulator` / `oasis7 Viewer` 品牌；旧 `oasis7*` 标题仅允许出现在正文历史上下文中，不改动内部实现兼容名与历史证据正文。
  - AC-46: `oasis7_client_launcher` 在 Web/native 两端对外暴露的新前端运行时 key（如 localStorage key、UX 状态文件名、Canvas id、字体/env key）默认切到 `oasis7` 前缀，同时继续兼容读取旧 key；不得因为改名前缀变更而丢失既有浏览器配置或阻断 bundle/script 现网入口。
  - AC-47: `oasis7_client_launcher` Web 静态入口 `<title>` 必须使用 `oasis7 Launcher (Web)`；旧 `oasis7 Launcher (Web)` 不得继续作为当前公开标题。
  - AC-48: `oasis7_client_launcher` 原生窗口标题与应用内主标题必须统一使用 `oasis7 Client Launcher` / `oasis7 客户端启动器`；旧 `oasis7 Client Launcher` 文案不得继续作为当前公开标题。
  - AC-49: `oasis7_viewer` 与 `oasis7_game_launcher` 的 viewer auth/bootstrap 链路必须默认写入并优先读取 `OASIS7_VIEWER_*` 与 `__OASIS7_VIEWER_AUTH_ENV`；`software_safe`、chat auth、automation、embedded launcher 注入与右侧面板持久化路径不得因改名失配。
  - AC-50: `oasis7_game_launcher` 与 `oasis7_web_launcher` 必须默认优先读取 `OASIS7_VIEWER_LIVE_BIN`、`OASIS7_CHAIN_RUNTIME_BIN`、`OASIS7_GAME_STATIC_DIR`、`OASIS7_GAME_LAUNCHER_BIN`、`OASIS7_WEB_LAUNCHER_STATIC_DIR`；帮助文案、错误信息和控制面静态目录校验不得再暴露旧品牌 env 名。
  - AC-51: `oasis7_viewer` 的 3D 配置、theme runtime、panel/headless 控制与 release UI profile 默认必须写入并优先读取 `OASIS7_VIEWER_*`；`viewer_3d_config`、theme preset 解析、无头 auto-play、panel mode/experience mode 与 profile env 文件不得因前缀切换而丢失既有本地覆盖或回归脚本能力。
  - AC-52: `oasis7_viewer` 的 automation、auto-focus、auto-degrade、event window 与 internal capture 默认必须写入并优先读取 `OASIS7_VIEWER_*`；startup automation、聚焦/降级策略、事件抽样与内部抓帧入口不得因前缀切换而失效。
  - AC-53: `scripts/capture-viewer-frame.sh`、`scripts/viewer-texture-inspector*.sh` 与 `scripts/viewer-owr4-stress.sh` 必须默认写入或展示 `OASIS7_VIEWER_*`；脚本输出的运行命令、状态文件、meta 记录与清理逻辑不得再把旧前缀当作唯一真值。
  - AC-54: `simulator/llm_defaults`、`simulator/llm_agent` 与 `scripts/llm-longrun-stress.sh` 必须默认优先读取或写入 `OASIS7_LLM_MODEL`、`OASIS7_LLM_BASE_URL`、`OASIS7_LLM_API_KEY`、`OASIS7_LLM_TIMEOUT_MS`、`OASIS7_LLM_PROMPT_*`、`OASIS7_LLM_DEBUG_MODE`、`OASIS7_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS`；agent-scoped goal override、调试提示与 longrun stress 命令注入不得因前缀切换而失效。
  - AC-55: `viewer/runtime_live/llm_sidecar`、`viewer/runtime_live/control_plane`、`oasis7_game_launcher`、`viewer/runtime_live/tests` 与 `scripts/viewer-software-safe-chat-regression.sh` 必须默认优先读取或写入 `OASIS7_AGENT_DECISION_SOURCE`、`OASIS7_AGENT_PROVIDER_BACKEND`、`OASIS7_AGENT_PROVIDER_CONTRACT`、`OASIS7_AGENT_PROVIDER_TRANSPORT`、`OASIS7_AGENT_PROVIDER_URL`、`OASIS7_AGENT_PROVIDER_AUTH_TOKEN`、`OASIS7_AGENT_PROVIDER_CONNECT_TIMEOUT_MS`、`OASIS7_AGENT_PROVIDER_PROFILE`、`OASIS7_AGENT_EXECUTION_LANE` 与 `OASIS7_RUNTIME_AGENT_CHAT_ECHO`；`OASIS7_AGENT_PROVIDER_MODE`、`OASIS7_OPENCLAW_*` 与对应旧 alias 仅保留兼容 fallback，OpenClaw provider 解析、player_parity 元数据、QA echo 注入与 software-safe Web 回归不得因结构化字段迁移而失效。
  - AC-56: `scripts/build-game-launcher-bundle.sh` 生成的 wrapper 脚本必须默认写入 `OASIS7_GAME_LAUNCHER_BIN`、`OASIS7_GAME_STATIC_DIR`、`OASIS7_CHAIN_RUNTIME_BIN`、`OASIS7_WEB_LAUNCHER_STATIC_DIR` 与 `OASIS7_CHAIN_STORAGE_PROFILE`；README 中的 bundle-first operator 示例不得再把旧前缀作为默认真值。
  - AC-57: `crates/oasis7_viewer/assets/themes/**/presets/*.env` 必须默认导出 `OASIS7_VIEWER_*`；同一主题的 default/glossy/matte 预设都必须完成一致迁移，且 theme preset 仍能被现有 viewer env 解析链路消费。
  - AC-58: `scripts/setup-openclaw-oasis7-runtime.sh`、`.agents/skills/oasis7/scripts/oasis7-run.sh`、`.agents/skills/oasis7/**/*.md` 与 `oasis7_openclaw_local_bridge` 相关测试样例必须只使用 `oasis7_openclaw_agent` 与 `OPENCLAW_OASIS7_{AGENT_ID,WORKSPACE,MODEL}`；repo-owned runtime workspace 默认路径必须切到 `tools/openclaw/oasis7_openclaw_workspace`，旧品牌 runtime setup 路径仅可作为历史路径上下文保留，不得继续作为 operator-facing 真值。
  - AC-59: `crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs`、`crates/oasis7/src/bin/oasis7_web_launcher/oasis7_web_launcher_tests.rs` 与 `crates/oasis7/src/bin/oasis7_web_launcher/static_files.rs` 里的临时目录前缀必须默认改为 `oasis7_*`；launcher/web-launcher 定向测试通过后，临时目录命名不得继续输出 `oasis7_*` 作为当前默认值。
  - AC-60: `scripts/ci-tests.sh` 里的 required/full/support 分层 helper 与分发入口必须默认改为 `run_oasis7_*` 命名；脚本帮助、tier dispatch 与执行行为保持不变，且脚本内不得残留 `run_oasis7_*` 作为当前 helper 真值。
  - AC-61: `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api_tests.rs` 的临时目录前缀必须默认改为 `oasis7_transfer_submit_api_tests_*`；转账提交/状态/浏览器查询定向测试通过后，不得继续输出 `oasis7_transfer_submit_api_tests_*` 作为当前默认值。
  - AC-62: `oasis7_game_launcher`、`oasis7_openclaw_parity_bench`、`viewer/runtime_live/llm_sidecar`、`oasis7_client_launcher`、`scripts/openclaw-parity-p0.sh` 与 `.agents/skills/oasis7/**` 必须只使用 `oasis7_p0_low_freq_npc`；`oasis7_openclaw_local_bridge` 不得继续接受旧 `oasis7_p0_low_freq_npc` 别名，定向回归必须明确断言旧 profile id 已失效。
  - AC-63: `crates/oasis7_distfs/src/{lib.rs,manifest.rs,replication.rs,feedback/tests.rs,feedback_p2p.rs,challenge/tests.rs,challenge_scheduler.rs}` 与 `crates/oasis7_consensus/src/**/*tests*.rs` 中的临时目录前缀必须默认改为 `oasis7_distfs_*` / `oasis7_consensus_*` 系列；对应包测试通过后，不得继续输出 `oasis7-*` 或 `oasis7_*` 作为当前默认值。
  - AC-64: `crates/oasis7/src/bin/oasis7_chain_runtime/{execution_bridge.rs,reward_runtime_worker.rs,storage_metrics.rs}`、`crates/oasis7/src/runtime/{builtin_wasm_materializer.rs,segmenter.rs,node_points_runtime.rs}`、`crates/oasis7/src/runtime/tests/{audit.rs,builtin_wasm_materializer.rs,gameplay_protocol_split_part1.rs,persistence.rs,power_bootstrap_release_manifest_full.rs,storage_cold_index.rs,storage_footprint_fixture.rs}`、`crates/oasis7/src/simulator/tests/persist.rs` 与 `crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs` 的临时目录/工作目录前缀必须默认改为 `oasis7_*`；定向回归通过后，主包不得继续输出这些旧 `oasis7-*` / `oasis7_*` 默认值。
  - AC-65: `crates/oasis7/src/bin/oasis7_openclaw_parity_bench.rs` 的 `parse_options_accepts_custom_openclaw_agent_profile` 样例值必须改为 `oasis7_p1_memory_loop`，避免 bench 的自定义 profile 参数解析回归继续把旧 `oasis7_p1_memory_loop` 当作当前样例真值。
  - AC-66: `crates/oasis7/src/runtime/world/{mod.rs,governance.rs}`、`crates/oasis7/src/runtime/tests/{mod.rs,governance.rs,module_action_loop_split_part1.rs}`、`crates/oasis7/src/bin/oasis7_chain_runtime.rs` 与 `crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge.rs` 中的 deterministic signer seed / namespace 字符串必须改为 `oasis7-test-module-artifact-signer-v1`、`oasis7-governance-local-finality-signer-{1,2}-v1`、`oasis7-governance-test-finality-signer-3-v1` 与 `oasis7-node-consensus-signer-v1`；相关 governance / module / execution 回归通过后，不得继续把旧 `oasis7-*signer*` 当作当前真值。
  - AC-67: `crates/oasis7/src/bin/oasis7_game_launcher.rs`、`crates/oasis7/src/bin/oasis7_web_launcher/{runtime_paths.rs,control_plane.rs}` 与对应测试必须把 viewer 开发态 fallback 候选顺序改为 `oasis7_viewer/dist` 优先、`oasis7_viewer/dist` 次之；`oasis7_viewer_demo` 的 operator 提示不得继续把 `oasis7_viewer` 当作当前连接指引真值。
  - AC-68: `crates/oasis7_client_launcher/src/{platform_ops.rs,launcher_core.rs,main_tests.rs}` 必须把源码态 viewer fallback 候选顺序统一为 `oasis7_viewer/dist` 优先、`oasis7_viewer/dist` 次之；`resolve_static_dir_path` 与 launcher config 校验不得继续把旧 viewer 目录名当作 client launcher 当前默认真值。
  - AC-69: `crates/oasis7_viewer/src/egui_right_panel_chat_tests.rs` 的 viewer auth 临时目录前缀必须改为 `oasis7_viewer_chat_auth_*`；对应 node-config signer 定向回归通过后，不得继续输出 `oasis7_viewer_chat_auth_*` 作为当前默认测试产物命名。
  - AC-70: `crates/oasis7/tests/module_store.rs` 的集成测试临时目录前缀必须改为 `oasis7-store-*` 系列，且 `crates/oasis7/tests/common/mod.rs` 的 deterministic module artifact signer seed 必须改为 `oasis7-test-module-artifact-signer-v1`；module store 定向回归通过后，不得继续把 `oasis7-store-*` 或旧 `oasis7-test-module-artifact-signer-v1` 当作当前默认真值。
  - AC-71: `crates/oasis7_wasm_executor/src/lib.rs` 的 disk cache 临时目录前缀必须改为 `oasis7-wasm-cache-*`，且 `crates/oasis7_wasm_store/src/lib.rs` 的测试临时目录前缀必须改为 `oasis7-wasm-store-*` / `oasis7-wasm-store-version-*`；对应包定向回归通过后，不得继续输出 `oasis7-wasm-*` 作为当前默认产物命名。
  - AC-72: `crates/oasis7_net/src/{bootstrap.rs,execution_storage.rs,head_validation.rs,observer.rs,observer_metrics.rs,observer_replay.rs}` 的测试临时目录前缀必须改为 `oasis7-net-*`，且 `crates/oasis7_node/src/{tests_split_part1.rs,tests_hardening.rs,replication/tests.rs}` 的测试临时目录前缀必须改为 `oasis7-node-*` / `oasis7-replication-tests-*`；对应包测试通过后，不得继续输出 `oasis7-net-*`、`oasis7-node-*` 或 `oasis7-replication-tests-*` 作为当前默认产物命名。
  - AC-73: `crates/oasis7_client_launcher/src/{self_guided.rs,llm_settings_web.rs}`、`crates/oasis7_viewer/src/right_panel_module_visibility.rs` 与 `crates/oasis7/src/bin/oasis7_openclaw_local_bridge.rs` 中保留旧品牌 fallback literal 的源码常量/测试名必须改为 `compat` 语义命名；兼容读取行为保持不变，但源码中不得继续把 `LEGACY_*oasis7*` 当作当前维护口径。
  - AC-74: `crates/oasis7_viewer/{software_safe.js,src/viewer_automation.rs,src/egui_right_panel_chat.rs,src/egui_right_panel_chat_auth.rs,src/viewer_env.rs,src/perf_probe.rs}` 中原旧品牌 fallback literal 对应的源码常量与定向测试名必须改为 `compat` 语义命名；源码中不得继续把旧品牌 viewer 维护口径当作当前命名规范。
  - AC-75: `crates/oasis7/src/runtime/{builtin_wasm_materializer.rs,module_source_compiler.rs}`、`crates/oasis7/src/simulator/llm_agent.rs` 与对应定向测试中原旧品牌 fallback literal 对应的源码常量/辅助函数/测试名必须改为 `compat` 语义命名；源码中不得继续把旧品牌运行时维护口径当作当前命名规范。
  - AC-76: `crates/oasis7/src/bin/oasis7_game_launcher.rs`、`crates/oasis7/src/viewer/runtime_live/{control_plane.rs,llm_sidecar.rs,tests.rs}` 中原旧品牌 launcher/provider/chat echo fallback literal 对应的源码常量/辅助函数/定向测试名必须改为 `compat` 语义命名；源码中不得继续把旧品牌维护口径当作当前命名规范。
  - AC-77: `crates/oasis7/src/bin/oasis7_web_launcher/{runtime_paths.rs,control_plane.rs,oasis7_web_launcher_tests.rs}` 与 `crates/oasis7_client_launcher/src/{main.rs,platform_ops.rs,main_tests.rs}` 中原旧品牌 launcher path fallback literal 对应的源码常量/辅助函数/定向测试名必须改为 `compat` 语义命名；源码中不得继续把旧品牌维护口径当作当前命名规范。
  - AC-78: `crates/oasis7/src/bin/oasis7_web_launcher/oasis7_web_launcher_tests.rs`、`crates/oasis7_client_launcher/src/platform_ops.rs` 与 `crates/oasis7_client_launcher/src/main_tests.rs` 中涉及 `viewer_dev_dist_candidates` 的定向测试函数名必须改为 `compat` 语义命名；测试行为保持不变，但源码中不得继续把 `legacy_name` 当作当前测试维护口径。
  - AC-79: `crates/oasis7/src/runtime/tests/power_bootstrap_release_manifest_full.rs` 中原旧品牌 fallback literal 对应的源码常量与测试局部变量名必须改为 `compat` 语义命名；`test_tier_full` 行为保持不变，且源码中不得继续把旧品牌维护口径当作当前命名规范。
  - AC-80: `scripts/{build-wasm-module.sh,ci-m1-wasm-summary.sh,sync-m1-builtin-wasm-artifacts.sh}` 中原旧品牌 fallback literal 对应的 helper/局部变量名必须改为 `compat` 语义命名；脚本行为保持不变，且源码中不得继续把旧品牌 helper 命名当作当前维护口径。
  - AC-81: `scripts/{setup-openclaw-oasis7-runtime.sh,viewer-texture-inspector-lib.sh,viewer-texture-inspector.sh,capture-viewer-frame.sh}` 中原旧品牌 fallback literal 对应的 helper/局部变量名与调用点必须改为 `compat` 语义命名；脚本行为保持不变，且源码中不得继续把旧品牌 helper 命名当作当前维护口径。
  - AC-82: `crates/oasis7_viewer/src/{egui_right_panel_chat_tests.rs,right_panel_module_visibility.rs,viewer_3d_config_profile_tests.rs}` 中涉及 compat fallback 的定向测试函数名必须改为 `compat` 语义命名；Viewer 行为保持不变，但源码中不得继续把 `legacy_text_format` / `legacy_dir` / `legacy_key` 当作当前测试维护口径。
  - AC-83: `crates/oasis7_viewer/src/{egui_right_panel_chat.rs,main_connection.rs}`、`crates/oasis7_proto/src/viewer.rs` 与 `crates/oasis7/src/viewer/live/tests.rs` 中涉及旧文本 fallback、预 hello profile、协议 payload 与 player binding 的 helper/定向测试名必须改为 `compat` 语义命名；Viewer/协议行为保持不变，但源码中不得继续把 `legacy_*` 当作当前兼容层维护口径。
  - AC-84: `crates/oasis7/src/{runtime/state.rs,runtime/world/resources.rs,runtime/world/module_runtime.rs,bin/oasis7_chain_runtime/execution_bridge.rs}` 中 material ledger 迁移、world materials 缓存同步、execution bridge pre-v2 检测与 module registry fallback 的 helper/局部变量名必须改为 `compat_*` 或准确中性命名；运行时行为保持不变，但源码中不得继续把这些兼容层路径写成 `legacy_*` 当前维护口径。
  - AC-85: `crates/oasis7_node/src/replication/{commit_retention.rs,tests.rs}` 中 commit cold index canonical manifest 与旧文件名 alias 的 helper/局部变量/定向测试名必须改为 `compat` 语义命名；节点复制行为保持不变，但源码中不得继续把旧文件名 alias 写成 `legacy_*` 当前维护口径。
  - AC-86: `crates/oasis7_distfs/src/replication.rs` 中 replication record/guard 旧 JSON 兼容样例的定向测试函数名与局部变量必须改为 `compat` 语义命名；DistFS 反序列化行为保持不变，但源码中不得继续把这些兼容样例写成 `legacy_*` 当前测试维护口径。
  - AC-87: workspace 根 `Cargo.toml`、`crates/oasis7_wasm_sdk/Cargo.toml`、builtin wasm modules 与模板中的 `oasis7_wasm_sdk` crate 依赖/路径/源码入口必须全部改为 `oasis7_wasm_sdk`，并将目录 `crates/oasis7_wasm_sdk` 重命名为 `crates/oasis7_wasm_sdk`；变更后 `cargo` 必须能按新 crate 名解析 SDK。
  - AC-88: workspace 根 `Cargo.toml`、`crates/oasis7_wasm_abi/Cargo.toml`、`crates/{oasis7,oasis7_net,oasis7_wasm_executor,oasis7_wasm_router,oasis7_wasm_store}` 与相关测试源码中的 `oasis7_wasm_abi` 依赖/路径/源码入口必须全部改为 `oasis7_wasm_abi`，并将目录 `crates/oasis7_wasm_abi` 重命名为 `crates/oasis7_wasm_abi`；变更后 `cargo` 必须能按新 crate 名解析 ABI。
  - AC-89: workspace 根 `Cargo.toml`、`Cargo.lock`、`crates/oasis7_wasm_router/Cargo.toml` 与 `crates/oasis7/src/runtime/world/{base_layer.rs,module_runtime.rs}` 中的 `oasis7_wasm_router` 依赖/路径/源码入口必须全部改为 `oasis7_wasm_router`，并将目录 `crates/oasis7_wasm_router` 重命名为 `crates/oasis7_wasm_router`；变更后 `cargo` 必须能按新 crate 名解析 router。
  - AC-90: workspace 根 `Cargo.toml`、`Cargo.lock`、`crates/oasis7_wasm_{executor,store}/Cargo.toml` 与 `crates/oasis7/**` 中的 `oasis7_wasm_executor` / `oasis7_wasm_store` 依赖、特性声明、路径与源码入口必须全部改为 `oasis7_wasm_executor` / `oasis7_wasm_store`，并将目录 `crates/oasis7_wasm_{executor,store}` 重命名为 `crates/oasis7_wasm_{executor,store}`；变更后 `cargo` 必须能按新 crate 名解析执行器和存储层。
  - AC-91: workspace 根 `Cargo.toml`、`Cargo.lock`、`crates/oasis7_proto/Cargo.toml`、`crates/{oasis7_distfs,oasis7_net,oasis7_consensus,oasis7_node,oasis7}/**` 与相关脚本中的 `oasis7_proto` 依赖、路径、源码入口和临时 stub 路径必须全部改为 `oasis7_proto`，并将目录 `crates/oasis7_proto` 重命名为 `crates/oasis7_proto`；变更后 `cargo` 必须能按新 crate 名解析协议层。
  - AC-92: workspace 根 `Cargo.toml`、`Cargo.lock`、`crates/oasis7_{distfs,consensus,net,node}/Cargo.toml`、`crates/oasis7/**` 与相关脚本中的 `oasis7_{distfs,consensus,net,node}` 依赖、路径、源码入口和 `cargo -p` 包名必须全部改为 `oasis7_{distfs,consensus,net,node}`，并将目录 `crates/oasis7_{distfs,consensus,net,node}` 重命名为 `crates/oasis7_{distfs,consensus,net,node}`；变更后 `cargo` 必须能按新 crate 名解析网络栈。
  - AC-93: workspace 根 `Cargo.toml`、`Cargo.lock`、`crates/oasis7_launcher_ui/Cargo.toml` 与依赖 `launcher_ui` 的 crate / 脚本中的 `oasis7_launcher_ui` 依赖、路径、源码入口和 `cargo -p` 包名必须全部改为 `oasis7_launcher_ui`，并将目录 `crates/oasis7_launcher_ui` 重命名为 `crates/oasis7_launcher_ui`。
  - AC-94: workspace 根 `Cargo.toml`、`Cargo.lock`、`crates/oasis7_client_launcher/Cargo.toml`、bundle/Trunk/launcher 相关脚本与下游依赖中的 `oasis7_client_launcher` 依赖、路径、源码入口和 `cargo -p` 包名必须全部改为 `oasis7_client_launcher`，并将目录 `crates/oasis7_client_launcher` 重命名为 `crates/oasis7_client_launcher`。
  - AC-95: workspace 根 `Cargo.toml`、`Cargo.lock`、`crates/oasis7_viewer/Cargo.toml`、launcher fallback、viewer 主题/抓帧/构建脚本与下游依赖中的 `oasis7_viewer` 依赖、路径、源码入口和 `cargo -p` 包名必须全部改为 `oasis7_viewer`，并将目录 `crates/oasis7_viewer` 重命名为 `crates/oasis7_viewer`。
  - AC-96: workspace 根 `Cargo.toml`、`Cargo.lock`、`crates/oasis7/Cargo.toml`、所有 workspace 下游 crate、测试、脚本与源码入口中的 `oasis7` 依赖、路径、`use oasis7::...` 与 `cargo -p oasis7` 包名必须全部改为 `oasis7`，并将目录 `crates/oasis7` 重命名为 `crates/oasis7`；变更后主 runtime/launcher/chain runtime 必须能按新包名编译。
  - AC-97: `crates/oasis7_builtin_wasm_modules/**`、各模块 `Cargo.toml` / `Cargo.lock`、builtin manifest map、模板与构建脚本中的 `oasis7_builtin_wasm_modules` / `oasis7_builtin_wasm_*` 路径和包名必须全部改为 `oasis7_builtin_wasm_modules` / `oasis7_builtin_wasm_*`，并将目录 `crates/oasis7_builtin_wasm_modules` 重命名为 `crates/oasis7_builtin_wasm_modules`。
  - AC-98: `testing-manual.md`、`scripts/ci-tests.sh`、`scripts/viewer-release-qa-loop.sh` 与 `crates/oasis7/src/viewer/{server.rs,runtime_live.rs,live_split_part2.rs}` 中作为当前默认说明/输出使用的 `oasis7` 字符串必须全部改为 `oasis7`；`crates/oasis7_proto/src/viewer.rs` 中用于兼容测试的旧 payload 样例可保留，但其测试语义必须继续明确这是 compat payload，而非默认值。
  - AC-99: `README.md`、`site/{index.html,en/index.html}`、`doc/world-simulator/viewer/viewer-manual.md`、`site/doc/{cn,en}/viewer-manual.html`、`oasis7_viewer_live.release.example.toml` 与 `tools/scenario_test_runner/**` 中作为当前入口使用的 `oasis7*` crate/path/command/env/path 说明必须全部改为 `oasis7*` / `OASIS7_VIEWER_*` / `.oasis7_viewer`；其中 `tools/scenario_test_runner` 必须恢复到可解析 `../../crates/oasis7` 的依赖路径并能通过 cargo 检查。
  - AC-100: `tools/wasm_build_suite/src/lib.rs` 中处理旧品牌 fallback 的 helper / 常量 / 测试命名必须改为 `compat_old_brand_*` 语义；变更后 `cargo test --manifest-path tools/wasm_build_suite/Cargo.toml` 必须通过。
  - AC-101: `crates/oasis7_viewer/{software_safe.js,src/viewer_env.rs,src/egui_right_panel_chat.rs,src/egui_right_panel_chat_auth.rs,src/viewer_automation.rs,src/perf_probe.rs,src/right_panel_module_visibility.rs}` 必须移除旧品牌 viewer fallback 读取；相应 compat 测试必须删除或改为断言旧 alias 已失效，且 `cargo test -p oasis7_viewer` 的定向用例与 `cargo check -p oasis7_viewer --target wasm32-unknown-unknown` 必须通过。
  - AC-102: `crates/oasis7_client_launcher/{src/main.rs,src/platform_ops.rs,src/launcher_core.rs,src/app_process.rs,src/self_guided.rs,src/llm_settings_web.rs}` 必须移除 `OASIS7_*` / `oasis7_*` fallback 读取；相应 compat 测试必须删除或改为断言旧 alias 已失效，且 `cargo test -p oasis7_client_launcher --bin oasis7_client_launcher` 与 `cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown` 必须通过。
  - AC-103: `crates/oasis7/src/bin/{oasis7_game_launcher.rs,oasis7_web_launcher/runtime_paths.rs,oasis7_web_launcher/control_plane.rs}` 必须移除 `OASIS7_*` launcher/runtime fallback 读取；`oasis7_game_launcher` 只能注入 `__OASIS7_VIEWER_AUTH_ENV` 与 `OASIS7_VIEWER_*` auth key，`oasis7_web_launcher` 只能接受 `OASIS7_*` 的 launcher/runtime/static dir 覆盖 env，相应定向测试必须改为断言旧 alias 已失效，且 `cargo test -p oasis7 --bin oasis7_game_launcher`、`cargo test -p oasis7 --bin oasis7_web_launcher`、`./scripts/doc-governance-check.sh` 与 `git diff --check` 必须通过。
  - AC-104: `crates/oasis7/src/viewer/runtime_live/{llm_sidecar.rs,control_plane.rs,tests.rs}` 必须移除旧品牌 provider/chat echo fallback 读取；OpenClaw provider 配置与 chat echo 定向测试必须改为断言旧 alias 已失效，且 `cargo test -p oasis7 runtime_live::tests -- --nocapture`、`./scripts/doc-governance-check.sh` 与 `git diff --check` 必须通过。
  - AC-105: `crates/oasis7/src/{runtime/module_source_compiler.rs,runtime/builtin_wasm_materializer.rs,simulator/llm_agent.rs}` 与相关定向测试必须移除旧品牌 module source / builtin wasm / LLM fallback 读取，并把 compat 测试改为断言旧 alias 已失效；变更后至少 `cargo test -p oasis7 builtin_wasm_materializer -- --nocapture`、`cargo test -p oasis7 module_action_loop -- --nocapture`、`cargo test -p oasis7 llm_agent -- --nocapture`、`./scripts/doc-governance-check.sh` 与 `git diff --check` 必须通过。
  - AC-106: `scripts/{build-wasm-module.sh,ci-m1-wasm-summary.sh,sync-m1-builtin-wasm-artifacts.sh}` 必须移除旧品牌 wasm fallback 读取与旧品牌 operator 文案，使 wasm build/sync 脚本只认 `OASIS7_WASM_*` 当前入口；变更后 `bash -n` 校验这三支脚本、`./scripts/doc-governance-check.sh` 与 `git diff --check` 必须通过。
  - AC-107: `scripts/{capture-viewer-frame.sh,viewer-texture-inspector-lib.sh,viewer-texture-inspector.sh,build-game-launcher-bundle.sh}` 必须移除旧品牌 viewer/profile fallback 读取及相关旧品牌 operator 文案，使 Viewer 调试脚本与 bundle wrapper 只认 `OASIS7_VIEWER_*` / `OASIS7_CHAIN_STORAGE_PROFILE` 当前入口；变更后 `bash -n` 校验这四支脚本、`./scripts/doc-governance-check.sh` 与 `git diff --check` 必须通过。
  - AC-108: `tools/wasm_build_suite/src/lib.rs` 与相关测试必须移除旧品牌 wasm fallback 读取，并把 compat 测试改为断言旧 alias 已失效；变更后 `cargo test --manifest-path tools/wasm_build_suite/Cargo.toml`、`./scripts/doc-governance-check.sh` 与 `git diff --check` 必须通过。
  - AC-109: `crates/oasis7/src/bin/oasis7_openclaw_local_bridge.rs` 与相关测试必须移除 `oasis7_p0_low_freq_npc` compat alias，使 profile 校验、provider diagnostics 与 bridge 默认入口只认 `oasis7_p0_low_freq_npc` 当前 profile；变更后 `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_openclaw_local_bridge -- --nocapture`、`./scripts/doc-governance-check.sh` 与 `git diff --check` 必须通过。
  - AC-110: `crates/oasis7_viewer/src/{viewer_env.rs,perf_probe.rs}`、`crates/oasis7_client_launcher/src/self_guided.rs` 与其他源码内嵌负向测试中围绕旧品牌 alias 的 helper / fixture 命名必须改为 `removed_old_brand` 等中性语义，且旧品牌字面量仅保留为“输入已失效 alias”断言；变更后至少 `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher --bin oasis7_client_launcher -- --nocapture`、`./scripts/doc-governance-check.sh` 与 `git diff --check` 必须通过。
  - AC-111: `tools/wasm_build_suite/src/lib.rs`、`crates/oasis7/src/{viewer/runtime_live/tests.rs,runtime/module_source_compiler.rs,runtime/builtin_wasm_materializer.rs,simulator/llm_agent/tests_split_part1.rs,bin/oasis7_openclaw_local_bridge.rs}` 等源码内嵌负向测试中围绕旧品牌 alias 的 helper / fixture 命名必须改为 `removed_old_brand` 等中性语义，且旧品牌字面量仅保留为“输入已失效 alias”断言；变更后至少 `cargo test --manifest-path tools/wasm_build_suite/Cargo.toml`、`env -u RUSTC_WRAPPER cargo test -p oasis7 runtime_live::tests -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 builtin_wasm_materializer -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 llm_agent -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_openclaw_local_bridge -- --nocapture`、`./scripts/doc-governance-check.sh` 与 `git diff --check` 必须通过。
  - AC-112: `.agents/roles/{wasm_platform_engineer,viewer_engineer}.md`、`doc/core/project.md`、`doc/world-runtime/{prd.md,project.md}` 中作为当前 owner 说明、活跃 artifact/path/command 或模块主入口现行约束使用的 `oasis7*` / `OASIS7_*` 路径与 env 口径必须更新为 `oasis7*` / `OASIS7_*`；历史任务正文可保留原始记录，但主状态与活跃入口不得继续把旧品牌写成当前真值。变更后 `./scripts/doc-governance-check.sh`、`git diff --check`，以及 `rg -n 'oasis7_|OASIS7_' .agents/roles/wasm_platform_engineer.md .agents/roles/viewer_engineer.md doc/core/project.md doc/world-runtime/prd.md doc/world-runtime/project.md` 仅允许命中历史任务正文，不得命中当前 owner/path/env 说明。
  - AC-113: 自 2026-04-01 起，`world-simulator` 不再接受新的 3D scene/camera/render/material/light/post-process 可视化需求进入 active delivery；相关需求必须标记为 paused，当前用户交互分支仅作为暂存参考保留，直到 `producer_system_designer` 明确完成恢复门禁复核后才允许重新进入实施。
- Non-Goals:
  - 不在本 PRD 中详细列出每个 UI 像素级规范。
  - 不替代 world-runtime/p2p 的底层协议设计。
  - 不在主 PRD 中展开专题实现细节（转账细节迁移至分册）。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: LLM 回归脚本、agent-browser 闭环、场景矩阵测试、启动器集成测试。
- Evaluation Strategy: 以场景启动成功率、关键交互完成率、LLM 链路稳定性、闭环缺陷收敛时间评估。

## 4. Technical Specifications
- Architecture Overview: world-simulator 连接 runtime 与 viewer，负责把世界状态转化为可交互体验，并通过场景系统与启动器提供可复现实验环境。专题能力通过分册文档按域维护。
- Integration Points:
  - `doc/world-simulator/scenario/scenario-files.prd.md`
  - `doc/world-simulator/viewer/viewer-web-closure-testing-policy.prd.md`
  - `doc/ui_review_result/ui_review_list.md`
  - `doc/world-simulator/prd/acceptance/unified-checklist.md`
  - `doc/world-simulator/prd/acceptance/web-llm-evidence-template.md`
  - `doc/world-simulator/prd/acceptance/visual-review-score-card.md`
  - `doc/world-simulator/prd/quality/experience-trend-tracking.md`
  - `doc/world-simulator/prd/launcher/blockchain-transfer.md`
  - `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-ui-schema-share-2026-03-04.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-egui-web-unification-2026-03-04.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-web-wasm-time-compat-2026-03-04.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-web-required-config-gating-2026-03-04.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-web-transfer-closure-2026-03-06.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-transfer-product-grade-parity-2026-03-06.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-web-settings-feedback-parity-2026-03-06.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-panel-2026-03-07.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p0-2026-03-07.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p1-address-contract-assets-mempool-2026-03-08.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-web-console-gui-agent-interface-2026-03-08.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.prd.md`
  - `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04.prd.md`
  - `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.prd.md`
  - `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.prd.md`
  - `doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.prd.md`
  - `crates/oasis7_launcher_ui/src/lib.rs`
  - `crates/oasis7_client_launcher/src/main.rs`
  - `crates/oasis7_client_launcher/src/app_process.rs`
  - `crates/oasis7_client_launcher/src/app_process_web.rs`
  - `crates/oasis7_client_launcher/src/transfer_window.rs`
  - `crates/oasis7_client_launcher/src/launcher_core.rs`
  - `crates/oasis7_client_launcher/Cargo.toml`
  - `crates/oasis7/src/bin/oasis7_web_launcher.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/gui_agent_api.rs`
  - `crates/oasis7/src/bin/oasis7_viewer_live.rs`
  - `crates/oasis7/src/viewer/runtime_live.rs`
  - `crates/oasis7_client_launcher/index.html`
  - `scripts/build-game-launcher-bundle.sh`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 网络异常：链状态探针或转账提交失败时，返回结构化错误并在 UI 展示可诊断提示。
  - 接口超时：请求超时后不得阻塞主线程，状态回落 `unreachable` 并保留超时上下文。
  - 空数据：余额/场景列表为空时展示空态，不允许进入依赖数据的后续动作。
  - 权限不足：未启用链能力或配置不合法时，禁用转账入口并提示原因。
  - 并发冲突：同账户并发转账以 nonce 作为幂等与反重放边界，不满足递增规则即拒绝。
  - 数据异常：收到非预期响应结构时转为“失败且可重试”状态，并写入诊断日志。
  - 远端不可达回退：反馈提交流程在 `Connection refused` 时必须本地落盘，保证证据不丢失。
  - 渲染能力缺口：当色调映射依赖缺失时，viewer 必须避免进入粉紫回退屏，并保留可诊断日志用于回归。
  - 产物覆写冲突：当 bundle 目标二进制正在运行时，打包脚本必须通过“删除后复制”避免 `Text file busy` 并确保输出目录完整可启动。
  - 无 GUI 环境：桌面 GUI 不可用时需通过 Web 控制台操作启动器，且必须支持远程状态可见与错误可诊断。
  - wasm 时间兼容：浏览器运行路径不得使用不支持的平台时间实现，避免页面初始化阶段 panic 直接阻断闭环。
  - Web 必填误判：Web API 配置不含 native-only 字段时，必填校验必须按平台分流，防止误报阻断。
  - 控制面分离回归：native 若无法拉起本地 `oasis7_web_launcher`，必须回传可诊断错误且禁止误导性“运行中”状态。
  - Web 设置存储失败：浏览器禁用本地存储时，设置窗口需返回明确失败提示，不得 silent fail。
  - Web 反馈代理失败：`/api/chain/feedback` 不可达或上游拒绝时，前端需展示结构化错误并保留重试能力。
  - 转账最终态悬挂：`accepted` 长时间未进入 `confirmed/failed` 时需明确标记为 `pending/timeout`，避免“仅 action_id”误判成功。
  - 账户与 nonce 漂移：余额/nonce 辅助信息过期时需支持刷新并在提交前给出可诊断提示。
  - explorer 参数非法：`status/action_id` 查询参数不合法时必须返回结构化 `invalid_request`，并允许前端立即重试。
  - explorer P0 分页参数非法：`limit/cursor` 非法时必须返回结构化 `invalid_request`，并允许前端保留当前页状态重试。
  - explorer P1 查询参数非法：`account_id/contract_id/status` 非法时必须返回结构化 `invalid_request`，并允许前端保留当前视图重试。
  - explorer P1 能力边界：当前链未启用 NFT 资产时需返回 `nft_supported=false`，禁止返回误导性成功数据。
  - explorer 索引恢复失败：索引文件缺失/损坏时必须回退空索引并继续提供查询服务，不得阻断 runtime 启动。
  - runtime 映射覆盖不足：runtime `DomainEvent` 未全量映射时，需降级输出可诊断事件并保留序列一致性。
  - runtime llm 桥接缺口：LLM 决策动作若无 runtime 映射实现，需返回结构化拒绝并继续服务循环，禁止 panic/卡死。
  - required 基线失败下线漂移：临时下线必须限定白名单；若 ignore 数量超出已知 10 项需视为异常并阻断合入。
  - 启动器品牌 key 迁移：浏览器 localStorage、native UX 状态文件、环境变量覆盖与测试临时目录在改名前缀切换后，必须先读 `oasis7` 再回退旧 `OASIS7_*` / `oasis7*`；禁止一次性硬切导致现有配置、bundle 或 QA 脚本失效。
  - Viewer auth/bootstrap 迁移：`software_safe.js`、viewer wasm/native 鉴权、`oasis7_game_launcher` 注入脚本与右侧面板持久化路径在切到 `oasis7` 前缀后，必须仍能消费旧 `oasis7` 对象名、env 名和默认目录，避免旧 bundle、旧浏览器缓存和 operator 环境变量同时失效。
  - 服务端 launcher 路径 key 迁移：`oasis7_game_launcher` / `oasis7_web_launcher` 在切到 `OASIS7_*` 路径覆盖 env 后，必须继续接受旧 `OASIS7_*`，并同步更新 help / validation / error 文案；禁止出现“运行时兼容新旧 key、但诊断仍只提示旧 key”这种半迁移状态。
  - Viewer 运行时调参 key 迁移：`viewer_3d_config`、theme runtime、panel/headless 行为控制与 release profile 在切到 `OASIS7_VIEWER_*` 后，必须完全停止消费旧品牌 viewer key；禁止出现“主程序已迁新 key、但旧 preset / 脚本 / 无头回归仍被静默忽略”的半迁移状态。
- Non-Functional Requirements:
  - 性能目标:
    - NFR-1: 启动器链状态探针刷新周期 <= 1s，状态可见延迟 <= 2s。
    - NFR-2: 本地链路下转账提交 API `p95` 响应时间 <= 500ms。
  - 兼容性目标:
    - NFR-3: Launcher/Web 闭环流程在 Linux/macOS 开发环境可执行并产出一致证据结构。
    - NFR-8: Viewer 2D/3D 默认渲染在 Linux native + Web 环境可稳定启动，且不出现粉紫回退屏。
    - NFR-9: 复用同一 `--out-dir` 连续执行 `build-game-launcher-bundle.sh`（含“上一次 bundle 仍运行”场景）时，打包成功率需为 100%（`test_tier_required`）。
    - NFR-10: `oasis7_web_launcher` 在受信网络下可绑定 `0.0.0.0`，并在浏览器端稳定轮询状态接口（`p95 <= 200ms`，本地网络）。
    - NFR-11: `/api/ui/schema` 响应 `p95 <= 100ms`（本地网络），且 schema 新增字段不破坏既有渲染逻辑。
    - NFR-12: launcher wasm 静态资源由 `oasis7_web_launcher` 托管时，首屏可交互时间（本地网络）`p95 <= 2s`。
    - NFR-13: launcher wasm 在 headed 浏览器启动后 `console error` 不得包含 `time not implemented on this platform`，且不出现 `RuntimeError: unreachable`。
    - NFR-14: Web 必填校验分流后不得新增 native 校验退化，`launcher_bin`/`chain_runtime_bin` 在 native 仍为必填。
    - NFR-15: native 与 web 客户端状态刷新节奏一致（默认 1s），不得出现持续状态漂移（>2 个轮询周期）。
    - NFR-16: runtime live 模式下，`Step` 控制请求本地执行延迟 `p95 <= 100ms`（默认场景、无外部依赖）。
    - NFR-17: runtime live llm 模式下，prompt/chat 鉴权失败请求返回延迟 `p95 <= 100ms`（本地环境）。
    - NFR-18: runtime live action 映射不可覆盖项必须稳定返回结构化拒绝，且 `oasis7_viewer_live` runtime-only 启动路径在 required 回归中通过率为 100%。
    - NFR-19: Web 转账提交（控制面代理 + runtime 受理）本地链路 `p95 <= 500ms`，失败路径 `p95 <= 1s` 返回结构化结果。
    - NFR-20: Web 反馈提交（控制面代理 + runtime 受理）本地链路 `p95 <= 500ms`，失败路径 `p95 <= 1s` 返回结构化结果。
    - NFR-21: native/web 转账交互一致性差异为 0（字段、门控、状态机、文案、错误语义），由同一前端实现来源保证。
    - NFR-22: 转账提交后最终状态可见延迟 <= 2 个轮询周期（本地链路），历史面板最近 50 条查询 `p95 <= 300ms`。
    - NFR-23: 区块链浏览器查询（overview/transactions/transaction）本地链路 `p95 <= 500ms`，默认刷新周期 1s。
    - NFR-24: 区块链浏览器 P0 查询（blocks/block/txs/tx/search）在默认分页 `limit=50` 下本地链路 `p95 <= 500ms`，翻页响应 `p95 <= 700ms`。
    - NFR-25: `POST /api/gui-agent/action` 在本地链路下 `p95 <= 500ms`（查询类动作）与 `p95 <= 1s`（控制/提交类动作）。
    - NFR-26: runtime required 临时下线数量必须固定为 10 项，且执行 `env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required` 时不得再出现这 10 项失败。
  - 安全与隐私目标:
    - NFR-4: 日志与证据中不得输出私钥、口令、完整凭据；敏感字段需脱敏。
    - NFR-5: 转账请求必须经过 nonce anti-replay 与余额约束校验。
  - 数据规模目标:
    - NFR-6: 场景清单规模 200 条时，场景选择与启动流程仍可在可接受等待时间内完成（< 3s 首屏可操作）。
  - 可扩展性目标:
    - NFR-7: 新增 launcher 链上动作时，不破坏既有转账请求结构与验收模板字段。
- Security & Privacy:
  - Viewer/launcher 链路涉及配置与鉴权注入时必须最小暴露。
  - 调试接口需受限并可审计。
  - 专题安全规则在对应分册中维护（含转账安全约束）。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03-03): 固化 world-simulator 主 PRD 边界。
  - v1.1: 完成分册化治理与启动器链上转账专题建模。
  - v2.0: 建立体验质量趋势指标（启动、交互、性能、稳定性）。
- Technical Risks:
  - 风险-1: 前端体验迭代快导致行为回归频发。
  - 风险-2: LLM 外部依赖波动影响端到端稳定性。
  - 风险-3: 分册索引维护缺失导致需求追踪断链。

## 6. Validation & Decision Record
- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-001 | TASK-WORLD_SIMULATOR-001/002/010/011/018 | `test_tier_required` | 文档结构校验、统一验收清单覆盖检查、关键入口引用可达检查 | 场景系统基线、模块导航与入口一致性 |
| PRD-WORLD_SIMULATOR-002 | TASK-WORLD_SIMULATOR-002/003/012/013/018 | `test_tier_required` | agent-browser Web 闭环、反馈回退回归、链状态探针单测 | Viewer 交互、Launcher 状态提示、故障诊断可见性 |
| PRD-WORLD_SIMULATOR-003 | TASK-WORLD_SIMULATOR-003/004/010/018 | `test_tier_required` + `test_tier_full` | LLM 证据模板审查、趋势指标周报抽样、长稳回归 | LLM 链路稳定性、发布证据完整性 |
| PRD-WORLD_SIMULATOR-004 | TASK-WORLD_SIMULATOR-005/006/008/009/018 | `test_tier_required` | 转账 API 单测、启动器转账提交流程测试、闭环证据归档 | Launcher 转账入口、runtime API 契约 |
| PRD-WORLD_SIMULATOR-005 | TASK-WORLD_SIMULATOR-005/007/009/018 | `test_tier_required` + `test_tier_full` | runtime main token 转账语义测试（余额/nonce anti-replay）、多轮回归 | 账本一致性、反重放策略、发布前链路风险 |
| PRD-WORLD_SIMULATOR-006 | TASK-WORLD_SIMULATOR-014/015 | `test_tier_required` | 启动器链/游戏独立启动与反馈门控回归测试 | 启动链路可预测性与反馈可用性 |
| PRD-WORLD_SIMULATOR-007 | TASK-WORLD_SIMULATOR-016/017 | `test_tier_required` | 设置中心分区配置读写与生效验证 | 启动器配置可用性与一致性 |
| PRD-WORLD_SIMULATOR-008 | TASK-WORLD_SIMULATOR-019/020 | `test_tier_required` + `test_tier_full` | native 抓帧脚本复现/回归、`oasis7_viewer` 单测与构建检查 | Viewer 2D/3D 渲染稳定性、native 交互可用性 |
| PRD-WORLD_SIMULATOR-009 | TASK-WORLD_SIMULATOR-021/022 | `test_tier_required` | 启动 `run-game.sh` 占用二进制后重复执行 bundle 脚本，验证无 `Text file busy` 且新产物可启动 | 启动器发行打包稳定性、重复发布可靠性 |
| PRD-WORLD_SIMULATOR-010 | TASK-WORLD_SIMULATOR-023/024 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher` + 启动 `oasis7_web_launcher` 后通过 `/api/start`/`/api/stop`/`/api/state` 回归 + `bash -n scripts/build-game-launcher-bundle.sh` 校验打包入口脚本 | 无 GUI 服务器远程运维、launcher Web 控制能力 |
| PRD-WORLD_SIMULATOR-011 | TASK-WORLD_SIMULATOR-025/026 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo test -p oasis7_launcher_ui` + `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher`，验证 shared schema 定义、native/web 同源渲染与接口输出 | 启动器 UI 一致性、跨端配置项治理能力 |
| PRD-WORLD_SIMULATOR-012 | TASK-WORLD_SIMULATOR-027/028 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown` + `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher` + `bash -n scripts/build-game-launcher-bundle.sh`，验证同层 egui 复用、静态托管与打包入口 | 启动器 UI 统一维护能力、headless 运维体验一致性 |
| PRD-WORLD_SIMULATOR-013 | TASK-WORLD_SIMULATOR-029/030/097 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown` + headed agent-browser 打开 `oasis7_web_launcher` 控制台并校验 `console error` 无 `time not implemented` + 归档 screenshot/console/snapshot 证据 | 启动器 Web 端可用性、wasm 运行时兼容稳定性 |
| PRD-WORLD_SIMULATOR-014 | TASK-WORLD_SIMULATOR-031/032 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown` + headed agent-browser 打开 launcher web 并回归 `/api/start` `/api/stop`，验证 Web 端不再受 native-only 必填项阻断 | 启动器 Web 配置校验准确性、跨端校验边界一致性 |
| PRD-WORLD_SIMULATOR-015 | TASK-WORLD_SIMULATOR-033/034/035 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher` + `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown` + headed agent-browser 覆盖链/游戏独立启停 | 启动器 native/web 控制面一致性、链路维护成本与回归稳定性 |
| PRD-WORLD_SIMULATOR-016 | TASK-WORLD_SIMULATOR-036/037 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_viewer_live` + `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_viewer_live`，验证 runtime 驱动 live 链路与协议兼容适配 | viewer live runtime/simulator 双模式一致性与迁移风险可控 |
| PRD-WORLD_SIMULATOR-017 | TASK-WORLD_SIMULATOR-038/039 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_viewer_live` + `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_viewer_live`，验证 runtime llm/chat/prompt 控制链路打通与脚本模式边界错误码 | viewer live runtime llm/script 体验连续性、鉴权与桥接稳定性 |
| PRD-WORLD_SIMULATOR-018 | TASK-WORLD_SIMULATOR-040/041 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_viewer_live` + `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_viewer_live`，验证 action 映射覆盖扩展、等价回归与 runtime-only 启动分支收敛 | viewer live runtime 映射覆盖稳定性、旧分支移除风险与体验一致性 |
| PRD-WORLD_SIMULATOR-019 | TASK-WORLD_SIMULATOR-042/043/044/045 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_viewer_live` + `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_viewer_live`，验证真实 LLM 决策链路、100% 映射覆盖与硬失败语义 | runtime live LLM 行为真实性与观测完整性 |
| PRD-WORLD_SIMULATOR-020 | TASK-WORLD_SIMULATOR-046/047 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown`，验证 Web 转账代理与 wasm 提交流程 | 启动器 Web 转账闭环可用性与跨端语义一致性 |
| PRD-WORLD_SIMULATOR-021 | TASK-WORLD_SIMULATOR-048/049 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher` + `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown`，验证 Web 设置中心可用化与反馈代理提交闭环 | 启动器 Web 设置/反馈跨端一致性与功能可达性 |
| PRD-WORLD_SIMULATOR-022 | TASK-WORLD_SIMULATOR-050/051 | `test_tier_required` | `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown`，验证 native 遗留代码清理后行为稳定 | 启动器 native 维护面收敛与跨端行为稳定性 |
| PRD-WORLD_SIMULATOR-023 | TASK-WORLD_SIMULATOR-052/053 | `test_tier_required` + `test_tier_full` | `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required transfer_submit_api::tests:: -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_full transfer_submit_api::tests:: -- --nocapture`，验证账户/余额辅助、自动 nonce、最终状态与历史面板跨端一致 | 启动器转账产品化体验、跨端前端一致性与链路可观测性 |
| PRD-WORLD_SIMULATOR-024 | TASK-WORLD_SIMULATOR-054/055/056 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime transfer_submit_api::tests:: -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown`，验证 explorer RPC、控制面代理与启动器面板闭环 | 启动器区块链浏览器可用性、跨端一致性与发布前诊断效率 |
| PRD-WORLD_SIMULATOR-025 | TASK-WORLD_SIMULATOR-057/058/059 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime transfer_submit_api::tests:: -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown`，验证 explorer P0 API（blocks/block/txs/tx/search）、持久化索引与跨端分页搜索 UI | 启动器区块链浏览器公共主链视角 P0 能力、可观测性与跨端一致性 |
| PRD-WORLD_SIMULATOR-026 | TASK-WORLD_SIMULATOR-060/061/062 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime transfer_submit_api::tests:: -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown`，验证 explorer P1 API（address/contracts/contract/assets/mempool）与启动器四视图闭环 | 启动器区块链浏览器公共主链视角 P1 能力、可观测性与跨端一致性 |
| PRD-WORLD_SIMULATOR-027 | TASK-WORLD_SIMULATOR-063/064/065/066 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown` + agent-browser（桌面 + 390x844）采证，验证路径回退、禁用态提示、参数编码、stop no-op 语义、移动端可读性、favicon 噪声治理与启动阻断引导 | 启动器可用性稳定性、跨端体验一致性与运维可诊断性 |
| PRD-WORLD_SIMULATOR-028 | TASK-WORLD_SIMULATOR-067/068 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown` + `env -u RUSTC_WRAPPER cargo fmt --all`，验证浏览器面板视觉层级、状态可视化、筛选恢复与列表-详情协同交互 | 启动器区块链浏览器日常核查效率、跨端体验一致性 |
| PRD-WORLD_SIMULATOR-029 | TASK-WORLD_SIMULATOR-069/070/071 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown`，验证配置防回写、请求按域并发、反馈草稿保护、顶栏响应式与转账过滤清空 | 启动器高频交互稳定性、并发可用性与窄屏可读性 |
| PRD-WORLD_SIMULATOR-030 | TASK-WORLD_SIMULATOR-072/073/074/075/076/077/078/079/080/081/082/083/084/085/086/087/088/089/090 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown` + agent-browser（桌面 + 390x844）采证 + `wc -l crates/oasis7_client_launcher/src/main.rs crates/oasis7_client_launcher/src/explorer_window.rs`，验证首次引导、任务流、错误恢复、preflight、持续轻提示、术语解释、快捷入口、成功配置画像、演示模式、本地计数与超长文件治理 | 启动器新用户自引导闭环、失败恢复效率、跨端一致性与代码维护可持续性 |
| PRD-WORLD_SIMULATOR-031 | TASK-WORLD_SIMULATOR-091/092 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_web_launcher`，验证 GUI Agent 能力声明、统一动作执行与结构化响应稳定性 | Web Console 机器控制面、人工操作可替代性、既有控制面兼容性 |
| PRD-WORLD_SIMULATOR-032 | TASK-WORLD_SIMULATOR-093/094/094A | `test_tier_required` | `./scripts/doc-governance-check.sh` + 临时下线阶段的 required 回归 + 根因修复后的 `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features 'test_tier_required,wasmtime' runtime::tests::power_bootstrap:: -- --nocapture` 与 `runtime::tests::agent_default_modules:: -- --nocapture`，验证 10 项白名单可被回收且定向覆盖恢复 | runtime required 回归可用性、pre-commit 稳定性、测试资产可追溯性 |
| PRD-WORLD_SIMULATOR-033 | TASK-WORLD_SIMULATOR-095/096 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher oasis7_game_launcher_tests::build_oasis7_chain_runtime_args_includes_storage_profile -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher oasis7_web_launcher_tests::build_chain_runtime_args_includes_chain_overrides_when_on -- --nocapture` + `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_game_launcher --bin oasis7_web_launcher`，验证双启动器均显式传递 `--execution-world-dir` 并固定到 `output/chain-runtime/<node_id>/reward-runtime-execution-world` | 运行时产物路径可控性、源码目录洁净度、launcher 对 runtime 参数透传稳定性 |
| PRD-WORLD_SIMULATOR-034 | TASK-WORLD_SIMULATOR-103/104/107/108/284 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher -- --nocapture` + GUI Agent 闭环（默认 node id stale 失败 + fresh node id 恢复 + explorer overview 查询成功） + `./scripts/run-game-test.sh --with-llm` / agent-browser Web 闭环（fresh `chain_node_id` 默认值） | launcher 链启动恢复体验、GUI Agent 契约、chain-enabled 试玩可达性、一键试玩栈稳定性与 formal gameplay 的 LLM-required 入口约束 |
| PRD-WORLD_SIMULATOR-035 | TASK-WORLD_SIMULATOR-105/106 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer` + `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer --target wasm32-unknown-unknown` + agent-browser Web 闭环（验证 `__AW_TEST__.getState().lastError` 能命中浏览器 fatal 并触发快失败） | Viewer Web 图形链路可诊断性、producer/QA 闭环失败透明度、手册与脚本口径一致性 |
| PRD-WORLD_SIMULATOR-039 | TASK-WORLD_SIMULATOR-139/140/141/142/143/158/301/302/303 | `test_tier_required` | `./scripts/doc-governance-check.sh` + Web agent-browser 闭环（验证默认/正式 Web 主路径由 `software_safe` 承接，并能完成 formal gameplay 主链路） + 定向 viewer/runtime 协议回归 + stale viewer dist freshness 回归 + 主入口文案/blocked-handoff 语义检查 | Web Viewer 无 GPU 硬件依赖主入口、低保真正式 Web gameplay、source-tree stale dist 门禁与 `#39` 的产品级收口路径 |
| PRD-WORLD_SIMULATOR-038 | TASK-WORLD_SIMULATOR-114/115/116/118/123/125/126/128/129/154 | `test_tier_required` / `test_tier_full` | `./scripts/doc-governance-check.sh` + fixture benchmark + 真实 builtin/OpenClaw `P0` 对照采证 + QA/producer 评分结论 | OpenClaw 与 builtin 体验等价门禁、`experimental` / 默认启用准入与后续扩面策略 |
| PRD-WORLD_SIMULATOR-040 | TASK-WORLD_SIMULATOR-148/149/150/151/152/153 | `test_tier_required` / `test_tier_full` | `./scripts/doc-governance-check.sh` + 双模式 contract review + `headless_agent`/`player_parity` 真实 smoke + QA/producer 对照采证 | OpenClaw 双轨模式、无 GUI 回归主链路、Viewer 旁路调试边界与默认模式策略 |
| PRD-WORLD_SIMULATOR-041 | TASK-WORLD_SIMULATOR-285/286 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `bash -n scripts/capture-viewer-frame.sh scripts/viewer-release-qa-loop.sh scripts/viewer-texture-inspector.sh scripts/viewer-theme-pack-preview.sh` + `rg -n "hold-only|暂停|暂存|恢复门禁|auto-focus-force-3d|默认保持 2D" doc/world-simulator/prd.md doc/world-simulator/project.md doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.prd.md doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.project.md doc/world-simulator/viewer/viewer-manual*.md doc/scripts/viewer-tools/capture-viewer-frame.prd.md scripts/capture-viewer-frame.sh scripts/viewer-release-qa-loop.sh scripts/viewer-texture-inspector.sh scripts/viewer-theme-pack-preview.sh` + `git diff --check` | Viewer 3D workstream 冻结、用户交互分支暂存治理、operator 脚本默认值与后续恢复门禁的可审计性 |

- Decision Log:

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-WS-001 | 采用 Web-first 作为默认 UI 闭环链路，native 抓帧仅 fallback | 以 native 图形链路为默认 | 与 `testing-manual.md` 的 S6 约束一致，能提升复现稳定性与自动化覆盖。 |
| DEC-WS-002 | 主 PRD 保持总览，专题细节下沉到 `doc/world-simulator/prd/*` 分册 | 所有条款继续堆叠在主 PRD | 主入口可读性和变更追踪效率更高，且满足单文档长度约束。 |
| DEC-WS-003 | 反馈分布式提交在 `Connection refused` 时强制本地回落并保留错误签名 | 远端失败直接报错终止，不落盘 | 可确保证据不丢失，便于回归和线上诊断；已被 `TASK-WORLD_SIMULATOR-012` 验证。 |
| DEC-WS-004 | 保留 `TonyMcMapFace` 默认色调映射并显式启用 `bevy/tonemapping_luts` 依赖 | 改默认 tonemapping 或运行时静默降级 | 保持既有视觉基线，同时消除 native 粉紫回退与不可交互回归；已由 `TASK-WORLD_SIMULATOR-020` 的 native 抓帧与 viewer 回归验证。 |
| DEC-WS-005 | `build-game-launcher-bundle.sh` 在 copy 前删除目标二进制，规避运行中覆盖 `Text file busy` | 保持直接 `cp` 覆写（遇占用即失败） | Linux 上删除运行中可执行文件不会中断现有进程，可确保同一路径重建 bundle 不进入半更新态；已由 `TASK-WORLD_SIMULATOR-022` 通过“运行中重复打包”回归验证。 |
| DEC-WS-006 | 新增 `oasis7_web_launcher` 作为 headless 场景控制平面 | 仅保留桌面 GUI / 仅依赖 CLI 手工操作 | headless 服务器无图形会话，Web 控制台可在浏览器统一操作并保留日志可观察性；由 `TASK-WORLD_SIMULATOR-024` 落地。 |
| DEC-WS-007 | 采用共享 launcher UI schema，由 native 与 web 双端适配渲染 | 继续维持 native/web 双端字段硬编码 | 单点维护字段与文案可显著降低配置漂移风险，且可保持 UI 行为一致性；由 `TASK-WORLD_SIMULATOR-026` 落地。 |
| DEC-WS-008 | 采用 `oasis7_client_launcher` 同一套 egui UI 跨 native/wasm 双目标，并由 `oasis7_web_launcher` 托管 launcher 静态资源 | 继续维护独立 HTML 控制台并与 native 并行演进 | 可彻底消除 UI 双栈分叉，降低维护与验收成本；由 `TASK-WORLD_SIMULATOR-028` 落地。 |
| DEC-WS-009 | launcher wasm 轮询计时切换到 Web 兼容时间实现，并将 `agent-browser --headed` 闭环作为回归门禁 | 接受已知 panic 并仅做文档告警 | 该问题会直接导致 Web UI 不可用，必须通过代码修复 + 自动化采证闭环防止回归；由 `TASK-WORLD_SIMULATOR-030` 落地。 |
| DEC-WS-010 | 启动器必填校验按平台分流（Web 排除 native-only binary 必填；native 保持阻断） | 在 Web 端注入伪二进制路径默认值 | 分流更符合字段语义边界，避免伪配置污染与误导；由 `TASK-WORLD_SIMULATOR-032` 落地。 |
| DEC-WS-011 | native 客户端改为“客户端 + 本地 oasis7_web_launcher 服务端”，与 web 客户端共用同一控制面 API | 继续维护 native 直连本地进程 + web API 双路径 | 单一控制面可保证行为一致并降低并行回归成本；由 `TASK-WORLD_SIMULATOR-035` 落地。 |
| DEC-WS-012 | viewer live 采用“runtime 驱动 + simulator 协议兼容适配”的 Phase 1 迁移 | 一次性替换 viewer 协议与前端模型 | 先切 runtime 主驱动可快速降低双轨风险，同时控制改动面与回归成本；由 `TASK-WORLD_SIMULATOR-037` 落地。 |
| DEC-WS-013 | viewer live runtime Phase 2 采用“LLM sidecar + prompt/chat/auth + 动作桥接子集”渐进方案 | 等待 runtime action 全量 1:1 映射后再开放控制面 | 先打通 runtime 的 LLM/chat/prompt 体验可立即消除双套控制断裂，并将动作映射风险限制在可诊断范围内；由 `TASK-WORLD_SIMULATOR-039` 落地。 |
| DEC-WS-014 | viewer live runtime Phase 3 采用“补齐高频动作映射 + 等价回归 + runtime-only 启动”方案 | 保留 simulator fallback 分支与部分映射缺口 | fallback 分支会持续制造双轨行为与回归成本，统一 runtime-only 并补齐映射可把风险收敛到单一可诊断链路；由 `TASK-WORLD_SIMULATOR-041` 落地。 |
| DEC-WS-015 | 在不改变 native 功能语义前提下，优先清理“无读写路径 + 无编译入口引用”的启动器遗留代码 | 维持遗留字段/旧测试文件并通过忽略告警维持现状 | 迁移到统一控制面后遗留代码会持续制造维护歧义；结构化清理可降低后续需求迭代成本；由 `TASK-WORLD_SIMULATOR-051` 落地。 |
| DEC-WS-016 | 启动器转账升级为产品级能力并强制 native/web 同层前端复用（账户选择/余额辅助/自动 nonce/最终状态与历史可视化） | 继续维护 native/web 分叉转账实现，仅补局部按钮或文案 | 分叉实现已导致门控与反馈语义漂移；统一前端与状态机可一次性收敛体验差异并降低长期维护成本；对应 `TASK-WORLD_SIMULATOR-052/053`。 |
| DEC-WS-017 | 先补齐 explorer RPC 与控制面代理，再接区块链浏览器 UI 面板 | 先做 UI 再倒推接口 | RPC 语义先冻结可避免 UI 返工并降低跨端漂移风险；对应 `TASK-WORLD_SIMULATOR-054/055/056`。 |
| DEC-WS-018 | 使用统一 explorer store 单点消费 committed batches，并扩展为持久化索引（blocks/txs/search）供旧/新查询接口共享 | 多个查询模块分别 drain committed batches | committed batches 为单消费语义，多点消费会导致索引漂移；统一状态源可保证查询一致性并降低维护复杂度；对应 `TASK-WORLD_SIMULATOR-058/059`。 |
| DEC-WS-019 | 以“默认可用 + 可解释失败 + 跨端一致”一次性硬化启动器可用性基线（路径回退、禁用态原因、参数编码、stop no-op 语义、移动端可读性、favicon） | 仅修复单点缺陷，保留其余体验债务 | 该批问题会叠加放大运维诊断成本与用户失败感知；合并治理能在一次回归中收敛可用性风险；对应 `TASK-WORLD_SIMULATOR-063/064`。 |
| DEC-WS-020 | 启动器采用“默认自引导 + 专家模式可切换 + 本地可复盘计数”策略 | 保持纯按钮面板 + 外链文档说明 | 新用户首会话需在产品内闭环完成关键任务，且要兼顾专家用户效率与后续迭代可观测性；对应 `TASK-WORLD_SIMULATOR-072~084`。 |
| DEC-WS-021 | 在 `oasis7_web_launcher` 增加 `/api/gui-agent/*` 统一机器接口并复用既有控制面能力 | 要求 GUI Agent 直接拼接分散 `/api/*` 旧路由 | 单入口 + 统一响应结构可显著降低自动化复杂度，并保持与人工功能的一致映射；对应 `TASK-WORLD_SIMULATOR-091/092`。 |
| DEC-WS-022 | 对 runtime required 已知 10 项失败测试执行函数级 `#[ignore]` 临时下线，并保留恢复追踪 | 模块级屏蔽整组测试或删除失败测试 | 函数级白名单可最小化影响面，保证 required 链路恢复执行的同时保留测试资产与回收路径；对应 `TASK-WORLD_SIMULATOR-093/094`。 |
| DEC-WS-023 | 将默认 node id 命中 stale execution world 视为 launcher 级可恢复错误，并优先提供 fresh node id 恢复 | 保持底层日志直出，由用户手工改 node id 或删目录 | 默认试玩/QA 路径会高频复用默认 node id；若不提升为产品级恢复问题，链模式体验难以稳定复跑。 |
| DEC-WS-024 | 暂停所有新的 3D 可视化推进，并把当前用户交互分支冻结为暂存参考 | 继续并行推进 3D 视觉专题，或直接删除当前用户交互分支实验资产 | 当前阶段最稀缺的是把 formal gameplay、`software_safe` 与非 3D 交互主链路压实；直接删除会丢失未来恢复上下文，继续并行推进则会稀释交付资源。 |
