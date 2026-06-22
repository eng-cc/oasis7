# world-simulator PRD Project（审计轮次: 7；任务拆解含 PRD-ID 映射）

## 任务拆解（活跃面）
- [ ] simulator-kernel-persistence-state-hardening (PRD-WORLD_SIMULATOR-002/003) [test_tier_required]: 收口 `WorldKernel` snapshot/replay 的可恢复状态边界，补齐 `intel_ttl_ticks` 持久化与 legacy fallback 回归，并把进程内 cache/hook 重置语义显式固定在 persistence contract 内，避免恢复后行为配置静默漂移。 Trace: .pm/tasks/task_6780f8bf31a042dea2c929673ef8db40.yaml
- [ ] software-safe-playability-unblock (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 让 `software_safe` formal summary 将 canonical `available_actions` 重新暴露为可执行入口，并在 gameplay summary 与空实体快照并存时显式标记 `runtime_snapshot_empty_entities` blocker。 Trace: .pm/tasks/task_1c5ac527bed54e969b737137fc998ab8.yaml

### 最近完成（保留一跳 Trace）
- [x] viewer-retired-visualization-route-cleanup (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 清理活跃 Viewer/source/docs/site 中旧 `standard_3d`、native Viewer 与旧 visual QA 路线残留，保留历史 provenance 但不再作为当前入口或评审权威。 Trace: .pm/tasks/task_e6edcb09bd774941bdcbde32bb9ea007.yaml
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
- 初始 world-simulator PRD/schema/acceptance、launcher 转账、Web 控制台、shared UI schema、Web wasm 与 native/web control plane 历史：回看 `doc/world-simulator/prd.index.md`、相关 topic project 与 `.pm/tasks/*.execution.md`。
- Viewer live/runtime-world、LLM/provider、software_safe、pixel-world 与 visual cleanup 历史：回看 `doc/world-simulator/viewer/`、`doc/world-simulator/llm/`、`doc/testing/evidence/` 与对应 task trace。
- Release distribution、platform native entrypoints、Windows installer、Linux AppImage 与 upgrade policy 历史：回看 `doc/world-simulator/launcher/`、`doc/site/github-pages/` 与对应 task trace。
- 本主项目页只维护当前/最近任务索引；完整执行证据以 topic project、testing evidence 与 `.pm/tasks/*.execution.md` 为准。

## 依赖
- 模块设计总览：`doc/world-simulator/design.md`
- doc/world-simulator/prd.index.md
- `doc/world-simulator/scenario/scenario-files.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.prd.md`
- `doc/world-simulator/prd/acceptance/unified-checklist.md`
- `doc/world-simulator/prd/acceptance/web-llm-evidence-template.md`
- `doc/world-simulator/prd/quality/experience-trend-tracking.md`
- `doc/world-simulator/prd/launcher/blockchain-transfer.md`
- `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-ui-schema-share-2026-03-04.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-egui-web-unification-2026-03-04.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-web-wasm-time-compat-2026-03-04.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-web-required-config-gating-2026-03-04.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-web-transfer-closure-2026-03-06.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-web-settings-feedback-parity-2026-03-06.prd.md`、`doc/world-simulator/launcher/game-client-launcher-native-legacy-cleanup-2026-03-06.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-transfer-product-grade-parity-2026-03-06.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-panel-2026-03-07.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p0-2026-03-07.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p1-address-contract-assets-mempool-2026-03-08.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08.project.md`
- `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-ui-ux-optimization-2026-03-08.project.md`
- `doc/world-simulator/launcher/game-client-launcher-full-usability-remediation-2026-03-08.project.md`
- `doc/world-simulator/launcher/game-client-launcher-self-guided-experience-2026-03-08.{prd,project}.md`、`doc/world-simulator/launcher/game-client-launcher-web-console-gui-agent-interface-2026-03-08.{prd,project}.md`
- `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.{prd,project}.md`
- `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.{prd,project}.md`
- `doc/world-simulator/llm/llm-decision-provider-standard-loopback-provider-feasibility-2026-03-12.prd.md`
- `doc/world-simulator/llm/llm-decision-provider-standard-loopback-provider-feasibility-2026-03-12.project.md`
- `doc/world-simulator/llm/llm-provider-loopback-http-integration-2026-03-12.prd.md`
- `doc/world-simulator/llm/llm-provider-loopback-http-integration-2026-03-12.project.md`
- `doc/world-simulator/llm/llm-provider-agent-experience-parity-2026-03-12.prd.md`
- `doc/world-simulator/llm/llm-provider-agent-experience-parity-2026-03-12.project.md`
- `doc/world-simulator/llm/{llm-provider-agent-dual-mode-2026-03-16.{prd,project}.md,provider-agent-dual-mode-contract-2026-03-16.md}`
- `doc/world-simulator/llm/provider-agent-profile-oasis7_p0_low_freq_npc-2026-03-13.md`
- `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04.prd.md`、`doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.prd.md`、`doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-mainnet-grade-rebuild-2026-04-18.{prd,design,project}.md`
- `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-i18n-required-config-2026-03-02.prd.md`
- `doc/world-simulator/launcher/game-client-launcher-feedback-distributed-submit-2026-03-02.prd.md`、`.agents/skills/prd/check.md`
- `crates/oasis7/src/bin/{oasis7_chain_runtime.rs,oasis7_game_launcher.rs,oasis7_web_launcher.rs}`、`crates/oasis7/src/bin/oasis7_web_launcher/gui_agent_api.rs`、`crates/oasis7/src/bin/oasis7_chain_runtime/{transfer_submit_api.rs,transfer_submit_api_tests.rs}`
- `crates/oasis7_launcher_ui/src/lib.rs`
- `crates/oasis7_client_launcher/src/{main.rs,main_app_shell.rs,app_process.rs,app_process_web.rs,explorer_window.rs,explorer_window_view.rs}`
- `crates/oasis7/src/runtime/world/event_processing/action_to_event_core.rs`、`crates/oasis7/src/runtime/tests/{agent_default_modules.rs,power_bootstrap.rs}`、`scripts/build-game-launcher-bundle.sh`、`testing-manual.md`
## 状态
- 更新日期: 2026-05-26
- 当前状态: active
- 下一任务: 待下一个模块任务明确。
- 当前优先任务: 回到模块后续排队项；当前无新 blocker。
- 当前窗口摘要: launcher “打开游戏页”URL、launcher explorer 主链级重构、`/api/state.chain_replication_status` 透传与节点观测摘要卡均已收口，详情回看对应 task trace。
- 边界说明: 已知环境限制仍是 source stack formal 启动前需要 `OASIS7_LLM_MODEL` 或等价配置；旧第二 Viewer 入口相关代码、脚本与活跃文档已移除，当前仅保留 `viewer` canonical Web 主入口与 `software_safe` compat alias。
- 历史追溯: 最近完成项不再压缩在标题行中维护；需要追 launcher / viewer / provider-backed NPC / release distribution 历史时，先从上方任务项、topic project、`doc/world-simulator/prd.index.md` 与 `.pm/tasks/*.execution.md` 进入。
