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
    - `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.prd.md`
    - `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.project.md`
    - `doc/headless-runtime/project.md`
    - 已退役删除：`nonviewer-design-alignment-closure/review` 两组一次性设计对齐审查三件套。
  - 验收命令 (`test_tier_required`):
    - `rg -n "oasis7(_node|_consensus|_distfs|_proto|_viewer)?|crates/oasis7|crates/oasis7_node|crates/oasis7_consensus|crates/oasis7_distfs|crates/oasis7_proto|crates/oasis7_viewer" doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.prd.md doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.project.md doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.prd.md doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.project.md`
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
- [x] headless-runtime-nonviewer-design-alignment-triplet-retirement (PRD-ENGINEERING-021/025) [test_tier_required]: 删除已完成且继续暴露旧 `nonviewer` 设计审查语义的 `nonviewer-design-alignment-closure/review` 两组三件套，将追溯入口收敛到 `doc/core/reviews/`、`doc/headless-runtime/README.md`、`doc/headless-runtime/prd.index.md` 与 GitHub task issue evidence comments。 Trace: #1790 (task_747c60075cb6474fbb16d7b276eb86e4)

## 依赖
- 模块设计总览：`doc/headless-runtime/design.md`
- doc/headless-runtime/prd.index.md
- `doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.prd.md`
- `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.prd.md`
- `testing-manual.md`
- `skills/prd/check.md`

## 状态
- 更新日期: 2026-03-30
- 当前状态: completed
- 下一任务: 无（当前模块主项目无未完成任务）
- 最新完成: `TASK-NONVIEWER-009`（已为 `headless-runtime` README 增加轻量“从这里开始”，明确命名迁移说明、执行追踪、历史 `nonviewer` 专题、检查清单与模板目录的阅读顺序。）
- 最新完成: `headless-runtime-nonviewer-design-alignment-triplet-retirement`（已删除两组一次性 `nonviewer-design-alignment-*` 旧审查三件套，当前 active surface 只保留主 PRD 引用的鉴权与长稳专题。）
- 最新完成: `TASK-NONVIEWER-008`（已补齐 `nonviewer-onchain-auth-protocol-hardening` 项目文档中遗漏的当前实现 crate 名，统一切到 `oasis7::viewer`。）
- 最新完成: `TASK-NONVIEWER-007`（已完成 `doc/headless-runtime/nonviewer/**` 活跃专题中旧 `oasis7*` crate/path 当前真值口径的 `oasis7*` 收口。）
- 最新完成: `TASK-NONVIEWER-006`（headless-runtime 模块 README / PRD 索引入口同步）。
- 阶段收口优先级: `P1`
- 阶段 owner: `runtime_engineer`（验证：`qa_engineer`；排序裁剪：`producer_system_designer`）
- 阻断条件: 在当前阶段 `P0`（玩法 / runtime / testing / playability）未收口前，headless-runtime 不作为首要发布驱动项；但若发现生命周期 / 鉴权阻断性缺口，需升级回 `P0` 评审。
- 承接约束: 先完成 `TASK-NONVIEWER-002/003`，再与 testing 联动推进 `TASK-NONVIEWER-004`。
- PRD 质量门状态: strict schema 已对齐（含第 6 章验证与决策记录）。
- ROUND-010 入口治理状态: 已补齐命名迁移后的轻量入口，当前模块无需再拆更重的 README 层级。
- 说明: 本文档仅维护 headless-runtime（原 nonviewer）设计执行状态；历史过程归档见 `doc/devlog/README.md`，当前任务执行证据以 GitHub task issue evidence comments 为准。
- 当前追溯入口：`TASK-NONVIEWER-001~009`、`doc/headless-runtime/checklists/lifecycle-auth-consistency-checklist.md`、`doc/headless-runtime/templates/headless-runtime-release-gate-linkage.md`、`doc/headless-runtime/templates/longrun-archive-incident-template.md`、`doc/core/reviews/` 中的 round review 记录与 GitHub task issue evidence comments / role review evidence；旧 2026-03-11 root 状态 closure / handoff 文档与 `nonviewer-design-alignment-*` 一次性审查三件套已退役删除。
- 旧 `HO-CORE-20260310-HR-001` 阶段收口角色交接块已并入上方 `TASK-NONVIEWER-002/003/004` 完成记录与当前追溯入口；不再保留过期待办状态语义。
