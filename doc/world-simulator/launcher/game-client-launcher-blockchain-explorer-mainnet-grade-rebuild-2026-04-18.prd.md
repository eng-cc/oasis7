# 客户端启动器区块链浏览器主链级信息架构重构（2026-04-18）

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-mainnet-grade-rebuild-2026-04-18.design.md`
- 对应项目管理文档: `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-mainnet-grade-rebuild-2026-04-18.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: 当前启动器 explorer 已具备 `overview/blocks/txs/search/address/contracts/assets/mempool` 能力，但界面仍以日志行和线性堆叠为主，缺少主链浏览器应有的命令区、概览主卡、标签导航、列表密度、详情检查板和状态空态层级，导致高频核查仍偏“人工解析文本”。
- Proposed Solution: 在不改变 explorer API 与轮询协议的前提下，按主链浏览器的信息架构重构 `oasis7_client_launcher` explorer 视图层，统一为“命令区 + 链健康概览 + 标签导航 + 主列表/主卡 + 详情检查板 + 空态/错误态”的跨 tab 结构，并同步治理渲染辅助模块，避免 UI 文件继续失控膨胀。
- Success Criteria:
  - SC-1: Explorer 打开后第一屏即可看到链健康概览，至少包含高度、节点/world 标识、最新区块/执行哈希与交易状态分布。
  - SC-2: `Blocks/Txs/Search/Address/Contracts/Assets/Mempool` 七个视图都采用可扫描的信息分层，不再以纯文本长行作为主呈现模式。
  - SC-3: `Blocks/Txs/Contracts/Mempool` 支持稳定的“左侧列表/右侧详情检查板”或等价的双区布局，用户定位详情的交互成本 <= 1 次点击。
  - SC-4: `Address/Assets` 视图必须把余额、供应、nonce、持仓与关联交易拆成分组卡片，达到公共主链浏览器的阅读优先级。
  - SC-5: 请求中、链未就绪、空结果、结构化错误必须在 explorer 面板内直接可见，不要求用户回头读主日志。
  - SC-6: native/web 继续复用同一 `egui` UI 逻辑，且 `explorer_window*.rs` 相关文件长度仍满足 1200 行治理约束。

## 2. User Experience & Functionality
- User Personas:
  - 启动器玩家: 需要像查主链浏览器一样快速确认链是否健康、交易是否上链、地址余额是否正确。
  - 测试/运维人员: 需要在一个窗口内完成“概览判断 -> 列表扫描 -> 详情确认 -> 跨 tab 跳转”闭环。
  - 启动器维护者: 需要在不改协议的前提下持续演进 explorer UI，并保持 native/web 行为一致。
- User Scenarios & Frequency:
  - 转账后核查链上结果: 每笔交易后 1~5 次。
  - 回归阶段核对 explorer 输出: 每个候选版本至少 1 次。
  - 日常排障和演示: 有链状态异常或对外演示时按需高频触发。
- User Stories:
  - PRD-WORLD_SIMULATOR-044: As a 启动器玩家 / 运维人员, I want the launcher explorer to read like a mainnet-grade block explorer, so that I can inspect chain state with clear hierarchy, faster scanning, and lower context-switch cost.
- Critical User Flows:
  1. Flow-LAUNCHER-EXPLORER-REFRESH-001（链健康首屏）:
     `打开 explorer -> 先看链健康概览与请求状态 -> 判断是否需要继续钻取区块/交易/地址`
  2. Flow-LAUNCHER-EXPLORER-REFRESH-002（列表到详情）:
     `切到 Blocks/Txs/Contracts/Mempool -> 在高密度列表定位目标 -> 单击后在详情板查看完整字段与关联跳转`
  3. Flow-LAUNCHER-EXPLORER-REFRESH-003（地址与资产核查）:
     `输入 account_id 或过滤账户 -> 查看余额/nonce/供应/持仓分组卡片 -> 必要时跳转交易详情`
  4. Flow-LAUNCHER-EXPLORER-REFRESH-004（错误与空态诊断）:
     `链未就绪或查询结果为空 -> explorer 面板内直接看到状态/错误/恢复提示 -> 再决定刷新、重置或切换 tab`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Explorer 命令区 | `refresh/reset/current_tab/request_state/chain_ready` | 顶部提供刷新当前视图、重置当前筛选、请求中/链未就绪提示 | `idle -> inflight -> ready/failed` | 不改变原轮询节奏 | 查询只读 |
