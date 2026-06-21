# oasis7 Simulator：统一大世界种子与 Fragment Runtime 复用（需求文档）

- 对应设计文档: `doc/world-simulator/scenario/unified-world-seed-fragment-runtime.design.md`
- 对应项目管理文档: `doc/world-simulator/scenario/unified-world-seed-fragment-runtime.project.md`

## 1. Executive Summary
- 统一大世界、local testnet、public testnet 与 runtime live 必须共享同一套 `world_seed -> chunk -> fragment -> resource budget` 生成语义。
- 当前 simulator 已有 deterministic chunk/fragment/resource 生成能力；runtime live / formal release 默认世界仍主要依赖固定 bootstrap 库存与 `runtime:x:y:z` 坐标锚点，需要收敛。
- 新用户第一轮体验应从统一 seed 派生的 starter-safe chunk 开始，而不是单独硬编码一个与世界资源无关的 Agent/Location。
- 资源、出生点、可采碎片、Location 展示、viewer pixel-world 地形与链上/共识快照应来自同一份世界生成事实。
- 资源生成必须链上化：正式世界的 chunk/resource provenance、remaining budget 与后续采集变更必须能由 chain/genesis/commit 证明，runtime live 只能消费或提交这些事实，不能在链下生成一套正式资源账本。

## 目标
- 将统一大世界的 starter spawn、fragment location、resource budget 与后续 resource delta 收敛为同一份 chain-authoritative seed/chunk/fragment 事实。
- 让 local testnet、public testnet、viewer、runtime live 与 provider observation 对同一个 `world_seed` 和 resource provenance 给出一致解释。

## 范围
- In scope: seed manifest、chunk/fragment/resource provenance、starter chunk/onboarding 接线、viewer/provider observation 一致性、resource delta replay 合同。
- Out of scope: 新经济数值平衡、新资源品类、完整删除兼容坐标锚点、非 testnet/正式世界的调试 fixture 迁移。

## 接口 / 数据
- `SeedManifest` / `oasis7.world_resource_manifest.v1`: `world_id`、`chain_id`、`genesis_ref`、`world_seed`、`chunk_generation_schema_version`、`world_config_hash`、`generation_algorithm_id/hash`、`created_at_height/block_hash`。
- `ChunkResourceManifest` / `oasis7.world_resource_manifest.v1.generated_chunks`: chunk coord、`chunk_seed`、`chunk_status`（`committed` / `chain_pending` / `provisional` / `exhausted`）、fragment ids、profile hash、budget total hash、budget remaining hash、commit ref。
- `ResourceDelta` / `oasis7.world_resource_delta.v1`: delta id、action/event id、ordering key、fragment/location/chunk ref、material/resource key、delta amount、commit height/hash、base/result manifest hash、actor/source/replay status。
- `/v1/chain/status.world_resource`: testnet readiness 必须暴露 `schema_version`、`delta_schema_version`、`world_id`、`chain_id`、`world_seed`、`chunk_generation_schema_version`、seed/starter/latest commit hash、committed/provisional/pending delta counters、`readiness_status` 与 `failed_gates[]`。

## 里程碑
- M1: 文档合同冻结，明确资源生成必须链上化。
- M2: runtime live snapshot 暴露 seed/chunk/resource provenance。
- M3: claim/onboarding 绑定 starter chunk spawn 与 starter resource context。
- M4: chain commit/replay 支持 resource manifest 与 delta。
- M5: testnet readiness 证明 viewer/runtime/provider resource provenance 一致。

## 风险
- 既有 runtime live 与测试仍依赖固定 bootstrap 库存，需要阶段迁移。
- 链上承载完整 fragment/profile/budget 明细可能过重，需要 hash/manifest/ref 折中。
- optimistic UI 与 committed resource delta 之间需要清晰 pending/rollback 状态。

## 2. Problem Statement
当前存在两条语义：

- simulator/scenario 侧：已有基于 seed 的 chunk、fragment、材质、元素预算、补种和可采资源账本。
- runtime live/formal release 侧：默认启动会注册 starter Agent，Location 多数由 Agent 坐标派生；资源主要是固定启动库存。

这会导致本地测试入口看起来像大世界，但地点和资源并不是同一个世界生成系统的产物，后续接入 testnet / bridge / LLM 时容易出现“世界事实不一致”。

