# PowerStorage 全量下线（2026-03-06）

- 对应设计文档: `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.design.md`
- 对应项目管理文档: `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.project.md`

审计轮次: 3


## 1. Executive Summary
- Problem Statement: 当前 simulator / viewer / 脚本仍保留 `PowerStorage` 语义与入口，导致设计层“已删除储能设施”与实现层不一致，增加维护和评审噪音。
- Proposed Solution: 一次性删除 `PowerStorage` 全链路（数据结构、动作、事件、渲染实体、自动化目标、脚本参数与文档口径），仅保留 `PowerPlant`。
- Success Criteria:
  - SC-1: `WorldModel`、`WorldInitConfig`、`WorldScenarioSpec` 不再包含 `power_storages` 字段。
  - SC-2: simulator 不再暴露 `RegisterPowerStorage` / `StorePower` / `DrawPower` 动作与 `PowerStorage*` 事件。
  - SC-3: viewer 不再存在 `SelectionKind::PowerStorage` 及其 3D 资产/实体/UI 详情链路。
- SC-4: 当时仍在维护的主题包校验、贴图巡检与视觉评审模板不再要求 `power_storage` 资源与截图项；当前这批 3D/视觉 QA 工具已从仓库移除。
  - SC-5: `env -u RUSTC_WRAPPER cargo check`（`oasis7`、`oasis7_viewer`）与 targeted tests 可通过或给出可追踪的已知阻塞说明。

## 2. User Experience & Functionality
### User Personas
- 系统玩法开发：维护 simulator 行为模型与 action/event 稳定性。
- Viewer/UI 开发：维护实体渲染、选中详情、自动化截图链路。
- QA/评审同学：依赖脚本与评审卡进行视觉验收。

### User Scenarios & Frequency
- 开发者每日多次运行 `llm_bootstrap` / `power_bootstrap` 场景进行回归。
- 历史上 QA 会在发布前执行主题包校验与 texture inspector。
- 历史上 UI 评审会在每轮视觉基线时按卡片打分。

### User Stories
- 作为 simulator 开发者，我希望系统中不再有储能设施动作和事件，以便电力语义与当前设计一致。
- 作为 viewer/QA 用户，我希望视觉脚本和评审模板只覆盖有效实体，以便减少无效截图与误判。

### Critical User Flows
1. Flow-PSR-001（初始化链路）: `加载场景 -> 解析 init/spec -> 构建 world_model`，输出中不再出现 `power_storages`。
2. Flow-PSR-002（运行时动作）: `agent 提交动作 -> kernel 校验/执行 -> 产出事件`，不得出现储能动作与储能事件。
3. Flow-PSR-003（viewer 交互）: `加载 snapshot/event -> 3D 重建/选中详情/自动化对焦`，不得出现 `power_storage` 实体或目标类型。
4. Flow-PSR-004（视觉评审）: `运行校验脚本 -> 生成截图清单 -> UI 打分`，清单仅包含有效实体。

### Functional Specification Matrix
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 场景与初始化 | 删除 `power_storages` | 解析未知字段时报错（`deny_unknown_fields`） | `spec_loaded -> init_built` | 设施冲突仅比较 `power_plants/factories/mines` | 本地开发配置 |
| simulator 动作/事件 | 删除储能相关 action/event | 提交已删除动作在编译期不可构造 | `action_submitted -> event_emitted` | 电力仅按 plant owner 入账 | kernel 规则统一 |
| viewer 实体与选中 | 删除 `SelectionKind::PowerStorage` | 不渲染储能 mesh/material，不提供选中详情 | `snapshot_received -> scene_synced` | entity count 不再含 storage | 只读观测 |
| 脚本与评审模板 | 删除 `power_storage` inspect 项 | 旧工具链存在时参数校验拒绝 `power_storage` | `script_start -> artifacts_ready` | 截图集合固定为有效实体集合 | 本地脚本执行 |

### Acceptance Criteria
- AC-1: `rg "power_storages"` 在 `crates/oasis7/src/simulator` 中不再命中运行时代码（历史注释/日志除外）。
- AC-2: `rg "SelectionKind::PowerStorage|power_storage_entities"` 在 `crates/oasis7_viewer/src` 中不再命中。
- AC-3: `power_bootstrap.json` 不包含 `power_storages` 字段并可被场景解析。
- AC-4: 历史上的 `scripts/validate-viewer-theme-pack.py`、`scripts/viewer-texture-inspector*.sh` 已不再声明 `power_storage` inspect 维度；当前这些脚本已从仓库移除。
- AC-5: 历史视觉评审模板与首张评审卡已删除 storage 截图项；当前视觉评审模板文件已从活跃文档树移除。

