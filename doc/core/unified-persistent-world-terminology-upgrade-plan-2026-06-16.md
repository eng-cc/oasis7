# oasis7 统一持久大世界术语升级计划（2026-06-16）

审计轮次: 0

## 目标

- 将 oasis7 的默认玩家视角和产品叙事统一为“统一持久大世界”。
- 清理 active product / player-facing docs 中把测试环境、网络层级或候选窗口写成“世界类型”的口径。
- 保留历史证据、脚本参数、manifest 字段和兼容 alias 的可追溯性，避免为了改名破坏 QA、ops、runtime 回放链路。

## 范围

### In Scope

- `README.md`、`world-rule.md`、`doc/core/**`、`doc/game/**`、`doc/world-simulator/**`、`site/**` 中的玩家可见、产品可见、入口说明类术语。
- `testing-manual.md`、`doc/p2p/**`、`doc/engineering/governance/**` 中会被引用到产品判断或对外 claim 的环境/网络术语边界。
- 新增术语 lint / grep gate 的规划和验收标准。

### Out of Scope

- 本计划不直接重命名历史 evidence 文件、历史 runbook 文件名、已有 branch/task 名、已落档 PRD 标题或脚本参数。
- 本计划不直接部署 public testnet、mainnet 或 production 服务。
- 本计划不改变 `world_id`、network tier manifest、runtime topic、storage path、checkpoint/replay 等技术字段的语义。

## 决策摘要

默认产品模型：

> oasis7 是一个由 runtime / 共识维护的统一持久大世界。玩家通过 `viewer` 或 `pure_api` 进入同一世界叙事，间接影响 Agent、组织、工业和治理演化。

重要边界：

- “统一持久大世界”是玩家和产品默认模型，不等于当前已经具备无限容量、正式 public launch、mainnet 价值网络或无准入限制。
- `world_id` 是 runtime / storage / consensus 的技术分区键，不应在玩家叙事里表现为“多个游戏世界”。
- local / test / production 是研发与运维环境；它们承载同一统一大世界模型在不同阶段的候选实现，不是玩家侧的世界品牌。

## Canonical Terms

| 场景 | 标准术语 | 说明 |
| --- | --- | --- |
| 玩家/产品默认世界 | 统一持久大世界 | 默认中文玩家叙事。 |
| 英文产品叙事 | unified persistent world | 英文 canonical term。 |
| 世界状态 | 世界状态 / world state | 可用于 runtime、viewer、QA。 |
| 玩家入口 | `viewer` / `pure_api` | 与玩家访问模式总契约对齐。 |
| 本地研发 | local environment / 本地环境 | 只描述研发场景，不描述玩家世界。 |
| 受控测试 | test environment / 测试环境 | 只描述验证环境，不作为玩家世界名。 |
| 正式环境 | production environment / 正式环境 | 只描述未来正式服务 lane，不替代大世界术语。 |
| 链/网络技术层 | network tier / chain tier | 只在 p2p/runtime/ops 文档使用。 |
| 技术分区键 | `world_id` | 仅在实现、manifest、storage、topic、evidence 中使用。 |

## Legacy / Forbidden Product Terms

以下词汇不得作为 active product / player-facing 世界称呼出现：

| Legacy term | 新文档处理规则 | 允许保留位置 |
| --- | --- | --- |
| 旧共享开发网 machine token | 不得作为目标环境、玩家世界、产品阶段或宣传词；引用时必须写成 legacy rehearsal evidence。 | 历史 evidence、历史 runbook、兼容 manifest、脚本/测试兼容路径。 |
| shared devnet / shared network | 不得作为世界模型名；如确需引用旧专题，只能写“历史共享网络预演证据”。 | 历史专题、QA 追溯、operator 迁移备注。 |
| hosted world | 不得作为玩家默认世界名；改写为“玩家接入面 / hosted player entry / hosted access”。 | 安全边界 PRD、session/auth/runbook 中的兼容术语。 |
| public testnet world | 不得作为玩家世界名；改写为“测试环境中的统一大世界候选运行”。 | network tier docs / readiness evidence。 |
| local devnet world | 不得作为玩家世界名；改写为“本地环境中的统一大世界研发实例”。 | runtime/local smoke docs。 |
| large shared world readiness | 不得作为模糊 claim；拆成“统一持久大世界默认模型”和具体 readiness gate。 | QA claim boundary。 |

强规则：

1. 玩家可见文本默认只说“统一持久大世界”，不把环境名暴露成世界名。
2. 产品文档可以解释“当前是 limited playable technical preview”，但不能退回“多个共享开发世界”的表述。
3. 技术文档若必须出现 legacy term，必须同时标注 `legacy / rehearsal / compatibility / historical evidence` 中至少一个限定词。
4. 新增对外口径不得出现 “当前所有玩家已经在正式无边界大世界中游玩” 这类 readiness 越界 claim。

## Evidence Baseline

本计划基于以下已有仓库口径：