## 3. Requirements
### R1 统一世界种子
- 每个大世界必须有唯一 canonical `world_seed`。
- `world_seed` 必须写入 genesis / manifest / runtime snapshot 可验证字段。
- local testnet、public testnet、launcher、viewer 与 provider observation 必须引用同一世界种子或其派生证明。

### R2 复用现有 chunk/fragment/resource 生成
- 可玩大世界的地点与资源必须优先来自现有 simulator chunk/fragment 生成链路。
- `chunk_seed(world_seed, coord)` 是 chunk 内 fragment/resource 的唯一随机根。
- 资源可采量必须使用 `fragment_budget` / `chunk_resource_budgets` 的 total/remaining 账本，不允许运行时临时重算。

### R3 Runtime live 消费世界生成事实
- runtime live 不再自造一套独立资源世界。
- `runtime:x:y:z` Location 只能作为短期兼容锚点或调试 fallback；可玩入口不得把它当作正式资源地点。
- formal release 默认世界应加载或生成 starter-safe chunk，并从该 chunk 派生 starter spawn、nearby resource pocket 与初始可见目标。

### R4 新用户入口
- `claim_first_agent` 应将 Agent 绑定到统一大世界中的 starter-safe spawn，而不是固定 `(0,0,0)` 或另一个与资源生成无关的位置。
- 初始 OC / LLM 预算仍可以是 onboarding grant，但世界物料与可采资源应来自 starter chunk。
- 如果 starter chunk 生成失败，页面必须阻塞并显示明确世界生成错误，而不是展示空快照或假地点。

### R5 Viewer 与 Provider 观察一致
- Viewer 的 `model.locations`、pixel-world fragment terrain、resource summaries 与 provider observation 必须来自同一 snapshot。
- Provider 不能看到与玩家 viewer 不一致的 location/resource ids。
- UI 可以隐藏低层 frag 细节，但不能把派生锚点误标成正式资源地点。

### R6 资源生成全部链上化
- 正式资源生成事实必须由链上可验证记录承载，包括 `world_seed`、`chunk_generation_schema_version`、chunk coord、fragment ids、fragment profile hash、fragment budget hash、chunk resource budget hash。
- genesis 可以预提交 starter chunk；后续 chunk 可以按需生成，但生成请求、生成结果摘要和 schema/version 必须进入 chain commit 或等价可复放日志。
- runtime live 允许在本地预览未提交 chunk，但未提交资源只能标记为 `provisional`，不得进入正式采矿、生产、provider observation 或玩家可交易资源账本。
- `fragment_budget.remaining_*` 的减少必须由链上 action / commit 驱动；runtime/viewer 不得把本地重算结果当成 authoritative remaining balance。
- testnet readiness 必须能证明 viewer/runtime/provider 读取的是同一个链上 resource provenance，而不是各自本地重新生成后碰巧一致。

## 4. Non-Goals
- 本专题不新增复杂分布 DSL；继续复用现有 `AsteroidFragmentConfig`、material distribution 和 replenish 策略。
- 不在本专题定义完整经济平衡数值，只定义统一来源与接线合同。
- 不要求一次性删除所有 `runtime:x:y:z` 兼容代码；但正式可玩入口必须迁移到 unified seed path。

## 5. Acceptance Criteria
- AC1: 同一个 `world_seed + chunk coord` 在 simulator、runtime live snapshot 和 testnet replay 中生成一致 chunk/resource summary。
- AC2: formal release 默认入口首个可玩 snapshot 至少包含一个 seed-derived starter-safe location/fragment 或明确 empty-world onboarding state，不再只依赖坐标锚点。
- AC3: `claim_first_agent` 后 Agent 的 `location_id` 能解析到 seed-derived Location，且该 Location 具备可解释的 fragment/resource context。
- AC4: Viewer 与 provider observation 对同一 Agent 返回相同 `location_id`、附近可见资源摘要和世界 seed/version。
- AC5: 本地 testnet / public testnet readiness 证据能同时证明 `world_id`、`world_seed`、chain/genesis 与 viewer runtime snapshot 的一致性。
- AC6: starter chunk 的 seed/chunk/resource manifest 已在 genesis 或首个 chain commit 中出现，viewer snapshot 与 provider observation 都能引用该链上 provenance。
- AC7: 对同一 fragment 的采集会产生链上 resource delta；刷新或重放后 `remaining` budget 与链上 commit 一致，不会被 runtime 本地重算复原。
- AC8: 未提交的本地 preview chunk 在 UI/provider 中显示为 provisional 或不可用，不能被正式 gameplay action 消耗。
