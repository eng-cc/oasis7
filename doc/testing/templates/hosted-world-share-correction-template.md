# Unified World Hosted Access Correction Template

审计轮次: 1

## Meta
- `world_id`:
- `deployment_mode`: `hosted_public_join`
- `incident_id`:
- `operator_owner`:
- `channel`: `group` / `social` / `dm` / `other`
- `wrong_url`:
- `correct_join_url`:

## 使用规则
- 先撤回/替换错误 URL，再发更正文案。
- 更正文案里只能出现 `correct_join_url`，不能再次带出 operator/control URL。
- 对外口径只允许使用 preview 表述，不允许升级到 hosted access readiness / `production-ready` 承诺。
- 若问题仍在排查，用“正在纠正/正在收口”，不要写“已完全修复”。

## Short Correction
适合群消息、评论区、公告顶楼快速更正。

```text
链接更正：刚才发出的不是玩家入口，请不要继续使用。

当前正确的游戏入口是：
<correct_join_url>

统一持久大世界的 hosted access 当前仍处于 limited playable technical preview。如你此前打开的是错误链接，请关闭该页面，并改用上面的玩家入口重新进入。
```

## Full Correction
适合正式公告、社区帖、群公告更新。

```text
更正说明：

我们刚才分享的链接不是面向玩家的正确入口，现已停止继续传播。请以以下地址作为唯一有效的玩家进入方式：

<correct_join_url>

如果你已经打开过之前的错误链接，请直接关闭原页面，并重新通过上述玩家入口进入。当前统一持久大世界的 hosted access 仍处于 limited playable technical preview，我们正在继续收口 access hardening，不会因此升级任何对外承诺。
```

## Revoke Follow-up
适合已经执行过 `revoke-session`，需要提醒玩家重新获取 hosted player session 的情况。

```text
访问更正：

此前会话已失效，请不要继续使用原页面。请重新通过以下玩家入口进入：

<correct_join_url>

如果页面提示 Hosted Recovery / Re-acquire Hosted Player Session，请按页面提示重新获取会话。这是一次受控纠正动作，不代表统一持久大世界的 hosted access 已进入 production-ready 状态。
```

## Internal Record
- `posted_at`:
- `posted_by`:
- `message_variant`: `short` / `full` / `revoke_follow_up`
- `claim_boundary_checked`: `yes` / `no`
- `incident_template_updated`: `yes` / `no`
