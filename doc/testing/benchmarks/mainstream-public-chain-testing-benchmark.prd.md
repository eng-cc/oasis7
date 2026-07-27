# oasis7 主流公链测试体系对标与缺口矩阵

- 对应设计文档: `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.design.md`
- 对应项目管理文档: `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.project.md`
- 原始基准日期: `2026-03-24`

审计轮次: 1

> Authority boundary: this benchmark document explains testing-maturity
> background and preserves the original 2026-03-24 benchmark history only. It
> must not maintain current `shared_devnet`, `public_testnet`, `mainnet`, or
> release status. Current network-tier truth lives in
> `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.project.md`.

## 1. Executive Summary
- Problem Statement: oasis7 近几轮已经补齐签名交易、生产 signer custody、治理 signer 外部化和创世 ceremony 的专题规格，也已经开始做真实 governance registry import/audit/runbook，但团队仍缺一份“主流公链到底怎样分层测试、oasis7 当前已经覆盖到哪一层、还缺什么才能把 preview 做成更像主流链的工程体系”的正式基准。若继续只用零散 required/full、长跑和 drill 结果沟通，很容易把“局部门禁已存在”误判成“整体测试体系已接近主流公链”。
- Proposed Solution: 由 testing 专业域维护测试体系对标 PRD，把主流公链常见测试层拆成 `spec/reference -> deterministic/unit/integration -> distributed/multi-node -> ui/playability -> longrun/chaos/drill -> network rehearsal / release-train readiness` 六层，再把 oasis7 可追溯的命令、证据与缺口映射进去；producer 只据此决定后续 execution workstreams 的优先级，不由本 benchmark 维护当前网络状态或成熟度。
- Success Criteria:
  - SC-1: 明确给出不少于 6 层的“主流公链测试体系”分层模型，并区分“单客户端项目的等价替代要求”与“多客户端公链的原生做法”。
  - SC-2: 明确给出 oasis7 当前 `已有 / 部分具备 / 缺失` 的测试矩阵，而不是只给抽象建议。
  - SC-3: 明确指出当前强项至少包括：`required/full 基础门禁`、`分布式子系统库测试`、`Web-first UI 闭环`、`长跑套件`、`governance registry audit + drill runbook`。
  - SC-4: 明确指出当前缺口至少包括：`fuzz/property-based gate 缺失`、formal `public_testnet` / `mainnet` readiness 仍需独立 gate、`真实 ceremony/drill 证据仍未完成`。
  - SC-5: 形成 producer 可直接使用的下一步顺序：`先补 drill evidence -> 再补 fault/chaos/negative gate -> 再补 network rehearsal / release-train readiness`。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要知道 oasis7 的测试体系离主流公链还差哪些层，避免过早升级阶段口径。
  - `qa_engineer`：需要把“主流公链怎么测”翻译成 oasis7 的 `test_tier_required` / `test_tier_full` / S4 / S6 / S9 / S10 / governance drill 组合。
  - `runtime_engineer`：需要知道 execution workstreams 需要补哪些类型的验证，而不只是继续加功能。
  - `liveops_community`：需要知道对外为何仍不能说“对标主流公链安全/测试成熟度”。
- User Scenarios & Frequency:
  - 每次有人提“现在是不是已经接近主流主链测试体系”时，先对照本专题。
  - 每次 `MAINNET-1~3` execution workstream 完成一个阶段时，回填本专题矩阵。
  - 每次准备做更高安全口径或创世前评审时，先复查 network rehearsal、drill、chaos、QA 证据是否到位。
- User Stories:
  - PRD-P2P-BENCH-001: As a `producer_system_designer`, I want one explicit benchmark for mainstream public-chain testing systems, so that oasis7 phase decisions are grounded in testing reality rather than intuition.
  - PRD-P2P-BENCH-002: As a `qa_engineer`, I want oasis7 suites mapped to benchmark layers, so that release gates and missing evidence are visible.
  - PRD-P2P-BENCH-003: As a `runtime_engineer`, I want the missing testing layers frozen as execution backlog, so that hardening work includes validation design instead of only code paths.
  - PRD-P2P-BENCH-004: As a `liveops_community`, I want public claims tied to benchmark coverage, so that the project does not overstate maturity.
