# oasis7 hosted world 玩家访问与会话鉴权（Hosted Operator Runbook）

- 对应需求文档: `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`
- 对应设计文档: `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.project.md`

审计轮次: 1

## Meta
- Owner Role: `liveops_community`
- Review Role: `producer_system_designer`
- Scope: `hosted_public_join share discipline + operator/public URL boundary + incident first response + public claims`
- Audience: `hosted world host / operator`
- Source Docs:
  - `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`
  - `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.project.md`
  - `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`

## 1. 适用范围
- 这份 runbook 只覆盖 `deployment_mode=hosted_public_join` 的 hosted world 分享与事故收口。
- 它定义的是 operator 执行方法，不替代 runtime/viewer 的实现细节，也不替代正式账户系统、钱包插件或 invite-only 方案。
- 当前 hosted world 的对外口径仍然是：
  - `limited playable technical preview`
  - `crypto-hardened preview`
  - `hosted-world player access verdict = specified_not_implemented`

## 2. 先认清三类入口
- `public join URL`
  - 给玩家使用。
  - 典型形态是 viewer/game URL，包含公开 `ws` 与 `hosted_access` hint。
- `private control plane`
  - 给 operator 自己使用。
  - 典型是 `oasis7_web_launcher` 的 console / control origin。
  - 当前目标态是 loopback-only；不应该被公网玩家直接访问。
- `signer / approval path`
  - 给敏感动作二次授权使用。
  - 当前只有 preview-grade backend reauth，不是 production custody。

一句话规则：
- 玩家只该拿到 `public join URL`。
- 不要把 operator/control URL 当成玩家分享链接。

## 3. 分享前检查
每次准备把 hosted world 发给别人前，先做下面 6 项：

1. 确认 `deployment_mode` 是 `hosted_public_join`。
2. 确认你准备发出去的是 game/viewer URL，而不是 launcher console 地址。
3. 确认公开页面不会再注入长期 signer bootstrap。
4. 确认 public snapshot 仍显示：
   - `verdict = specified_not_implemented`
   - `main_token_transfer = blocked_until_strong_auth`
5. 确认你没有对外宣称：
   - `hosted-ready`
   - `production-ready`
   - `safe to share with anyone`
6. 如果你走了反向代理或 tunnel，确认公网只暴露玩家 join 面，不暴露 operator/control 面。

## 4. 正确分享方法
- 对外只分享 game/viewer join URL。
- 推荐同时补一句说明：
  - `这是一个 limited playable technical preview。`
  - `如果你能打开页面并进入世界，说明你拿到的是玩家入口，不是 operator 管理入口。`
- 推荐直接复用公告模板：
  - `doc/testing/templates/hosted-world-share-announcement-template.md`
- 不要分享：
  - launcher console URL
  - 任意 `/api/state`
  - 任意 `/api/start`、`/api/stop`
  - 任意 `/api/chain/start`、`/api/chain/stop`
  - 任意 `/api/gui-agent/*`

## 5. 远程 Operator Tunnel / Reverse Proxy 最低策略
如果 operator 不是在本机 loopback 上操作，而是通过远程 tunnel / reverse proxy / 云主机对外提供 hosted world，至少满足下面 6 条：

1. 公网玩家入口与 operator/control 入口必须分离。
   - 最低要求是不同 origin 或不同仅内网可达的 bind。
2. 面向公网的代理只允许转发：
   - viewer/game 静态页
   - `public player plane`
   - WebSocket 玩家连接
3. 下面这些路径必须继续停留在 loopback、VPN 或人工 tunnel 内，不得直接暴露公网：
   - `/api/state`
   - `/api/start`、`/api/stop`
   - `/api/chain/start`、`/api/chain/stop`
   - `/api/gui-agent/*`
   - console/operator 静态页
4. 如果边缘层做白名单路由，默认拒绝未知路径，不要用“整站转发再靠应用层兜底”的方式上线。
5. 如果必须远程操作，优先用 SSH tunnel、Tailscale/WireGuard、堡垒机或等价内网链路，只把 operator 面开放给受控操作者。
6. 如果你无法确认代理是否只暴露了玩家 join 面，就不要对外分享该世界。

上线前最小自查：
1. 从公网玩家视角访问分享链接，只能看到 game/viewer 页面。
2. 从公网玩家视角访问上述 operator 路径，应统一失败或返回 `operator_plane_only`，而不是进入控制台。
3. 对外公告、群消息、文档里只出现玩家 join URL，不出现 operator 地址。

## 5A. 邮箱登录 + Tablestore MVP 最小运行法
当 `hosted_public_join` 已启用中心化 hosted account 邮箱登录时，operator 至少满足下面 7 条：

