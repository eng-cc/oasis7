# headless-runtime PRD Project（原 nonviewer）

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-NONVIEWER-001 (PRD-NONVIEWER-001) [test_tier_required]: 完成 headless-runtime PRD 改写，建立无界面链路设计入口。
- [x] TASK-NONVIEWER-002 (PRD-NONVIEWER-001/002) [test_tier_required]: 补齐生命周期与鉴权协议的一致性检查清单。
  - 产物文件:
    - `doc/headless-runtime/checklists/lifecycle-auth-consistency-checklist.md`
  - 验收命令 (`test_tier_required`):
    - `test -f doc/headless-runtime/checklists/lifecycle-auth-consistency-checklist.md`
    - `rg -n "生命周期阶段一致性|鉴权协议一致性|异常恢复与升级条件|阻断条件|结论记录模板" doc/headless-runtime/checklists/lifecycle-auth-consistency-checklist.md`
- [x] TASK-NONVIEWER-003 (PRD-NONVIEWER-002/003) [test_tier_required]: 建立长稳归档与故障追溯证据模板。
  - 产物文件:
    - `doc/headless-runtime/templates/longrun-archive-incident-template.md`
  - 验收命令 (`test_tier_required`):
    - `test -f doc/headless-runtime/templates/longrun-archive-incident-template.md`
    - `rg -n "归档证据|故障追溯|复盘摘要|失败签名|恢复动作" doc/headless-runtime/templates/longrun-archive-incident-template.md`
- [x] TASK-NONVIEWER-004 (PRD-NONVIEWER-003) [test_tier_required]: 联动 testing 模块完善 headless-runtime 长稳门禁。
  - 产物文件:
    - `doc/headless-runtime/templates/headless-runtime-release-gate-linkage.md`
  - 验收命令 (`test_tier_required`):
    - `test -f doc/headless-runtime/templates/headless-runtime-release-gate-linkage.md`
    - `rg -n "对接规则|引用字段映射|testing 证据包|core go/no-go" doc/headless-runtime/templates/headless-runtime-release-gate-linkage.md`
- [x] TASK-NONVIEWER-005 (PRD-NONVIEWER-001/002/003) [test_tier_required]: 对齐 strict PRD schema，补齐关键流程/规格矩阵/边界异常/NFR/验证与决策记录。
- [x] TASK-NONVIEWER-006 (PRD-NONVIEWER-001) [test_tier_required]: 同步 `doc/headless-runtime/README.md` 与 `doc/headless-runtime/prd.index.md` 的模块入口索引，补齐近期专题、模块职责与根目录收口口径。
- [x] TASK-NONVIEWER-007 (PRD-NONVIEWER-001/002/003) [test_tier_required]: 收口 `doc/headless-runtime/nonviewer/**` 活跃专题中仍把旧 `oasis7*` crate/path 写成当前实现载体的口径，统一到 `oasis7*`。
  - 产物文件:
    - `doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.prd.md`
    - `doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.project.md`
    - `doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.prd.md`
    - `doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.project.md`
    - `doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25.prd.md`
    - `doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25.project.md`
    - `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.prd.md`
    - `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.project.md`
    - `doc/headless-runtime/project.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "oasis7(_node|_consensus|_distfs|_proto|_viewer)?|crates/oasis7|crates/oasis7_node|crates/oasis7_consensus|crates/oasis7_distfs|crates/oasis7_proto|crates/oasis7_viewer" doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.prd.md doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.project.md doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.prd.md doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.project.md doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25.prd.md doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25.project.md doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.prd.md doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-NONVIEWER-008 (PRD-NONVIEWER-002) [test_tier_required]: 补齐 `nonviewer-onchain-auth-protocol-hardening` 项目文档中遗漏的当前实现 crate 名，统一到 `oasis7::viewer`。
  - 产物文件:
    - `doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.project.md`
    - `doc/headless-runtime/project.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "oasis7::viewer" doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-NONVIEWER-009 (PRD-NONVIEWER-001) [test_tier_required]: 执行 ROUND-010 `headless-runtime` 入口治理，为模块 README 增加命名迁移后的轻量阅读顺序，并明确 README 与 `nonviewer/`、`checklists/`、`templates/`、`prd.index.md` 的边界。

## 依赖
- 模块设计总览：`doc/headless-runtime/design.md`
- doc/headless-runtime/prd.index.md
- `doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.prd.md`
- `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.prd.md`
- `testing-manual.md`
- `.agents/skills/prd/check.md`