| 主链级概览卡 | `latest/committed/network height`、`node_id/world_id`、`last_block_hash/last_execution_block_hash`、`transfer counters` | 打开窗口或刷新时展示为分组卡片与状态统计 | `loading -> ready/failed` | counters 继续沿用 runtime 聚合语义 | 查询只读 |
| 标签导航与计数 | `tab label`、`result count/health count` | 切换 tab 时仍保持结构化导航与当前视图上下文 | `selected <-> unselected` | 标签计数来自当前缓存结果或 overview | 查询只读 |
| 主列表/详情检查板 | `blocks/txs/contracts/mempool/search hits` + `selected detail` | 单击列表项更新详情板，详情板暴露关键字段与跨 tab 跳转 | `none -> selected` | 列表继续沿用接口返回顺序 | 查询只读 |
| 地址/资产分组卡片 | `balance/nonce/supply/holders`、`related txs` | 查询后以 summary cards + secondary list 结构展示 | `idle -> ready/failed` | 数值沿用后端真值，不做本地重算 | 查询只读 |
| 空态/错误态/链未就绪态 | `no_data/error_code/error/not_ready/inflight` | 在当前 tab 面板内显示解释与恢复动作提示 | `ready -> empty/failed/not_ready` | 不改变请求参数与节奏 | 查询只读 |
- Acceptance Criteria:
  - AC-1: Explorer 顶部存在独立命令区与链健康概览区，不再以一串 `small()` 文本作为主层级。
  - AC-2: 七个 tab 均采用统一的信息架构，至少包含“顶部工具/过滤区 + 主内容区 + 当前状态提示”。
  - AC-3: `Blocks/Txs/Contracts/Mempool` 具备主链浏览器风格的列表密度和详情检查板，而不是单列日志行堆叠。
  - AC-4: `Address/Assets` 把余额、nonce、供应、持仓、相关交易拆为有明确主次关系的卡片分组。
  - AC-5: 请求中、链未就绪、空结果、结构化错误在 explorer 面板内直达可见。
  - AC-6: 不新增 explorer API，不改变 runtime / control-plane 协议，也不增加默认查询频率。
  - AC-7: 对应实现与验证可追溯到 `launcher-explorer-mainnet-grade-rebuild` 和 `test_tier_required`。
- Non-Goals:
  - 不新增 explorer 后端接口、字段或索引策略。
  - 不引入新的 UI 技术栈，仍保持 `egui` native/wasm 同源实现。
  - 不在本轮扩展交易语义、链规则或控制面权限。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属能力。

## 4. Technical Specifications
- Architecture Overview:
  - 视图层: 重构 `explorer_window_view.rs` 与 `explorer_window_p1.rs` 的布局、卡片与状态呈现。
  - 状态层: 继续复用 `ExplorerPanelState` / `ExplorerP1State` 与现有请求调度逻辑。
  - 治理层: 允许把渲染辅助下沉到新 helper，以控制 explorer 相关文件长度与复杂度。
- Integration Points:
  - `crates/oasis7_client_launcher/src/explorer_window.rs`
  - `crates/oasis7_client_launcher/src/explorer_window_view.rs`
  - `crates/oasis7_client_launcher/src/explorer_window_p1.rs`
  - `crates/oasis7_client_launcher/src/main.rs`
  - `crates/oasis7_client_launcher/src/main_tests.rs`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 链未就绪: 面板内展示显式不可用提示，不让用户只看到空白列表。
  - 请求进行中: 顶部命令区和当前 tab 内均能感知 inflight，避免“按钮没反应”的误判。
  - 轮询刷新导致选中项失效: 如果当前选中记录在刷新后消失，详情板必须回退到清晰空态，而不是残留旧字段。
  - 超长 `hash/account_id/contract_id`: 采用截断显示 + 明细面板保留完整值，避免列表横向失控。
  - wasm 窄宽度: 结构可压缩为纵向布局，但不得退回纯日志行模式。
- Non-Functional Requirements:
  - NFR-1: 默认 1s explorer 轮询频率保持不变，不新增额外后台请求。
  - NFR-2: native/web 同源行为一致率 100%。
  - NFR-3: `explorer_window.rs`、`explorer_window_view.rs`、`explorer_window_p1.rs` 单文件长度均保持在 1200 行内。
  - NFR-4: 重构后仍可通过 `cargo test` 与 wasm `cargo check` required 回归。
- Security & Privacy:
  - 仅调整展示结构，不新增敏感信息暴露面。
  - 错误输出继续沿用结构化 `error_code + error` 语义，不泄露本地机密路径。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 完成 PRD / Design / Project 建模与模块主入口回写。
  - v1.1: 完成 explorer 视图层主链级信息架构重构。
  - v1.2: 完成 native/wasm required 回归与必要 Web 闭环验证。
- Technical Risks:
  - 风险-1: 过度视觉强化导致信息密度下降或点击路径变长。
  - 风险-2: explorer 渲染辅助继续堆在单文件内，触发 1200 行治理风险。
  - 风险-3: 跨 tab 重排时若没控制好状态复用，可能引入选中态或空态回归。

## 6. Validation & Decision Record
- Test Plan & Traceability:
  - PRD-WORLD_SIMULATOR-044 -> `launcher-explorer-mainnet-grade-rebuild` -> `test_tier_required`。
  - 计划验证命令:
    - `./scripts/doc-governance-check.sh`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture`
    - `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown`
    - `env -u RUSTC_WRAPPER cargo fmt --all`
    - Web-first explorer smoke（agent-browser）
- Decision Log:
  - DEC-LAUNCHER-EXPLORER-REFRESH-001: 本轮按“主链级信息架构重构”推进，而不是继续在既有日志式布局上叠 patch。理由: 当前主要瓶颈在 hierarchy 和 scanability，不是单个控件缺失。
  - DEC-LAUNCHER-EXPLORER-REFRESH-002: 不改后端协议，只重构展示层与渲染辅助分层。理由: 风险最可控，也能保持 native/web 同源闭环。
  - DEC-LAUNCHER-EXPLORER-REFRESH-003: 优先保证 `Blocks/Txs/Contracts/Mempool` 的列表/详情双区结构，同时把 `Address/Assets` 做成 summary-first 卡片。理由: 这是最接近主链浏览器的核心使用范式。
