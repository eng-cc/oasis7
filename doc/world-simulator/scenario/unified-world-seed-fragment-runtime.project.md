# oasis7 Simulator：统一大世界种子与 Fragment Runtime 复用（项目管理文档）

- 对应需求文档: `doc/world-simulator/scenario/unified-world-seed-fragment-runtime.prd.md`
- 对应设计文档: `doc/world-simulator/scenario/unified-world-seed-fragment-runtime.design.md`

## 目标
将统一大世界、runtime live、testnet、viewer 与 provider observation 对齐到同一套 chain-authoritative 的 `world_seed -> chunk -> fragment -> resource budget -> resource delta` 生成事实。

正式资源必须链上化：genesis/commit/execution record 承载或引用 resource provenance，runtime live、viewer、provider 只消费同一链上事实；本地 provisional 生成不得进入正式采矿、生产、交易或 provider observation。

## 任务拆解
1. UWS1.1 completed: 记录设计层统一口径，统一大世界复用 simulator/chunk/fragment 生成链路。
2. UWS1.1a completed: 记录 chain-authoritative resource contract，资源生成、remaining budget 与 resource delta 必须由链上 provenance 证明。
3. UWS1.2 planned: 定义 seed/resource manifest 的 runtime / chain / viewer 字段位置。
4. UWS1.3 planned: 定义 `ChunkResourceManifest` / `ResourceDelta` 的 chain action/event/execution-record 归属。
5. UWS2.1 planned: 为 runtime live snapshot 增加 world seed / schema / resource provenance。
6. UWS2.2 planned: Viewer 区分 seed-derived resource Location 与 runtime coord anchor。
7. UWS2.3 planned: Provider observation 暴露并校验同一 `resource_commit_ref`。
8. UWS3.1a completed: formal release 默认 bootstrap 改为从 simulator seed model 派生 starter Agent 与 fragment location。
9. UWS3.1b planned: formal release 默认 bootstrap 改为验证 starter chunk 的 chain-committed resource manifest。
10. UWS3.2 planned: `claim_first_agent` 改为绑定 starter chunk spawn location。
11. UWS3.3 planned: starter OC/LLM grant 与 starter resource pocket 联动展示。
12. UWS4.1 planned: genesis 预提交 starter chunk resource manifest，或在首个 committed action 中提交 starter chunk manifest。
13. UWS4.2 planned: chunk 按需生成走 chain action/result，未 committed 前标记 `chain_pending` 或 `provisional`。
14. UWS4.3 planned: 采矿/生产/补种写入 chain resource delta，刷新/replay 后 `remaining` 不被本地重算复原。
15. UWS5.1 planned: provider observation 从 runtime snapshot 派生统一 location/resource 摘要。
16. UWS5.2 planned: testnet readiness 增加 world seed / chunk schema / resource commit ref 一致性证据。
17. UWS5.3 planned: replay smoke 验证同一 fragment 采集后的 remaining budget 与链上 delta 一致。

## 验收
- 同 seed + chunk coord 在 simulator、runtime live 与 testnet replay 中生成一致资源摘要，并能引用同一链上 resource provenance。
- 新用户进入后看到的第一个可玩目标来自 seed-derived starter area。
- Viewer 与 provider 对同一 Agent 报告同一 location/resource context。
- 兼容坐标锚点不再被标为正式资源地点。
- 资源生成和 resource delta 可以通过 genesis/commit/execution record 重放；刷新后不会因 runtime 本地重算而恢复已消耗资源。

## 风险
- 现有 runtime live 测试大量依赖固定 bootstrap 库存，需要分阶段迁移。
- public_testnet 已有 genesis 可能缺少 explicit `world_seed` 字段，需要兼容读取或新增 manifest 引用。
- 若直接把 Agent 放在 frag 上，视觉和采矿语义可能混淆；可能需要 base location + nearby frag 的双锚点模型。
- 链上写入完整 fragment/profile/budget 明细可能过重，需要以 hash/manifest/execution-record ref 平衡可验证性与链上负载。
- committed resource delta 与 optimistic runtime UI 之间需要清晰 pending/rollback 语义，否则玩家可能看到资源短暂“复原”。

## 依赖
- simulator chunk/fragment/resource generation 现有确定性生成器。
- runtime live snapshot / chain runtime committed execution world。
- viewer pixel-world Rust bridge render-state derivation。
- provider observation 与 local/public testnet readiness evidence。

## 状态
- 当前状态: documentation contract frozen; first runtime/bootstrap alignment landed in PR #547.
- 已完成: seed/chunk/fragment 统一口径、chain-authoritative resource contract、formal-release starter spawn seed derivation。
- 待办: chain resource manifest/delta schema、committed starter chunk provenance、provider/readiness replay evidence。
