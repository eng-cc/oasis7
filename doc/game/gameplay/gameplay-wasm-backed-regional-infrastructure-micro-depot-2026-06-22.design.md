# Gameplay WASM-backed Regional Infrastructure: micro_depot 设计（2026-06-22）

- 对应需求文档: `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.prd.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.project.md`
- 对应任务证据: GitHub issue / task `task_3c55fc4c17ea44aa850adf7a6a4463f4`；退役前 `.pm/tasks/*` execution log 仅通过 `.pm/github-project-sync/task-archive.jsonl` 作 migration/audit 追溯
- 关联玩法真值:
  - `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`
  - `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md`
  - `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
  - `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.prd.md`
  - `doc/world-simulator/viewer/viewer-pixel-world-player-leverage-production-readability-2026-05-28.brainstorm.md`

## 1. 设计定位

`micro_depot` 是第一版可编程区域设施，用来验证一件事：

> 玩家通过一个小型、可审计、带 upkeep 的区域设施，改变一次 repair / logistics 行动的条件。

它不是 Minecraft 式逐块建造，不是自由脚本，不是通用 UGC 市场，也不是 UI buff。它是一个 WASM-backed facility：WASM 负责规则化 proposal，runtime 负责权限、扣费、状态变更和 receipt。

核心原则：

```text
WASM proposes, runtime validates / applies / signs.
```

## 2. 玩家循环

目标体验：

1. 玩家看到区域阻塞，例如 `supply_missing` 导致 repair quote 高成本或高风险。
2. 系统建议部署 `micro_depot`。
3. 玩家查看 canonical quote：部署成本、upkeep、覆盖范围、预期收益、仍然存在的 blocker。
4. 玩家确认安装，消耗 starter funding / claim budget，并占用局部设施压力。
5. Runtime 调用 `micro_depot.wasm` 评估后续 repair / logistics quote。
6. Runtime 校验 proposal，生成 before / after preview。
7. 玩家执行行动，receipt 说明 depot 对本次行动的贡献。

玩家应该能回答：

- 我为什么需要这个 depot？
- 它花了什么？
- 它改变了哪一次 repair / logistics 行动？
- 还有什么 blocker 没解决？

## 3. 解锁阶段与准入流程

`micro_depot` 不属于首 10 分钟新手循环，也不应作为随时可点的自由建造物出现。它的正确阶段是中后期的 `regional specialist / limited-scope regional influence`：玩家已经理解 agent intent、repair / logistics blocker、claim / upkeep 成本和区域压力后，才解锁第一个可编程区域设施。

阶段定位：

- 前期：只暴露 agent intent、repair / logistics blocker、一次性协助或专家行动。目标是建立“玩家下指令，世界有可读反馈”的信任，不让设施建造抢走核心教学。
- 中期：当玩家完成至少一次 repair / logistics 闭环，并拥有稳定 agent claim 后，`micro_depot` 作为第一个持续性区域杠杆解锁。
- 后期：再扩展到 `route_contract`、`repair_station`、`audit_beacon`，以及多设施组合、模块升级和区域治理。

创建准入必须来自游戏流程，而不是裸 action：

1. 当前区域存在重复或高价值 blocker，例如 `supply_missing`、`route_blocked`、repair 等待过长或 logistics quote 风险过高。
2. 玩家已完成第一次 repair / logistics 闭环，理解 before / after quote 和 receipt。
3. 玩家拥有可操作 agent claim，且 claim / agent 与目标区域存在合理关系。
4. 目标 location / facility 已生成，且处于 depot 服务半径与区域设施上限规则内。
5. 玩家可支付 install 成本和后续 upkeep；restricted starter funding 不能被扩成无限设施补贴。
6. 系统通过 regional pressure card、repair 失败反馈或 logistics bottleneck 暴露部署建议。
7. Runtime 在 `InstallMicroDepot` 前再次校验 owner、claim scope、资源、upkeep 初始化、重复设施、module hash / schema allowlist。

当前内核 first slice 的硬约束：

- `InstallMicroDepot` 必须携带 `regional_blocker_receipt_id`，它代表前置 repair / logistics blocker 闭环产出的 receipt；没有这个 receipt，runtime 直接拒绝创建。
- 同一 `owner_claim_id + location_id` 只能存在一个 `micro_depot`，避免把它变成自由堆叠建造物。
- 安装会扣固定 `Data` 成本，upkeep 会扣固定 `Data` 成本；数值先作为 kernel 常量落地，后续再迁入平衡表。
- 服务动作必须由 depot owner agent 提交；非 owner 不能触发服务，也不能消耗 owner 资源。
- 服务 quote 必须与 facility pinned 的 `module_id / wasm_hash / entrypoint` 匹配，且 WASM 提议消耗的资源必须属于 facility 的 `supported_resource_kinds`。
- `MicroDepotServiceApplied` receipt 记录 `schema_version / module_id / module_version / wasm_hash / entrypoint / proposal_hash`，保证玩家可见 receipt 和治理审计能追溯到模块证据。

玩家不应该从全局菜单直接“建一个 depot”。推荐入口是：

```text
regional pressure card
  -> repair/logistics blocker explanation
  -> suggested intervention: deploy micro_depot
  -> install quote + upkeep + service radius preview
  -> player confirms
  -> MicroDepotInstalled receipt
  -> subsequent repair/logistics service preview and receipt
