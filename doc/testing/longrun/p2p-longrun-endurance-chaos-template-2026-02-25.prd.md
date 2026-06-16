# oasis7：P2P 长跑 180 分钟 Chaos 模板方案（2026-02-25）

- 对应设计文档: `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.design.md`
- 对应项目管理文档: `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.project.md`
- strict-schema 迁移映射: `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.migration.md`

审计轮次: 4

## 1. Executive Summary
- Problem Statement: S9 长跑缺少可复用、可审计的大规模固定 chaos 基线，导致 180 分钟以上 endurance 场景难以稳定复现与跨版本比对。
- Proposed Solution: 提供仓库内可追踪的 `chaos-plan` 模板（180 分钟窗口），并与 continuous chaos 组合形成“固定基线回归 + 连续探索覆盖”的执行模式，同时在手册中明确其与 `test_tier_*` 的语义边界。
- Success Criteria:
  - SC-1: 新增模板文件 `doc/testing/chaos-plans/p2p-soak-endurance-full-chaos-v1.json` 并可被脚本直接消费。
  - SC-2: 模板覆盖 `restart/pause/disconnect` 与多节点轮换，事件时间轴可审计。
  - SC-3: `testing-manual.md` S9 提供模板命令与验收口径。
  - SC-4: 完成短窗可执行性验证并保留计数证据。
  - SC-5: 文档中明确 `soak_*` 档位与 Cargo `test_tier_*` 语义不等价。

## 2. User Experience & Functionality
- User Personas:
  - 测试维护者：需要稳定可复跑的长跑 chaos 基线。
  - 发布负责人：需要可审计注入计划与门禁证据。
  - 脚本维护者：需要在不改脚本语义下复用模板。
- User Scenarios & Frequency:
  - 发布前 endurance 验证：按版本执行 180 分钟模板基线。
  - 回归对比：同模板跨版本对照观察退化趋势。
  - 混合探索：固定模板 + continuous chaos 做补充覆盖。
- User Stories:
  - PRD-TESTING-LONGRUN-CHAOSTPL-001: As a 测试维护者, I want a reusable 180-minute chaos template, so that endurance regressions are reproducible.
  - PRD-TESTING-LONGRUN-CHAOSTPL-002: As a 发布负责人, I want clear S9 commands and acceptance criteria, so that gate decisions are consistent.
  - PRD-TESTING-LONGRUN-CHAOSTPL-003: As a 脚本维护者, I want template semantics separated from Cargo test tiers, so that execution intent is not confused.
- Critical User Flows:
  1. Flow-CHAOSTPL-001: `加载 p2p-soak-endurance-full-chaos-v1.json -> 校验 schema 与时间轴 -> 启动 S9 longrun`
  2. Flow-CHAOSTPL-002: `执行固定模板注入（可选叠加 continuous）-> 汇总 chaos 计数与 overall_status`
  3. Flow-CHAOSTPL-003: `在 testing-manual S9 对照命令与验收口径 -> 留存 run 证据用于审计`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 模板文件结构 | `meta`（版本/时长/策略）、`events[]`（`id/at_sec/topology/node/action/down_secs/duration_secs`） | 脚本读取模板并按事件执行 | `loaded -> validated -> scheduled` | `events.at_sec` 递增，便于审计与排障 | 文档/脚本维护者可更新模板 |
| 动作与节点约束 | `action=restart/pause/disconnect`；`triad_distributed` 节点限定 `sequencer/storage/observer` | 按约束选择目标并执行注入 | `pending -> injected -> completed/failed` | 固定节点集合与动作集合降低歧义 | QA/发布可审阅 |
| S9 执行模式 | 固定计划/混合探索命令组合 | `--chaos-plan` 单独或叠加 `--chaos-continuous-enable` | `baseline -> mixed` | 先基线再探索，保证可比性 | 测试维护者控制运行档位 |
| 验收证据 | `summary.json` 计数与 `overall_status` | 产出并核对 `chaos_plan_events_total/chaos_continuous_events_total/chaos_events_total` | `collected -> verified` | total 与分项一致性必须成立 | 发布门禁消费 |
- Acceptance Criteria:
  - AC-1: 模板文件纳入仓库并通过短窗可执行性验证。
  - AC-2: 模板注入事件覆盖多动作、多节点轮换与 180 分钟窗口策略。
  - AC-3: S9 手册命令与验收口径可直接执行。
  - AC-4: `soak_*` 与 `test_tier_*` 语义边界在文档中清晰声明。
  - AC-5: 验证 run 的计数关系与状态结论可追溯。