## 状态
- 更新日期: 2026-03-30
- 当前状态: completed
- 下一任务: 无（当前模块主项目无未完成任务）
- 最新完成: `TASK-NONVIEWER-009`（已为 `headless-runtime` README 增加轻量“从这里开始”，明确命名迁移说明、执行追踪、历史 `nonviewer` 专题、检查清单与模板目录的阅读顺序。）
- 最新完成: `TASK-NONVIEWER-008`（已补齐 `nonviewer-onchain-auth-protocol-hardening` 项目文档中遗漏的当前实现 crate 名，统一切到 `oasis7::viewer`。）
- 最新完成: `TASK-NONVIEWER-007`（已完成 `doc/headless-runtime/nonviewer/**` 活跃专题中旧 `oasis7*` crate/path 当前真值口径的 `oasis7*` 收口。）
- 最新完成: `TASK-NONVIEWER-006`（headless-runtime 模块 README / PRD 索引入口同步）。
- 阶段收口优先级: `P1`
- 阶段 owner: `runtime_engineer`（验证：`qa_engineer`；排序裁剪：`producer_system_designer`）
- 阻断条件: 在当前阶段 `P0`（玩法 / runtime / testing / playability）未收口前，headless-runtime 不作为首要发布驱动项；但若发现生命周期 / 鉴权阻断性缺口，需升级回 `P0` 评审。
- 承接约束: 先完成 `TASK-NONVIEWER-002/003`，再与 testing 联动推进 `TASK-NONVIEWER-004`。
- PRD 质量门状态: strict schema 已对齐（含第 6 章验证与决策记录）。
- ROUND-010 入口治理状态: 已补齐命名迁移后的轻量入口，当前模块无需再拆更重的 README 层级。
- 说明: 本文档仅维护 headless-runtime（原 nonviewer）设计执行状态；过程记录在 `doc/devlog/2026-03-03.md`。

## 阶段收口角色交接
### Meta
- Handoff ID: `HO-CORE-20260310-HR-001`
- Date: `2026-03-10`
- From Role: `producer_system_designer`
- To Role: `runtime_engineer`
- Related Module: `headless-runtime`
- Related PRD-ID: `PRD-NONVIEWER-001/002/003`
- Related Task ID: `TASK-NONVIEWER-002/003/004`
- Priority: `P1`
- Expected ETA: `待接收方确认`

### Objective
- 目标描述：建立 headless-runtime 生命周期 / 鉴权一致性与长稳追溯门禁骨架，作为当前阶段的次一级收口项。
- 成功标准：生命周期、鉴权、归档、追溯四类基础门禁形成模板，并能与 testing 模块对接。
- 非目标：本轮不扩展新的 headless 产品功能。

### Current State
- 当前实现 / 文档状态：模块主 PRD 已重写完成，但 `002/003/004` 仍未承接。
- 已确认事实：core 将 headless-runtime 列为 `P1`，在 P0 收口后推进。
- 待确认假设：是否存在需立即升级为 `P0` 的鉴权或生命周期缺口。
- 当前失败信号 / 用户反馈：若无长稳归档与追溯模板，后续远程运行链路难以稳定运营。

### Scope
- In Scope: `TASK-NONVIEWER-002/003/004` 的文档和门禁骨架。
- Out of Scope: 新的 headless 功能扩展、与当前链路无关的控制台功能。

### Inputs
- 关键文件：`doc/headless-runtime/project.md`、`doc/headless-runtime/prd.md`、相关 nonviewer 专题文档。
- 关键命令：沿用 headless / longrun / auth 相关检查命令。
- 上游依赖：`testing` 模块的门禁矩阵与证据包模板。
- 现有测试 / 证据：现有 auth / longrun 文档与脚本基础。

### Requested Work
- 工作项 1：完成生命周期与鉴权一致性检查清单。
- 工作项 2：建立长稳归档与故障追溯证据模板。
- 工作项 3：与 testing 联动承接长稳门禁。

### Expected Outputs
- 代码改动：如需，仅限支撑 headless 验证与证据采集的必要脚本改动。
- 文档回写：`doc/headless-runtime/project.md` 与相关专题文档。
- 测试记录：补齐 `test_tier_required`，并标注何处需后续 `test_tier_full`。
- devlog 记录：记录门禁骨架与是否需升级优先级。

### Done Definition
- [ ] 输出满足目标与成功标准
- [ ] 影响面已核对 `producer_system_designer` / `qa_engineer`
- [ ] 对应 `prd.md` / `project.md` 已回写
- [ ] 对应 `doc/devlog/YYYY-MM-DD.md` 已记录
- [ ] required/full 测试证据已补齐或明确挂起原因

### Risks / Decisions
- 已知风险：如果 testing 模块证据包模板尚未落地，`TASK-NONVIEWER-004` 会继续被阻塞。
- 待拍板事项：是否需要把某些鉴权失败签名提升为发布级阻断。
- 建议决策：先做 `002/003` 门禁骨架，再和 testing 一起收 `004`。

### Validation Plan
- 测试层级：`test_tier_required`（必要时补 `test_tier_full`）
- 验证命令：沿用 headless / longrun / auth 相关命令并回写证据路径。
- 预期结果：headless-runtime 长稳与追溯链路有统一模板，可被 testing 复用。
- 回归影响范围：headless-runtime / testing / 长稳与鉴权链路。

### Handoff Acknowledgement
- 接收方确认范围：`已接收 TASK-NONVIEWER-002/003/004；当前提交完成生命周期鉴权清单、长稳归档模板与 testing 门禁对接说明`
- 接收方确认 ETA：`TASK-NONVIEWER-002/003/004 已完成`
- 接收方新增风险：`真实运行阈值仍需在后续实际样本中填充，当前先统一对接口径`