1. `OASIS7_HOSTED_ACCOUNT_STORE_BACKEND` 推荐显式设为 `tablestore` 或 `auto`。
2. `OASIS7_HOSTED_ACCOUNT_TABLESTORE_ENDPOINT` 必须指向当前机器真实可达的 endpoint。
   - 若走 VPC，先确认 `https://<instance>.<region>.vpc.tablestore.aliyuncs.com:443` 能从部署机连通。
   - 不能把“能解析 DNS”误当成“能访问实例”。
3. 若未显式指定 `OASIS7_HOSTED_ACCOUNT_TABLESTORE_TABLE`，当前默认表名是 `oasis7_hosted_account_identity`。
4. 首次启动若表还不存在，`OASIS7_HOSTED_ACCOUNT_TABLESTORE_AUTO_CREATE=true` 时允许先出现一次 `OTSObjectNotExist`，随后自动建表并继续启动。
5. 面向真实玩家时，`OASIS7_HOSTED_LOGIN_DELIVERY_MODE` 应切到 `smtp`。
   - `preview_inline` 只适合 smoke / staging，不适合公开玩家入口。
6. SMTP 与 Tablestore 要分开看待：
   - SMTP 负责把 OTP 送出去。
   - Tablestore 负责把 `hosted_account_id -> player_id` 稳定落盘。
7. 不要在对外公告里把这条链路称为 `production custody` 或“稳定大规模可用”。

MVP 最小 smoke：
1. 启动 `oasis7_game_launcher --deployment-mode hosted_public_join`。
2. 对同一邮箱调用一次 `/api/public/hosted-account/login/start` 和 `/api/public/hosted-account/login/complete`。
3. 记录返回的 `hosted_account_id` 与 `player_id`。
4. 重启 launcher。
5. 对同一邮箱再次完成一次登录。
6. 两次若返回同一个 `hosted_account_id` 与 `player_id`，即可判定“邮箱登录 + 账户持久化”主链路成立。

当前已实测通过的 MVP 证据：
1. 2026-05-20 已在 ECS 上验证 `https://oasis7.cn-huhehaote.vpc.tablestore.aliyuncs.com` 可达。
2. 同一邮箱 `cc@ncuhome.tech` 在 launcher 重启前后两次登录，返回同一个 `hosted_account_id=oasis-account-00000001` 与 `player_id=hosted-player-account-00000001`。

常见失败签名：
1. `OTSAuthFailed: Request denied by instance ACL policies`
   - 说明实例 ACL / instance policy 仍未放通。
2. 访问 `*.vpc.tablestore.aliyuncs.com:443` 出现 `TCP timeout`
   - 说明部署机不在可达该 VPC endpoint 的网络里。
3. `hosted account tablestore table <...> does not exist and auto create is disabled`
   - 说明要么先建表，要么打开 `AUTO_CREATE`。

## 6. 如何判断自己分享错了
下面任一条成立，都按“误分享 operator URL / operator 面暴露”处理：

- 访客反馈自己打开的是控制台、管理面或非游戏页面。
- 访客能直接命中 `/api/state`、`/api/start|stop`、`/api/chain/start|stop`、`/api/gui-agent/*`。
- 你发现自己发出去的是 launcher/control origin，而不是 game/viewer URL。
- 反向代理或 tunnel 把 operator/control 面一起暴露到了公网。

## 7. 误分享后的第一响应
按顺序执行，不要跳步：

1. 立即停止继续传播错误链接。
2. 撤回或替换所有公开帖子、群消息、文档中的错误 URL。
3. 暂停对外新增玩家流量，直到确认公网只剩 join URL。
4. 重新自查：
   - 远端访问 private-control-plane 应返回 `operator_plane_only`
   - 公开返回体只能带 public snapshot，不应带 operator state / logs / config
5. 如果无法确认是否有人已经命中过私有面：
   - 先按 incident 处理
   - 暂停 public claims
   - 重新开一个干净的分享窗口再恢复

## 8. 最小 Incident 记录
每次误分享或疑似暴露，至少记录以下字段：

- `incident_id`
- `discovered_at`
- `who_found_it`
- `shared_url`
- `intended_join_url`
- `exposed_surface`
- `publicly_visible_duration`
- `immediate_actions`
- `claims_frozen`
- `follow_up_owner`

模板入口：
- `doc/testing/templates/hosted-world-operator-incident-template.md`

推荐把结论同步回：
- `doc/devlog/YYYY-MM-DD.md`
- 当前 topic 的 `project.md`

## 9. 何时必须 Freeze 对外口径
出现下面任一情况，立刻冻结对外升级口径，只保留 preview 表述：

- operator/control 面被公网直接访问
- 浏览器或公开 API 返回里出现长期 signer 真值
- 玩家入口与 operator 入口已经无法稳定区分
- hosted strong-auth 或 session 边界出现未解释的穿透

冻结后对外只允许说：
- `当前仍是 limited playable technical preview。`
- `hosted access hardening is in progress。`
- `operator boundary issue is being corrected before wider sharing。`

