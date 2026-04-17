# `world-simulator/viewer` 热点路径治理（2026-04-17）

- 对应设计文档: `doc/engineering/doc-governance/world-simulator-viewer-path-governance-2026-04-17.design.md`
- 对应项目管理文档: `doc/engineering/doc-governance/world-simulator-viewer-path-governance-2026-04-17.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: `doc-corpus-maintenance-governance` 已把文档治理债界定为“入口减重之后的存量维护成本”，而 `doc/world-simulator/viewer/` 仍是当前最高密度热点路径，已有 296 份 Markdown 文件。虽然 `doc/world-simulator/README.md` 和 `prd.index.md` 已完成模块级首读分流，但 `viewer/` 子目录内部仍缺少一个 canonical 子域入口，读者进入该路径后仍会直接面对接近 300 份文档的平铺文件面。
- Proposed Solution: 建立 `world-simulator-viewer-path-governance` 专题，把 `viewer/` 作为 `PRD-ENGINEERING-025` 的第二条已执行 follow-up；新增 `doc/world-simulator/viewer/README.md` 作为子域 landing page，按“操作手册 / software_safe Web / runtime live / chat&panel / release&visual / 3D hold / 定向检索”分流读者，并同步回写 `world-simulator` 与 `engineering` 上游入口。
- Success Criteria:
  - SC-1: engineering 存在正式 `world-simulator-viewer-path-governance` 专题三件套，冻结为什么优先处理 `viewer/`、这批动作边界以及后续顺序。
  - SC-2: `doc/world-simulator/viewer/README.md` 成为 `viewer/` 子目录 canonical 入口，能把进入该路径的默认阅读方式从“文件系统平铺浏览”收口到“按问题分流”。
  - SC-3: `doc/world-simulator/README.md` 与 `doc/world-simulator/prd.index.md` 能明确把 `viewer/README.md` 作为热点子域首读入口，而不是继续把 `viewer-manual.manual.md` 当成唯一 Viewer 入口。
  - SC-4: 本批不删除任何 Viewer 专题文档，不在 `viewer/` 内做大规模物理合并，也不同时扩散到 `launcher/llm` 等次级热点路径。
  - SC-5: engineering 主 PRD、主项目、索引、README 与 `doc-corpus-maintenance-governance` 项目页完成回写，明确 `world-simulator/viewer` 已完成首个路径级治理切片，下一步应转入 `p2p`。

## 2. User Experience & Functionality
- User Personas:
  - 项目经理 / `producer_system_designer`: 需要在高密度路径里快速判断“先看哪里”，而不是把子目录当文件仓库顺扫。
  - `viewer_engineer` / `qa_engineer`: 需要先进入操作手册、`software_safe` 主入口或 runtime live 主题，而不是在大量 Viewer 专题里凭文件名猜测。
  - 文档治理评审者: 需要识别 `viewer/` 的主要阅读簇，判断哪些是活跃入口，哪些只是保留可检索性。
- User Scenarios & Frequency:
  - 查看 Viewer 操作或 Web 闭环: 高频，几乎每次 Viewer 验证都可能触发。
  - 判断 `software_safe` / runtime live / 3D hold 的现行口径: 中高频，在玩法边界、对外口径或 QA 闭环中触发。
  - 追溯 Viewer 历史专题: 低频，但需要一个稳定的定向检索入口，而不是平铺浏览。
- User Stories:
  - PRD-ENGINEERING-027: As a 项目经理/Viewer owner, I want a canonical `doc/world-simulator/viewer/` path entrypoint, so that I can navigate the largest hotspot path by intent instead of scanning nearly 300 files blindly.
- Critical User Flows:
  1. Flow-VPG-001:
     `进入 doc/world-simulator/viewer/README.md -> 根据“操作手册 / software_safe / runtime live / chat&panel / release&visual / 3D hold”选择簇 -> 再进入对应专题`
  2. Flow-VPG-002:
     `从 doc/world-simulator/README.md 进入 Viewer 热点子域 -> 命中 viewer/README.md -> 再决定看 manual 还是 prd.index`
  3. Flow-VPG-003:
     `需要精确文件名检索 -> 先读 viewer/README.md 的簇级说明 -> 再返回 world-simulator/prd.index.md 长表`
- Functional Specification Matrix:

| 对象/能力 | 字段定义 | 动作/行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| `doc/world-simulator/viewer/README.md` | 首读分流、主题簇、现行边界、定向检索入口 | 作为 `viewer/` 子域 landing page | `missing -> canonical -> maintained` | 先按读者问题，再按推荐文档 | 所有人可读，治理 owner 可更新 |
| Viewer 主题簇导航 | `manual`、`software_safe Web`、`runtime live`、`chat&panel`、`release&visual`、`3D hold` | 把近 300 份 Viewer 文档压成有限入口组 | `flat -> clustered` | 先 active 入口，再历史专题检索 | 所有人可读 |
| 模块上游回链 | `world-simulator/README.md`、`world-simulator/prd.index.md`、engineering 根入口 | 把 `viewer/README.md` 提升为热点子域默认入口 | `manual_only -> path_entrypoint` | Viewer 热点优先命中子域入口 | 所有人可读 |
| 路径级治理专题 | 问题定义、边界、验证、后续顺序 | 明确 `viewer/` 是存量维护成本阶段的首个路径治理切片 | `implicit -> formalized` | `devlog -> viewer -> p2p -> testing` | 仅治理 owner 可写 |
- Acceptance Criteria:
  - AC-1: 存在 `world-simulator-viewer-path-governance` 正式专题三件套。
  - AC-2: `doc/world-simulator/viewer/README.md` 明确说明自身是 `viewer/` 子目录 canonical 入口，并给出首读分流。
  - AC-3: `doc/world-simulator/README.md` 与 `doc/world-simulator/prd.index.md` 明确把 `viewer/README.md` 作为热点子域默认入口之一。
  - AC-4: 本批不删除任何 `doc/world-simulator/viewer/*.md` 既有专题，也不在本批为 `launcher/llm` 同时建立子域入口。
  - AC-5: engineering 根入口、主项目、索引与 `doc-corpus-maintenance-governance` 项目页能够追溯本批任务与下一步顺序。
- Non-Goals:
  - 不在本批做 Viewer 专题物理合并或大规模删档。
  - 不在本批修改 `viewer-manual.manual.md` 的操作内容。
  - 不在本批同时处理 `launcher/llm/kernel` 子域入口。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 主要依赖 `find` / `rg` / `python3` 做 `viewer/` 目录体量统计与簇级分流，配合 Markdown 入口文档回写。
- Evaluation Strategy:
  - 复算 `doc/world-simulator/viewer/` 文件数与主要主题簇，确认 `viewer/README.md` 的分流说明与当前路径结构一致。
  - 人工验证从 `doc/world-simulator/README.md` 进入 `viewer/README.md` 后，能在 2 分钟内命中 `manual`、`software_safe` 或 runtime live 入口。

## 4. Technical Specifications
- Architecture Overview:
  - `doc/world-simulator/viewer/*.md` 继续保留原专题文件。
  - `doc/world-simulator/viewer/README.md` 成为子域 landing page，负责问题导向分流，而不是替代 `viewer-manual.manual.md` 或 `world-simulator/prd.index.md`。
  - `world-simulator-viewer-path-governance` 负责冻结这条 follow-up 的边界与后续顺序。
- Integration Points:
  - `doc/world-simulator/viewer/README.md`
  - `doc/world-simulator/README.md`
  - `doc/world-simulator/prd.index.md`
  - `doc/engineering/prd.md`
  - `doc/engineering/project.md`
  - `doc/engineering/README.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md`
- Edge Cases & Error Handling:
  - `viewer/` 文件数继续增长: 允许继续新增专题，但必须维护 `viewer/README.md` 的簇级分流，否则该入口会再次失效。
  - 某个簇出现主文档合并: 入口应优先指向新的主文档，而不是把旧阶段文档重新提升为首读面。
  - 新读者其实只想找精确文件名: 允许直接回到 `doc/world-simulator/prd.index.md` 长表，不要求在 `viewer/README.md` 内复制完整索引。
- Non-Functional Requirements:
  - NFR-1: `viewer/README.md` 必须在首屏说明“先看哪里”，不能退化成另一份长表索引。
  - NFR-2: 新专题与新入口均不得突破 Markdown 1000 行门禁。
  - NFR-3: 路径级治理入口必须保持纯 Markdown，可被仓库静态阅读链路直接消费。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-04-17): 建立 Viewer 路径级治理专题与 `viewer/README.md`，先解决 `viewer/` 子目录没有 landing page 的问题。
  - v1.1: 若 `p2p` 成为下一热点，按同样方法建立 `p2p` 子域路径治理专题。
  - v1.2: 若 `viewer/` 内部某些簇仍过于高密度，再单独开“簇内合并/归档”专题，而不是在当前入口页继续堆说明。
- Technical Risks:
  - 风险-1: 只加入口页、不减文件数时，维护成本本体仍然存在。
  - 风险-2: 若后续不维护 `viewer/README.md`，新入口会再次和实际结构脱节。
  - 风险-3: 若把子域入口误写成完整长表，会重新制造第二份高噪音索引。

## 6. Validation & Decision Record
- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-027 | `world-simulator-viewer-path-governance` | `test_tier_required` | 专题三件套互链、`doc/world-simulator/viewer/README.md` 首读分流、`world-simulator/README.md` / `prd.index.md` / engineering 根入口回写、`doc-governance-check.sh` 通过 | `world-simulator/viewer` 热点子域入口、`PRD-ENGINEERING-025` 第二条 follow-up 收口与后续 `p2p` 路径级治理 |

- Decision Log:
  - DEC-VPG-001: 先为 `viewer/` 新增子域 landing page，而不是直接要求主题合并，因为当前最直接的维护成本问题是“进入热点路径后没有首读入口”。
  - DEC-VPG-002: 继续保留 `viewer-manual.manual.md` 的 operator 手册职责，但不再让它单独承担整个 `viewer/` 子域的首读分流。
  - DEC-VPG-003: 选择把 `viewer/` 作为 `doc/devlog` 之后的第一条路径级治理 follow-up，因为它是当前库存报告里最高密度的活跃子目录。