- Non-Goals:
  - 不新增 chaos 动作类型。
  - 不修改 `scripts/p2p-longrun-soak.sh` 执行语义。
  - 不做跨机房/跨地域故障编排。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题为 longrun chaos 模板与手册策略，不涉及 AI 推理系统改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 通过静态 `chaos-plan` 模板定义长窗口注入时间轴，并由现有 `p2p-longrun-soak.sh` 执行器消费；continuous chaos 作为可选叠加，不改变模板语义。
- Integration Points:
  - `doc/testing/chaos-plans/p2p-soak-endurance-full-chaos-v1.json`
  - `scripts/p2p-longrun-soak.sh`
  - `testing-manual.md`（S9）
  - `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.project.md`
- Edge Cases & Error Handling:
  - 事件过密噪声：按阶段分布注入，避免长期观测被高频扰动淹没。
  - 模板不可复现：固定 `id` 与时间轴，作为跨版本比较基线。
  - 语义混淆：文档显式区分 `soak_*` 执行档位与 Cargo `test_tier_*`。
  - schema 不兼容：短窗 smoke 先行验证脚本兼容性。
- Non-Functional Requirements:
  - NFR-CHAOSTPL-1: 模板具备可复跑、可审计属性（固定事件 ID 与时间轴）。
  - NFR-CHAOSTPL-2: 模板执行不破坏既有脚本语义与门禁逻辑。
  - NFR-CHAOSTPL-3: 验收计数字段具备一致性与可解释性。
  - NFR-CHAOSTPL-4: 文档口径需避免与 `test_tier_*` 术语冲突。
- Security & Privacy: 模板与日志仅面向测试环境；避免在文档示例中暴露敏感配置。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (CHAOSTPL-1): 方案与项目文档建档。
  - v1.1 (CHAOSTPL-2): 提交 180 分钟模板文件（190 事件）。
  - v2.0 (CHAOSTPL-3): 完成 S9 手册接线与语义边界声明。
  - v2.1 (CHAOSTPL-4): 完成短窗验证与证据收口。
  - v2.2 (CHAOSTPL-5): 按 strict schema 人工迁移并统一 `.prd` 命名。
- Technical Risks:
  - 风险-1: 注入密度失衡导致结果噪声过高。
  - 风险-2: 模板版本漂移导致历史对比断裂。
  - 风险-3: 执行档位语义混淆造成门禁误用。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-LONGRUN-CHAOSTPL-001 | CHAOSTPL-1/2 | `test_tier_required` | 模板 schema 与脚本兼容短窗验证 | S9 固定 chaos 基线 |
| PRD-TESTING-LONGRUN-CHAOSTPL-002 | CHAOSTPL-2/3/4 | `test_tier_required` | S9 命令执行与 summary 计数核验 | 发布门禁证据一致性 |
| PRD-TESTING-LONGRUN-CHAOSTPL-003 | CHAOSTPL-3/4/5 | `test_tier_required` | 手册语义边界检查 + 文档治理检查 | 测试术语一致性与可追溯性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-CHAOSTPL-001 | 引入静态 180 分钟模板作为固定基线 | 仅依赖 continuous 随机注入 | 固定模板更利于跨版本可比与审计。 |
| DEC-CHAOSTPL-002 | 支持模板与 continuous 叠加 | 两者互斥 | 兼顾基线回归与覆盖探索。 |
| DEC-CHAOSTPL-003 | 在 S9 明确 `soak_*` 与 `test_tier_*` 边界 | 沿用模糊表述 | 降低执行与门禁解释歧义。 |
