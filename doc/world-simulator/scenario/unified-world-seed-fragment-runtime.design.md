# oasis7 Simulator：统一大世界种子与 Fragment Runtime 复用（设计文档）

- 对应需求文档: `doc/world-simulator/scenario/unified-world-seed-fragment-runtime.prd.md`
- 对应项目管理文档: `doc/world-simulator/scenario/unified-world-seed-fragment-runtime.project.md`

## 1. 设计定位
本专题把“统一大世界”定义为一个 chain-authoritative、seed-derived world。所有环境只允许从同一个世界种子派生 chunk、fragment、资源预算和 starter spawn；正式资源事实必须由链上 genesis/commit 承载或引用。runtime live、viewer、provider bridge 与 testnet 节点是同一链上世界事实的消费者和提交者，不再各自发明地点和资源。

## 2. 现状对齐
已有能力：
- `chunk_seed(world_seed, coord)` 已提供稳定 chunk 随机根。
- `generate_chunk_fragments` 已把 chunk 生成落到 `WorldModel.locations`、`fragment_profile`、`fragment_budget` 与 `chunk_resource_budgets`。
- `chunked-fragment-generation` 统一定义 starter core、min fragments、material distribution 与 replenish 语义。

缺口：
- runtime live 的 formal release path 仍用固定 material/electricity bootstrap。
- runtime snapshot 映射层会从 Agent 坐标派生 `runtime:x:y:z` Location。
- first Agent claim 和 starter resource flow 尚未绑定 seed-derived starter chunk。
- chunk/resource 生成目前可以在 simulator/runtime 本地完成；正式链路还缺少 chain commit 级的 resource provenance、remaining budget delta 与 replay 验证合同。

## 3. Canonical World Seed Contract
### 3.1 Seed Manifest
每个统一大世界需要一个 seed manifest，至少包含：
- `world_id`
- `world_seed`
- `chunk_generation_schema_version`
- `asteroid_fragment_seed_offset`
- `world_config_hash`
- `genesis_ref` / `chain_id`（testnet/public 环境）

该 manifest 是 runtime live、chain runtime、viewer snapshot 和 provider observation 的共同输入或可验证引用。

### 3.2 Seed 派生
统一派生链：

```text
world_seed
  -> asteroid_fragment_seed = world_seed + seed_offset
  -> chunk_seed(asteroid_fragment_seed, chunk_coord)
  -> generate_fragments(seed, chunk_bounds, AsteroidFragmentConfig)
  -> synthesize_fragment_profile(profile_seed, radius, material)
  -> synthesize_fragment_budget(fragment_profile)
```

任何 runtime-only 随机源、系统时钟随机、进程启动随机都不得参与正式资源生成。

## 4. Chain-Authoritative Resource Contract
### 4.1 链上资源事实
正式资源生成分为两个阶段：

1. Deterministic derivation：由 `world_seed + chunk_coord + chunk_generation_schema_version` 派生 fragment/profile/budget。
2. Chain commitment：把该 derivation 的结果摘要写入 genesis、chain commit 或可复放 execution record。

进入正式玩法的 chunk 必须有链上资源事实记录，至少包含：
- `world_id`
- `world_seed` 或 seed manifest hash
- `chunk_coord`
- `chunk_generation_schema_version`
- `asteroid_fragment_seed_offset`
- `fragment_ids`
- per-fragment `profile_hash`
- per-fragment `budget_total_hash`
- per-fragment `budget_remaining_hash`
- `chunk_resource_budget_hash`
- `commit_height` / `commit_hash` / `execution_record_ref`

### 4.2 Provenance 状态
资源地点必须显式标记 provenance：

- `chain_committed`: 已进入 genesis/commit，可用于正式采矿、生产、交易、provider observation。
- `chain_pending`: 已提交生成或变更请求，但尚未 committed；UI 可以展示等待态，provider 不得把它当作可消耗资源。
- `provisional`: runtime 本地预览或测试 fixture；只能用于调试/视觉预览，不得进入正式玩法账本。
- `compat_anchor`: `runtime:x:y:z` 坐标锚点；不是正式资源地点。

### 4.3 资源变更权威
`remaining` budget 的变化必须来自链上 action / commit：
- 采矿、回收、补种、生产消耗和 material ledger 增量都必须产生可重放的 resource delta。
- runtime 可以先做 optimistic/pending 显示，但 committed snapshot 必须以链上 delta 为准。
- viewer 刷新、provider observation 和 testnet replay 不得通过重新运行 generation 还原 `remaining`，只能从 committed delta 或 checkpoint 恢复。