### Non-Goals
- 不移除 `PowerPlant` 及辐射电厂建造链路。
- 不在本任务内重做电力平衡参数。
- 不重构 runtime builtin（`m1.power.storage`）模块加载机制。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 推理能力，仅更新行为提示中已删除动作映射。

## 4. Technical Specifications
### Architecture Overview
- Simulator:
  - 删除 `PowerStorage` 类型、`WorldModel.power_storages`、初始化 seed 与冲突校验。
  - 删除 `Action::{RegisterPowerStorage,StorePower,DrawPower}` 与 `PowerEvent::{PowerStorageRegistered,PowerStored,PowerDischarged}`。
  - 删除 kernel/replay/power 辅助函数中 storage 分支。
- Viewer:
  - 删除 storage 资产槽位、实体 marker、selection kind、UI 文本、自动化目标、状态抓取字段。
  - 删除 profile/env 对 storage mesh/material/texture 的配置入口。
- Scripts/Docs:
  - 在旧 3D/视觉 QA 工具仍存在时，删除 texture inspector / theme pack / release baseline 中 storage 选项与预设。
  - 更新当时仍存在的视觉评审模板与 UI 评审卡示例；当前这些 3D/视觉 QA 文档已整体退场。

### Integration Points
- `crates/oasis7/src/simulator/*`
- `crates/oasis7_viewer/src/*`
- 历史上的 `scripts/validate-viewer-theme-pack.py`
- 历史上的 `scripts/viewer-texture-inspector*.sh`
- 历史上的 `historical removed standard_3d viewer doc set: visual-review-score-card`
- `doc/ui_review_result/*.md`

### Edge Cases & Error Handling
- 旧场景 JSON 若仍含 `power_storages`，应在解析阶段直接失败并提示未知字段。
- 回放历史事件若包含旧储能事件，按“当前版本不兼容旧储能回放”处理并记录拒绝原因。
- 历史自动化脚本收到 `--inspect power_storage` 时返回明确错误并给出支持列表。

### Non-Functional Requirements
- NFR-1: 删除后 `cargo check -p oasis7` 与 `cargo check -p oasis7_viewer` 不得新增 unrelated 警告爆炸。
- NFR-2: 本任务完成当时，viewer 自动化与 texture inspector 的定向回归需保持可验证；当前 texture inspector 已从仓库移除。
- NFR-3: 文档树中所有活跃 PRD/手册不再声明 storage 为必检实体。

### Security & Privacy
- 本任务不引入新数据通道或权限模型；仅做能力删减。

## 5. Risks & Roadmap
### Phased Rollout
- M1: 文档建档与任务拆解（PRD + project）。
- M2: simulator 链路删除（模型/动作/事件/场景/测试）。
- M3: viewer 与脚本链路删除（渲染/选择/自动化/纹理巡检）。
- M4: 文档口径回写 + required 回归 + 提交收口。

### Technical Risks
- 风险 1: 删除 action/event 后影响 replay 与测试基线，需要同步修订断言。
- 风险 2: viewer 配置字段删除会影响主题包与脚本环境变量兼容性，需要统一清理。
- 风险 3: 文档/脚本若遗漏 storage 词条，会导致评审流程继续产出无效项。

## 6. Validation & Decision Record
### Test Plan & Traceability
- PSR-001（M1）-> 文档建档/索引接入 -> 文档审查（`test_tier_required`）。
- PSR-002（M2）-> simulator 删除与回归 -> `env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required`（允许记录既有 unrelated 失败）。
- PSR-003（M3）-> viewer 删除与回归 -> `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer --features test_tier_required`。
- PSR-004（M4）-> 脚本/视觉模板更新 -> 当时通过 `python3 scripts/validate-viewer-theme-pack.py ...` 与 `./scripts/viewer-texture-inspector.sh ...` 进行 smoke；当前相关脚本已从仓库移除。

### Decision Log
- 选型: 采用“全链路硬删除”，而非继续保留空壳字段兼容。
  - 理由: 设计已明确删除储能设施，继续兼容会持续引入无效成本与歧义。
- 放弃方案 A: 保留 `power_storages` 字段但禁用行为。
  - 否决原因: 会保留脚本/文档误导入口，且测试口径仍不收敛。
- 放弃方案 B: 仅删 simulator，不删 viewer/scripts。
  - 否决原因: UI/评审链路仍可选到不存在实体，验收结果不可用。