- `README.md` 已将 oasis7 定义为持久多主体文明模拟游戏，并说明世界状态可落盘恢复、单个玩家离线不影响世界持续运行。
- `world-rule.md` 已规定离散 tick、不可暂停、全局同步 tick 的目标架构，并声明世界有总空间限制、无内部人为边界。
- `doc/game/gameplay/gameplay-top-level-design.prd.md` 已将玩家定位为“文明的战略引导者”，并把“与其他玩家较量”列为核心动机。
- `doc/core/player-access-mode-contract-2026-03-19.prd.md` 已把当前玩家入口收口为 `viewer / pure_api`。
- `doc/engineering/governance/environment-lanes-and-inventory-2026-05-29.md` 已将项目环境收束为 `local / test / production`，并要求历史共享开发网络只作为 legacy/rehearsal 资产。
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md` 已将网络 tier 与 public/mainnet claims 绑定到 manifest 和 readiness gate。

## Upgrade Phases

### Phase 0: 冻结术语真值

Owner: `producer_system_designer` with `tpm` integration.

工作项：

- 将本计划纳入 core/project 任务入口。
- 在 core glossary 中新增“统一持久大世界 / unified persistent world”。
- 在玩家访问模式契约旁建立“世界模型不等于访问模式、不等于环境名”的边界说明。

验收：

- `rg -n "统一持久大世界|unified persistent world" doc/core README.md world-rule.md`
- `rg -n "玩家访问模式|viewer|pure_api|world_id" doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md`
- `./scripts/doc-governance-check.sh`

### Phase 1: Product-facing 文档迁移

Owner: `producer_system_designer`; review: `liveops_community`, `qa_engineer`.

工作项：

- 更新 `README.md`、`world-rule.md`、`doc/game/gameplay/gameplay-top-level-design.prd.md`、`site/**` 中的默认世界叙事。
- 将“当前阶段”写成产品 readiness 状态，而不是世界类型。
- 将所有玩家可见入口文案改为“进入/观察/影响统一持久大世界”。

验收：

- active product/player-facing docs 中 legacy term 命中数为 0，除非同一行含 `legacy`、`historical`、`rehearsal` 或 `compatibility`。
- `README.md` 第一屏能同时表达“统一持久大世界目标”和“limited playable technical preview 边界”。
- site 文案不把 test/public/mainnet 作为玩家世界品牌。

### Phase 2: QA / Claim Boundary 迁移

Owner: `qa_engineer`; review: `producer_system_designer`.

工作项：

- 在 `testing-manual.md` 中增加 unified world claim taxonomy。
- 将 readiness 拆成：
  - unified world product model aligned
  - local implementation evidence
  - test environment candidate evidence
  - production/mainnet readiness evidence
- 新增 forbidden claim examples，避免“统一大世界”被误读成已正式上线。

验收：

- QA gate 能区分“术语已迁移”和“runtime/network readiness 已通过”。
- 测试报告必须绑定 `mode_id`、environment、candidate bundle、`world_id`，但用户摘要默认使用统一大世界语言。
- 旧共享开发网 pass 或同类 legacy evidence 不得作为 unified world readiness 的唯一依据。

### Phase 3: Runtime / Ops Compatibility 迁移

Owner: `runtime_engineer`, `blockchain_ops_engineer`; review: `qa_engineer`.

工作项：

- 保留 `world_id`、network tier、manifest、topic、storage path 的技术含义。
- 为新文档提供 alias map：
  - user-facing world: unified persistent world
  - runtime partition key: `world_id`
  - environment: `local` / `test` / `production`
  - network tier: `local_devnet` / `public_testnet` / `mainnet`
- 脚本输出面向 operator 时继续可显示技术 tier；面向玩家或 release summary 时必须转换为统一大世界口径。

验收：

- 技术脚本兼容不破坏现有 manifest validation、checkpoint/replay、p2p topic、storage path。
- 新增或更新的 operator docs 明确“环境/tier 是运行载体，不是玩家世界名”。
- 任何重命名脚本参数的任务必须提供 backward-compatible alias 和 migration notice。

### Phase 3B: Code-layer Follow-up TODO

Owner: `runtime_engineer`, `blockchain_ops_engineer`; review: `qa_engineer`, `repository_health_engineer`.

目的：把本轮术语/脚本迁移后暴露的代码层剩余问题独立收口，避免把“文档与 operator 入口已迁移”误判为“所有 runtime/bin/release gate 已完全通过”。

工作项：

- 修复 `oasis7_chain_runtime` bin test 编译面漂移：
  - `oasis7_node::LiveTransportTransition`
  - `oasis7_node::LiveTransportTransitionCounters`
  - `NodeConsensusSnapshot.storage_challenge_network_degraded_{height,reason}`
- 确认 `status_payload.rs`、observability tests 与当前 `oasis7_node` API 的真实字段/类型契约，选择恢复字段、改用新字段，或删除过期测试断言。
- 收口 release gate 下游长跑脚本的 shell 运行时契约：
  - 方案 A：把 `scripts/p2p-longrun-soak.sh`、`scripts/s10-five-node-game-soak.sh` 等脚本改成 macOS Bash 3.2 兼容，移除 `mapfile` / `declare -A`。
  - 方案 B：显式要求 Bash 4+，并在入口 preflight 中给出清晰错误，而不是运行到中途失败。
- 为 legacy wrapper 设定退场策略：
  - `scripts/shared-network-track-gate.sh`
  - `scripts/shared-devnet-rehearsal.sh`
  - `scripts/shared-devnet-blocker-packet.sh`
  - 退场前必须确认外部 automation、evidence scripts、operator runbook 不再调用旧入口。
- 增加或扩展代码层 terminology / compatibility scan，确保 `shared_devnet` 不再回到 manifest tier、runtime readiness branch、new fixture 或 new output schema。

建议验证：

- `./scripts/cargo-dev.sh test -p oasis7 --lib network_tier_manifest -- --nocapture`
- `./scripts/cargo-dev.sh test -p oasis7 --bin oasis7_chain_runtime network_tier -- --list`
- `./scripts/release-gate-smoke.sh`
- `rg -n "shared_devnet|shared-network|shared network|shared-devnet" crates scripts doc/testing/templates testing-manual.md`
- `git diff --check`

验收：

- `oasis7_chain_runtime` bin test 至少能完成 network-tier 相关 test discovery，不再被 status/observability API 漂移阻断。
- release gate smoke 在本仓库声明支持的 shell runtime 下给出完整 pass 或明确、前置的环境要求错误。
- legacy wrapper 只作为迁移窗口兼容入口存在，不产生旧 schema、旧 track、旧 manifest tier 或玩家可见旧语义。

### Phase 4: Automated Terminology Gate

Owner: `repository_health_engineer`; review: `producer_system_designer`, `qa_engineer`.

工作项：

- 增加文档术语扫描脚本或扩展现有 doc governance。
- 对 active player-facing/product docs 启用 forbidden term gate。
- 对 historical/evidence/docs 降级为 warning，并要求限定词。

建议规则：

```text
deny in active product docs:
  legacy shared-development machine token
  shared devnet
  shared network
  hosted world
  public testnet world
  local devnet world

allow with qualifier in historical/technical docs:
  legacy
  historical
  rehearsal
  compatibility
  operator
  manifest
  world_id
```

验收：

- `./scripts/doc-governance-check.sh` 或新增脚本能阻断 active docs 中的未限定 legacy term。
- historical/evidence paths 不被批量重写，也不会导致 doc governance 误报。
- PR template / release summary checklist 增加 unified world terminology check。

## Acceptance Criteria

- AC-1: 新增或更新的玩家/产品文档默认使用“统一持久大世界 / unified persistent world”。
- AC-2: active player-facing docs 不再把环境、network tier、candidate window 或 `world_id` 写成玩家世界名。
- AC-3: legacy term 只能出现在历史证据、技术兼容、operator/runbook 或 migration 文档中，且必须带限定词。
- AC-4: QA 文档能同时表达“统一大世界是默认模型”和“readiness 仍需 evidence gate”。
- AC-5: runtime/ops 技术字段保持兼容；不得为了产品术语迁移破坏 replay、storage、topic、manifest 或 script 参数。
- AC-6: 对外 claim 必须避免宣称正式上线、无限容量、mainnet live、production settlement 或无准入限制，除非相应 readiness gate 已过。

## Residual Risks

- 术语迁移可能让外部读者误以为正式大世界已上线；必须用 limited preview / readiness gate 约束 claim。
- 历史证据文件名大量包含 legacy term，强制重命名会破坏追溯链；本计划选择保留历史文件名。
- `world_id` 是必要技术字段，不能因为产品叙事统一而删除；风险在于 UI/报告层继续把它暴露成“世界选择器”。
- network tier 仍是 p2p/runtime 必要概念；迁移重点是隐藏其玩家世界含义，而非消灭运维术语。
- 当前未实际完成专业 subagent 实现/验证 slice；后续执行 Phase 1-4 前仍需按 repo workflow 派发相应角色 review。

## Decision Record

| Decision ID | 决策 | 被否决方案 | 依据 |
| --- | --- | --- | --- |
| DEC-UW-001 | 玩家/产品默认模型统一为“统一持久大世界”。 | 继续把不同环境或候选窗口写成不同世界。 | 用户明确要求“整个游戏默认就是个统一大世界”。 |
| DEC-UW-002 | 历史 evidence 和兼容脚本不批量重命名，只加限定词和引用规则。 | 全仓机械删除 legacy term。 | 防止破坏 QA 证据、runbook、manifest、脚本兼容。 |
| DEC-UW-003 | `world_id` 保留为 runtime 技术键，不上升为产品世界名。 | 将 `world_id` 直接映射给玩家作为多个世界。 | 统一大世界叙事与 runtime 分区/恢复机制需要分层。 |
| DEC-UW-004 | readiness 与术语迁移分开验收。 | 术语改完即宣称大世界 ready。 | 避免 product claim 越过 runtime、QA、ops evidence。 |