## 10. 当前已知边界
- 当前不支持 invite-only 作为基础安全方案。
- 当前 `main_token_transfer` 仍不能通过 hosted public join 放行。
- 当前 hosted `prompt_control` 只是 preview-grade backend reauth，不是 production custody。
- 当前 operator 仍以 loopback private control plane 为主；即使走远程 tunnel，也只能把受控 operator 面留在私网或人工链路内。
- 当前邮箱登录 + Tablestore 只证明了 hosted identity MVP 主链路已通，不等于 SMTP、并发、freeze/recovery、监控告警和大规模运维都已完成。

## 11. 当前推荐执行法
- 小范围分享时：
  - 只发 join URL
  - 不发 operator/control URL
  - 不升级 public claims
- 如果要公开到更大范围：
  - 先完成 QA first slices
  - 再补 operator runbook 演练记录
  - 再由 `producer_system_designer` 决定是否扩大分享范围

## 12. Session Revoke 实操步骤
适用场景：
- 公开玩家被确认需要踢出
- 浏览器侧 session 疑似泄露或异常复用
- hosted handoff 前需要显式回收旧 session
- 误分享 operator URL 后，无法确认旧浏览器 session 是否仍应继续存活

执行前先确认 4 项：
1. 你要撤销的是 `public player plane` 的浏览器 session，不是 operator 自己的 private control plane。
2. 你已经拿到目标 `player_id`，必要时也拿到 `session_pubkey`。
3. 你已经准备好可审计的 `revoke_reason`，不要只写 `test`、`kick` 这类模糊词。
4. 你知道当前 runtime live 地址，例如 `127.0.0.1:<live_bind_port>`。

建议的数据来源：
- 浏览器 QA/事故证据中的 `__AW_TEST__.getState()`：
  - `authPlayerId`
  - `authPublicKey`
- runtime / operator 记录中的当前绑定关系
- incident 模板中的受影响玩家条目

推荐命令：
```bash
env -u RUSTC_WRAPPER cargo run -q -p oasis7 --bin oasis7_pure_api_client -- \
  --addr 127.0.0.1:<live_bind_port> \
  --client <operator_id> \
  revoke-session \
  --player-id <player_id> \
  --session-pubkey <session_pubkey> \
  --revoke-reason <reason>
```

字段要求：
- `--client`:
  - 使用可审计的 operator 标识，例如 `hosted-revoke-operator`、`ops-oncall-a`
- `--revoke-reason`:
  - 使用面向 incident 可复盘的原因，例如 `operator_kick_for_abuse_drill`
  - 避免含糊表述，如 `kick`、`cleanup`

预期返回：
- `authoritative_recovery_ack.status = session_revoked`
- `revoke_reason = <reason>`
- `revoked_by = <operator_id>`
- `session_epoch` 递增

执行后检查：
1. 若目标页面仍在线，等待下一轮 hosted heartbeat。
2. 玩家面应收敛到：
   - `authTier = guest_session`
   - `authRevokeReason = <reason>`
   - `authRevokedBy = <operator_id>`
   - `hostedRecoveryHint.kind = revoked`
3. 如需留证，优先记录：
   - operator 命令输出
   - 浏览器 `__AW_TEST__.getState()`
   - `Hosted Recovery` 页面截图

执行后必须回写：
- `doc/testing/templates/hosted-world-operator-incident-template.md` 对应 incident
- 当日 `doc/devlog/YYYY-MM-DD.md`
- 若已对外沟通，再补 `correction_message_ref`

## 13. 对外分享公告模板
模板入口：
- `doc/testing/templates/hosted-world-share-announcement-template.md`

使用时机：
- 第一次把 hosted world 发给玩家
- 需要重复提醒“哪个才是正确 join URL”
- 需要把 preview claims、重入提示与 join discipline 一起说清楚

使用规则：
1. 对外只发 `public join URL`，不要附带 operator/control 地址。
2. 文案里必须保留 `limited playable technical preview` 口径。
3. 不要写 `hosted-ready`、`production-ready`、`invite-only secure` 一类升级承诺。
4. 若玩家可能命中过旧页面，可在公告里提示“若页面提示 Hosted Recovery / Re-acquire Hosted Player Session，请按页面提示重新获取会话”。

## 14. 对外更正模板
模板入口：
- `doc/testing/templates/hosted-world-share-correction-template.md`

使用时机：
- 分享了错误 URL，需要公开更正
- 玩家已经接触到错误口径，需要统一说法
- revoke/incident 后需要恢复正确 join URL

使用规则：
1. 先替换错误 URL，再发更正文案。
2. 只发 `public join URL`，不要附带 operator/control 地址。
3. 继续使用 preview 口径，不升级任何对外承诺。
4. 若当前还在排查，不给出“已完全修复/生产可用”之类表述。
