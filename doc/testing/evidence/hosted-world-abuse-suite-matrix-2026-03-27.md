# Hosted World Abuse Suite Matrix (2026-03-27)

审计轮次: 1

## Meta
- Owner Role: `qa_engineer`
- Review Role: `producer_system_designer`
- Scope: `TASK-P2P-041-E abuse suite closure for replay / expiry / revocation / operator-public URL confusion / admission limit / capability bypass`
- Topic authority: `doc/p2p/prd.md`; operator procedure: `doc/p2p/blockchain/hosted-player-access-operator-runbook.md`; historical `TASK-P2P-041` provenance remains in `GitHub Issue / GitHub Project` and Git history.

## 目标
- 把 hosted world 玩家访问与会话鉴权专题中 `TASK-P2P-041-E` 的 six-case abuse coverage 收敛成一份可引用矩阵。
- 明确本专题的 `required/full` 真值：
  - `required`: 以当前 topic 的定向 Rust 测试为真值，冻结错误码、绑定规则和 admission contract。
  - `full`: 对玩家可见面补 browser/live evidence；对于纯 runtime-side failure signature，则以当前 topic 的定向 runtime/bin 测试重跑记录作为 full-tier closure。

## Evidence Matrix
| 类别 | Required Evidence | Full Evidence | 当前结论 |
| --- | --- | --- | --- |
| replay | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib runtime_prompt_control_hosted_public_join -- --nocapture`，其中 `runtime_prompt_control_hosted_public_join_rejects_replayed_auth_nonce_even_with_valid_grant = ok` | 同一轮命令已同时覆盖 hosted prompt-control runtime-live 栈；另有 `runtime_agent_chat_replay_returns_idempotent_ack` 常驻保护同类 replay 语义 | `pass`。grant 存在时仍不能绕过 nonce 防重放。 |
| expiry | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib runtime_prompt_control_hosted_public_join -- --nocapture`，其中 `runtime_prompt_control_hosted_public_join_rejects_expired_backend_grant = ok`；`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher hosted_player_session_ -- --nocapture`，其中 `hosted_player_session_pending_registration_slots_expire_before_full_lease_ttl = ok` | `doc/testing/evidence/hosted-world-browser-auth-surface-2026-03-26.md` 已固化 `pending_registration_ttl_ms`、旧 `release_token` 失效和 guest recovery 提示 | `pass`。过期 grant 与过期 pending slot 都会被结构化拒绝。 |
| revocation | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib runtime_authoritative_recovery_rotate_and_revoke_session_enforced_for_agent_chat -- --nocapture` | `doc/testing/evidence/hosted-world-browser-revoke-recovery-2026-03-27.md` 已固化 remote revoke 后页面掉回 `guest_session`，并显示 `authRevokeReason/authRevokedBy` | `pass`。revoke 会同时命中 runtime 校验、reconnect recovery 和玩家可见恢复面板。 |
| operator/public URL confusion | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher hosted_mode_rejects_remote_private_control_plane_matrix -- --nocapture` | `doc/p2p/blockchain/hosted-player-access-operator-runbook.md` + `doc/testing/templates/hosted-world-operator-incident-template.md` + `doc/testing/templates/hosted-world-share-correction-template.md` + `doc/testing/templates/hosted-world-share-announcement-template.md` | `pass`。远端误命中私有面会统一返回 `operator_plane_only`，LiveOps 侧也有冻结和更正模板。 |
| admission limit | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher hosted_player_session_ -- --nocapture`，其中 `hosted_player_session_issue_enforces_max_player_sessions = ok`、`hosted_player_session_issue_counts_runtime_only_occupancy_toward_world_full = ok`、`hosted_player_session_runtime_reconcile_releases_seen_players_missing_from_runtime = ok` | `doc/testing/evidence/hosted-world-browser-auth-surface-2026-03-26.md` + `doc/testing/evidence/hosted-world-browser-concurrency-2026-03-27.md` 已固化 `activeSlots/runtimeBound/runtimeOnly/runtimeProbe` 和双页接入 | `pass`。`world_full` 已按 issuer active slot 与 runtime occupancy 有效并集计算。 |
| capability bypass | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher hosted_strong_auth_ -- --nocapture`，其中 `hosted_strong_auth_grant_rejects_main_token_transfer_until_lane_lands = ok`；`env -u RUSTC_WRAPPER cargo test -p oasis7 --lib runtime_prompt_control_hosted_public_join -- --nocapture` | `doc/testing/evidence/hosted-world-browser-auth-surface-2026-03-26.md` + `doc/testing/evidence/hosted-world-browser-strong-auth-success-2026-03-27.md` 已固化 `prompt_control_*` 可升到 preview grant，而 `main_token_transfer` 继续 `blocked_until_strong_auth` | `pass`。prompt-control preview reauth 不会顺带打开资产动作。 |

## 本轮执行命令
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib runtime_prompt_control_hosted_public_join -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib runtime_agent_chat_requires_explicit_session_registration -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib runtime_session_register_ -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib runtime_authoritative_recovery_rotate_and_revoke_session_enforced_for_agent_chat -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher hosted_player_session_ -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher hosted_prompt_control_strong_auth_grant_ -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher hosted_strong_auth_ -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher hosted_mode_rejects_remote_private_control_plane_matrix -- --nocapture
```

## 结论
- `TASK-P2P-041-E` 在当前 PRD 范围内可判定为 `pass`。
- 本专题剩余未做项属于后续增强，而不是本轮 abuse suite 的 blocker：
  - 更完整的 production custody / wallet 插件 / externalized signer 真值
  - hosted handoff / batch kick 的产品化操作流
  - 更细粒度的 hosted action matrix 与未来更高等级 strong-auth 证明