- Critical User Flows:
  1. Flow-P2P-BENCH-001: `读取 testing-manual 与安全/readiness 专题 -> 提炼 oasis7 当前测试层 -> 对照主流公链分层模型 -> 形成 gap matrix`
  2. Flow-P2P-BENCH-002: `有人提出“接近主流公链测试体系” -> 检查 benchmark layers -> 若关键层仍缺失则直接 block 该口径`
  3. Flow-P2P-BENCH-003: `execution workstream 完成一次 drill/chaos/network-rehearsal 里程碑 -> 回填 benchmark 状态 -> producer 重新排序下一步`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算/判定规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 主流公链分层模型 | `layer_id/layer_name/mainstream_expectation/oasis7_equivalent` | 冻结主流链常见测试层，并给出 oasis7 的等价要求 | `draft -> frozen` | 单客户端项目不得照搬“多客户端”字面要求，必须给出本项目等价替代层 | `producer_system_designer` 拍板，`qa_engineer` 联审 |
| oasis7 当前映射 | `layer_id/current_coverage/evidence_paths/status` | 将 `required/full/S4/S6/S9/S10/governance audit` 映射到 benchmark | `unknown -> mapped` | 必须引用仓库内已有命令、手册或专题文档 | `qa_engineer` 主持，producer 收口 |
| 缺口矩阵 | `gap_id/layer_id/severity/owner/next_action` | 为每个缺失层给出 owner、优先级和下一动作 | `identified -> prioritized` | 若缺口直接影响安全口径或创世 gate，则至少记为 `high` | `producer_system_designer` 拍板 |
| 口径门禁 | `claim_phrase/min_layer_status/reject_reason` | 根据 benchmark 覆盖状态决定是否允许某类外部表述 | `draft -> enforced` | 只要 network rehearsal、chaos/drill 或真实 ceremony evidence 未完成，就不得宣称“对标主流公链测试成熟度” | `liveops_community` 执行，producer 审批 |
- Acceptance Criteria:
  - AC-1: 本专题必须明确写出主流公链测试体系至少包含六层：`spec/reference`、`deterministic/unit/integration`、`distributed/multi-node`、`ui/playability`、`longrun/chaos/drill`、`network rehearsal / release-train readiness`。
  - AC-2: 本专题必须明确写出 oasis7 当前已具备 `S0/S1/S2/S4/S6/S9/S10 + governance registry audit/runbook` 这一类基础，但这些还不足以推导出“主流公链级测试体系已完成”。
  - AC-3: 本专题必须明确写出：仓库当前没有冻结成正式 gate 的 fuzz/property-based 测试层。
  - AC-4: 本专题必须明确写出：仓库当前没有共享 `devnet/testnet/canary release train` 的正式执行层。
  - AC-5: 本专题必须明确写出：首轮真实 governance drill / genesis ceremony QA evidence 仍是当前高优先级缺口，而不是纯文档问题。
  - AC-6: 本专题必须输出一份 `已有/部分具备/缺失/下一步` 矩阵，可直接给 producer 排序。
  - AC-7: `testing-manual.md` 必须能找到本专题入口，避免测试分层与对标口径继续分离。
- Non-Goals:
  - 不在本专题内直接实现 fuzz 框架、legacy shared_devnet rehearsal / formal test environment 或新的 chaos 平台。
  - 不把当前 verdict 从 `not_mainnet_grade` 升级为任何更高级别。
  - 不替代具体 execution workstream 的详细测试卡。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 本专题属于“解释 + 对标 + 执行优先级”文档，不新增协议。它把 `testing-manual.md`、`p2p-mainnet-crypto-security-baseline`、`p2p-mainnet-grade-readiness-hardening`、`p2p-governance-signer-externalization` 当前已存在的测试与门禁，映射到主流公链更完整的测试体系上，判断 oasis7 目前在哪一层做得够、哪一层还没有正式形成 release gate。
