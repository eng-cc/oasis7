# world-simulator PRD Project（审计轮次: 7；任务拆解含 PRD-ID 映射）

## 任务拆解（活跃面）
- [ ] simulator-kernel-persistence-state-hardening (PRD-WORLD_SIMULATOR-002/003) [test_tier_required]: 收口 `WorldKernel` snapshot/replay 的可恢复状态边界，补齐 `intel_ttl_ticks` 持久化与 legacy fallback 回归，并把进程内 cache/hook 重置语义显式固定在 persistence contract 内，避免恢复后行为配置静默漂移。 Trace: .pm/tasks/task_6780f8bf31a042dea2c929673ef8db40.yaml
- [ ] software-safe-playability-unblock (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 让 `software_safe` formal summary 将 canonical `available_actions` 重新暴露为可执行入口，并在 gameplay summary 与空实体快照并存时显式标记 `runtime_snapshot_empty_entities` blocker。 Trace: .pm/tasks/task_1c5ac527bed54e969b737137fc998ab8.yaml
  - 当前命名注记：仓库现行 canonical Web/UI 名称已收口为 `viewer`，该旧任务 slug / title 中的 `software_safe` 仅作为 compat / historical tracker 保留，不再代表当前 formal mode taxonomy。
  - gameplay 回指：`available_actions` 暴露不是单纯 UI/debug 字段问题；它归属 PRD-GAME-004 micro-loop 下一步动机与 blocker 可读性，见 `doc/game/gameplay/gameplay-top-level-design.prd.md`、`doc/product/world-rules-core-gameplay/first-session-and-continuation.prd.md` 与 GitHub #2166。
  - gameplay 回指：工业资源流转的 `ScheduleRecipe`、`TransferMaterial`、`ProductValidated`、电力恢复/售电、维护、市场与 data-access 可读性 quote 仍是 GitHub #2166 未收口债务；现行专业入口为 `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`，玩家合同为 `doc/game/gameplay/gameplay-top-level-design.prd.md`。这些条目不代表 quote、ABI、Viewer 或 release readiness 已实现。
  - gameplay 回指：scenario `FragmentsReplenished` / 运行期 frag 补种需要 `resource_replenishment_quote` / `fragment_refill_preview` 解释下一次补种 tick、预计补种量、等待成本、第一工业目标关联，以及等待、换 frag/chunk 或切材料路线的推荐，属于 PRD-GAME-012 缺料恢复 / 首条稳定产线可读性，现行 scenario 权威见 `doc/world-simulator/scenario/chunked-fragment-generation.prd.md`，产品选择边界见 `doc/product/world-rules-core-gameplay/first-session-and-continuation.prd.md`，追踪见 GitHub #2166。
  - gameplay 回指：kernel `PublishSocialFact` / `ChallengeSocialFact` / `DeclareSocialEdge` 需要 `social_fact_impact_quote` / `relationship_consequence_preview` 解释影响对象、可见社交表面、合作机会变化、黑名单/争议风险和治理/claim 关联，属于 PRD-GAME-014 社交策略动作后果可读性，见 social fact ledger PRD/design 与 GitHub #2166。
  - gameplay 回指：gameplay governance `OpenGovernanceProposal` / `CastGovernanceVote` 需要 `governance_vote_quote` / `proposal_outcome_preview` 解释剩余时间、quorum/pass 缺口、玩家票权影响、通过后世界变化和失败/冷却代价，属于 PRD-GAME-014 治理动作后果可读性，见 gameplay war-politics baseline、layer lifecycle closure PRD/design 与 GitHub #2166。
  - gameplay 回指：gameplay war `DeclareWar` 需要 `war_declaration_quote` / `conflict_outcome_preview` 解释宣战胜算、推荐强度、冲突窗口占用、结算风险和谈判/补强/等待等替代行动，属于 PRD-GAME-014 战争策略动作后果可读性，见 gameplay war-politics baseline、war/governance/crisis/meta closure PRD/design 与 GitHub #2166。
  - gameplay 回指：M4 高负载工厂折旧需要维护 runway、停机/critical 临界点和推荐维护动作，属于 PRD-GAME-012 首条稳定产线可读性，见 M4 维护压力 / playability PRD 与 GitHub #2166。
  - gameplay 回指：M4 `market_quotes` 需要 `market_quote_decision_preview` 解释本地采购 vs 外部调运、税费/运输成本主因和下一步降本动作，属于 PRD-GAME-012 工业经济可读性，见 M4 P2 market/governance PRD 与 GitHub #2166。
  - gameplay 回指：`RefineCompound` 需要 `refine_quote` / `refine_preview` 解释电力机会成本、hardware 产出和第一工业目标缺口变化，属于 PRD-GAME-012 首个工厂/制成品可读性，见 chunked fragment / LLM 工业采矿 / LLM 工厂策略 PRD 与 GitHub #2166。
  - gameplay 回指：首局推荐 starter frag 需要材质预期、可达性理由和第一工业目标关联，属于 PRD-GAME-012 / first industrial goal readability，见 scenario/viewer 三份 PRD 与 GitHub #2166。

### 最近完成（保留一跳 Trace）
- [x] local-only-world-playtest-startup (PRD-WORLD_SIMULATOR-039/046) [test_tier_required]: 收敛本地大世界试玩脚本为 local-only 语义，修复 first Agent claim / starter OC pending 体验、链提交快照同步与 Viewer 本地世界措辞，并保留 testnet attach 为独立 runbook。 Trace: .pm/tasks/task_52ecb41a63a54808bad86bc9ffc77c15.yaml
- [x] viewer-retired-visualization-route-cleanup (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 清理活跃 Viewer/source/docs/site 中旧第二 Viewer 入口、native 启动口径与视觉专项工具链残留，保留历史 provenance 但不再作为当前入口或评审权威。 Trace: .pm/tasks/task_e6edcb09bd774941bdcbde32bb9ea007.yaml
- [x] chain-side-manifest-delta-runtime-readiness (PRD-WORLD_SIMULATOR-039/046) [test_tier_required]: 定义链侧资源 manifest/delta schema，接入 simulator/runtime snapshot、provider/testnet readiness 与本地 standalone submit 闭环。 Trace: .pm/tasks/task_a0e15f2d5d0547a3a13c93caab49b611.yaml
- [x] viewer-visual-hierarchy-polish (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 按 Image2 视觉目标与总体/分模块设计优化 `software_safe` Viewer 首屏层级、command strip、action receipt、focus HUD 与移动 focus overlay，并用 UI/build/pixel-world visual smoke 验证收口。 Trace: .pm/tasks/task_e7760ad76a0046dfa5a17d0a5a89e59c.yaml
- [x] launcher-visual-comp-workflow-ui-optimization (PRD-WORLD_SIMULATOR-039/046) [test_tier_required]: 用 Image2 目标图、真实 native 截图对比和专业角色 review 收敛 launcher 首屏、弹窗/重型窗口视觉系统，并把 visual companion 方法论边界写回 workflow/skill/governance 文档。 Trace: .pm/tasks/task_54647d0add024a98b801d3736700ff22.yaml
- [x] module-project-log-slimming (PRD-ENGINEERING-030) [test_tier_required]: 压缩 world-simulator 主项目页历史流水为当前/最近任务索引与历史追溯入口，保留未完成任务、模块状态和一跳 task trace。 Trace: .pm/tasks/task_49ef9270afc646d98d4a8386c0888eab.yaml
- [x] viewer-layout-spacing-polish (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 优化 `software_safe` Viewer 首屏布局层级、移动端快捷入口、fallback diagnostic 顺序与沉浸模式移动端间距。 Trace: .pm/tasks/task_0c3cddc969d24f48b0575be3d7aa87f7.yaml
- [x] viewer-pixel-world-semantic-positioning (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 增加 `location_derived` 确定性语义定位、关系线恢复与 fallback DOM world-coordinate placement。 Trace: .pm/tasks/task_4ade083740bc4d9f9f9bb742a7ce153f.yaml
- [x] viewer-anchor-reference-render-boundary (PRD-WORLD_SIMULATOR-039/046) [test_tier_required]: 移除突兀 Anchor 可见 DOM 浮层，保留 screen-reader/fallback reference 文本。 Trace: .pm/tasks/task_6ce7f0ddb880400fab57c70624669ea3.yaml
- [x] viewer-pixel-world-action-receipt-surface (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 为 pixel-world `commercial_surface` 增加玩家行动回执。 Trace: .pm/tasks/task_cc47f34ea897420cb20a44c7a77c5424.yaml
- [x] viewer-pixel-world-commercial-rendering-loop (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 将 pixel-world 主舞台收口为商业化游戏棋盘并默认折叠 renderer diagnostics。 Trace: .pm/tasks/task_b399bf37eff94c44a300c55f5db739d3.yaml
- [x] launcher-open-game-page-url-fix (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 对齐 launcher 打开游戏页按钮与控制面 `game_url` 的 `render_mode=software_safe`。 Trace: .pm/tasks/task_241f25085f754d868313462b879e4d01.yaml
- [x] launcher-rust-governance-provider-contract (PRD-WORLD_SIMULATOR-037/040/043) [test_tier_required]: 收口 launcher Rust governance review 的四项优化：web launcher agent-provider schema/config/args contract、provider-backed validation/transport policy、`trusted_local_only` internal local-playtest wording、shared HTTP base URL parser coverage。 Trace: .pm/tasks/task_169255fb26a2410a9c9edfaa839fc466.yaml

### 历史压缩索引
- 初始 world-simulator PRD/schema/acceptance、launcher 转账、Web 控制台、shared UI schema、Web wasm 与 native/web control plane 历史：回看 `doc/world-simulator/prd.index.md`、相关 topic project 与 GitHub task issue evidence comments。
- 早期 launcher 链上转账基础条款 `PRD-WORLD_SIMULATOR-004/005` 已从旧 singleton 分册退役，当前追溯通过 `doc/world-simulator/prd.index.md`、`doc/world-simulator/launcher/game-client-launcher-web-transfer-closure-2026-03-06.prd.md` 与 `doc/world-simulator/launcher/game-client-launcher-transfer-product-grade-parity-2026-03-06.prd.md` 进入。
- Viewer live/runtime-world、LLM/provider、software_safe、pixel-world 与 visual cleanup 历史：回看 `doc/world-simulator/viewer/`、`doc/world-simulator/llm/`、`doc/testing/evidence/` 与对应 task trace。
- Release distribution、platform native entrypoints、Windows installer、Linux AppImage 与 upgrade policy 历史：回看 `doc/world-simulator/launcher/`、`doc/site/github-pages/` 与对应 task trace。
- 本主项目页只维护当前/最近任务索引；完整执行证据以 topic project、testing evidence 与 GitHub task issue evidence comments 为准。

## 依赖
- 模块设计总览：`doc/world-simulator/design.md`
- doc/world-simulator/prd.index.md
- `doc/world-simulator/scenario/scenario-files.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.prd.md`
- `doc/world-simulator/prd/acceptance/unified-checklist.md`
- `doc/world-simulator/prd/acceptance/web-llm-evidence-template.md`
- `doc/world-simulator/prd/quality/experience-trend-tracking.md`
- `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-web-wasm-time-compat-2026-03-04.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-web-required-config-gating-2026-03-04.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-web-transfer-closure-2026-03-06.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-web-settings-feedback-parity-2026-03-06.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-transfer-product-grade-parity-2026-03-06.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.project.md`
- `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.{prd,project}.md`
- `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.{prd,project}.md`
- `doc/world-simulator/llm/decision-provider-contract.prd.md`
- `doc/world-simulator/llm/decision-provider-contract.project.md`
- `doc/world-simulator/llm/provider-loopback-http-contract.prd.md`
- `doc/world-simulator/llm/provider-loopback-http-contract.project.md`
- `doc/world-simulator/llm/provider-agent-experience-parity.prd.md`
- `doc/world-simulator/llm/provider-agent-experience-parity.project.md`
- `doc/world-simulator/llm/{provider-agent-dual-mode.{prd,project}.md,provider-agent-dual-mode-contract.md}`
- `doc/world-simulator/llm/provider-agent-profile-oasis7_p0_low_freq_npc-2026-03-13.md`
- Runtime live migration phase1/2/3 旧三件套已退役删除；当前追溯入口收敛到 `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.prd.md`、`doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27.prd.md`、`doc/world-simulator/viewer/viewer-manual.manual.md`、GitHub task issue evidence comments 与 git history。
- `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-feedback.prd.md`、`skills/prd/check.md`
- `crates/oasis7/src/bin/{oasis7_chain_runtime.rs,oasis7_game_launcher.rs,oasis7_web_launcher.rs}`、`crates/oasis7/src/bin/oasis7_web_launcher/gui_agent_api.rs`、`crates/oasis7/src/bin/oasis7_chain_runtime/{transfer_submit_api.rs,transfer_submit_api_tests.rs}`
- `crates/oasis7_launcher_ui/src/lib.rs`
- `crates/oasis7_client_launcher/src/{main.rs,main_app_shell.rs,app_process.rs,app_process_web.rs,explorer_window.rs,explorer_window_view.rs}`
- `crates/oasis7/src/runtime/world/event_processing/action_to_event_core.rs`、`crates/oasis7/src/runtime/tests/{agent_default_modules.rs,power_bootstrap.rs}`、`scripts/build-game-launcher-bundle.sh`、`testing-manual.md`
## 状态
- 更新日期: 2026-05-26
- 当前状态: active
- 下一任务: 待下一个模块任务明确。
- 当前优先任务: 回到模块后续排队项；当前无新 blocker。
- 当前窗口摘要: launcher “打开游戏页”URL、launcher explorer 当前 authority、`/api/state.chain_replication_status` 透传与节点观测摘要卡均已收口；explorer 只记录既有只读查询与七视图，不能外推为 mainnet/readiness/public/settlement/validator/no-reset/full-archive 结论，详情回看对应 task trace。
- 当前 launcher feedback authority: `doc/world-simulator/launcher/game-client-launcher-feedback.{prd,design,project}.md` 收敛当前 native Ready 后远端提交失败时的本地回落与 Web 控制面代理边界；三个 2026-03 源三件套已删除，追溯仅使用 Git 与 `.pm` task evidence。
- 边界说明: 已知环境限制仍是 source stack formal 启动前需要 `OASIS7_LLM_MODEL` 或等价配置；旧第二 Viewer 入口相关代码、脚本与活跃文档已移除，当前仅保留 `viewer` canonical Web 主入口与 `software_safe` compat alias。
- 历史追溯: 最近完成项不再压缩在标题行中维护；需要追 launcher / viewer / provider-backed NPC / release distribution 历史时，先从上方任务项、topic project、`doc/world-simulator/prd.index.md` 与 GitHub task issue evidence comments 进入。
- 当前追溯入口: 活跃任务、最近完成项、topic project、`doc/world-simulator/prd.index.md` 与 GitHub task issue evidence comments；旧 2026-03-11 viewer 状态 closure / viewer-to-producer handoff 文档已退役删除，当前状态、活跃任务与下一步以本文档为准。
