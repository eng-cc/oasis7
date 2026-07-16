# Gameplay Long-Run P0：Replay/Rollback 运行手册（2026-03-06）

审计轮次: 5

- 关联 PRD：`doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
- 覆盖任务：`TASK-GAME-014`（`PRD-GAME-006-02`）

## 1. 触发条件
- `verify_tick_consensus_chain()` 返回 `DistributedValidationFailed`。
- `first_tick_consensus_drift()` 返回非空（可定位 `mismatch_tick`）。
- 长稳门禁出现共识链路漂移告警，需要执行状态恢复演练或实战回滚。

## 2. 标准处置流程（Runbook）
1. 漂移定位：调用 `first_tick_consensus_drift()` 获取首个 `mismatch_tick` 与原因。
2. 影响区间确认：锁定最近稳定 `snapshot` 与对应 `journal`。
3. 权限注册表预检：确认 runtime 已配置两条独立且启用的 Ed25519 authority，角色分别为 `on_call` 与 `governance`，authority id 与公钥均不同。注册表未配置或 authority 不匹配时禁止发起回滚。
4. 签名信封确认：两位签名者分别对 schema v1 canonical intent 签名；intent 必须精确绑定 `rollback_ticket`、snapshot hash/journal len、`target_batch_id`、reason、签发/过期时间与一次性 nonce。Viewer wire 的 `approval` 是可选字段，因此旧客户端仍可被解码，但无签名 v1 请求会明确 fail closed 为 `rollback_approval_required`，不代表存在 Hello/capability negotiation。
5. 回滚恢复：在信封有效期内执行 `rollback_to_snapshot_with_reconciliation(snapshot, journal, reason, approval, now_ms)`；runtime 是签名、registry、有效期和 nonce 的唯一授权判定方，Viewer 只转发完整信封并在变更 Viewer 状态前处理 runtime 结果。
6. 恢复对账：再次执行 `first_tick_consensus_drift()` 与 `verify_tick_consensus_chain()`，必须均为“无漂移”。
7. 防重放与审计：确认 nonce 已持久消费；相同信封再次提交必须被拒绝且状态不变。确认 `RollbackApplied` 已写入事件链，并保留 ticket、两个 authority id 与 authorization nonce。

## 3. 演练命令（required-tier）
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required runtime::tests::persistence::rollback_with_reconciliation_recovers_from_detected_tick_consensus_drift -- --nocapture
```

## 4. 通过标准
- 演练命令返回 `rc=0`。
- 漂移被成功定位到具体 `mismatch_tick`。
- 回滚后 `first_tick_consensus_drift() == None`。
- 回滚后 `verify_tick_consensus_chain()` 通过。
- 缺少 Viewer `approval` 的请求返回 `rollback_approval_required`，且 world、journal、batch 与 reorg epoch 均不变。
- 畸形、篡改、过期、目标不匹配、未知/停用 authority 或重放信封均在 mutation 前拒绝；Viewer 对 runtime 授权拒绝稳定映射为 `rollback_authorization_invalid`。
- 事件链存在 `RollbackApplied` 记录，且 ticket、两个独立 authority id 与 nonce 和已验证信封完全一致。

## 5. 失败处置
- 若回滚后仍有漂移：立即阻断发布，保留快照/日志，升级到治理应急流程。
- 若漂移定位失败：先执行一次完整快照恢复重放，再人工比对 `tick_consensus_records` 链路。