```

设计约束：

- `micro_depot` 是区域专业化阶段的第一个可编程基础设施，不是新手建造物。
- 没有区域 blocker、没有稳定 claim、没有可读 receipt 时，不应出现部署入口。
- 玩家只看到“部署建议 / quote / receipt / upkeep 状态”，不直接接触 WASM 内部规则。
- 如果用于 onboarding，只能作为后续目标预告，不能作为首轮必做任务。

## 4. Facility Taxonomy

第一批区域设施只保留四类，但 MVP 只实现 `micro_depot`。

| Schema | 用途 | 首要玩家价值 | 当前阶段 |
| --- | --- | --- | --- |
| `micro_depot` | 小型区域补给点 | 让附近 repair / logistics quote 更可执行 | MVP |
| `route_contract` | 区域路线承诺 / 运输合约 | 把不稳定路线变成可重复路径 | 后续 |
| `repair_station` | 区域修复服务点 | 降低重复维修成本 | 后续 |
| `audit_beacon` | 风险 / 低效节点标记器 | 揭示隐藏阻塞和异常消耗 | 后续 |

非目标：

- 不做 block placement / digging / terraforming。
- 不开放任意玩家上传 WASM。
- 不让设施绕过 claim / upkeep 成本。
- 不让设施直接提供 global governance 权力。
- 不把 starter funding 扩成无限补贴。

## 5. 一步到位 WASM-backed 架构

第一版不做 runtime-only stub。`micro_depot` 从第一版开始就是 WASM-backed，但 WASM 只参与 proposal。

### 5.1 First-Slice Flow

```text
repair/logistics quote request
  -> runtime builds MicroDepotEvalInput
  -> runtime calls micro_depot.wasm::evaluate_quote(input)
  -> wasm returns MicroDepotProposal
  -> runtime validates proposal
  -> runtime emits canonical preview
  -> player confirms action
  -> runtime validates again or checks proposal_hash
  -> runtime applies accounting/state mutation
  -> runtime emits events and receipt
  -> viewer / pure API render canonical DTO
