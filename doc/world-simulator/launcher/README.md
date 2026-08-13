# `world-simulator/launcher` 热点子域入口

更新时间: 2026-07-24

## 从这里开始
- 想确认普通用户下载、安装和升级体验边界：先读 `../../product/player-entry-distribution/prd.md`；再读 `../prd.md`、`../../site/prd.md` 与 `../../site/GitHub task issue evidence comments` 确认 Pages、Release 资产、专业实现和 blocker
- 想确认本地 launcher/playtest 如何启动、哪些路径是当前 operator 入口：先读 `../README.md`、本页专题和对应操作手册；活跃任务状态读 GitHub task issue evidence
- 想确认 hosted login、可试玩本地栈、provider preflight 或 trusted-local 启动口径：先读本页相关 PRD/design/manual，再按 GitHub task issue evidence comments 与 `.pm/github-project-sync` mapping/archive 进入最新任务证据
- 想确认反馈入口、native Ready 后远端失败的本地回落与 Web 代理边界：先读 `game-client-launcher-feedback.prd.md`
- 想确认 native/Web 配置、设置、反馈与转账的共享语义、平台差异、结果和恢复边界：先读 `game-client-launcher-cross-surface-action-parity.prd.md`；control plane、GUI-agent 机器接口仍读 `game-client-launcher-control-plane-and-machine-interface.prd.md`
- 想确认 blockchain explorer 的当前只读查询、七视图与状态呈现边界：先读 `game-client-launcher-blockchain-explorer.prd.md`；它不构成 mainnet、readiness、公开服务、结算或 validator 承诺
- 想确认 launcher 的语言/配置清晰度、blocked/observer 状态与响应式自引导边界：先读 `game-client-launcher-guided-configuration-and-usability.prd.md`；它不构成玩家入口、network readiness 或发布承诺
- 想确认 launcher 与 chain runtime、execution world、stale session/recovery 或 browser/WASM runtime compatibility 的边界：先读 `game-client-launcher-runtime-session-continuity.prd.md`；native/Web 控制请求、结果和诊断表现再读 control-plane authority
- 想精确找某份 launcher 专题文档，而不是按问题阅读：回到 `../prd.index.md`

## 入口分工
- 当前页只承担 `launcher/` 子目录 landing page 职责，不复制完整长表。
- `../README.md` 是 world-simulator 模块级 landing page，负责跨 `viewer / launcher / llm / kernel / scenario / m4` 分流。
- GitHub task issue evidence 是 launcher 活跃任务、阻断和验证状态的唯一 mutable truth；`.pm/github-project-sync` 只作为 task_uid 到 issue/project item 的本地 mapping/archive 辅助。
- Viewer 本地依赖由各仓库自有 build/test wrapper 在实际执行前按需检查；缺失依赖时只执行一次 `npm --prefix crates/oasis7_viewer ci`，随后重新校验。依赖已就绪时不安装；安装失败会立即报错。CI 仍由显式 job 负责依赖准备，不在 CI 命令中隐藏安装。
- `../prd.index.md` 是 world-simulator 模块完整文件级索引，适合已知主题后按文件名查找。
- 本页只维护簇级入口；当某个专题退化为历史执行证据时，继续让它通过 `../prd.index.md` 可检索，而不是回到默认首读路径。

## 密度快照
- 当前 inventory 快照（2026-07-24，本次删除完成后）:
  - `doc/world-simulator/launcher/`: 55 份 Markdown
  - `doc/world-simulator/`: 356 份 Markdown
- 该子域仍超过热点阈值；本页目标是降低首读扫描成本，并持续退役不再承担当前入口职责的历史专题。

## 首读主题簇

### 1. 发布、分发与整体可用性
- 首读入口: `../../product/player-entry-distribution/prd.md`（玩家体验）；`../prd.md`、`../../site/prd.md` 与 `../../site/GitHub task issue evidence comments`（Pages、Release 资产和专业实现）
- 适合问题:
  - launcher 是否已经能给更广泛用户使用
  - release/distribution 的当前阻断和验收边界是什么
  - broad-user distribution 与本地 playtest 入口如何区分

### 2. Web / native 控制面与设置
- 首读入口:
  - `game-client-launcher-control-plane-and-machine-interface.prd.md`
  - `game-client-launcher-cross-surface-action-parity.prd.md`
- 适合问题:
  - native 与 Web control plane 哪些能力要保持一致
  - Web console / settings / feedback 的当前 canonical 口径是什么
  - UI schema、required config 与 wasm time compat 相关问题从哪里下钻：控制表现读本簇两份 stable authority；runtime/session/WASM mechanics 转至下一簇的 successor

### 3. Blockchain explorer 与链上可见性
- 首读入口:
  - `game-client-launcher-blockchain-explorer.prd.md`
- 适合问题:
  - explorer 当前的概览、Blocks/Txs/Search/Address/Contracts/Assets/Mempool 入口和状态语义
  - native/Web 只读控制面、错误态或数据保留边界在哪里
  - 历史 public-chain / mainnet-grade 命名如何追溯，而不外推成发布或网络结论

### 4. Runtime / execution world 边界
- 首读入口:
  - `game-client-launcher-runtime-session-continuity.prd.md`
- 适合问题:
  - launcher 与 chain runtime 的职责边界在哪里
  - execution world 输出、session continuity、恢复与 stale world 处理如何收口
  - browser/WASM runtime compatibility、轮询或 lifecycle 细节应由哪个专业 authority 复核
  - 这类问题何时需要转给 runtime / QA 角色复核

### 5. 引导配置、feedback、transfer 与自引导体验
- 首读入口:
  - `game-client-launcher-guided-configuration-and-usability.prd.md`
  - `game-client-launcher-feedback.prd.md`
  - `game-client-launcher-cross-surface-action-parity.prd.md`
- 适合问题:
  - feedback entry/window/distributed submit 的职责怎么分
  - transfer 产品级 parity 的当前约束是什么
  - 语言/配置问题、blocked/observer 状态与响应式自引导应该从哪里开始

## 定向检索边界
- 如果你已经知道准确文件名，直接回 `../prd.index.md`。
- 如果问题涉及当前本地 launcher/playtest 操作口径，先读本页相关 PRD/design/manual 与 GitHub task issue evidence comments，再决定是否进入某个历史专题。
- 如果问题需要判断 runtime 正确性、hosted auth、chain behavior 或 release readiness，本页只提供文档入口，结论必须回到对应专业角色和当前任务证据。
- 四组 2026-03 跨表面动作源三件套已由 `game-client-launcher-cross-surface-action-parity.prd.md` 收敛并删除；追溯仅使用 Git 与 GitHub task issue evidence。其余历史完成专题除非仍是当前 operator 入口，不再提升为默认首读路径。

## 维护约定
- 新增 launcher 专题后，若改变默认首读路径，应同步更新本页。
- 改变本地启动、hosted login、provider preflight、chain runtime 或 release/distribution 口径时，必须同步更新对应 PRD/design/manual，并在 GitHub task issue evidence 记录状态。
- 本页只维护簇级入口，不维护完整文件清单。
