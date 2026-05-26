# oasis7 hosted world 玩家访问与会话鉴权（设计文档）

- 对应需求文档: `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.project.md`

审计轮次: 1
## 设计目标
- 让“某个玩家部署服务，另一个玩家加载网页并游玩”成为一个可设计、可实施、可验收的 hosted-world 体系，而不是继续依赖 preview-only bootstrap。
- 明确当前实现里哪些边界不成立，并给出 runtime/viewer/QA/LiveOps 可直接分工的收口路径。

## 当前真值与问题
| 维度 | 当前状态 | 设计结论 |
| --- | --- | --- |
| Web control plane | `crates/oasis7/src/bin/oasis7_web_launcher.rs` 同时暴露 `/api/state`、`/api/gui-agent/action`、`/api/start`、`/api/stop`、`/api/chain/start`、`/api/chain/stop` 等接口 | public player access 与 private operator control 尚未拆分 |
| Browser signer bootstrap | `crates/oasis7/src/bin/oasis7_web_launcher/viewer_auth_bootstrap.rs` 会把 `OASIS7_VIEWER_AUTH_PRIVATE_KEY` 或 `config.toml` 里的 `node.private_key` 注入 HTML | hosted world 公网分享时不可接受 |
| Native launcher 同构路径 | `crates/oasis7/src/bin/oasis7_game_launcher.rs` 也保留 viewer auth bootstrap 常量与配置回退 | 当前“网页登录”仍建立在 preview signer 假设上 |
| Runtime key truth | `crates/oasis7/src/bin/oasis7_chain_runtime/node_keypair_config.rs` 会自动生成并写回 `config.toml` | node signer 仍偏向单机 bootstrap，而不是 hosted-world externalized truth |
| Viewer 敏感动作 | `crates/oasis7_viewer/src/egui_right_panel_chat_auth.rs`、`crates/oasis7_viewer/src/viewer_automation.rs`、`crates/oasis7_client_launcher/src/transfer_auth.rs` 都依赖浏览器或本地读取 signer | `agent chat`、`prompt control`、`main token transfer` 仍未按 hosted-world session/capability 解耦 |

## 目标模型
### 1. Identity Split
| 身份 | 说明 | 是否允许出现在浏览器 |
| --- | --- | --- |
| `host operator identity` | 启停世界、改配置、处理事故的管理员身份 | 否 |
| `node signer identity` | world/runtime/consensus 节点 signer | 否 |
| `governance / treasury signer identity` | 资产与治理 signer | 否 |
| `player identity` | 玩家或访客的会话身份 | 是，但只以 session/capability 形式出现 |

### 2. Plane Split
| Plane | 承载内容 | 入口要求 |
| --- | --- | --- |
| `public player plane` | 静态网页、公开只读状态、join/session issue、玩家输入 WebSocket | 可公开暴露 |
| `private control plane` | 启停 world、管理 GUI action、配置变更、应急操作 | 私有 bind、私有 origin 或受控隧道 |
| `signer plane` | 强鉴权 proof、签名验签、delegated auth backend | 只允许受信执行面访问 |

### 3. Session Ladder
| 档位 | 典型动作 | 不允许的动作 |
| --- | --- | --- |
| `guest session` | 打开世界、观战、读基础状态 | 写入玩家输入、资产动作、prompt/control |
| `player session` | 移动、普通玩法输入、低风险聊天 | operator 控制、资产转账、治理动作 |
| `strong auth` | 主链 token 转账、敏感资产/治理动作、特定 creator/admin 能力 | 默认长期驻留浏览器 |

## Hosted World 最小组件
1. Public join gateway
   - 返回 `world_id/public_ws/public_http/session_issuer_ref`。
   - 不返回长期 signer、管理地址、私有 bind。
2. Session issuer
   - 负责签发 `guest session`、升级到 `player session`、处理 revoke/expiry。
   - 可以是 runtime 内模块，也可以是独立 auth service，但必须与 signer plane 解耦。