```

Runtime validation includes:

- owner / claim scope
- distance and service radius
- upkeep state
- supported resource kind
- resource existence
- effect cap
- module hash / schema version
- replay-safe proposal hash

## 6. Minimal ABI

### 6.1 Input

```ts
type MicroDepotEvalInput = {
  schema_version: string
  module_id: string
  module_version: string
  action: {
    action_id: string
    kind: "repair" | "logistics"
    target_id: string
    base_cost_class: number
    base_risk_class: number
    blocker_type?:
      | "supply_missing"
      | "route_blocked"
      | "upkeep_unpaid"
      | "permission_denied"
  }
  player: {
    account_id: string
    claim_id: string
  }
  depot: {
    facility_id: string
    status: "active" | "degraded" | "upkeep_grace" | "suspended"
    owner_claim_id: string
    capacity_class: number
    supported_resource_kinds: string[]
    service_radius_cm: number
    upkeep_paid: boolean
  }
  context: {
    distance_to_target_cm: number
    resource_gap_class: number
    route_pressure_class: number
    region_pressure_class: number
  }
}
```

### 6.2 Output

```ts
type MicroDepotProposal = {
  decision: "applicable" | "not_applicable" | "blocked"
  proposal_id: string
  effect: {
    cost_delta_class?: number
    risk_delta_class?: number
    wait_delta_class?: number
    blocker_change?: {
      from: string
      to: string
    }
  }
  consumed_resource_classes: string[]
  explanation_code:
    | "DEPOT_WITHIN_RANGE"
    | "DEPOT_OUT_OF_RANGE"
    | "DEPOT_UPKEEP_MISSING"
    | "RESOURCE_KIND_UNSUPPORTED"
    | "ROUTE_STILL_BLOCKED"
  proposal_hash: string
}
```

WASM 不允许输出真实库存变更、扣费结果、ownership 变更或 canonical world-state patch。

## 7. Canonical Runtime Model

### 7.1 State

```text
RegionalInfrastructure {
  facility_id
  kind = micro_depot
  region_id
  location_id
  fragment_anchor_cm
  owner_account_id
  owner_claim_id
  installer_agent_id
  status
  claim_scope
  capacity_class
  service_radius_cm
  supported_resource_kinds
  upkeep_per_epoch
  upkeep_paid_through_epoch
  funding_provenance
  module_id
  module_version
  module_hash
  schema_version
  last_proposal_hash
  last_blocker_type
  last_receipt_id
  created_tick
  updated_tick
}
```

### 7.2 Actions

- `QuoteMicroDepotInstall`
- `InstallMicroDepot`
- `EvaluateMicroDepotQuote`
- `ServiceRepairFromDepot`
- `ServiceLogisticsFromDepot`
- `PayDepotUpkeep`
- `SuspendDepot`
- `ReclaimDepot`
- `InspectDepot`

### 7.3 Events

- `MicroDepotInstallProposed`
- `MicroDepotInstallAccepted`
- `MicroDepotInstalled`
- `MicroDepotQuoteEvaluated`
- `MicroDepotServiceApplied`
- `MicroDepotBlocked`
- `MicroDepotUpkeepSettled`
- `MicroDepotEnteredGrace`
- `MicroDepotSuspended`
- `MicroDepotReclaimed`

所有 player-facing accepted action 必须能追溯：

- `accepted_intent_id`
- `execution_status`
- `blocker_type`
- `world_change_summary`
- `receipt_id`
- `module_id / module_version / module_hash / proposal_hash`

## 8. Viewer / Pure API DTO

Viewer 和 pure API 不读取 WASM，不猜测收益，只渲染 runtime DTO。

必需 DTO：

- `infrastructure_nodes[]`
  - id, kind, status, anchor, owner claim, upkeep state, visible effect
- `facility_details`
  - upkeep, service radius, supported actions, module identity, provenance, next useful action
- `action_previews[]`
  - before quote, after quote, depot contribution, remaining blockers
- `player_action_receipts[]`
  - before / after cost, risk, wait, route, repair/logistics outcome
- `depot_contributions[]`
  - depot id, proposal hash, effect class, explanation code
- `bottlenecks[]`
  - missing resource, unpaid upkeep, out of range, permission denied, degraded
- `module_evidence`
  - module id, version, hash, schema version, proposal hash

## 9. Governance Minimum

MVP 只允许 repo-authored module，不开放玩家上传任意 WASM。

治理最小面：

- allowlist `module_id + version + hash`
- runtime 拒绝 unknown module hash
- schema version 必须匹配 runtime adapter
- module upgrade 需要显式 migration rule，否则设施 pinned 到旧 hash
- receipt 必须记录 module identity
- old receipt replay 必须使用原始 module hash

## 10. 安全与确定性约束

WASM 模块必须满足：

- no wall clock
- no network
- no filesystem
- no host escape
- no floating-point drift
- bounded memory / fuel
- no RNG，除非 runtime 提供 deterministic seed
- pure function：input snapshot + module hash -> proposal

WASM 永远不能：

- mint resources
- bypass upkeep
- change ownership
- expand claim scope
- transfer restricted starter funds
- write canonical world state

Runtime 必须：

- cap all deltas
- validate all permissions
- validate all resource mutations
- emit structured blockers
- record proposal hash in replay / audit
- preserve funding provenance

## 11. 实施切片

1. WASM ABI + host adapter
   - Gate: same input produces byte-identical proposal hash.
2. `micro_depot` canonical state / events
   - Gate: install / service / reclaim replay produces identical state.
3. quote pipeline integration
   - Gate: repair / logistics preview shows before / after with depot contribution.
4. runtime validation / apply path
   - Gate: invalid owner, unpaid upkeep, out-of-range, unsupported resource reject with structured blockers.
5. DTO / viewer surface
   - Gate: pure API and viewer show same quote, blocker, receipt, module evidence.
6. receipt / audit
   - Gate: every applied service records module hash, proposal hash, before / after quote, world change summary.
7. gameplay smoke
   - Gate: one repair / logistics action becomes measurably cheaper, faster, or less risky because of depot, and player can identify the remaining blocker.

## 12. 10 分钟验证样本

```text
0-2 min: player sees regional pressure card: repair target blocked by supply shortage
2-4 min: player opens repair quote and sees high cost / high risk
4-6 min: system suggests micro_depot; player reviews cost, upkeep, radius, expected effect
6-8 min: micro_depot.wasm proposal changes quote; runtime emits before / after preview
8-10 min: player executes repair/logistics action and receives receipt
```

通过标准：

- 玩家能说明为什么部署 depot。
- 玩家能说明 depot 花了什么。
- 玩家能指出哪一次行动被 depot 改变。
- 玩家能看到 remaining blocker 或下一步建议。

## 13. 风险

- 若 quote / receipt 字段不足，viewer 会退回 recent event 猜因果。
- 若收益太弱，depot 像税；太强，depot 变成唯一正确解。
- 若 upkeep 太早压迫新手，会削弱 starter funding 的引导价值。
- 若 module governance 提前开放玩家上传，会把 MVP 变成安全平台问题。
- 若后续设施扩展太快，oasis7 会被误读成建造游戏，而不是间接控制文明模拟。
