# `p2p/node` 热点路径治理（2026-04-17）

- 对应设计文档: `doc/engineering/doc-governance/p2p-node-path-governance-2026-04-17.design.md`
- 对应项目管理文档: `doc/engineering/doc-governance/p2p-node-path-governance-2026-04-17.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: `doc-corpus-maintenance-governance` 已把文档治理债界定为“入口减重之后的存量维护成本”，而 `doc/p2p/node/` 仍是当前 `p2p` 模块内最高密度热点路径，治理前快照已有 68 份 Markdown 文件。虽然 `doc/p2p/README.md` 与 `prd.index.md` 已完成模块级首读分流，但 `node/` 子目录内部仍缺少一个 canonical 子域入口，读者进入该路径后依然会直接面对节点奖励、复制链路、PoS 时间、身份引导和 WASM 编译约束的平铺文件面。
- Proposed Solution: 建立 `p2p-node-path-governance` 专题，把 `node/` 作为 `PRD-ENGINEERING-025` 的第三条已执行 follow-up；新增 `doc/p2p/node/README.md` 作为热点子域 landing page，按“奖励与资产 / 复制与网络 / PoS 时间 / 身份引导 / WASM 编译 / 定向检索”分流读者，并同步回写 `p2p` 与 `engineering` 上游入口。
- Success Criteria:
  - SC-1: engineering 存在正式 `p2p-node-path-governance` 专题三件套，冻结为什么优先处理 `p2p/node`、这批动作边界以及后续顺序。
  - SC-2: `doc/p2p/node/README.md` 成为 `node/` 子目录 canonical 入口，能把进入该路径的默认阅读方式从“文件系统平铺浏览”收口到“按问题分流”。
  - SC-3: `doc/p2p/README.md` 与 `doc/p2p/prd.index.md` 能明确把 `node/README.md` 作为热点子域首读入口，而不是继续让模块 README 或完整索引单独承担全部分流职责。
  - SC-4: 本批不删除任何 `node/` 专题文档，不在 `node/` 内做大规模物理合并，也不同时扩散到 `distfs/`、`blockchain/` 等其他热点子域。
  - SC-5: engineering 主 PRD、主项目、索引、README 与 `doc-corpus-maintenance-governance` 项目页完成回写，明确 `p2p/node` 已完成第三条路径级治理 follow-up，下一步应转入 `testing`。

## 2. User Experience & Functionality
- User Personas:
  - 项目经理 / `producer_system_designer`: 需要在高密度路径里快速判断“先看奖励、复制、PoS 时间还是编译约束”，而不是把 `node/` 当文件仓库顺扫。
  - `runtime_engineer` / `qa_engineer`: 需要先进入复制链路、签名绑定、PoS 时间或奖励专题，而不是在几十份节点文档里凭文件名猜测。
  - 文档治理评审者: 需要识别 `node/` 的主要阅读簇，判断哪些是活跃入口，哪些只是保留可检索性。
- User Scenarios & Frequency:
  - 查看节点奖励、资产或结算口径: 高频，在 token/奖励/链上治理对齐中反复触发。
  - 查看复制链路、PoS 时间或 signer binding 约束: 高频，在 p2p/runtime 验证与故障分析中触发。
  - 精确追溯 `node/` 历史专题: 中低频，但需要一个稳定的定向检索入口，而不是平铺浏览。
- User Stories:
  - PRD-ENGINEERING-028: As a 项目经理/P2P owner, I want a canonical `doc/p2p/node/` path entrypoint, so that I can navigate the densest `p2p` hotspot path by intent instead of scanning nearly 70 files blindly.
- Critical User Flows:
  1. Flow-NPG-001:
     `进入 doc/p2p/node/README.md -> 根据“奖励与资产 / 复制与网络 / PoS 时间 / 身份引导 / WASM 编译”选择簇 -> 再进入对应专题`
  2. Flow-NPG-002:
     `从 doc/p2p/README.md 进入 node 热点子域 -> 命中 node/README.md -> 再决定看主文档还是回到 prd.index`
  3. Flow-NPG-003:
     `需要精确文件名检索 -> 先读 node/README.md 的簇级说明 -> 再返回 doc/p2p/prd.index.md 长表`
- Functional Specification Matrix:

| 对象/能力 | 字段定义 | 动作/行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| `doc/p2p/node/README.md` | 首读分流、主题簇、现行边界、定向检索入口 | 作为 `node/` 子域 landing page | `missing -> canonical -> maintained` | 先按读者问题，再按推荐文档 | 所有人可读，治理 owner 可更新 |
| `node/` 主题簇导航 | `reward-and-asset`、`replication-and-network`、`pos-time`、`identity-bootstrap`、`wasm-build` | 把近 70 份 `node/` 文档压成有限入口组 | `flat -> clustered` | 先 active 入口，再历史专题检索 | 所有人可读 |
| 模块上游回链 | `p2p/README.md`、`p2p/prd.index.md`、engineering 根入口 | 把 `node/README.md` 提升为热点子域默认入口 | `module_only -> path_entrypoint` | `node` 热点优先命中子域入口 | 所有人可读 |
| 路径级治理专题 | 问题定义、边界、验证、后续顺序 | 明确 `node/` 是当前 `p2p` 热点路径治理切片 | `implicit -> formalized` | `devlog -> viewer -> p2p/node -> testing` | 仅治理 owner 可写 |
- Acceptance Criteria:
  - AC-1: 存在 `p2p-node-path-governance` 正式专题三件套。
  - AC-2: `doc/p2p/node/README.md` 明确说明自身是 `node/` 子目录 canonical 入口，并给出首读分流。
  - AC-3: `doc/p2p/README.md` 与 `doc/p2p/prd.index.md` 明确把 `node/README.md` 作为热点子域默认入口之一。
  - AC-4: 本批不删除任何 `doc/p2p/node/*.md` 既有专题，也不在本批为 `distfs/` 或 `blockchain/` 同时建立子域入口。
  - AC-5: engineering 根入口、主项目、索引与 `doc-corpus-maintenance-governance` 项目页能够追溯本批任务与下一步顺序。
- Non-Goals:
  - 不在本批做 `node/` 专题物理合并或大规模删档。
  - 不在本批修改 `node/` 具体业务规则正文。
  - 不在本批同时处理 `distfs/`、`blockchain/` 或 `observer/` 子域入口。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 主要依赖 `find`、`rg` 与库存报告结果做 `node/` 目录体量统计与簇级分流，配合 Markdown 入口文档回写。
- Evaluation Strategy:
  - 复算 `doc/p2p/node/` 文件数与主要主题簇，确认 `node/README.md` 的分流说明与当前路径结构一致。
  - 人工验证从 `doc/p2p/README.md` 进入 `node/README.md` 后，能在 2 分钟内命中奖励、复制或 PoS 时间入口。

## 4. Technical Specifications
- Architecture Overview:
  - `doc/p2p/node/*.md` 继续保留原专题文件。
  - `doc/p2p/node/README.md` 成为子域 landing page，负责问题导向分流，而不是替代 `doc/p2p/prd.index.md` 或把 `node/` 再复制成一份长表。
  - `p2p-node-path-governance` 负责冻结这条 follow-up 的边界与后续顺序。
- Integration Points:
  - `doc/p2p/node/README.md`
  - `doc/p2p/README.md`
  - `doc/p2p/prd.index.md`
  - `doc/engineering/prd.md`
  - `doc/engineering/project.md`
  - `doc/engineering/README.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md`
- Edge Cases & Error Handling:
  - `node/` 文件数继续增长: 允许继续新增专题，但必须维护 `node/README.md` 的簇级分流，否则该入口会再次失效。
  - 某个簇出现主文档合并: 入口应优先指向新的主文档，而不是把旧阶段文档重新提升为首读面。
  - 新读者只想找精确文件名: 允许直接回到 `doc/p2p/prd.index.md` 长表，不要求在 `node/README.md` 内复制完整索引。
- Non-Functional Requirements:
  - NFR-1: `node/README.md` 必须在首屏说明“先看哪里”，不能退化成另一份长表索引。
  - NFR-2: 新专题与新入口均不得突破 Markdown 1000 行门禁。
  - NFR-3: 路径级治理入口必须保持纯 Markdown，可被仓库静态阅读链路直接消费。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-04-17): 建立 `p2p/node` 路径级治理专题与 `node/README.md`，先解决 `node/` 子目录没有 landing page 的问题。
  - v1.1: 若 `testing` 成为下一热点，按同样方法建立 `testing` 子域路径治理专题。
  - v1.2: 若 `node/` 内部某些簇仍过于高密度，再单独开“簇内合并/归档”专题，而不是在当前入口页继续堆说明。
- Technical Risks:
  - 风险-1: 只加入口页、不减文件数时，维护成本本体仍然存在。
  - 风险-2: 若后续不维护 `node/README.md`，新入口会再次和实际结构脱节。
  - 风险-3: 若把子域入口误写成完整长表，会重新制造第二份高噪音索引。

## 6. Validation & Decision Record
- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-028 | `p2p-node-path-governance` | `test_tier_required` | 专题三件套互链、`doc/p2p/node/README.md` 首读分流、`doc/p2p/README.md` / `doc/p2p/prd.index.md` / engineering 根入口回写、`doc-governance-check.sh` 通过 | `p2p/node` 热点子域入口、`PRD-ENGINEERING-025` 第三条 follow-up 收口与后续 `testing` 路径级治理 |

- Decision Log:
  - DEC-NPG-001: 先为 `node/` 新增子域 landing page，而不是直接要求主题合并，因为当前最直接的维护成本问题是“进入热点路径后没有首读入口”。
  - DEC-NPG-002: 继续保留 `doc/p2p/prd.index.md` 的完整文件级索引职责，但不再让它单独承担整个 `node/` 子域的首读分流。
  - DEC-NPG-003: 选择把 `node/` 作为 `world-simulator/viewer` 之后的下一条路径级治理 follow-up，因为它是当前 `p2p` 模块内最高密度的活跃子目录。
