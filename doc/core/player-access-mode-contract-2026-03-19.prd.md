# oasis7：玩家访问模式总契约（2026-03-19）

- 对应设计文档: `doc/core/player-access-mode-contract-2026-03-19.design.md`
- 对应项目管理文档: `doc/core/player-access-mode-contract-2026-03-19.project.md`

审计轮次: 8

## 目标
- 将玩家访问模式正式收口到 `software_safe / pure_api` 双模式真值。
- 清除活跃文档中把已删除 3D 入口写成当前模式的双真值。
- 明确玩家访问模式、execution lane 与 provider alias 的分层边界。

## 范围
- 覆盖 `doc/core/**`、`doc/world-simulator/**`、`doc/game/**`、`doc/testing/**` 中的当前模式口径。
- 覆盖 `software_safe` Web 正式入口与 `pure_api` no-UI 正式入口的 claim 边界。
- 不恢复已删除的 3D 玩家入口，也不重写 provider/runtime 底层实现。

## 接口 / 数据
- 主 PRD: `doc/core/player-access-mode-contract-2026-03-19.prd.md`
- 设计文档: `doc/core/player-access-mode-contract-2026-03-19.design.md`
- 项目文档: `doc/core/player-access-mode-contract-2026-03-19.project.md`
- 下游入口: `testing-manual.md`、`doc/world-simulator/prd.md`、`doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- 核心字段: `mode_id`、`claim_scope`、`execution_lane`、`agent_decision_source`、`agent_provider_*`

## 里程碑
- M1: 冻结 `software_safe / pure_api` 双模式总契约。
- M2: 回写 core/world-simulator/game/testing 活跃载体。
- M3: 清空活跃文档里把旧 3D 入口写成当前模式的残留。

## 风险
- 历史专题、handoff 与项目管理文档仍可能引用旧 3D 术语，造成误读。
- 若把 execution lane/provider alias 重新写成玩家模式，会再次产生 taxonomy 漂移。
- 若 active docs 与脚本入口不同步，QA/release 结论会继续分叉。

## 1. Executive Summary
- Problem Statement: 仓库已经删除 `standard_3d` 相关代码、脚本与活跃文档后，跨模块口径必须同步收敛，否则会继续出现“代码真值是 `software_safe` 单 Web 入口，但文档仍宣称存在 3D 玩家入口”的双真值。
- Proposed Solution: 将玩家访问模式正式收口为 `software_safe / pure_api` 两项；其中 `software_safe` 负责唯一 Web 入口，`pure_api` 负责无 UI 正式入口。`player_parity / headless_agent / debug_viewer` 继续只作为 execution lane，`non-3D / 2D 优先` 继续只作为交付优先级或交互范围描述。
- Success Criteria:
  - SC-1: `software_safe` 是唯一正式 Web 玩家入口。
  - SC-2: `pure_api` 保持一等公民 no-UI 正式入口，formal gameplay 继续要求 active LLM access。
  - SC-3: 活跃文档、测试口径与对外 claim 不再把 `standard_3d` 写成现行模式。
  - SC-4: `agent_direct_connect/provider_loopback_http` 继续只作为兼容 alias；正式 operator-facing provider 口径仍是 `agent_decision_source + agent_provider_backend/contract/transport/url/auth/connect_timeout_ms/profile + agent_execution_lane`。
  - SC-5: `non-3D`、`2D 优先` 不得再被包装成模式名。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要唯一的玩家入口 taxonomy，避免发布/试玩结论继续分叉。
  - `qa_engineer`: 需要把 Web 证据与纯 API 证据绑定到正确模式。
  - `viewer_engineer`: 需要明确 Web 侧只维护 `software_safe`。
  - `agent_engineer`: 需要继续把 provider/lane 与玩家访问模式分层。
- Critical User Flows:
  1. `判断结论属于 Web 入口还是纯接口入口 -> 绑定到 software_safe / pure_api`
  2. `若文档仅写 non-3D / 2D 优先 -> 回补真实 mode_id`
  3. `若涉及 Local Provider -> 先绑定玩家访问模式，再补 execution lane 与 provider 维度`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 行为 | 状态 | 规则 | 权限 |
| --- | --- | --- | --- | --- | --- |
| 玩家访问模式注册表 | `mode_id`、`claim_scope`、`forbidden_claims`、`gameplay_prerequisites` | 任一结论先绑定模式 | `draft -> frozen -> audited` | 只允许 `software_safe`、`pure_api` | `producer_system_designer` 冻结 |
| execution lane 映射 | `execution_lane`、`lane_scope`、`player_mode_binding` | 记录如何执行/观战 | `unbound -> bound -> audited` | lane 不能替代玩家入口 | 模块 owner 联审 |
| provider 结构化口径 | `agent_decision_source`、`agent_provider_*` | 记录 provider 真值 | `undefined -> mapped -> documented` | alias 仅保留兼容解析 | `agent_engineer` / `producer_system_designer` 联审 |

## 3. Technical Specifications
- Architecture Overview:
  - `software_safe`: 唯一正式 Web 玩家入口。
  - `pure_api`: 唯一正式 no-UI 玩家入口。
  - `player_parity / headless_agent / debug_viewer`: execution lane。
  - `agent_direct_connect/provider_loopback_http`: 兼容 alias，不是玩家模式。
- Integration Points:
  - `testing-manual.md`
  - `doc/world-simulator/prd.md`
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
  - `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
- Edge Cases & Error Handling:
  - 若 Web 证据命中 `--no-llm` / `llm_required` / `llm_init_failed`，只能给 `software_safe blocked` 结论，不得降格成另一个模式。
  - 若需要纯接口长稳或自动化结论，必须显式绑定 `pure_api`，不得借用 `headless_agent` 作为玩家模式。
  - 若旧文档提到 `standard_3d`，应视为历史归档而非当前真值。
- Non-Functional Requirements:
  - NFR-1: 活跃文档中的当前玩家访问模式混淆命中数为 0。
  - NFR-2: QA / release / playability 证据 100% 显式标注 `mode_id`。

## 4. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-CORE-009 | T1-T10 | `test_tier_required` | `./scripts/doc-governance-check.sh` + 活跃文档残留 grep | 模式 taxonomy 与 claim 边界 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-PCM-001` | 玩家访问模式只保留 `software_safe / pure_api` | 继续保留已删除实现对应的 `standard_3d` | 当前仓库真值已无 3D 玩家入口，实现与文档必须一致。 |
| `DEC-PCM-002` | 继续保留 provider/lane 分层 | 把 lane 或 provider alias 当入口 | 可避免再次把执行方式误写成玩家模式。 |
