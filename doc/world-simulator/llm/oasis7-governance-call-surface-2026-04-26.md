# oasis7 治理调用面与首个 agent claim 审批闭环（2026-04-26）

## 目的

这份分册只解决一个问题：当前 `oasis7` 产品主链路里，“新账号认领第一个 agent / slot-1”到底有哪些真实可调用接口、哪些状态会进入链上/runtime 真值、运营者如何审批，以及玩家应该从哪里回读结果。

## 当前真实产品链路

1. `oasis7-run.sh play` 或 bundle `run-game.sh` 默认会拉起 `oasis7_game_launcher`，并进一步启动 `oasis7_chain_runtime`，除非显式传 `--chain-disable`。
2. 玩家在前台看到的状态，应该以 `snapshot.player_gameplay.agent_claim` 为准，而不是只看按钮点击是否返回成功。
3. 现在首个 agent claim 已经有 dedicated chain-runtime 直调面：
   - `POST /v1/chain/agent-claim/approval-request/submit`
   - `GET /v1/chain/agent-claim/approval-requests`
   - `POST /v1/chain/agent-claim/approval-request/approve`
   - `POST /v1/chain/agent-claim/approval-request/reject`
   - `POST /v1/chain/agent-claim/submit`
4. 这些接口都会先基于当前 execution world 做 preflight，再把动作送进 `oasis7_chain_runtime` 的 consensus action 队列。
5. 真正的最终结果要等 commit 后，再通过 snapshot 读回。

## 玩家侧闭环

### 1. 玩家提交首个 claim 审批申请

```bash
curl -sS http://127.0.0.1:8765/v1/chain/agent-claim/approval-request/submit \
  -H 'Content-Type: application/json' \
  -d '{"claimer_agent_id":"agent-0"}'
```

请求体：

```json
{
  "claimer_agent_id": "agent-0"
}
```

当前 preflight 约束：

- `claimer_agent_id` 必须存在
- 只支持首个 slot，也就是 slot 1
- 当前已有 `pending` 审批单时不能重复提交
- 已有 active restricted grant 或 restricted balance 时不能再申请

成功后会在 runtime/world state 里落一条 `FirstAgentClaimApprovalRequested`，并生成 `request_id`。

### 2. 玩家从 snapshot 回读自己的审批状态

现在玩家可见快照新增：

- `player_gameplay.agent_claim.first_agent_claim_approval_request`

当前会返回：

- `request_id`
- `status`
- `requested_slot_index`
- `requested_reputation_tier`
- `requested_total_upfront_amount`
- `requested_at_epoch`
- `updated_at_epoch`
- `operator_account_id`
- `approved_amount`
- `expires_at_epoch`
- `rejection_reason`

所以“点了 claim / submit 之后发生了什么”现在不是黑箱，前台可以直接回读。

## 运营审批闭环

### 1. 运营者查看 pending 队列

```bash
curl -sS 'http://127.0.0.1:8765/v1/chain/agent-claim/approval-requests?status=pending'
```

可选过滤：

- `status=pending|approved|rejected`
- `claimer_agent_id=<agent-id>`

这一步读取的是 committed world state，不是本地缓存文案。

### 2. 运营者 approve

```bash
curl -sS http://127.0.0.1:8765/v1/chain/agent-claim/approval-request/approve \
  -H 'Content-Type: application/json' \
  -d '{"operator_account_id":"liveops","request_id":1,"expires_at_epoch":10}'
```

当前 approve 约束：

- `operator_account_id` 必须在 `restricted_starter_claim_admin_account_ids` allowlist 里
- request 必须仍是 `pending`
- `expires_at_epoch` 必须有效

approve 成功后不是只改一个状态字段，而是会在同一条 runtime 事件流里：

1. 把审批单标记为 `approved`
2. 从 `restricted_starter_claim_liveops_pool` 发出 slot-1 restricted grant
3. 把 restricted balance 记到 claimer 账户

也就是说，审批结果已经进入 runtime/链上真值，不再只是文档口径。

### 3. 运营者 reject

```bash
curl -sS http://127.0.0.1:8765/v1/chain/agent-claim/approval-request/reject \
  -H 'Content-Type: application/json' \
  -d '{"operator_account_id":"liveops","request_id":1,"reason":"manual_review_failed"}'
```

reject 成功后会把 request 状态落成 `rejected`，并保存 `rejection_reason`。

## 玩家 claim 闭环

当 snapshot 显示审批已 `approved`，且 `restricted_starter_claim_balance` 已到账后，玩家可以直接调用：

```bash
curl -sS http://127.0.0.1:8765/v1/chain/agent-claim/submit \
  -H 'Content-Type: application/json' \
  -d '{"claimer_agent_id":"agent-0","target_agent_id":"agent-1"}'
```

请求体：

```json
{
  "claimer_agent_id": "agent-0",
  "target_agent_id": "agent-1"
}
```

当前 runtime 规则仍然成立：

- slot 1 可以使用 `restricted_starter_claim_balance`
- slot 2/3 仍然只允许 liquid balance
- activation fee / bond / upkeep / release cooldown / idle reclaim 仍由 runtime 规则控制

## “会不会上链/被记录” 的边界

现在这套 dedicated `/v1/chain/agent-claim/**` 直调面，走的是：

1. chain-runtime preflight
2. consensus action submit
3. commit
4. snapshot readback

因此：

- `approval-request/submit`、`approve`、`reject`、`agent-claim/submit` 都会进入链/runtime 的正式记录链路
- HTTP `ok=true` 只表示 preflight 通过并已入队
- 玩家和运营者都应以 commit 后 snapshot / request list 为最终真值

## 当前安全边界

这套接口现在已经可直调、可运营、可回读，但还应明确：

- 它们当前是 repo-owned internal control-plane API
- 还不应写成“任何公网客户端都可直接安全调用的 auth-hardened public API”
- 真正 public 化前，还需要额外鉴权/暴露策略设计

## 关联入口

- skill 分发入口：`site/skills/oasis7.md`
- runtime API：`crates/oasis7/src/bin/oasis7_chain_runtime/agent_claim_api.rs`
- 玩家快照：`crates/oasis7/src/viewer/runtime_live/claim_snapshot.rs`
- 模块主 PRD：`doc/world-simulator/prd.md`
- 模块项目台账：`doc/world-simulator/project.md`