3. Runtime capability validator
   - 负责把 session/capability 映射到 `player_id/entity_id/action_class`。
   - 必须独立于前端按钮显隐。
4. Private control gateway
   - 承载 `/api/start`、`/api/stop`、`/api/chain/start`、`/api/chain/stop`、operator-only `gui-agent/action`。
5. Strong-auth adapter
   - 为 token transfer 和更高风险动作提供 challenge/proof/verification。
   - 可后接钱包、delegated signer 或托管签名器，但都不应把长期私钥下发到浏览器。

## Endpoint 分层建议
| 当前接口 | 建议归属 | 说明 |
| --- | --- | --- |
| `/api/start` `/api/stop` | `private control plane` | hosted world 公开 join 场景不应直连 |
| `/api/chain/start` `/api/chain/stop` | `private control plane` | 节点生命周期属于 operator 权限 |
| `/api/gui-agent/action` | `legacy private control plane`，目标拆成 `/api/player/agent-action` + `/api/operator/gui-agent/action` | hosted-ready 前不允许继续作为 shared public surface |
| `/api/state` | 拆成 `public player snapshot` 与 `private operator state` | 玩家可见状态与管理状态需要分离 |
| 玩家 WebSocket / world 输入 | `public player plane` | 必须由 session/capability 驱动 |
| `/api/chain/transfer` | `public player plane` + `strong auth` | 不再接受浏览器长期私钥 bootstrap |

## Join Admission Control
| 字段 | 作用 | 最小规则 |
| --- | --- | --- |
| `max_guest_sessions` | 限制公开 guest 观察者数量 | 达上限时新 guest 直接返回 `world_full` 或 `guest_limit_reached` |
| `max_player_sessions` | 限制可玩的 player session 数量 | player slot 满后不得继续签发可写 session |
| `issue_rate_limit` | 限制单位时间 session 发放速率 | 超限时返回 `rate_limited` 并记录审计 |
| `world_full_policy` | 满员时的公开行为 | 必须冻结为 `reject/queue/observe_only` 之一 |
| `kick_policy` | host/operator 如何回收占位 | 必须可审计、可回放，且不允许静默踢出后继续保留旧 capability |

## Web 行为能力矩阵
| 行为 | guest | player session | strong auth | operator private plane |
| --- | --- | --- | --- | --- |
| 观战 / 读取世界 | 可 | 可 | 可 | 可 |
| 普通移动 / 玩法输入 | 否 | 可 | 可 | 可 |
| Agent Chat | 否或只读 | 可 | 可 | 可 |
| Prompt Control Preview | 否 | 默认否 | 可选 | 可 |
| Prompt Control Apply / Rollback | 否 | 否 | 仅 creator/admin 且建议走强鉴权 | 可 |
| Main Token Transfer | 否 | 否 | 可 | 可 |
| world 启停 / 配置 | 否 | 否 | 否 | 可 |

## 迁移顺序
1. TASK-P2P-041-A
   - 先把 public/private plane taxonomy 和接口边界冻结。
   - public join 场景中去掉 browser private-key bootstrap。
2. TASK-P2P-041-B
   - viewer 落 `guest session -> player session` UX。
   - 没有 session 或能力不足时，按钮明确禁用并显示原因。
3. TASK-P2P-041-C
   - runtime 落 session 校验、entity bind、resume/revoke。
   - `agent_engineer` 明确玩家实体 ownership 与并发抢占规则。
4. TASK-P2P-041-D
   - 资产与敏感动作接 strong auth。
   - 不再接受 host node key 作为 web 玩家动作默认 signer。
5. TASK-P2P-041-E/F
   - QA 落 abuse suite。
   - LiveOps 落 hosted operator runbook、分享规范和 claims gate。

## 当前阶段口径
- 当前允许：
  - `limited playable technical preview`
  - `crypto-hardened preview`
  - `hosted-world player access model exposes preview/blocking verdicts and is not hosted_ready`
- 当前禁止：
  - `public hosted web multiplayer is already safe by default`
  - `join URL can be shared publicly without additional architecture work`
  - `browser-based player access already matches production-grade custody`