## 5. Runtime Live 接入设计
### 5.1 World Bootstrap
formal release 默认世界启动时：
1. 加载或构造 seed manifest。
2. 从 chain/genesis/manifest 获取 starter chunk provenance；如果缺失，只能生成并提交 pending chunk，不得直接进入正式资源账本。
3. 初始化 `WorldConfig` 与 `WorldInitConfig`，启用 asteroid fragment。
4. 生成或验证 starter-safe chunk：
   - 默认选择世界中心 chunk；
   - 或从 manifest 指定 `starter_chunk_coord`；
   - 使用现有 `generate_chunk_fragments` 生成资源；
   - 比对链上 `profile_hash`、`budget_total_hash` 与 `chunk_resource_budget_hash`。
5. 从 starter chunk 内选择 spawn anchor：
   - 优先选择具备可采 budget 的 frag location；
   - 若无可用 frag，则按确定性 fallback 选择 chunk 中心并标记 `starter_chunk_resource_missing`。
6. 注册 starter Agent 到该 spawn anchor，Agent `location_id` 必须指向 seed-derived、chain-committed Location 或明确 fallback reason。

### 5.2 Location 语义
- `frag-*` / seed-derived / chain-committed Location：正式资源地点，可被 viewer、provider、采矿/生产链路引用。
- `runtime:x:y:z`：兼容坐标锚点，只能用于过渡、调试和无资源动作，不应作为正式可采地点。
- Viewer 需要把两者视觉区分：正式资源地点显示资源摘要；坐标锚点显示 fallback/compat badge。

### 5.3 Resource Ledger
正式资源读取顺序：
1. chain-committed resource delta / checkpoint
2. `Location.fragment_budget.remaining_by_element_g` with matching chain provenance
3. `WorldModel.chunk_resource_budgets[coord].remaining_by_element_g` with matching chain provenance
4. 已采矿/加工后的 runtime material ledger

固定 bootstrap material 只能作为 migration / test fixture / grant，不得被描述为世界自然资源。

## 6. Claim / Onboarding Flow
`claim_first_agent` 改为：
1. 确认世界 seed manifest 已加载。
2. 确认 starter chunk 已 chain-committed；若未 committed，则提交生成请求并返回等待态。
3. 选择 starter-safe spawn location。
4. 注册/绑定 Agent。
5. 发放 OC/LLM starter grant。
6. 将玩家引导到可见 starter resource pocket 的下一步操作。

如果任一步失败，返回明确错误：
- `world_seed_missing`
- `starter_chunk_generation_failed`
- `starter_chunk_not_committed`
- `starter_resource_provenance_missing`
- `starter_resource_pocket_missing`
- `starter_spawn_location_missing`

## 7. Viewer / Provider Observation
Snapshot 新增或确保暴露：
- `world_seed` 或 `world_seed_ref`
- `chunk_generation_schema_version`
- per-location `position_source`: `seed_fragment` / `runtime_coord_anchor` / `fallback`
- per-location `resource_provenance`: `chain_committed` / `chain_pending` / `provisional` / `compat_anchor`
- per-location `resource_commit_ref`
- nearby resource summary: dominant material, remaining key elements, chunk coord

Provider observation 使用同一 snapshot 派生，禁止单独查本地缓存生成不同资源摘要。

## 8. Migration Plan
### M1 文档与 contract
- 增加本专题 PRD/design/project。
- 在 runtime live/formal release 文档中引用 seed-authoritative contract。
- 明确 chain-authoritative resource contract。

### M2 Snapshot contract
- 为 runtime live snapshot 增加 seed/version/resource provenance 字段。
- Viewer 显示正式资源地点与 compat anchor 区分。

### M3 Runtime bootstrap
- formal release 默认世界改为 seed manifest bootstrap。
- `claim_first_agent` 改为 starter chunk spawn。

### M4 Chain resource commitment
- genesis 预提交 starter chunk resource manifest。
- chunk 按需生成时提交 `generate_chunk` action/result。
- 采矿/生产/补种提交 resource delta，committed snapshot 从 chain delta 恢复 remaining budget。

### M5 Testnet evidence
- public/local testnet readiness 增加 world seed 与 chunk generation schema 验证。
- provider bridge smoke 增加 observation seed/resource provenance 校验。
- 增加 refresh/replay 后 remaining budget 不被本地重算复原的验证。

## 9. Open Decisions
- `world_seed` 在现有 public_testnet genesis 中的字段名与存储位置。
- starter chunk 默认是否固定为世界中心，还是由 manifest 显式指定。
- first Agent 是否直接出生在 frag location，还是出生在附近 logical base location 并把 frag 作为 nearby resource。
- resource commitment 是写入现有 chain action/event schema，还是新增专用 `ChunkResourceManifest` / `ResourceDelta` execution record。
