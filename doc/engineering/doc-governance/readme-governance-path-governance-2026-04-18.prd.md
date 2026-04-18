# `readme/governance` 热点路径治理（2026-04-18）

- 对应设计文档: `doc/engineering/doc-governance/readme-governance-path-governance-2026-04-18.design.md`
- 对应项目管理文档: `doc/engineering/doc-governance/readme-governance-path-governance-2026-04-18.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: `doc-corpus-maintenance-governance` 已把文档治理债界定为“入口减重之后的存量维护成本”，而 `doc/readme/governance/` 仍是当前第四个需要正式处理的热点路径，现有 96 份 Markdown 文件。虽然 `doc/readme/README.md` 与 `prd.index.md` 已完成模块级首读分流，但 `governance/` 子目录内部仍缺少一个 canonical 子域入口，读者进入该路径后仍会直接面对接近 100 份治理专题、runbook、素材包与 handoff 文档的平铺文件面。
- Proposed Solution: 建立 `readme-governance-path-governance` 专题，把 `doc/readme/governance/` 作为 `PRD-ENGINEERING-025` 的第五条已执行 follow-up；新增 `doc/readme/governance/README.md` 作为子域 landing page，按“治理控制 / release communication / Moltbook / limited preview 与 reward / 小红书与外宣激励 / 公开定位与世界规则 / 定向检索”分流读者，并同步回写 `readme` 与 `engineering` 上游入口。
- Success Criteria:
  - SC-1: engineering 存在正式 `readme-governance-path-governance` 专题三件套，冻结为什么优先处理 `readme/governance`、这批动作边界以及后续顺序。
  - SC-2: `doc/readme/governance/README.md` 成为 `governance/` 子目录 canonical 入口，能把进入该路径的默认阅读方式从“文件系统平铺浏览”收口到“按问题分流”。
  - SC-3: `doc/readme/README.md` 与 `doc/readme/prd.index.md` 能明确把 `governance/README.md` 作为热点子域首读入口，而不是继续把若干具体 runbook / 素材包直接挂在模块首屏。
  - SC-4: 本批不删除任何 `doc/readme/governance/*.md` 既有专题，不在 `governance/` 内做大规模物理合并，也不同时扩散到 `gap/production` 等次级子域。
  - SC-5: engineering 主 PRD、主项目、索引、README 与 `doc-corpus-maintenance-governance` 项目页完成回写，明确 `readme/governance` 已完成第五条 follow-up，下一步才正式转入季度复核。

## 2. User Experience & Functionality
- User Personas:
  - 项目经理 / `producer_system_designer`: 需要在高密度治理路径里快速判断“先看哪类文档”，而不是把 `governance/` 当文件仓库顺扫。
  - `liveops_community`: 需要先进入 Moltbook、小红书、limited preview 或 reward 相关入口，而不是在大量素材包和专题里凭文件名猜测。
  - 文档治理评审者: 需要识别 `governance/` 的主要阅读簇，判断哪些是 operator 入口，哪些只是保留可检索性。
- User Scenarios & Frequency:
  - 查看 README / 外宣口径控制: 高频，在对外描述、站点主定位、release communication 讨论中触发。
  - 查看渠道运营与素材包: 高频，在 Moltbook / 小红书持续运营与发布准备中触发。
  - 查看 limited preview / reward 治理: 中高频，在 reward review、invite pack、ledger 与 distribution closure 场景中触发。
  - 精确追溯某份治理专题: 低频，但需要稳定的定向检索入口，而不是平铺浏览。
- User Stories:
  - PRD-ENGINEERING-030: As a 项目经理/README owner, I want a canonical `doc/readme/governance/` path entrypoint, so that I can navigate the densest readme hotspot path by intent instead of scanning nearly 100 governance docs blindly.
- Critical User Flows:
  1. Flow-RGPG-001:
     `进入 doc/readme/governance/README.md -> 根据“治理控制 / release communication / Moltbook / limited preview 与 reward / 小红书 / 公开定位”选择簇 -> 再进入对应专题`
  2. Flow-RGPG-002:
     `从 doc/readme/README.md 进入 governance 热点子域 -> 命中 governance/README.md -> 再决定看 runbook、素材包或 PRD`
  3. Flow-RGPG-003:
     `需要精确文件名检索 -> 先读 governance/README.md 的簇级说明 -> 再返回 readme/prd.index.md 长表`
- Functional Specification Matrix:

| 对象/能力 | 字段定义 | 动作/行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| `doc/readme/governance/README.md` | 首读分流、主题簇、现行边界、定向检索入口 | 作为 `governance/` 子域 landing page | `missing -> canonical -> maintained` | 先按读者问题，再按推荐文档 | 所有人可读，治理 owner 可更新 |
| Governance 主题簇导航 | `controls`、`release communication`、`Moltbook`、`limited preview & reward`、`Xiaohongshu`、`public positioning` | 把近 100 份治理文档压成有限入口组 | `flat -> clustered` | 先 active 入口，再历史专题检索 | 所有人可读 |
| 模块上游回链 | `doc/readme/README.md`、`doc/readme/prd.index.md`、engineering 根入口 | 把 `governance/README.md` 提升为热点子域默认入口 | `direct-doc-links -> path_entrypoint` | Governance 热点优先命中子域入口 | 所有人可读 |
| 路径级治理专题 | 问题定义、边界、验证、后续顺序 | 明确 `governance/` 是存量维护成本阶段的第五条 follow-up | `implicit -> formalized` | `devlog -> viewer -> p2p -> testing -> readme/governance -> 季度复核` | 仅治理 owner 可写 |
- Acceptance Criteria:
  - AC-1: 存在 `readme-governance-path-governance` 正式专题三件套。
  - AC-2: `doc/readme/governance/README.md` 明确说明自身是 `governance/` 子目录 canonical 入口，并给出首读分流。
  - AC-3: `doc/readme/README.md` 与 `doc/readme/prd.index.md` 明确把 `governance/README.md` 作为热点子域默认入口之一。
  - AC-4: 本批不删除任何 `doc/readme/governance/*.md` 既有专题，也不在本批为 `gap/production` 同时建立子域入口。
  - AC-5: engineering 根入口、主项目、索引与 `doc-corpus-maintenance-governance` 项目页能够追溯本批任务与下一步顺序。
- Non-Goals:
  - 不在本批做 `governance/` 专题物理合并或大规模删档。
  - 不在本批重写 Moltbook / 小红书 / reward 具体正文内容。
  - 不在本批同时处理 `gap/` 或 `production/` 子域入口。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 主要依赖 `find` / `python3` 做 `governance/` 目录体量统计与簇级分流，配合 Markdown 入口文档回写。
- Evaluation Strategy:
  - 复算 `doc/readme/governance/` 文件数与主要主题簇，确认 `governance/README.md` 的分流说明与当前路径结构一致。
  - 人工验证从 `doc/readme/README.md` 进入 `governance/README.md` 后，能在 2 分钟内命中治理控制、Moltbook、小红书或 reward 相关入口。

## 4. Technical Specifications
- Architecture Overview:
  - `doc/readme/governance/*.md` 继续保留原专题文件。
  - `doc/readme/governance/README.md` 成为子域 landing page，负责问题导向分流，而不是替代 `doc/readme/prd.index.md` 的完整索引职责。
  - `readme-governance-path-governance` 负责冻结这条 follow-up 的边界与后续顺序。
- Integration Points:
  - `doc/readme/governance/README.md`
  - `doc/readme/README.md`
  - `doc/readme/prd.index.md`
  - `doc/readme/project.md`
  - `doc/engineering/prd.md`
  - `doc/engineering/project.md`
  - `doc/engineering/README.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md`
- Edge Cases & Error Handling:
  - `governance/` 文件数继续增长: 允许继续新增专题，但必须维护 `governance/README.md` 的簇级分流，否则该入口会再次失效。
  - 某个簇出现新的主入口: 应优先指向新的主文档，而不是把旧阶段素材包重新提升为首读面。
  - 新读者其实只想找精确文件名: 允许直接回到 `doc/readme/prd.index.md` 长表，不要求在 `governance/README.md` 内复制完整索引。
- Non-Functional Requirements:
  - NFR-1: `governance/README.md` 必须在首屏说明“先看哪里”，不能退化成另一份长表索引。
  - NFR-2: 新专题与新入口均不得突破 Markdown 1000 行门禁。
  - NFR-3: 路径级治理入口必须保持纯 Markdown，可被仓库静态阅读链路直接消费。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-04-18): 建立 `readme/governance` 路径级治理专题与 `governance/README.md`，先解决 `governance/` 子目录没有 landing page 的问题。
  - v1.1: 若季度复核发现 `governance/` 内某个簇继续膨胀，再单独开“簇内合并/归档”专题，而不是在当前入口页继续堆说明。
  - v1.2: 若 `gap/` 或 `production/` 成为新的热点，再按同样方法建立对应子域治理专题。
- Technical Risks:
  - 风险-1: 只加入口页、不减文件数时，维护成本本体仍然存在。
  - 风险-2: 若后续不维护 `governance/README.md`，新入口会再次和实际结构脱节。
  - 风险-3: 若把子域入口误写成完整长表，会重新制造第二份高噪音索引。

## 6. Validation & Decision Record
- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-030 | `readme-governance-path-governance` | `test_tier_required` | 专题三件套互链、`doc/readme/governance/README.md` 首读分流、`doc/readme/README.md` / `doc/readme/prd.index.md` / `doc/readme/project.md` / engineering 根入口回写、`doc-governance-check.sh` 通过 | `doc/readme/governance` 热点子域入口、`PRD-ENGINEERING-025` 第五条 follow-up 收口与后续季度复核 |

- Decision Log:
  - DEC-RGPG-001: 先为 `governance/` 新增子域 landing page，而不是直接要求主题合并，因为当前最直接的维护成本问题是“进入热点路径后没有首读入口”。
  - DEC-RGPG-002: 继续保留 `doc/readme/prd.index.md` 的完整索引职责，但不再让它单独承担整个 `governance/` 子域的首读分流。
  - DEC-RGPG-003: 选择把 `governance/` 作为 `testing/evidence` 之后的第五条路径级治理 follow-up，因为它是当前库存报告里仍处于 `action_required` 的高密度热点路径之一，且直接承接 `readme` 模块的对外口径与 liveops/operator 入口。