- Integration Points:
  - `testing-manual.md`
  - `doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.prd.md`
  - `README.md`
  - `doc/product/player-entry-distribution/release-communications-and-public-claims.prd.md`
  - `doc/p2p/project.md`
- Edge Cases & Error Handling:
  - 若引用“主流公链多客户端一致性测试”，必须同时说明 oasis7 当前是单实现栈，因此等价要求应落在 `独立 replay/verifier`、`跨 world/节点副本一致性` 与 `共享网络升级演练`，而不是伪造不存在的第二客户端。
  - 若 oasis7 某层已有命令，但没有固定成 release gate、没有周期执行、没有证据模板，则该层最多只能记为 `partial`。
  - 若某项 drill 只有 clone-world 脚本，没有 default/live execution world 真实证据，则仍不得记为完成。
  - 若某项 network rehearsal 能力只有本地单机 smoke，没有共享节点、升级窗口或版本漂移管理，则不得记为 `devnet/testnet/release train complete`。
- Non-Functional Requirements:
  - NFR-P2P-BENCH-1: 所有“已有/缺失”判断都必须能回链到仓库命令、手册或正式专题文档。
  - NFR-P2P-BENCH-2: benchmark 必须同时给出 `mainstream expectation` 与 `oasis7 equivalent`，避免机械照搬其他链结构。
  - NFR-P2P-BENCH-3: benchmark 的下一步必须按 producer 视角排序，明确什么是当前最高杠杆验证工作。
  - NFR-P2P-BENCH-4: benchmark 结论必须与根 `README.md` 的当前公开状态和产品层公开口径规则一致，不得制造第二套安全成熟度口径。
- Security & Privacy: 本专题只记录测试层与验证缺口，不记录任何私钥、助记词、离线签名材料或 operator 敏感信息。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 建立 benchmark 文档、冻结 oasis7 当前映射与缺口矩阵。
  - v1.1: 补首轮真实 governance drill / negative drill / QA evidence。
  - v1.2: 补 network rehearsal / release-train readiness / 升级演练定义。
  - v2.0: 再决定是否需要引入 fuzz/property-based gate 或独立 verifier 路径。
- Technical Risks:
  - 风险-1: 若把 `required/full` 当成“主流公链完整测试体系”，会低估 network rehearsal、事故 drill、release train 的工程成本。
  - 风险-2: 若只在 clone-world 做 drill，不在真实 default/live execution world 留证据，readiness 评估会继续停在 spec-heavy 状态。
  - 风险-3: 若不把 fuzz/property-based 或 chaos 层明确记为缺口，后续很容易一直不补。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-BENCH-001 | BENCH-0/1 | `test_tier_required` | benchmark PRD/project/design 建档、分层模型冻结、模块入口映射 | producer 安全/测试阶段判断 |
| PRD-P2P-BENCH-002 | BENCH-1/2 | `test_tier_required` | `testing-manual`/governance/readiness 文档映射、suite 对位与 gap matrix 固化 | QA release gate 与证据体系 |
| PRD-P2P-BENCH-003 | BENCH-2/3 | `test_tier_required` | 缺口优先级、owner 与 execution next step 冻结 | runtime/QA 后续验证设计 |
| PRD-P2P-BENCH-004 | BENCH-3 | `test_tier_required` | public-claims 边界复核与对外口径回链 | liveops 对外成熟度表述 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-P2P-BENCH-001 | 用“主流公链测试分层 + oasis7 等价映射”做 benchmark | 直接照抄多客户端公链 checklist | oasis7 当前是单实现栈，必须给出等价要求才有执行意义。 |
| DEC-P2P-BENCH-002 | 当前把 `governance audit/runbook` 记为强正向信号，但仍不等于 drill 完成 | 因为已有 import/audit 工具就直接记完成 | 缺真实 execution evidence，不能跨越 QA gate。 |
| DEC-P2P-BENCH-003 | 把 `network rehearsal / release-train readiness` 记为当前明显缺口 | 继续只在本地/clone-world dry-run 评估 readiness | 主流公链的成熟度很大一部分来自共享环境、升级窗口和持续演练。 |
