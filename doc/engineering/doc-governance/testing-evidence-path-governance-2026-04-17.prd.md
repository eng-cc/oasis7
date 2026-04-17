# `testing/evidence` 热点路径治理（2026-04-17）

- 对应设计文档: `doc/engineering/doc-governance/testing-evidence-path-governance-2026-04-17.design.md`
- 对应项目管理文档: `doc/engineering/doc-governance/testing-evidence-path-governance-2026-04-17.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: `doc-corpus-maintenance-governance` 已把文档治理债界定为“入口减重之后的存量维护成本”，而 `doc/testing/evidence/` 仍是当前 `testing` 模块内最高密度热点路径，当前快照已有 49 份 Markdown 文件。虽然 `doc/testing/README.md` 与 `prd.index.md` 已完成模块级首读分流，但 `evidence/` 子目录内部仍缺少一个 canonical 子域入口，读者进入该路径后依然会直接面对 release gate、hosted world、shared network triad、claim matrix 与 assorted validation evidence 的平铺文件面。
- Proposed Solution: 建立 `testing-evidence-path-governance` 专题，把 `evidence/` 作为 `PRD-ENGINEERING-025` 的第四条已执行 follow-up；新增 `doc/testing/evidence/README.md` 作为热点子域 landing page，按“release gate / hosted-world & web / p2p & shared-network / governance drill / claim & audit matrix / 定向验证”分流读者，并同步回写 `testing` 与 `engineering` 上游入口。
- Success Criteria:
  - SC-1: engineering 存在正式 `testing-evidence-path-governance` 专题三件套，冻结为什么优先处理 `testing/evidence`、这批动作边界以及后续顺序。
  - SC-2: `doc/testing/evidence/README.md` 成为 `evidence/` 子目录 canonical 入口，能把进入该路径的默认阅读方式从“文件系统平铺浏览”收口到“按问题分流”。
  - SC-3: `doc/testing/README.md` 与 `doc/testing/prd.index.md` 能明确把 `evidence/README.md` 作为热点子域首读入口，而不是继续让模块 README 或完整索引单独承担全部分流职责。
  - SC-4: 本批不删除任何 `evidence/` 留痕文档，不在 `evidence/` 内做大规模物理合并，也不同时扩散到 `ci/`、`longrun/` 等其他热点子域。
  - SC-5: engineering 主 PRD、主项目、索引、README 与 `doc-corpus-maintenance-governance` 项目页完成回写，明确 `testing/evidence` 已完成第四条路径级治理 follow-up，当前这一轮应转入季度复核，而不是继续在同一 PR 横向扩新路径。

## 2. User Experience & Functionality
- User Personas:
  - 项目经理 / `producer_system_designer`: 需要在高密度 evidence 路径里快速判断“先看 release gate、hosted world、shared network triad 还是 claim/audit matrix”，而不是把留痕目录当文件仓库顺扫。
  - `qa_engineer` / `liveops_community`: 需要先进入正确证据簇，再决定是否下钻到某个具体 evidence 文件，而不是在近 50 份文档里凭文件名碰运气。
  - 文档治理评审者: 需要识别 `evidence/` 的主要阅读簇，判断哪些是活跃入口，哪些只是保留可检索性。
- User Scenarios & Frequency:
  - 查看 release/preview/trust gate 证据: 高频，在 release readiness 和口径评审中触发。
  - 查看 hosted world / browser / web surface 证据: 高频，在接入边界、权限与滥用分析中触发。
  - 查看 p2p/shared-network triad 证据: 高频，在 rollout、same-window snapshot 和 mixed-topology 讨论中触发。
  - 精确追溯某份矩阵、incident 或 validation evidence: 中低频，但需要一个稳定的定向检索入口，而不是平铺浏览。
- User Stories:
  - PRD-ENGINEERING-029: As a 项目经理/Testing owner, I want a canonical `doc/testing/evidence/` path entrypoint, so that I can navigate the densest testing hotspot path by intent instead of scanning nearly 50 evidence files blindly.
- Critical User Flows:
  1. Flow-TEPG-001:
     `进入 doc/testing/evidence/README.md -> 根据“release gate / hosted-world & web / p2p & shared-network / governance drill / claim & audit matrix / 定向验证”选择簇 -> 再进入对应证据`
  2. Flow-TEPG-002:
     `从 doc/testing/README.md 进入 evidence 热点子域 -> 命中 evidence/README.md -> 再决定看某一簇还是回到 prd.index`
  3. Flow-TEPG-003:
     `需要精确文件名检索 -> 先读 evidence/README.md 的簇级说明 -> 再返回 doc/testing/prd.index.md 长表或直接进 evidence 文件`
- Functional Specification Matrix:

| 对象/能力 | 字段定义 | 动作/行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| `doc/testing/evidence/README.md` | 首读分流、主题簇、现行边界、定向检索入口 | 作为 `evidence/` 子域 landing page | `missing -> canonical -> maintained` | 先按读者问题，再按推荐文档 | 所有人可读，治理 owner 可更新 |
| `evidence/` 主题簇导航 | `release-gate`、`hosted-world-and-web`、`p2p-shared-network`、`governance-drill`、`claim-and-audit`、`targeted-validation` | 把近 50 份 evidence 文档压成有限入口组 | `flat -> clustered` | 先高频问题，再按具体留痕下钻 | 所有人可读 |
| 模块上游回链 | `testing/README.md`、`testing/prd.index.md`、engineering 根入口 | 把 `evidence/README.md` 提升为热点子域默认入口 | `module_only -> path_entrypoint` | `evidence` 热点优先命中子域入口 | 所有人可读 |
| 路径级治理专题 | 问题定义、边界、验证、后续顺序 | 明确 `evidence/` 是当前 `testing` 热点路径治理切片 | `implicit -> formalized` | `devlog -> viewer -> p2p/node -> testing/evidence -> quarterly review` | 仅治理 owner 可写 |
- Acceptance Criteria:
  - AC-1: 存在 `testing-evidence-path-governance` 正式专题三件套。
  - AC-2: `doc/testing/evidence/README.md` 明确说明自身是 `evidence/` 子目录 canonical 入口，并给出首读分流。
  - AC-3: `doc/testing/README.md` 与 `doc/testing/prd.index.md` 明确把 `evidence/README.md` 作为热点子域默认入口之一。
  - AC-4: 本批不删除任何 `doc/testing/evidence/*.md` 既有留痕文档，也不在本批为 `ci/` 或 `longrun/` 同时建立子域入口。
  - AC-5: engineering 根入口、主项目、索引与 `doc-corpus-maintenance-governance` 项目页能够追溯本批任务与“下一步只剩季度复核”的顺序。
- Non-Goals:
  - 不在本批做 `evidence/` 文件物理合并或大规模删档。
  - 不在本批修改各条 evidence 的事实陈述正文。
  - 不在本批同时处理 `ci/`、`longrun/`、`launcher/` 或 `templates/` 子域入口。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 主要依赖 `find`、`rg` 与库存报告结果做 `evidence/` 目录体量统计与簇级分流，配合 Markdown 入口文档回写。
- Evaluation Strategy:
  - 复算 `doc/testing/evidence/` 文件数与主要主题簇，确认 `evidence/README.md` 的分流说明与当前路径结构一致。
  - 人工验证从 `doc/testing/README.md` 进入 `evidence/README.md` 后，能在 2 分钟内命中 release gate、hosted world 或 p2p/shared-network 入口。

## 4. Technical Specifications
- Architecture Overview:
  - `doc/testing/evidence/*.md` 继续保留原证据文件。
  - `doc/testing/evidence/README.md` 成为子域 landing page，负责问题导向分流，而不是替代 `doc/testing/prd.index.md` 或把 `evidence/` 再复制成一份长表。
  - `testing-evidence-path-governance` 负责冻结这条 follow-up 的边界与后续顺序。
- Integration Points:
  - `doc/testing/evidence/README.md`
  - `doc/testing/README.md`
  - `doc/testing/prd.index.md`
  - `doc/engineering/prd.md`
  - `doc/engineering/project.md`
  - `doc/engineering/README.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md`
- Edge Cases & Error Handling:
  - `evidence/` 文件数继续增长: 允许继续新增留痕文档，但必须维护 `evidence/README.md` 的簇级分流，否则该入口会再次失效。
  - 某个簇未来形成更明确的汇总主文档: 入口应优先指向新的主文档，而不是继续把散落 evidence 文件平铺在首屏。
  - 新读者只想找精确文件名: 允许直接回到 `doc/testing/prd.index.md` 或目标 evidence 文件，不要求在 `evidence/README.md` 内复制完整索引。
- Non-Functional Requirements:
  - NFR-1: `evidence/README.md` 必须在首屏说明“先看哪里”，不能退化成另一份长表索引。
  - NFR-2: 新专题与新入口均不得突破 Markdown 1000 行门禁。
  - NFR-3: 路径级治理入口必须保持纯 Markdown，可被仓库静态阅读链路直接消费。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-04-17): 建立 `testing/evidence` 路径级治理专题与 `evidence/README.md`，先解决 `evidence/` 子目录没有 landing page 的问题。
  - v1.1: 当前 PR 不再扩新热点路径；若季度复核后仍认定 `ci/` 或 `longrun/` 需要独立入口，再另开下一轮 follow-up。
  - v1.2: 若 `evidence/` 内部某些簇仍过于高密度，再单独开“簇内合并/归档”专题，而不是在当前入口页继续堆说明。
- Technical Risks:
  - 风险-1: 只加入口页、不减文件数时，维护成本本体仍然存在。
  - 风险-2: 若后续不维护 `evidence/README.md`，新入口会再次和实际结构脱节。
  - 风险-3: 若把子域入口误写成完整长表，会重新制造第二份高噪音索引。

## 6. Validation & Decision Record
- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-029 | `testing-evidence-path-governance` | `test_tier_required` | 专题三件套互链、`doc/testing/evidence/README.md` 首读分流、`doc/testing/README.md` / `doc/testing/prd.index.md` / engineering 根入口回写、`doc-governance-check.sh` 通过 | `testing/evidence` 热点子域入口、`PRD-ENGINEERING-025` 第四条 follow-up 收口与后续季度复核 |

- Decision Log:
  - DEC-TEPG-001: 先为 `evidence/` 新增子域 landing page，而不是直接要求证据汇总重构，因为当前最直接的维护成本问题是“进入热点路径后没有首读入口”。
  - DEC-TEPG-002: 继续保留 `doc/testing/prd.index.md` 的完整文件级索引职责，但不再让它单独承担整个 `evidence/` 子域的首读分流。
  - DEC-TEPG-003: 选择把 `evidence/` 作为 `p2p/node` 之后的下一条路径级治理 follow-up，因为它是当前 `testing` 模块内最高密度的活跃子目录。
