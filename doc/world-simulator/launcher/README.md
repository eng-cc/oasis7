# `world-simulator/launcher` 热点子域入口

更新时间: 2026-06-20

## 从这里开始
- 想确认 launcher 当前面向玩家/开发者的整体可用性、分发与 release readiness：先读 `game-client-launcher-broad-user-release-distribution-2026-04-14.prd.md`
- 想确认本地 launcher/playtest 如何启动、哪些路径是当前 operator 入口：先读 `../README.md` 与 `../project.md`，再按具体问题进入本页专题簇
- 想确认 hosted login、可试玩本地栈、provider preflight 或 trusted-local 启动口径：先读 `../project.md` 当前状态，再按 `.pm` task trace 进入最新任务证据
- 想确认 Web / native control plane、console、settings、feedback、transfer 的兼容边界：先读 `game-client-launcher-native-web-control-plane-unification-2026-03-04.prd.md`
- 想确认 blockchain explorer、public chain、address / contract / assets / mempool 或 mainnet-grade rebuild：先读 `game-client-launcher-blockchain-explorer-mainnet-grade-rebuild-2026-04-18.prd.md`
- 想确认 launcher 和 chain runtime / execution world dir / stale world recovery 的边界：先读 `game-client-launcher-chain-runtime-decouple-2026-02-28.prd.md`，再按需读 `game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.prd.md`
- 想精确找某份 launcher 专题文档，而不是按问题阅读：回到 `../prd.index.md`

## 入口分工
- 当前页只承担 `launcher/` 子目录 landing page 职责，不复制完整长表。
- `../README.md` 是 world-simulator 模块级 landing page，负责跨 `viewer / launcher / llm / kernel / scenario / m4` 分流。
- `../project.md` 是模块执行台账，适合确认当前 launcher 相关活跃任务、阻断和 `.pm` trace。
- `../prd.index.md` 是 world-simulator 模块完整文件级索引，适合已知主题后按文件名查找。
- 本页只维护簇级入口；当某个专题退化为历史执行证据时，继续让它通过 `../prd.index.md` 可检索，而不是回到默认首读路径。

## 密度快照
- 当前 inventory 快照（`bash scripts/doc-inventory-report.sh`，2026-06-20）:
  - `doc/world-simulator/launcher/`: 87 份 Markdown
  - `doc/world-simulator/`: 472 份 Markdown
- 该子域已经超过热点阈值；本页目标是降低首读扫描成本，不在本批直接减少文件数。

## 首读主题簇

### 1. 发布、分发与整体可用性
- 首读入口: `game-client-launcher-broad-user-release-distribution-2026-04-14.prd.md`
- 适合问题:
  - launcher 是否已经能给更广泛用户使用
  - release/distribution 的当前阻断和验收边界是什么
  - broad-user distribution 与本地 playtest 入口如何区分

### 2. Web / native 控制面与设置
- 首读入口:
  - `game-client-launcher-native-web-control-plane-unification-2026-03-04.prd.md`
  - `game-client-launcher-web-console-2026-03-04.prd.md`
  - `game-client-launcher-web-settings-feedback-parity-2026-03-06.prd.md`
- 适合问题:
  - native 与 Web control plane 哪些能力要保持一致
  - Web console / settings / feedback 的当前 canonical 口径是什么
  - UI schema、required config 与 wasm time compat 相关问题从哪里下钻

### 3. Blockchain explorer 与链上可见性
- 首读入口:
  - `game-client-launcher-blockchain-explorer-mainnet-grade-rebuild-2026-04-18.prd.md`
  - `game-client-launcher-blockchain-explorer-public-chain-p0-2026-03-07.prd.md`
  - `game-client-launcher-blockchain-explorer-public-chain-p1-address-contract-assets-mempool-2026-03-08.prd.md`
- 适合问题:
  - explorer 当前是否按 mainnet-grade 口径组织
  - public chain、address、contract、assets、mempool 的入口在哪里
  - explorer UI / UX 优化和 panel 设计从哪里追溯

### 4. Runtime / execution world 边界
- 首读入口:
  - `game-client-launcher-chain-runtime-decouple-2026-02-28.prd.md`
  - `game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.prd.md`
  - `game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12.prd.md`
- 适合问题:
  - launcher 与 chain runtime 的职责边界在哪里
  - execution world dir 输出、恢复与 stale world 处理如何收口
  - 这类问题何时需要转给 runtime / QA 角色复核

### 5. Feedback、transfer 与自引导体验
- 首读入口:
  - `game-client-launcher-feedback-entry-2026-03-02.prd.md`
  - `game-client-launcher-transfer-product-grade-parity-2026-03-06.prd.md`
  - `game-client-launcher-self-guided-experience-2026-03-08.prd.md`
- 适合问题:
  - feedback entry/window/distributed submit 的职责怎么分
  - transfer 产品级 parity 的当前约束是什么
  - 自引导体验和 full usability remediation 应该从哪份专题开始

## 定向检索边界
- 如果你已经知道准确文件名，直接回 `../prd.index.md`。
- 如果问题涉及当前本地 launcher/playtest 操作口径，优先看 `../project.md` 当前状态和 `.pm` task trace，再决定是否进入某个历史专题。
- 如果问题需要判断 runtime 正确性、hosted auth、chain behavior 或 release readiness，本页只提供文档入口，结论必须回到对应专业角色和当前任务证据。
- 历史完成的 launcher 专题继续保留可检索性；除非它仍是当前 operator 入口，不再提升为默认首读路径。

## 维护约定
- 新增 launcher 专题后，若改变默认首读路径，应同步更新本页。
- 改变本地启动、hosted login、provider preflight、chain runtime 或 release/distribution 口径时，必须同步评估是否更新 `../project.md` 当前状态。
- 本页只维护簇级入口，不维护完整文件清单。
